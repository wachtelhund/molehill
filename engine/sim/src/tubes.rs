//! Pressure tubes: a sealed pipe network that blows capsules along it with air (see transport.rs and
//! PROGRESSION.md).
//! * **Tube** is a built block (`Mat::Tube`, flag `TUBE`): iron-banded glass pipe, laid with the block brush
//!   and held up like any built block. Tube cells that touch edge to edge are one line.
//! * A **tube hopper** sits on a line: a 1 m mouth with a blower in it. Its blower runs while copperline in
//!   its block carries current, and a line is pressurised while any hopper on it is blowing.
//! * Loose items dropped or thrown into the mouth of a hopper whose blower runs are sealed in a capsule and
//!   blown along the pipe to that hopper's destination (another hopper on the line: the nearest at first;
//!   right mouse cycles through them), where they come out of its mouth. A hopper without current only
//!   receives. Right mouse on a blowing hopper with nothing in the paw seals the mole in a capsule instead.
//! * Capsules run at `TUBE_V`, uphill as easily as on the flat. A line must hold its pressure: with no
//!   hopper blowing, capsules wait where they are; if the pipe ahead is cut or caves in (a cell that's no
//!   longer standing tube), the capsule stops at the breach and spills there: its items fall out, the mole
//!   climbs out.
use crate::craft::Item;
use crate::mat::{CPM, S, SF, TUBE};
use crate::world::{Drop, Event, World};

/// A hopper's mouth: 1 m square (cells).
pub const HOPPER: usize = CPM;
/// Capsule speed along the pipe (cells/s): 12 m/s.
pub const TUBE_V: f32 = 12. * CPM as f32;
/// The longest line a capsule will be sent along (cells of pipe).
pub const TUBE_MAX: usize = 4096;

