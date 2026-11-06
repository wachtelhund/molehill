//! Non-electric logic: weight, water and rods, no current. A **linkage** is pull-rod cells and the sensors and
//! actuators touching them; when any of its sensors is pressed the rods are pulled (several sensors on one linkage
//! are an *either*), and every actuator on it moves. Solved every `LINK_EVERY` ticks.
//!
//! Sensors:
//! * **Tread plate** (`Mat::TreadPlate`, built into a floor): pressed by `PLATE_KG` or more on it: the mole, loose
//!   lumps (each `transport::ITEM_KG`) or a loaded cart.
//! * **Tipping scale** (`Item::Scale`, a pan a metre wide on a floor, with a counter-pan): tips when what lies on its pan
//!   outweighs the stone on its counter-pan. Right mouse with a stone in the paw puts one on the counter-pan
//!   (`STONE_KG` each), with an empty paw takes one off: that sets how much it weighs out (a batch).
//! * **Float** (`Mat::Float`): lifted, and so pressed, while water touches it.
//!
//! Actuators:
//! * **Sluice gate** (`Item::SluiceGate`): a timber gate filling a channel's opening, top to bottom (up to `GATE_M`).
//!   Pulled, it lifts: its cells are open and water and lumps pass; let go, it drops again (what's in the way is pushed
//!   up out of it, as a door does). Right mouse turns it round: it then **shuts** when pulled (a *turn*).
//! * **Clutch** (`Mat::Clutch`, machinery): joins its drive train only while its rod is pulled (power.rs).
//! * **Ore hopper**: a rod touching one holds its gate open while pulled.
use std::collections::{HashMap, HashSet};

use crate::craft::Item;
use crate::items::ITEM_HW;
use crate::mat::{Kind, Mat, CPM, LINK, S};
use crate::world::{Event, World, BODY_HW};

pub const LINK_EVERY: u64 = 6;
pub const PLATE_KG: f32 = 20.;
pub const STONE_KG: f32 = 5.;
pub const GATE_M: usize = 2;
pub const SCALE_W: usize = CPM;

#[derive(Clone, Debug)]
pub struct Gate {
    /// its cells (a column two cells wide), whether it's lifted, and whether it shuts when pulled
    pub cells: Vec<usize>, pub open: bool, pub invert: bool,
}
#[derive(Clone, Debug)]
pub struct Scale {
    /// the pan's middle column and the floor row it stands on (the pan is the tile row above), stones on its counter-pan,
    /// whether it's tipped
    pub x: usize, pub y: usize, pub ballast: u32, pub tipped: bool,
}
impl Scale {
    pub fn pan(&self) -> (usize, usize, usize) { (self.x - SCALE_W / 2, self.x + SCALE_W / 2, self.y - S) }
}

/// The linkages as last solved: which cells are pulled.
#[derive(Clone, Debug, Default)]
pub struct Links {
    pub gates: Vec<Gate>, pub scales: Vec<Scale>,
    /// pulled linkage cells (rods, sensors, clutches) at the last solve
    pub pulled: HashSet<usize>,
}

impl World {
    /// Is the cell part of a pulled linkage (at the last solve)?
    pub fn pulled(&self, i: usize) -> bool { self.tr.links.pulled.contains(&i) }
    /// What lies on the surface across columns x0..x1 whose top is row `top` (kg): loose lumps, the mole, carts.
    fn load_on(&self, x0: usize, x1: usize, top: usize) -> f32 {
        let (fx0, fx1, ft) = (x0 as f32, x1 as f32, top as f32);
        let lumps = self.drops.iter().filter(|d| d.px >= fx0 && d.px < fx1 && d.py + ITEM_HW >= ft - 1. && d.py < ft).map(|d| d.n).sum::<u32>() as f32 * crate::transport::ITEM_KG;
        let mole = self.moles.first().filter(|m| m.alive).map_or(0., |_| { let (px, py) = (self.pl.px, self.pl.py);
            if self.pl.grounded && px + BODY_HW > fx0 && px - BODY_HW < fx1 && (py - ft).abs() < 1. { crate::transport::RIDER_KG } else { 0. } });
        let carts = self.tr.carts.iter().filter(|c| c.x > fx0 - crate::transport::CART_W / 2. && c.x < fx1 + crate::transport::CART_W / 2. && (c.y - ft).abs() < 1.5).map(|c| c.mass(false)).sum::<f32>();
        lumps + mole + carts
    }

