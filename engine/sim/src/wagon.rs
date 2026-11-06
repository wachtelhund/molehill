//! Wagons: rear cars a steam borer (borer.rs) hauls on deep expeditions, a mobile base on wheels. A wagon set down or
//! pushed up behind a borer's tail (or the last wagon of its train) couples on: a coupling bar holds it `GAP` behind,
//! and from then on it follows the borer wherever it drives. Each wagon in a train costs the borer `WAGON_DRIVE_W` more
//! to drive; if any wagon can't follow (a wall, a step taller than a tile), the whole train stops.
//!
//! * **Store wagon:** a box on wheels. Loose items that fall into it ride along; inside it, right mouse with an item in
//!   paw stows that stack. `E` takes everything out into the pack.
//! * **Coal wagon:** a fuel bunker. Coal, charcoal and wood thrown or dropped in ride along, or shovelled in by hand
//!   (fuel in paw + right mouse). Coupled, its stoking chute feeds the borer's firebox a lump at a time, while the
//!   borer's steam is below `STOKE_BELOW` of full: with steam up, a catch on the chute holds it shut, so a parked borer
//!   doesn't burn the bunker away.
//! * **Water wagon:** a tank. It draws water touching it (as the borer does) and, coupled, tops up the borer's tank.
//! * **Bunk wagon:** climbing in is going to sleep: you wake there after a death, and the game saves.
//! * **Workshop wagon:** a saw bench under a roof. Inside it, you work as at a workbench.
//!
//! Right mouse (empty paw) climbs in or out of any wagon; Space gets out. The borer is driven from its cab only.
//! `E` on a wagon takes its load out, then uncouples it (it won't couple again until it has been pulled apart), then
//! picks it up. Nothing in a wagon is anywhere but in the wagon: it is all hauled with it.
use crate::borer::BORER_W;
use crate::craft::{Item, Station};
use crate::mat::{Kind, Mat, CPM, S};
use crate::steam::burn_ticks;
use crate::world::{Building, Event, World, BODY_HW};

pub const WAGON_W: f32 = CPM as f32;
pub const WAGON_H: f32 = (3 * CPM / 4) as f32;
/// The coupling bar: the gap between a wagon and what pulls it (cells); how close its front must come to couple on.
pub const GAP: f32 = S as f32; pub const COUPLE_D: f32 = (3 * S) as f32 / 2. + 1.;
/// What hauling a wagon costs the borer (W); what the stores hold (items, lumps, water cells).
pub const WAGON_DRIVE_W: f32 = 100.;
/// The coal wagon stokes only while the borer's steam is below this share of full.
pub const STOKE_BELOW: f32 = 0.5;
pub const STORE_CAP: u32 = 200; pub const COAL_CAP: u32 = 40; pub const WATER_CAP: f32 = 48.;
/// How fast the mole can shove a wagon along by hand (cells/s).
pub const PUSH_V: f32 = 1.5 * CPM as f32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WagonKind { Store, Workshop, Bunk, Water, Coal }
pub const WAGON_KINDS: [WagonKind; 5] = [WagonKind::Store, WagonKind::Workshop, WagonKind::Bunk, WagonKind::Water, WagonKind::Coal];
impl WagonKind {
    pub fn item(self) -> Item { [Item::StoreWagon, Item::WorkshopWagon, Item::BunkWagon, Item::WaterWagon, Item::CoalWagon][self as usize] }
    pub fn of(it: Item) -> Option<WagonKind> { WAGON_KINDS.into_iter().find(|k| k.item() == it) }
    pub fn name(self) -> &'static str { ["store", "workshop", "bunk", "water", "coal"][self as usize] }
}