#[derive(Clone, Debug)]
pub struct Hopper {
    /// the mouth's top-left cell, and the tube cell it feeds
    pub x: usize, pub y: usize, pub tube: usize,
    /// where its capsules go (index into the hoppers), None until a second hopper is on the line
    pub dest: Option<usize>,
}
impl Hopper {
    pub fn contains(&self, x: f32, y: f32) -> bool { x >= self.x as f32 && x < (self.x + HOPPER) as f32 && y >= self.y as f32 && y < (self.y + HOPPER) as f32 }
    /// Does the mouth cover any of the box [x0, x1) × [y0, y1)?
    pub fn overlaps(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> bool { (self.x as f32) < x1 && x0 < (self.x + HOPPER) as f32 && (self.y as f32) < y1 && y0 < (self.y + HOPPER) as f32 }
    pub fn centre(&self) -> (f32, f32) { (self.x as f32 + HOPPER as f32 / 2., self.y as f32 + HOPPER as f32 / 2.) }
}
#[derive(Clone, Debug)]
pub struct Capsule {
    /// the pipe cells it runs along (cell indices), how far along it is (cells), and a tick ago
    pub path: Vec<u32>, pub at: f32, pub pat: f32,
    pub items: Vec<(Item, u32)>, pub rider: bool, pub to: usize,
}
impl Capsule {
    pub fn cell(&self) -> usize { self.path[(self.at as usize).min(self.path.len() - 1)] as usize }
}

impl World {
    pub fn is_tube(&self, i: usize) -> bool { i < self.g.mat.len() && self.g.m(i).has(TUBE) && !self.g.loose(i) }
    /// The pipe between two tube cells (shortest, through standing tube only), if they're on one line.
    fn tube_path(&self, a: usize, b: usize) -> Option<Vec<u32>> {
        if !self.is_tube(a) || !self.is_tube(b) { return None; }
        let w = self.g.w; let mut prev = std::collections::HashMap::new(); prev.insert(a, a);
        let mut q = std::collections::VecDeque::from([a]);
        while let Some(i) = q.pop_front() {
            if i == b { let mut p = vec![b as u32]; let mut c = b; while c != a { c = prev[&c]; p.push(c as u32); } p.reverse(); return Some(p); }
            if prev.len() > TUBE_MAX { return None; }
            let (x, y) = (i % w, i / w);
            for j in [if x > 0 { i - 1 } else { usize::MAX }, if x + 1 < w { i + 1 } else { usize::MAX }, if y > 0 { i - w } else { usize::MAX }, i + w] {
                if j < self.g.mat.len() && !prev.contains_key(&j) && self.is_tube(j) { prev.insert(j, i); q.push_back(j); }
            }
        }
        None
    }
    /// Set a hopper down with its mouth at (x, y): the 1 m around it must be open, and touch tube.
    pub(crate) fn place_hopper(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        // it sits on what's under the cursor (the floor, or the tube it feeds), its 1 m mouth above that
        let floor = (y..(y + HOPPER + S).min(self.g.h - 1)).find(|&r| !self.passable(x, r));
        let (x0, y0) = (x.saturating_sub(HOPPER / 2), floor.map_or(y.saturating_sub(HOPPER / 2), |r| r.saturating_sub(HOPPER)));
        if x0 + HOPPER + 1 >= self.g.w || y0 + HOPPER + 1 >= self.g.h || x0 == 0 || y0 == 0 { return Err("too near the edge"); }
        if !(y0..y0 + HOPPER).all(|r| (x0..x0 + HOPPER).all(|c| self.passable(c, r))) { return Err("needs 1 m of open space"); }
        if self.tr.hoppers.iter().any(|h| h.x.abs_diff(x0) < HOPPER && h.y.abs_diff(y0) < HOPPER) { return Err("there's a tube hopper there"); }
        // an ore hopper is a different machine: a mouth inside its bin or legs would hide one from the other
        if self.tr.bins.iter().any(|b| b.overlaps(x0 as f32, y0 as f32, (x0 + HOPPER) as f32, (y0 + HOPPER) as f32)) { return Err("there's an ore hopper there"); }
        let w = self.g.w;
        // the tube it feeds: a standing tube cell on the ring round the mouth (below it first)
        // (within a tile of the mouth)
        let ring = (1..=S).flat_map(|d| (x0..x0 + HOPPER).map(move |c| (y0 + HOPPER - 1 + d) * w + c).chain((y0..y0 + HOPPER).flat_map(move |r| [r * w + x0 - d, r * w + x0 + HOPPER - 1 + d])).chain((x0..x0 + HOPPER).map(move |c| (y0 - d) * w + c)).collect::<Vec<_>>());
        let tube = ring.into_iter().find(|&i| self.is_tube(i)).ok_or("needs to touch a tube")?;
        self.tr.hoppers.push(Hopper { x: x0, y: y0, tube, dest: None }); self.route_hoppers(); self.events.push(Event::Place); Ok(())
    }
    /// Give every hopper without a (reachable) destination the nearest other hopper on its line.
    fn route_hoppers(&mut self) {
        for k in 0..self.tr.hoppers.len() {
            let ok = self.tr.hoppers[k].dest.is_some_and(|d| d < self.tr.hoppers.len() && d != k && self.tube_path(self.tr.hoppers[k].tube, self.tr.hoppers[d].tube).is_some());
            if ok { continue; }
            let from = self.tr.hoppers[k].tube;
            let best = (0..self.tr.hoppers.len()).filter(|&d| d != k).filter_map(|d| self.tube_path(from, self.tr.hoppers[d].tube).map(|p| (p.len(), d))).min();
            self.tr.hoppers[k].dest = best.map(|b| b.1);
        }
    }
    pub(crate) fn hopper_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.hoppers.iter().position(|h| h.contains(xf, yf)) }
    /// Is the line a hopper feeds pressurised (any hopper on it blowing)?
    pub fn line_pressurised(&self, k: usize) -> bool {
        let t = self.tr.hoppers[k].tube;
        (0..self.tr.hoppers.len()).any(|j| self.hopper_blowing(j) && (j == k || self.tube_path(t, self.tr.hoppers[j].tube).is_some()))
    }
    pub(crate) fn hopper_blowing(&self, k: usize) -> bool { let h = &self.tr.hoppers[k]; let (cx, cy) = h.centre(); self.logic.net_lit(self.logic.block_of(cx as usize, cy as usize)) }

