//! Sluicing: running water sorts what lies in it by how heavy it is for its size. Nothing here is
//! stored or moved by hand: the water's own flow (from the cellular sim) pushes loose items and loose
//! grains of rubble downstream, and what it can't shift stays put, so a sloping channel with water
//! running down it washes sand, dirt, gravel and coal off a heap of blasted rubble and leaves the ore.
//!
//! **The current.** The CA moves water cell by cell and keeps no velocities, so the current is read from
//! the shape of the water instead, as an open channel works: water runs down the slope of its own
//! surface. At a point under water the surface is found half a metre either side (the top of the water
//! in those columns); its slope `s` gives the speed, `WASH_V`·√|s| m/s (a 1-in-4 sheet runs about
//! 2 m/s), and with the depth `h` above the bed the stress on the bed, `h`·|s| (m of water). A pool,
//! level on top, has no current at all.
//!
//! **What moves.** A lump's *mobility* is how hard it is to shift: its weight in water over its face,
//! (ρ/ρw − 1)·size (m, `mobility`). Running water carries it when `WASH_K`·h·|s| exceeds that, and
//! keeps it up off the bed (it rides over a riffle bar) at twice that. Sand and silt (≈0.001), dirt and
//! clay (≈0.01), gravel and coal (≈0.03) wash away in any real flow; ore (≈0.3) and stone (≈0.26) need far
//! more than a sluice gives, so they sink and stay.
//!
//! **Where it applies.** Loose items in water (`items.rs` calls `wash_drag`), and loose cells (sand, a
//! blast's rubble) on the bed of running water (`step_wash`, every `WASH_EVERY` ticks over the awake chunks):
//! a grain the current can carry moves one cell downstream (or down the bed, or, kept up, over a lip),
//! at a rate set by the current's speed.
//!
//! **Riffle bars** (`Mat::Riffle`): low timber bars across a sluice floor. Lumps the water rolls along
//! the bed fetch up against them; fines kept up in the flow go over the top.
use crate::craft::{drop_of, Item};
use crate::grid::{hash3, CHUNK};
use crate::mat::{Kind, Mat, CPM};
use crate::world::World;

/// Current speed (m/s) for a surface slope of 1: v = WASH_V·√|s|.
pub const WASH_V: f32 = 4.;
/// A lump moves when WASH_K·depth·slope > its mobility (and is kept up off the bed at twice that).
pub const WASH_K: f32 = 1.;
/// Ticks between passes that wash loose cells along the bed.
pub const WASH_EVERY: u64 = 4;
/// Surface slopes below this are still water.
pub const STILL: f32 = 0.02;

/// How hard a lump of `density` (kg/m³) and `size` (m) is for water to shift: its weight in water per face.
pub fn mobility(density: f32, size: f32) -> f32 { ((density / 1000. - 1.).max(0.02)) * size }
/// A loose cell's lump: the item digging it gives (its size), else a stone-sized lump.
fn cell_mobility(m: Mat) -> f32 { mobility(m.p().density, drop_of(m).map_or(0.25, Item::size_m)) }

/// The current in the water at (px, py) (cells): (speed along x in m/s, + = right; stress depth·slope in m).
/// None if there is no water there or it lies still.
pub fn current(w: &World, px: f32, py: f32) -> Option<(f32, f32)> {
    let g = &w.g; let (x, y) = (px.floor() as i32, py.floor() as i32);
    let liquid = |x: i32, y: i32| g.in_bounds(x, y) && g.get(x as usize, y as usize) == Mat::Water;
    if !liquid(x, y) { return None; }
    // the free surface of the water in column c, from near row y (looking up to 1 m for the water first). Water
    // capped by rock (a flooded passage under its roof) has no free surface there, so that column doesn't count.
    let open = |c: i32, r: i32| g.in_bounds(c, r) && matches!(g.get(c as usize, r as usize).kind(), Kind::Empty | Kind::Gas);
    let top = |c: i32| -> Option<i32> {
        let mut r = (0..=CPM as i32).map(|k| y - k).chain((1..=CPM as i32).map(|k| y + k)).find(|&r| liquid(c, r))?;
        while liquid(c, r - 1) { r -= 1; } open(c, r - 1).then_some(r)
    };
    let t = top(x)?; let l = CPM as i32;
    let side = |dir: i32| (1..=l).rev().map(|d| x + dir * d).find_map(|c| top(c).map(|r| (c, r)));
    let (a, b) = (side(-1).unwrap_or((x, t)), side(1).unwrap_or((x, t)));
    if b.0 <= a.0 { return None; }
    let s = (b.1 - a.1) as f32 / (b.0 - a.0) as f32;
    if s.abs() < STILL { return None; }
    let mut bed = y; while liquid(x, bed + 1) { bed += 1; }
    let h = (bed + 1 - t) as f32 / CPM as f32;
    Some((s.signum() * WASH_V * s.abs().sqrt(), h * s.abs()))
}