    /// Set a sluice gate into the opening at (x, y): the open run of that column (and the next) from the cursor up to
    /// the rock above and down to the floor, at most `GATE_M`.
    pub(crate) fn place_gate(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let x = x / S * S; let gw = self.g.w;
        let open = |w: &World, r: usize| (x..x + S).all(|c| w.g.get(c, r).passable());
        if !open(self, y) { return Err("needs an opening to stand in"); }
        let mut top = y; while top > 1 && open(self, top - 1) && y - top < GATE_M * CPM { top -= 1; }
        let mut bot = y; while bot + 2 < self.g.h && open(self, bot + 1) && bot - top < GATE_M * CPM { bot += 1; }
        if open(self, top.saturating_sub(1)) && open(self, bot + 1) { return Err("the opening is too tall for a gate (2 m at most)"); }
        let cells: Vec<usize> = (top..=bot).flat_map(|r| (x..x + S).map(move |c| r * gw + c)).collect();
        if self.tr.links.gates.iter().any(|g| g.cells.iter().any(|c| cells.contains(c))) { return Err("there's a gate there"); }
        let mut g = Gate { cells, open: true, invert: false };
        self.shut_gate(&mut g); self.tr.links.gates.push(g); self.events.push(Event::Place); Ok(())
    }
    fn shut_gate(&mut self, gt: &mut Gate) {
        let gw = self.g.w;
        for &i in &gt.cells { let m = self.g.m(i);
            if m != Mat::Empty && (m.kind() != Kind::Solid || self.g.loose(i)) && m != Mat::SluiceGate {
                // what's in the way goes up out of the gate's line, as a door does
                let (x, y0) = (i % gw, i / gw);
                if let Some(j) = (1..12 * S).filter_map(|k| y0.checked_sub(k)).map(|r| r * gw + x).find(|&j| self.g.mat[j] == 0 && !gt.cells.contains(&j)) { self.g.set(j, m); }
            }
            if self.g.m(i).passable() || self.g.loose(i) { self.g.set(i, Mat::SluiceGate); } }
        gt.open = false;
    }
    fn lift_gate(&mut self, gt: &mut Gate) { for &i in &gt.cells { if self.g.m(i) == Mat::SluiceGate { self.g.set(i, Mat::Empty); } } gt.open = true; }
    pub fn gate_at(&self, x: usize, y: usize) -> Option<usize> { let i = y * self.g.w + x; self.tr.links.gates.iter().position(|g| g.cells.contains(&i)) }

    /// Stand a tipping scale on the floor under (x, y).
    pub(crate) fn place_scale(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let h = SCALE_W / 2; if x < h + 2 || x + h + 2 >= self.g.w { return Err("too near the edge"); }
        let x = (x - h) / S * S + h;
        let floor = (y..(y + 3 * CPM).min(self.g.h - 1)).find(|&r| (x - h..x + h).any(|c| !self.passable(c, r))).ok_or("needs a floor under it")?;
        if (x - h..x + h).any(|c| self.passable(c, floor)) { return Err("needs a level floor under all of it"); }
        if !(floor - S..floor).all(|r| (x - h..x + h).all(|c| self.g.get(c, r) == Mat::Empty)) { return Err("needs room for it"); }
        for r in floor - S..floor { for c in x - h..x + h { let i = self.g.idx(c, r); self.g.set(i, Mat::ScalePan); } }
        self.tr.links.scales.push(Scale { x, y: floor, ballast: 10, tipped: false }); self.events.push(Event::Place); Ok(())
    }
    pub fn scale_at(&self, x: usize, y: usize) -> Option<usize> { self.tr.links.scales.iter().position(|s| { let (x0, x1, top) = s.pan(); x >= x0 && x < x1 && y >= top.saturating_sub(S) && y < s.y }) }