    /// Right mouse on a hopper: with nothing in the paw on a pressurised line, the mole is sealed in a capsule
    /// and sent; otherwise it cycles the hopper's destination to the next hopper on the line.
    pub(crate) fn use_hopper(&mut self, x: usize, y: usize) -> bool {
        let Some(k) = self.hopper_at(x, y) else { return false };
        self.events.push(Event::Click);
        if self.gear.held.is_none() && self.tr.in_tube.is_none() && self.tr.riding.is_none() {
            if !self.hopper_blowing(k) { self.say("This tube hopper's blower needs current to send.".into()); return true; }
            if self.send_capsule(k, vec![], true) { self.say("Sealed in a capsule...".into()); } else { self.say("This tube hopper has nowhere to send to.".into()); }
            return true;
        }
        // cycle the destination
        let from = self.tr.hoppers[k].tube; let n = self.tr.hoppers.len();
        let cur = self.tr.hoppers[k].dest.unwrap_or(k);
        if let Some(d) = (1..n).map(|s| (cur + s) % n).find(|&d| d != k && self.tube_path(from, self.tr.hoppers[d].tube).is_some()) {
            self.tr.hoppers[k].dest = Some(d); let (dx, dy) = self.tr.hoppers[d].centre();
            self.say(format!("Tube hopper sends to the one {:.0} m {}.", ((dx - x as f32).powi(2) + (dy - y as f32).powi(2)).sqrt() / CPM as f32, if dx < x as f32 { "to the left" } else { "to the right" }));
        }
        true
    }
    fn send_capsule(&mut self, k: usize, items: Vec<(Item, u32)>, rider: bool) -> bool {
        self.route_hoppers();
        let Some(d) = self.tr.hoppers[k].dest else { return false };
        let Some(path) = self.tube_path(self.tr.hoppers[k].tube, self.tr.hoppers[d].tube) else { return false };
        let n = self.tr.capsules.len();
        self.tr.capsules.push(Capsule { path, at: 0., pat: 0., items, rider, to: d });
        if rider { self.tr.in_tube = Some(n); }
        self.events.push(Event::Place); true
    }
    /// `E` on a hopper: back into the pack.
    pub(crate) fn pick_up_hopper(&mut self, x: usize, y: usize) -> Option<Item> {
        let k = self.hopper_at(x, y)?;
        if self.tr.capsules.iter().any(|c| c.to == k) { return None; } // a capsule is on its way to it
        self.tr.hoppers.remove(k);
        for c in self.tr.capsules.iter_mut() { if c.to > k { c.to -= 1; } }
        for h in self.tr.hoppers.iter_mut() { if let Some(d) = h.dest { h.dest = if d == k { None } else if d > k { Some(d - 1) } else { Some(d) }; } }
        self.route_hoppers();
        self.inv[Item::TubeHopper as usize] += 1; Some(Item::TubeHopper)
    }