impl World {
    /// Running water's pull on a loose item in it: the speed (cells/s) its drag pulls it toward, and whether
    /// the flow keeps it up off the bed. (0, false) in still water, or for a lump too heavy for the flow.
    pub(crate) fn wash_drag(&self, it: Item, px: f32, py: f32) -> (f32, bool) {
        let Some((v, tau)) = current(self, px, py) else { return (0., false) };
        let f = WASH_K * tau / mobility(it.density(), it.size_m());
        if f > 1. { (v * CPM as f32, f > 2.) } else { (0., false) }
    }

    /// Wash loose grains along the bed of running water (sand, a blast's rubble): each one the current can
    /// carry moves a cell downstream, as often as the current's speed says.
    pub(crate) fn step_wash(&mut self) {
        if self.tick % WASH_EVERY != 0 { return; }
        let (gw, gh) = (self.g.w, self.g.h);
        let chunks: Vec<usize> = (0..self.g.awake.len()).filter(|&c| self.g.awake[c].load(std::sync::atomic::Ordering::Relaxed) != 0).collect();
        let mut moves = vec![];
        for c in chunks {
            let (x0, y0) = ((c % self.g.cx) * CHUNK, (c / self.g.cx) * CHUNK);
            for y in y0.max(1)..(y0 + CHUNK).min(gh - 1) { for x in x0.max(1)..(x0 + CHUNK).min(gw - 1) {
                let i = y * gw + x;
                if self.g.mat[i - gw] != Mat::Water as u8 { continue; } // (most cells: no water on them)
                let m = self.g.m(i);
                let grain = m.kind() == Kind::Powder || m.kind() == Kind::Solid && self.g.loose(i);
                if !grain { continue; }
                let Some((v, tau)) = current(self, x as f32 + 0.5, y as f32 - 0.5) else { continue };
                let f = WASH_K * tau / cell_mobility(m); if f <= 1. { continue; }
                // a move this pass with a chance of the cells it would cover at that speed
                let p = (v.abs() * CPM as f32 * WASH_EVERY as f32 / 60.).min(1.);
                if (hash3(x as u32, y as u32, self.tick as u32 ^ 0x5a17) & 1023) as f32 / 1024. >= p { continue; }
                let d = v.signum() as i32; let nx = (x as i32 + d) as usize;
                let water = |j: usize| self.g.m(j) == Mat::Water;
                // along the bed, down it, or (kept up in the flow) over a lip
                let to = [y * gw + nx, (y + 1) * gw + nx].into_iter().chain((f > 2.).then_some((y - 1) * gw + nx)).find(|&j| water(j));
                if let Some(j) = to { moves.push((i, j)); }
            } }
        }
        for (i, j) in moves {
            if self.g.m(j) != Mat::Water || self.g.m(i) == Mat::Water { continue; } // something already took it
            let g = &mut self.g; g.mat.swap(i, j); g.flags.swap(i, j); g.shade.swap(i, j); g.life.swap(i, j);
            g.wake(i % gw, i / gw); g.wake(j % gw, j / gw);
        }
    }
}