    /// Right mouse: a stone on or off a scale's counter-pan, or a gate turned round.
    pub(crate) fn use_linkage(&mut self, x: usize, y: usize) -> bool {
        let stone_in_paw = self.gear.held == Some(Item::Stone);
        if let Some(k) = self.scale_at(x, y) {
            let s = &mut self.tr.links.scales[k];
            if stone_in_paw && self.inv[Item::Stone as usize] > 0 { self.inv[Item::Stone as usize] -= 1; s.ballast += 1; }
            else if s.ballast > 0 { s.ballast -= 1; self.inv[Item::Stone as usize] += 1; }
            let b = self.tr.links.scales[k].ballast; self.say(format!("The scale's counter-pan holds {b} stones: it tips at more than {:.0} kg.", b as f32 * STONE_KG));
            self.events.push(Event::Click); return true;
        }
        if let Some(k) = self.gate_at(x, y) {
            let g = &mut self.tr.links.gates[k]; g.invert = !g.invert;
            let s = if g.invert { "The gate is turned round: it shuts when its rod is pulled." } else { "The gate lifts when its rod is pulled." };
            self.say(s.into()); self.events.push(Event::Click); return true;
        }
        false
    }
    /// `E`: a scale (its stones back into the pack) or a gate comes up.
    pub(crate) fn pick_up_linkage(&mut self, x: usize, y: usize) -> Option<Item> {
        if let Some(k) = self.scale_at(x, y) { let s = self.tr.links.scales.remove(k); let (x0, x1, top) = s.pan();
            for r in top..s.y { for c in x0..x1 { let i = self.g.idx(c, r); if self.g.m(i) == Mat::ScalePan { self.g.set(i, Mat::Empty); } } }
            self.inv[Item::Stone as usize] += s.ballast; self.inv[Item::Scale as usize] += 1; return Some(Item::Scale); }
        if let Some(k) = self.gate_at(x, y) { let g = self.tr.links.gates.remove(k);
            for &i in &g.cells { if self.g.m(i) == Mat::SluiceGate { self.g.set(i, Mat::Empty); } }
            self.inv[Item::SluiceGate as usize] += 1; return Some(Item::SluiceGate); }
        None
    }

    /// Solve the linkages: which are pulled, and move what they drive.
    pub(crate) fn step_linkage(&mut self) {
        if self.tick % LINK_EVERY != 0 { return; }
        let (gw, gh) = (self.g.w, self.g.h);
        // the scales and gates: drop any whose cells are gone
        self.tr.links.scales.retain(|s| { let (x0, x1, top) = s.pan(); (top..s.y).all(|r| (x0..x1).all(|c| self.g.get(c, r) == Mat::ScalePan)) });
        self.tr.links.gates.retain(|g| g.cells.iter().all(|&i| { let m = self.g.m(i); m == Mat::SluiceGate || g.open && (m.passable() || self.g.loose(i)) }));
        // the gates' and scales' cells count as linkage cells whether lifted or not
        let mut owner: HashMap<usize, (u8, usize)> = HashMap::new();
        for (k, g) in self.tr.links.gates.iter().enumerate() { for &i in &g.cells { owner.insert(i, (0, k)); } }
        for (k, s) in self.tr.links.scales.iter().enumerate() { let (x0, x1, top) = s.pan(); for r in top..s.y { for c in x0..x1 { owner.insert(r * gw + c, (1, k)); } } }
        // linkage cells in the active region
        let (x0, y0, x1, y1) = self.g.active.map_or((0, 0, gw, gh), |(a, b, c, d)| (a * crate::grid::CHUNK, b * crate::grid::CHUNK, ((c + 1) * crate::grid::CHUNK).min(gw), ((d + 1) * crate::grid::CHUNK).min(gh)));
        let link = |w: &World, i: usize| w.g.m(i).has(LINK) || owner.contains_key(&i);
        let mut cells = vec![];
        for y in y0.max(1)..y1.min(gh - 1) { for x in x0.max(1)..x1.min(gw - 1) { let i = y * gw + x; if crate::mat::PROPS[self.g.mat[i] as usize].flags & LINK != 0 { cells.push(i); } } }
        for &i in owner.keys() { cells.push(i); }
        // scales weigh what's on them
        for k in 0..self.tr.links.scales.len() { let s = &self.tr.links.scales[k]; let (a, b, top) = s.pan();
            let tip = self.load_on(a, b, top) > s.ballast as f32 * STONE_KG; self.tr.links.scales[k].tipped = tip; }
        let mut pulled: HashSet<usize> = HashSet::new(); let mut seen: HashSet<usize> = HashSet::new();
        let mut gate_pull: Vec<Option<bool>> = vec![None; self.tr.links.gates.len()];
        let mut bins_held: Vec<bool> = vec![false; self.tr.bins.len()];
        for &s in &cells {
            if !seen.insert(s) { continue; }
            let mut net = vec![s]; let mut q = 0;
            while q < net.len() { let i = net[q]; q += 1;
                for n in self.g.n4(i % gw, i / gw).collect::<Vec<_>>() { if link(self, n) && seen.insert(n) { net.push(n); } } }
            // pressed sensors in this linkage
            let pressed = net.iter().any(|&i| match self.g.m(i) {
                Mat::TreadPlate => { let (x, y) = (i % gw, i / gw); y > 0 && !self.g.get(x, y - 1).has(LINK) && self.load_on(x, x + 1, y) >= PLATE_KG },
                Mat::Float => self.g.n4(i % gw, i / gw).any(|j| self.g.m(j) == Mat::Water),
                _ => owner.get(&i).is_some_and(|&(t, k)| t == 1 && self.tr.links.scales[k].tipped),
            });
            if pressed { pulled.extend(net.iter().copied()); }
            for &i in &net { if let Some(&(0, k)) = owner.get(&i) { gate_pull[k] = Some(gate_pull[k].unwrap_or(false) || pressed); } }
            if pressed { for (k, b) in self.tr.bins.iter().enumerate() { let (bx0, by0, bx1, by1) = b.rect();
                if net.iter().any(|&i| { let (x, y) = ((i % gw) as f32 + 0.5, (i / gw) as f32 + 0.5); x >= bx0 - 1.5 && x < bx1 + 1.5 && y >= by0 - 1.5 && y < by1 + 1.5 }) { bins_held[k] = true; } } }
        }
        // move the gates
        for k in 0..self.tr.links.gates.len() { let Some(p) = gate_pull[k] else { continue };
            let mut g = self.tr.links.gates[k].clone(); let want_open = p != g.invert;
            if want_open && !g.open { self.lift_gate(&mut g); self.events.push(Event::Click); }
            else if !want_open && g.open { self.shut_gate(&mut g); self.events.push(Event::Click); }
            self.tr.links.gates[k] = g; }
        for (k, h) in bins_held.into_iter().enumerate() { self.tr.bins[k].rod_held = h; }
        self.tr.links.pulled = pulled;
    }