    /// The mole in a capsule: carried along the pipe with it (out of sight). Returns true while it is.
    pub(crate) fn tube_ride_step(&mut self) -> bool {
        let Some(k) = self.tr.in_tube else { return false };
        if k >= self.tr.capsules.len() { self.tr.in_tube = None; return false; }
        let i = self.tr.capsules[k].cell(); let w = self.g.w;
        self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py;
        self.pl.px = (i % w) as f32 + 0.5; self.pl.py = (i / w) as f32 + 1.; self.pl.vx = 0.; self.pl.vy = 0.; self.pl.cling = 0;
        let (x, y) = (i % w, i / w); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); self.pl.fall_from = y;
        self.pl.jump_was = self.input.jump;
        true
    }

    /// One tick of every hopper and capsule.
    pub(crate) fn step_tubes(&mut self) {
        if self.tr.hoppers.is_empty() && self.tr.capsules.is_empty() { return; }
        let dt = 1. / 60.;
        // intake: loose items in the mouth of a blowing hopper's line go into a capsule
        for k in 0..self.tr.hoppers.len() {
            let h = self.tr.hoppers[k].clone();
            // (only a hopper whose own blower runs sends; the others on the line receive)
            if !self.hopper_blowing(k) || !self.drops.iter().any(|d| d.fuse == 0 && h.contains(d.px, d.py)) { continue; }
            let mut items: Vec<(Item, u32)> = vec![]; let mut j = 0;
            while j < self.drops.len() { let d = &self.drops[j]; if d.fuse == 0 && h.contains(d.px, d.py) { let d = self.drops.swap_remove(j); if let Some(e) = items.iter_mut().find(|e| e.0 == d.item) { e.1 += d.n; } else { items.push((d.item, d.n)); } } else { j += 1; } }
            if !self.send_capsule(k, items.clone(), false) { for (it, n) in items { let (cx, cy) = h.centre(); self.drops.push(Drop::at(cx, cy, it, n)); } }
        }
        // capsules: on along the pipe while their line holds pressure; stopped and spilled at a breach
        let mut done = vec![];
        for k in 0..self.tr.capsules.len() {
            let mut c = self.tr.capsules[k].clone(); c.pat = c.at;
            // pressure reaches it from any blowing hopper still joined to where it is
            let here = c.cell() as usize;
            let pressured = (0..self.tr.hoppers.len()).any(|j| self.hopper_blowing(j) && self.tube_path(here, self.tr.hoppers[j].tube).is_some());
            if pressured {
                let next = (c.at + TUBE_V * dt).min((c.path.len() - 1) as f32);
                // the pipe ahead must still be standing tube
                match (c.at as usize + 1..=next as usize).find(|&p| !self.is_tube(c.path[p] as usize)) {
                    Some(p) => { c.at = (p - 1) as f32; done.push((k, false)); }
                    None => { c.at = next; if c.at as usize >= c.path.len() - 1 { done.push((k, true)); } }
                }
            }
            self.tr.capsules[k] = c;
        }
        for &(k, arrived) in done.iter().rev() {
            let c = self.tr.capsules.remove(k);
            if let Some(r) = self.tr.in_tube { self.tr.in_tube = if r == k { None } else if r > k { Some(r - 1) } else { Some(r) }; }
            // where it comes out: the destination's mouth, or the open cell nearest the breach
            let (ox, oy) = if arrived && c.to < self.tr.hoppers.len() { self.tr.hoppers[c.to].centre() } else { self.open_near(c.cell()) };
            for (it, n) in c.items { let mut d = Drop::at(ox, oy, it, n); d.vy = -6. * SF; self.drops.push(d); }
            if c.rider {
                let (x, y) = (ox as usize, (oy as usize + S).min(self.g.h - 2));
                let y = (y.saturating_sub(4 * S)..=y + 2 * S).rev().find(|&r| self.standable(x, r)).unwrap_or(y);
                self.pl.px = x as f32 + 0.5; self.pl.py = y as f32 + 1.; self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py; self.pl.vx = 0.; self.pl.vy = 0.;
                self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); self.pl.fall_from = y;
                self.say(if arrived { "Out of the capsule.".into() } else { "The tube is breached: the capsule stuck and you crawled out.".into() });
            }
            self.events.push(if arrived { Event::Place } else { Event::Crack });
        }
    }
    /// The centre of the open cell nearest cell i (within 2 m), for spilling at a breach.
    fn open_near(&self, i: usize) -> (f32, f32) {
        let w = self.g.w; let (x, y) = ((i % w) as i32, (i / w) as i32);
        for r in 0..2 * CPM as i32 { for dy in -r..=r { for dx in -r..=r { if dx.abs() != r && dy.abs() != r { continue; }
            if self.g.in_bounds(x + dx, y + dy) && self.passable((x + dx) as usize, (y + dy) as usize) { return ((x + dx) as f32 + 0.5, (y + dy) as f32 + 0.5); } } } }
        (x as f32 + 0.5, y as f32 + 0.5)
    }
    /// The hopper's cell row and its tube cell move with a window shift; capsule paths are rebuilt from the
    /// cells (a capsule whose pipe leaves the window is lost, as loose items are).
    pub(crate) fn shift_tubes(&mut self, rows: i64) {
        let (w, h) = (self.g.w as i64, self.g.h as i64); let d = rows * w;
        self.tr.hoppers.retain_mut(|hp| { let y = hp.y as i64 - rows; let t = hp.tube as i64 - d; if y < 1 || y + HOPPER as i64 >= h - 1 || t < 0 || t >= w * h { return false; } hp.y = y as usize; hp.tube = t as usize; true });
        let ride = self.tr.in_tube; let mut keep = vec![]; let mut nr = None;
        for (k, mut c) in std::mem::take(&mut self.tr.capsules).into_iter().enumerate() {
            if c.path.iter().any(|&p| { let q = p as i64 - d; q < 0 || q >= w * h }) || c.to >= self.tr.hoppers.len() { continue; }
            for p in c.path.iter_mut() { *p = (*p as i64 - d) as u32; }
            if ride == Some(k) { nr = Some(keep.len()); } keep.push(c);
        }
        self.tr.capsules = keep; self.tr.in_tube = nr;
    }
    pub(crate) fn encode_tubes(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.hoppers.len() as u32); for hp in &self.tr.hoppers { o.u32(hp.x as u32); o.u32(hp.y as u32); o.u32(hp.tube as u32); o.u32(hp.dest.map_or(u32::MAX, |d| d as u32)); }
        // capsules in flight come out where they are (they aren't saved mid-pipe): their items as loose items
        // at the nearest open cell, and a riding mole is put out there too, by the loader
        o.u32(self.tr.capsules.len() as u32);
        for c in &self.tr.capsules { let (x, y) = self.open_near(c.cell()); o.f32(x); o.f32(y); o.u8(c.rider as u8); o.u32(c.items.len() as u32); for &(it, n) in &c.items { o.u8(it as u8); o.u32(n); } }
    }
    pub(crate) fn decode_tubes(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (w, h) = (self.g.w, self.g.h);
        for _ in 0..r.len(4096)? { let (x, y, t, d) = (r.u32()? as usize, r.u32()? as usize, r.u32()? as usize, r.u32()?);
            if x > 0 && x + HOPPER < w && y > 0 && y + HOPPER < h && t < w * h { self.tr.hoppers.push(Hopper { x, y, tube: t, dest: (d != u32::MAX).then_some(d as usize) }); } }
        let n = self.tr.hoppers.len(); for hp in self.tr.hoppers.iter_mut() { if hp.dest.is_some_and(|d| d >= n) { hp.dest = None; } }
        for _ in 0..r.len(1 << 16)? {
            let (x, y, rider) = (r.f32()?, r.f32()?, r.u8()? != 0);
            for _ in 0..r.len(4096)? { let (it, k) = (r.u8()?, r.u32()?); if let Some(it) = r.item(it).filter(|_| x > 0. && y > 0. && x < w as f32 && y < h as f32) { self.drops.push(Drop::at(x, y, it, k)); } }
            if rider && x > 0. && y > 0. && x < w as f32 && y < h as f32 { let (xi, yi) = (x as usize, y as usize); self.moles[0].x = xi; self.moles[0].y = yi; self.pl.sync = None; }
        }
        Ok(())
    }
}