#[derive(Clone, Debug)]
pub struct Wagon {
    pub kind: WagonKind,
    /// its middle and the row its wheels stand on (cells), last tick's (for drawing), fall speed
    pub x: f32, pub y: f32, pub px: f32, pub py: f32, pub vy: f32,
    /// its coupling bar is hooked on; it was uncoupled and won't hook on again until pulled apart
    pub coupled: bool, pub parted: bool,
    /// what it carries (the store and the coal wagon) and its water (cells, the water wagon)
    pub cargo: Vec<(Item, u32)>, pub water: f32,
}
impl Wagon {
    pub fn rect(&self) -> (f32, f32, f32, f32) { (self.x - WAGON_W / 2., self.y - WAGON_H, self.x + WAGON_W / 2., self.y) }
    pub fn contains(&self, x: f32, y: f32) -> bool { let (x0, y0, x1, y1) = self.rect(); x >= x0 && x < x1 && y >= y0 && y < y1 }
    pub fn load(&self) -> u32 { self.cargo.iter().map(|c| c.1).sum() }
    fn add(&mut self, it: Item, n: u32) { if let Some(c) = self.cargo.iter_mut().find(|c| c.0 == it) { c.1 += n; } else { self.cargo.push((it, n)); } }
    /// Can it take a loose `it` in through its top?
    fn takes(&self, it: Item, n: u32) -> bool {
        match self.kind { WagonKind::Store => self.load() + n <= STORE_CAP, WagonKind::Coal => burn_ticks(it).is_some() && self.load() + n <= COAL_CAP, _ => false }
    }
}

impl World {
    fn wagon_hits(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = ((x - WAGON_W / 2.).floor() as i32, (x + WAGON_W / 2. - 1e-3).floor() as i32);
        let (y0, y1) = ((y - WAGON_H).floor() as i32, (y - 1e-3).floor() as i32);
        (y0..=y1).any(|r| (x0..=x1).any(|c| self.wagon_cell_blocks(c, r)))
    }
    /// Loose grains don't stop it: they're shoved up over it (`shove_from_wagon`).
    fn wagon_cell_blocks(&self, c: i32, r: i32) -> bool {
        if !self.g.in_bounds(c, r) { return true; }
        let i = self.g.idx(c as usize, r as usize); let m = self.g.m(i);
        !self.passable(c as usize, r as usize) && !(m.kind() == Kind::Powder || m.kind() == Kind::Solid && self.g.loose(i))
    }
    fn wagon_grounded(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = ((x - WAGON_W / 2. + 1.).floor() as i32, (x + WAGON_W / 2. - 1.).floor() as i32);
        (x0..=x1).any(|c| self.wagon_cell_blocks(c, y.floor() as i32))
    }
    /// Grains in its body go up over it, to the nearest open cell above (never lost).
    fn shove_from_wagon(&mut self, x: f32, y: f32) {
        let (x0, x1) = ((x - WAGON_W / 2.).floor() as i32, (x + WAGON_W / 2. - 1e-3).floor() as i32);
        let (y0, y1) = ((y - WAGON_H).floor() as i32, (y - 1e-3).floor() as i32);
        for r in y0..=y1 { for c in x0..=x1 {
            if !self.g.in_bounds(c, r) || self.passable(c as usize, r as usize) || self.wagon_cell_blocks(c, r) { continue; }
            let i = self.g.idx(c as usize, r as usize); let (m, f) = (self.g.mat[i], self.g.flags[i]);
            if let Some(t) = (1..3 * CPM as i32).map(|k| y0 - k).find(|&rr| self.g.in_bounds(c, rr) && self.g.get(c as usize, rr as usize) == Mat::Empty) {
                let j = self.g.idx(c as usize, t as usize); self.g.set(j, Mat::from_u8(m)); self.g.flags[j] = f; self.g.set(i, Mat::Empty);
            } } }
    }

