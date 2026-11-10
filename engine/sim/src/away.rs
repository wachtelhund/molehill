//! Ropes, rope ladders and lifts the window has left.
//!
//! They aren't cells, so a tile blob (`band.rs`) doesn't carry them. Each waits here instead, as it was in
//! the window it left together with that window's origin, and is put back when the window covers the whole
//! of it again: a ladder hung in the home shaft is still there after a trip 100 m down, or after the mole
//! dies deep and wakes in the burrow. (They used to be lost with the rows they hung in, which could leave
//! a mole that can't climb rock under a ledge it had laddered.) Saved with the transport.
use crate::band::{Rd, Wr, R};
use crate::lifts::{Ladder, Lift};
use crate::ropes::Rope;
use crate::world::World;

/// A thing as it was in the window whose cell (0, 0) was world cell (`ox`, `oy`).
#[derive(Clone, Debug)]
pub struct Away<T> { pub ox: i64, pub oy: i64, pub it: T }
#[derive(Clone, Debug, Default)]
pub struct AwayAll { pub ropes: Vec<Away<Rope>>, pub ladders: Vec<Away<Ladder>>, pub lifts: Vec<Away<Lift>> }
impl AwayAll { pub fn len(&self) -> usize { self.ropes.len() + self.ladders.len() + self.lifts.len() } pub fn is_empty(&self) -> bool { self.len() == 0 } }

/// Cell `v` after the window moved `d` cells along its axis of length `n`, if it stays `lo` cells clear of
/// the near edge and `hi` of the far one.
fn at(v: usize, d: i64, n: usize, lo: i64, hi: i64) -> Option<usize> { let u = v as i64 - d; (u >= lo && u + hi < n as i64).then_some(u as usize) }

impl Rope {
    /// This rope after the window moved `cols` east and `rows` down, if all of it is still inside.
    pub fn slid(&self, cols: i64, rows: i64, w: usize, h: usize) -> Option<Rope> {
        let (dx, dy) = (cols as f32, rows as f32);
        let (ax, ay) = (at(self.ax, cols, w, 1, 1)?, at(self.ay, rows, h, 1, 0)?);
        if self.pts.iter().any(|p| p.0 - dx < 1. || p.0 - dx >= w as f32 - 1. || p.1 - dy < 1. || p.1 - dy > h as f32 - 2.) { return None; }
        let mv = |v: &[(f32, f32)]| v.iter().map(|p| (p.0 - dx, p.1 - dy)).collect();
        Some(Rope { pts: mv(&self.pts), prev: mv(&self.prev), ax, ay })
    }
}
impl Ladder {
    /// This ladder after the window moved `cols` east and `rows` down, if all of it is still inside.
    pub fn slid(&self, cols: i64, rows: i64, w: usize, h: usize) -> Option<Ladder> {
        let top = at(self.top, rows, h, 1, 0)?; if top + self.len >= h { return None; }
        Some(Ladder { x: at(self.x, cols, w, 1, 1)?, top, len: self.len, ax: at(self.ax, cols, w, 0, 0)?, ay: at(self.ay, rows, h, 0, 0)? })
    }
}
impl Lift {
    /// This lift after the window moved `cols` east and `rows` down, if its whole shaft is still inside. Its
    /// stops and the stop it is hauled to are rows too, and move with it.
    pub fn slid(&self, cols: i64, rows: i64, w: usize, h: usize) -> Option<Lift> {
        let (x, head, bottom) = (at(self.x, cols, w, 4, 4)?, at(self.head, rows, h, 1, 0)?, at(self.bottom, rows, h, 0, 1)?);
        let row = |r: usize| (r as i64 - rows).max(0) as usize; let dy = rows as f32;
        Some(Lift { x, head, bottom, y: self.y - dy, py: self.py - dy, stops: self.stops.iter().map(|&r| row(r)).collect(), target: self.target.map(row), ..self.clone() })
    }
}

impl World {
    /// A rope, ladder or lift that didn't stay in the window on this slide of (`cols`, `rows`): it waits in
    /// `tr.away` with the origin the window had before the slide. (Call after `ox` / `oy` moved.)
    pub(crate) fn origin_before(&self, cols: i64, rows: i64) -> (i64, i64) { (self.ox - cols, self.oy - rows) }
    /// Put back what the window covers again (after a slide, once the entering tiles are in: a lift reads
    /// its stops off the shaft). A lift comes back braked where its cage stood.
    pub(crate) fn return_away(&mut self) {
        if self.tr.away.is_empty() { return; }
        let (w, h, ox, oy) = (self.g.w, self.g.h, self.ox, self.oy);
        let mut a = std::mem::take(&mut self.tr.away);
        a.ropes.retain(|p| match p.it.slid(ox - p.ox, oy - p.oy, w, h) { Some(r) => { self.tr.ropes.push(r); false } None => true });
        a.ladders.retain(|p| match p.it.slid(ox - p.ox, oy - p.oy, w, h) { Some(l) => { self.tr.ladders.push(l); false } None => true });
        a.lifts.retain(|p| match p.it.slid(ox - p.ox, oy - p.oy, w, h) {
            Some(mut l) => { l.stops = self.lift_stops(l.x, l.w, l.head, l.bottom); l.target = None; l.v = 0.; l.py = l.y; self.tr.lifts.push(l); false }
            None => true });
        self.tr.away = a;
    }

    pub(crate) fn encode_away(&self, o: &mut Wr) {
        let a = &self.tr.away; let at = |o: &mut Wr, ox: i64, oy: i64| { o.u64(ox as u64); o.u64(oy as u64); };
        o.u32(a.ropes.len() as u32); for p in &a.ropes { at(o, p.ox, p.oy); crate::ropes::write_rope(o, &p.it); }
        o.u32(a.ladders.len() as u32); for p in &a.ladders { at(o, p.ox, p.oy); crate::lifts::write_ladder(o, &p.it); }
        o.u32(a.lifts.len() as u32); for p in &a.lifts { at(o, p.ox, p.oy); crate::lifts::write_lift(o, &p.it); o.f32(p.it.w); }
    }
    pub(crate) fn decode_away(&mut self, r: &mut Rd) -> R<()> {
        let (w, h) = (self.g.w, self.g.h); let at = |r: &mut Rd| -> R<(i64, i64)> { Ok((r.u64()? as i64, r.u64()? as i64)) };
        for _ in 0..r.len(4096)? { let (ox, oy) = at(r)?; if let Some(it) = crate::ropes::read_rope(r, w, h)? { self.tr.away.ropes.push(Away { ox, oy, it }); } }
        for _ in 0..r.len(4096)? { let (ox, oy) = at(r)?; if let Some(it) = crate::lifts::read_ladder(r, w, h)? { self.tr.away.ladders.push(Away { ox, oy, it }); } }
        for _ in 0..r.len(4096)? { let (ox, oy) = at(r)?; let l = crate::lifts::read_lift(r, w, h)?; let cw = r.f32()?;
            if let Some(mut it) = l { if cw >= 1. && cw <= crate::lifts::CAGE_MAX { it.w = cw; } self.tr.away.lifts.push(Away { ox, oy, it }); } }
        Ok(())
    }
}