    pub(crate) fn shift_linkage(&mut self, rows: i64) {
        let (w, n) = (self.g.w as i64, (self.g.w * self.g.h) as i64); let h = self.g.h as i64;
        self.tr.links.gates.retain_mut(|g| { for i in g.cells.iter_mut() { let j = *i as i64 - rows * w; if j < 0 || j >= n { return false; } *i = j as usize; } true });
        self.tr.links.scales.retain_mut(|s| { let y = s.y as i64 - rows; if y < (S + 2) as i64 || y >= h - 1 { return false; } s.y = y as usize; true });
        self.tr.links.pulled.clear();
    }
    pub(crate) fn encode_linkage(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.links.gates.len() as u32);
        for g in &self.tr.links.gates { o.u32(g.cells.len() as u32); for &i in &g.cells { o.u32(i as u32); } o.u8(g.open as u8 | (g.invert as u8) << 1); }
        o.u32(self.tr.links.scales.len() as u32);
        for s in &self.tr.links.scales { o.u32(s.x as u32); o.u32(s.y as u32); o.u32(s.ballast); }
    }
    pub(crate) fn decode_linkage(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (gw, gh) = (self.g.w, self.g.h);
        for _ in 0..r.len(4096)? { let mut cells = vec![]; for _ in 0..r.len(4096)? { let i = r.u32()? as usize; if i < gw * gh { cells.push(i); } }
            let f = r.u8()?; if !cells.is_empty() { self.tr.links.gates.push(Gate { cells, open: f & 1 != 0, invert: f & 2 != 0 }); } }
        for _ in 0..r.len(4096)? { let (x, y, ballast) = (r.u32()? as usize, r.u32()? as usize, r.u32()?);
            if x > SCALE_W && x + SCALE_W < gw && y > S + 1 && y < gh - 1 { self.tr.links.scales.push(Scale { x, y, ballast, tipped: false }); } }
        Ok(())
    }
}