    /// Set a wagon down on the floor under (x, y).
    pub(crate) fn place_wagon(&mut self, kind: WagonKind, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let xf = x as f32 + 0.5;
        let floor = (y..(y + 2 * CPM).min(self.g.h - 1)).find(|&r| self.wagon_grounded(xf, r as f32)).ok_or("needs a floor under it")? as f32;
        if self.wagon_hits(xf, floor) { return Err("needs 1 m by 0.75 m of room"); }
        if self.tr.wagons.iter().any(|w| (w.x - xf).abs() < WAGON_W && (w.y - floor).abs() < WAGON_H) { return Err("there's a wagon there"); }
        if self.tr.borers.iter().any(|b| (b.x - xf).abs() < (BORER_W + WAGON_W) / 2. && (b.y - floor).abs() < crate::borer::BORER_H) { return Err("there's a borer there"); }
        self.tr.wagons.push(Wagon { kind, x: xf, y: floor, px: xf, py: floor, vy: 0., coupled: false, parted: false, cargo: vec![], water: 0. });
        self.events.push(Event::Place); Ok(())
    }
    pub fn wagon_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.wagons.iter().position(|w| w.contains(xf, yf)) }

    /// The wagons borer `k` hauls, front to back: each coupled one whose front is at its coupling bar's length behind
    /// the one before.
    pub fn train(&self, k: usize) -> Vec<usize> {
        let Some(b) = self.tr.borers.get(k) else { return vec![] }; let d = b.dir as f32;
        let (mut rear, mut ry) = (b.x - d * BORER_W / 2., b.y); let mut out: Vec<usize> = vec![];
        loop {
            let want = rear - d * (GAP + WAGON_W / 2.);
            let Some(j) = (0..self.tr.wagons.len()).find(|&j| { let w = &self.tr.wagons[j]; w.coupled && !out.contains(&j) && (w.x - want).abs() < COUPLE_D && (w.y - ry).abs() <= CPM as f32 }) else { break };
            out.push(j); let w = &self.tr.wagons[j]; rear = w.x - d * WAGON_W / 2.; ry = w.y;
        }
        out
    }
    /// The back of borer `k`'s train (the borer's tail, or its last wagon's) and that one's row.
    pub(crate) fn train_tail(&self, k: usize, train: &[usize]) -> (f32, f32) {
        let b = &self.tr.borers[k]; let d = b.dir as f32;
        match train.last() { Some(&j) => { let w = &self.tr.wagons[j]; (w.x - d * WAGON_W / 2., w.y) } None => (b.x - d * BORER_W / 2., b.y) }
    }
    /// Where the wagons of `train` go behind a borer now at (bx, by) facing `dir`: each its bar's length behind the one
    /// before, at its own height or a step (up to a tile) up. None if any can't (the train is stopped).
    pub(crate) fn haul(&self, bx: f32, dir: i8, train: &[usize]) -> Option<Vec<(f32, f32)>> {
        let d = dir as f32; let mut rear = bx - d * BORER_W / 2.; let mut out = vec![];
        for &j in train {
            let w = &self.tr.wagons[j]; let nx = rear - d * (GAP + WAGON_W / 2.);
            let ny = if !self.wagon_hits(nx, w.y) { w.y } else if !self.wagon_hits(nx, w.y - S as f32) { w.y - S as f32 } else { return None };
            out.push((nx, ny)); rear = nx - d * WAGON_W / 2.;
        }
        Some(out)
    }
    /// Is wagon `j` in any borer's train?
    pub fn in_train(&self, j: usize) -> bool { self.hauled_by(j).is_some() }
    /// The borer whose train wagon `j` is in, if any.
    pub fn hauled_by(&self, j: usize) -> Option<usize> { (0..self.tr.borers.len()).find(|&k| self.train(k).contains(&j)) }

    /// Right mouse on a wagon: climb in or out (empty paw); fuel in paw shovels it into a coal wagon; inside the store
    /// wagon, the stack in paw is stowed.
    pub(crate) fn use_wagon(&mut self, x: usize, y: usize) -> bool {
        let Some(j) = self.wagon_at(x, y) else { return false };
        let kind = self.tr.wagons[j].kind; let inside = self.tr.wagon_ride == Some(j);
        let held = self.gear.held.filter(|&it| self.inv[it as usize] > 0);
        if let Some(it) = held.filter(|&it| kind == WagonKind::Coal && burn_ticks(it).is_some()) {
            let room = COAL_CAP - self.tr.wagons[j].load(); let n = self.inv[it as usize].min(room);
            if n == 0 { self.say("The coal wagon is full.".into()); } else { self.inv[it as usize] -= n; self.tr.wagons[j].add(it, n);
                self.say(format!("Shovelled {n} {} into the coal wagon: {} lumps in it.", it.name().to_lowercase(), self.tr.wagons[j].load())); }
        } else if let Some(it) = held.filter(|_| inside && kind == WagonKind::Store) {
            let room = STORE_CAP - self.tr.wagons[j].load(); let n = self.inv[it as usize].min(room);
            if n == 0 { self.say("The store wagon is full.".into()); } else { self.inv[it as usize] -= n; self.tr.wagons[j].add(it, n);
                self.say(format!("Stowed {n} {} in the store wagon.", it.name().to_lowercase())); }
        } else if inside { self.leave_wagon(); }
        else {
            if self.tr.borer_ride.is_some() { self.leave_borer(); }
            if self.tr.riding.is_some() || self.tr.in_tube.is_some() { return false; }
            self.tr.wagon_ride = Some(j);
            match kind {
                WagonKind::Bunk => { let w = &self.tr.wagons[j]; self.pl.spawn = (w.x as usize, self.oy + (w.y - 1.) as i64); self.tr.slept = true;
                    self.say("You curl up in the bunk wagon: you'll wake here. (Space: get up)".into()); }
                WagonKind::Workshop => self.say("In the workshop wagon: you can work here as at a workbench. Space: get out.".into()),
                WagonKind::Store => self.say("In the store wagon: right mouse with something in paw stows it · Space: get out.".into()),
                _ => self.say(format!("In the {} wagon. Space: get out.", kind.name())),
            }
        }
        self.events.push(Event::Click); true
    }
    pub(crate) fn leave_wagon(&mut self) {
        self.tr.bench = None;
        if let Some(j) = self.tr.wagon_ride.take() { let w = &self.tr.wagons[j]; let (x0, y0, _, _) = w.rect();
            self.pl.px = w.x; self.pl.py = y0; self.pl.vx = 0.; self.pl.vy = 0.; self.pl.sync = None;
            if self.box_hits(self.pl.px, self.pl.py) { self.pl.px = x0 - BODY_HW - 0.5; self.pl.py = w.y; }
            let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); }
    }
    /// `E` on a wagon: its load into the pack; then uncouple it; then, empty and with nobody in it, it comes up itself
    /// (a water wagon's water pours out where it stood).
    pub(crate) fn pick_up_wagon(&mut self, x: usize, y: usize) -> Option<Item> {
        let j = self.wagon_at(x, y)?; let kind = self.tr.wagons[j].kind; let it = kind.item();
        if !self.tr.wagons[j].cargo.is_empty() { let mut got = vec![];
            for (c, n) in std::mem::take(&mut self.tr.wagons[j].cargo) { self.inv[c as usize] += n; got.push(format!("{n} {}", c.name().to_lowercase())); }
            self.say(format!("Unloaded the {} wagon: {}.", kind.name(), got.join(", "))); return Some(it); }
        if self.tr.wagons[j].coupled { let w = &mut self.tr.wagons[j]; w.coupled = false; w.parted = true;
            self.say(format!("Uncoupled the {} wagon.", kind.name())); self.events.push(Event::Click); return Some(it); }
        if self.tr.wagon_ride == Some(j) { return None; }
        let w = self.tr.wagons.remove(j);
        if let Some(r) = self.tr.wagon_ride { if r > j { self.tr.wagon_ride = Some(r - 1); } }
        // (its water goes on the floor where it stood: a wagon's footprint holds more than its tank)
        let mut water = w.water.floor() as i32; let (x0, y0, x1, y1) = w.rect();
        for r in (y0.floor() as i32..y1.ceil() as i32).rev() { for c in x0.floor() as i32..x1.ceil() as i32 {
            if water > 0 && self.g.in_bounds(c, r) && self.g.get(c as usize, r as usize) == Mat::Empty { let i = self.g.idx(c as usize, r as usize); self.g.set(i, Mat::Water); water -= 1; } } }
        self.inv[it as usize] += 1; Some(it)
    }

    /// The mole in a wagon: carried in it. Space gets out. True while in one.
    pub(crate) fn wagon_ride_step(&mut self) -> bool {
        let Some(j) = self.tr.wagon_ride else { return false };
        if j >= self.tr.wagons.len() || !self.moles[0].alive { self.tr.wagon_ride = None; self.tr.bench = None; return false; }
        let jump = self.input.jump && !self.pl.jump_was; self.pl.jump_was = self.input.jump;
        if jump { self.leave_wagon(); return true; }
        let w = &self.tr.wagons[j];
        self.tr.bench = (w.kind == WagonKind::Workshop).then(|| Building { kind: Station::Workbench, x: (w.x as usize).saturating_sub(Station::W / 2), y: w.y as usize - 1, built: true, claimed: false });
        self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py;
        self.pl.px = w.x; self.pl.py = w.y - 1.; self.pl.vx = 0.; self.pl.vy = 0.; self.pl.grounded = true; self.pl.cling = 0;
        let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); self.pl.fall_from = y;
        true
    }

    /// One tick of every wagon: falling, coupling on, what each kind does, and the mole shoving a loose one along.
    pub(crate) fn step_wagons(&mut self) {
        if self.tr.wagons.is_empty() { return; }
        let dt = 1. / 60.;
        let trains: Vec<Vec<usize>> = (0..self.tr.borers.len()).map(|k| self.train(k)).collect();
        let hauled: Vec<bool> = (0..self.tr.wagons.len()).map(|j| trains.iter().any(|t| t.contains(&j))).collect();
        for j in 0..self.tr.wagons.len() {
            let mut w = self.tr.wagons[j].clone(); w.px = w.x; w.py = w.y;
            // falling
            if !self.wagon_grounded(w.x, w.y) { w.vy = (w.vy + crate::world::TUNE.gravity * dt).min(crate::world::TUNE.terminal);
                let ny = w.y + w.vy * dt; if !self.wagon_hits(w.x, ny) { w.y = ny; } else { w.vy = 0.; w.y = w.y.floor(); } } else { w.vy = 0.; }
            // the water wagon draws water touching it
            if w.kind == WagonKind::Water && w.water + 1. <= WATER_CAP && self.tick % 6 == 0 {
                let (x0, y0, x1, y1) = w.rect(); let (x0, y0, x1, y1) = (x0 as i32 - 1, y0 as i32 - 1, x1 as i32, y1 as i32);
                let ring = (x0..=x1).flat_map(|c| [(c, y0), (c, y1)]).chain((y0..=y1).flat_map(|r| [(x0, r), (x1, r)]));
                if let Some((c, r)) = ring.filter(|&(c, r)| self.g.in_bounds(c, r)).find(|&(c, r)| self.g.get(c as usize, r as usize) == Mat::Water) { let i = self.g.idx(c as usize, r as usize); self.g.set(i, Mat::Empty); w.water += 1.; }
            }
            // loose things falling in through its top (not a lit charge)
            if matches!(w.kind, WagonKind::Store | WagonKind::Coal) { let (x0, y0, x1, y1) = w.rect(); let mut i = 0;
                while i < self.drops.len() { let d = &self.drops[i];
                    if d.fuse == 0 && d.px >= x0 && d.px < x1 && d.py >= y0 - 1. && d.py < y1 && w.takes(d.item, d.n) { let d = self.drops.swap_remove(i); w.add(d.item, d.n); } else { i += 1; } } }
            self.shove_from_wagon(w.x, w.y);
            // a parted wagon may couple again once it's been pulled apart
            if w.parted && !hauled[j] && (0..self.tr.borers.len()).all(|k| { let (rx, _) = self.train_tail(k, &trains[k]); (rx - w.x).abs() > WAGON_W / 2. + COUPLE_D + 1. }) { w.parted = false; }
            self.tr.wagons[j] = w;
        }
        // coupling on: a loose wagon whose front has come up to a train's tail
        for k in 0..self.tr.borers.len() {
            let (rx, ry) = self.train_tail(k, &trains[k]); let d = self.tr.borers[k].dir as f32;
            for j in 0..self.tr.wagons.len() {
                let w = &self.tr.wagons[j]; if hauled[j] || w.parted || (w.y - ry).abs() > S as f32 { continue; }
                let gap = (rx - (w.x + d * WAGON_W / 2.)) * d;
                if gap < -1. || gap > COUPLE_D { continue; }
                let nx = rx - d * (GAP + WAGON_W / 2.);
                if self.wagon_hits(nx, w.y) { continue; }
                let w = &mut self.tr.wagons[j]; w.x = nx; w.coupled = true; let kind = w.kind;
                self.say(format!("Clank: the {} wagon couples on.", kind.name())); self.events.push(Event::Place);
                break; // (one a tick: the next couples to it)
            }
        }
        // the coal and water wagons feed the borer they're coupled to
        for (k, t) in trains.iter().enumerate() { for &j in t {
            match self.tr.wagons[j].kind {
                WagonKind::Coal => if self.tr.borers[k].fuel.is_empty() && self.tr.borers[k].burning.is_none() && self.tr.borers[k].steam < STOKE_BELOW * crate::borer::STEAM_MAX { let w = &mut self.tr.wagons[j];
                    if let Some(c) = w.cargo.first_mut() { let it = c.0; c.1 -= 1; if c.1 == 0 { w.cargo.remove(0); } self.tr.borers[k].fuel.push((it, 1)); } },
                WagonKind::Water => if self.tick % 6 == 3 && self.tr.wagons[j].water >= 1. && self.tr.borers[k].water + 1. <= crate::borer::TANK { self.tr.wagons[j].water -= 1.; self.tr.borers[k].water += 1.; },
                _ => {}
            } } }
        self.push_wagons(&hauled);
    }
    /// The mole walking into a loose wagon shoves it along (no faster than `PUSH_V`), held at its side.
    fn push_wagons(&mut self, hauled: &[bool]) {
        if self.tr.wagon_ride.is_some() || self.tr.borer_ride.is_some() || self.tr.riding.is_some() || self.moles.is_empty() || !self.moles[0].alive { return; }
        let dt = 1. / 60.; let (px, py) = (self.pl.px, self.pl.py);
        for j in 0..self.tr.wagons.len() {
            if hauled.get(j).copied().unwrap_or(false) { continue; }
            let w = &self.tr.wagons[j]; let (x0, y0, x1, y1) = w.rect();
            if !(py > y0 + 1. && py - crate::world::BODY_H < y1 && px + BODY_HW > x0 && px - BODY_HW < x1) { continue; }
            let side = if px < w.x { -1. } else { 1. };
            if self.pl.vx * -side <= 0. { continue; }
            let nx = w.x + self.pl.vx.clamp(-PUSH_V, PUSH_V) * dt;
            if !self.wagon_hits(nx, w.y) { self.tr.wagons[j].x = nx; }
            let w = &self.tr.wagons[j]; let (x0, _, x1, _) = w.rect();
            self.pl.px = if side < 0. { x0 - BODY_HW } else { x1 + BODY_HW }; self.pl.vx = self.pl.vx.clamp(-PUSH_V, PUSH_V);
            let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y));
        }
    }

    pub(crate) fn shift_wagons(&mut self, rows: i64, h: usize) {
        let dy = rows as f32; let riding = self.tr.wagon_ride; let mut keep = vec![]; let mut nr = None;
        for (j, mut w) in std::mem::take(&mut self.tr.wagons).into_iter().enumerate() { w.y -= dy; w.py -= dy; if w.y < WAGON_H + 2. || w.y >= h as f32 - 2. { continue; } if riding == Some(j) { nr = Some(keep.len()); } keep.push(w); }
        self.tr.wagons = keep; self.tr.wagon_ride = nr; if nr.is_none() { self.tr.bench = None; }
    }
    pub(crate) fn encode_wagons(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.wagons.len() as u32);
        for w in &self.tr.wagons { o.u8(w.kind as u8); o.f32(w.x); o.f32(w.y); o.u8(w.coupled as u8 | (w.parted as u8) << 1);
            o.u32(w.cargo.len() as u32); for &(it, n) in &w.cargo { o.u8(it as u8); o.u32(n); } o.f32(w.water); }
    }
    pub(crate) fn decode_wagons(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (gw, gh) = (self.g.w as f32, self.g.h as f32); let ni = crate::craft::NITEM;
        for _ in 0..r.len(4096)? {
            let (k, x, y, fl) = (r.u8()?, r.f32()?, r.f32()?, r.u8()?);
            let mut cargo = vec![]; for _ in 0..r.len(ni)? { let (it, n) = (r.u8()?, r.u32()?); if (it as usize) < ni { cargo.push((crate::craft::ITEMS[it as usize], n)); } }
            let water = r.f32()?;
            if let Some(&kind) = WAGON_KINDS.get(k as usize) { if x > 2. && x < gw - 2. && y > WAGON_H + 1. && y < gh - 1. && water.is_finite() {
                self.tr.wagons.push(Wagon { kind, x, y, px: x, py: y, vy: 0., coupled: fl & 1 != 0, parted: fl & 2 != 0, cargo, water: water.clamp(0., WATER_CAP) }); } }
        }
        Ok(())
    }
}
