//! Wagons: the steam borer's body segments (borer.rs), a mobile base for deep expeditions. A loose wagon set down
//! or shoved up to the tail of a borer's train (within `COUPLE_R` of where the next segment would ride) couples on, and
//! from then on it follows the borer's head along its path like a segment of a worm, wherever it bores: up, down,
//! round curves. Each wagon in a train costs the borer `WAGON_DRIVE_W` more to drive.
//!
//! * **Store wagon:** a box on wheels. Loose items that fall into it ride along, and so does the ore and crystal the
//!   borer's sieve keeps; inside it, right mouse with an item in paw stows that stack. `E` takes everything out.
//! * **Coal wagon:** a fuel bunker. Coal, charcoal and wood thrown or dropped in ride along, or shovelled in by hand
//!   (fuel in paw + right mouse), and the coal the borer cuts goes in first. Coupled, its stoking chute feeds the
//!   borer's firebox a lump at a time, while the borer's steam is below `STOKE_BELOW` of full: with steam up, a catch
//!   on the chute holds it shut, so a parked borer doesn't burn the bunker away.
//! * **Bunk wagon:** climbing in is going to sleep: you wake there after a death, and the game saves.
//! * **Workshop wagon:** a saw bench under a roof. Inside it, you work as at a workbench.
//!
//! Right mouse (empty paw) climbs in or out of any wagon; Space gets out. The borer is steered from its cab only.
//! `E` on a wagon takes its load out, then uncouples it if it is the last of its train (it won't couple again until it
//! has been pulled apart), then picks it up. Nothing in a wagon is anywhere but in the wagon: it is all hauled with it.
use crate::craft::{Item, Station};
use crate::mat::{Kind, Mat, CPM, S};
use crate::steam::burn_ticks;
use crate::world::{Building, Event, World, BODY_HW};

pub const WAGON_W: f32 = CPM as f32;
pub const WAGON_H: f32 = (3 * CPM / 4) as f32;
/// The coupling bar: the gap between a wagon and what pulls it (cells); how close its front must come to couple on.
pub const GAP: f32 = S as f32; pub const COUPLE_D: f32 = (3 * S) as f32 / 2. + 1.;
/// How near a loose wagon's middle must come to the train's next segment place to couple on (cells).
pub const COUPLE_R: f32 = CPM as f32;
/// What hauling a wagon costs the borer (W); what the stores hold (items, lumps).
pub const WAGON_DRIVE_W: f32 = 100.;
/// The coal wagon stokes only while the borer's steam is below this share of full.
pub const STOKE_BELOW: f32 = 0.5;
pub const STORE_CAP: u32 = 200; pub const COAL_CAP: u32 = 40;
/// How fast the mole can shove a wagon along by hand (cells/s).
pub const PUSH_V: f32 = 1.5 * CPM as f32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WagonKind { Store, Workshop, Bunk, Coal }
pub const WAGON_KINDS: [WagonKind; 4] = [WagonKind::Store, WagonKind::Workshop, WagonKind::Bunk, WagonKind::Coal];
impl WagonKind {
    pub fn item(self) -> Item { [Item::StoreWagon, Item::WorkshopWagon, Item::BunkWagon, Item::CoalWagon][self as usize] }
    pub fn of(it: Item) -> Option<WagonKind> { WAGON_KINDS.into_iter().find(|k| k.item() == it) }
    pub fn name(self) -> &'static str { ["store", "workshop", "bunk", "coal"][self as usize] }
    /// A wagon kind by its number as saved; `old` for transport blobs before 13, when 3 was the water wagon (it comes
    /// back as a store wagon, its water gone) and 4 the coal wagon.
    fn from_save(k: u8, old: bool) -> Option<WagonKind> { if old && k == 3 { return Some(WagonKind::Store); } WAGON_KINDS.get(if old && k > 3 { k as usize - 1 } else { k as usize }).copied() }
}

#[derive(Clone, Debug)]
pub struct Wagon {
    pub kind: WagonKind,
    /// its own number (a borer's train lists its wagons by it)
    pub id: u32,
    /// its middle and the row its wheels stand on (cells), last tick's (for drawing), fall speed
    pub x: f32, pub y: f32, pub px: f32, pub py: f32, pub vy: f32,
    /// its coupling bar is hooked on; it was uncoupled and won't hook on again until pulled apart
    pub coupled: bool, pub parted: bool,
    /// what it carries (the store and the coal wagon)
    pub cargo: Vec<(Item, u32)>,
    /// the way it lies (radians, as the path runs; 0 level), for drawing
    pub ang: f32,
}
impl Wagon {
    pub fn rect(&self) -> (f32, f32, f32, f32) { (self.x - WAGON_W / 2., self.y - WAGON_H, self.x + WAGON_W / 2., self.y) }
    pub fn contains(&self, x: f32, y: f32) -> bool { let (x0, y0, x1, y1) = self.rect(); x >= x0 && x < x1 && y >= y0 && y < y1 }
    pub fn load(&self) -> u32 { self.cargo.iter().map(|c| c.1).sum() }
    pub(crate) fn add(&mut self, it: Item, n: u32) { if let Some(c) = self.cargo.iter_mut().find(|c| c.0 == it) { c.1 += n; } else { self.cargo.push((it, n)); } }
    /// Can it take a loose `it` in through its top?
    pub(crate) fn takes(&self, it: Item, n: u32) -> bool {
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
        if self.tr.borers.iter().any(|b| b.contains(xf, floor - WAGON_H / 2.)) { return Err("there's a borer there"); }
        let id = self.next_wagon_id();
        self.tr.wagons.push(Wagon { kind, id, x: xf, y: floor, px: xf, py: floor, vy: 0., coupled: false, parted: false, cargo: vec![], ang: 0. });
        self.events.push(Event::Place); Ok(())
    }
    pub fn wagon_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.wagons.iter().position(|w| w.contains(xf, yf)) }

    fn next_wagon_id(&self) -> u32 { self.tr.wagons.iter().map(|w| w.id + 1).max().unwrap_or(1) }
    /// The wagons borer `k` hauls, front to back (its segments, as wagon indices).
    pub fn train(&self, k: usize) -> Vec<usize> {
        let Some(b) = self.tr.borers.get(k) else { return vec![] };
        b.segs.iter().filter_map(|&id| self.tr.wagons.iter().position(|w| w.id == id && w.coupled)).collect()
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
                WagonKind::Bunk => { let w = &self.tr.wagons[j]; self.pl.spawn = (self.ox + w.x as i64, self.oy + (w.y - 1.) as i64); self.tr.slept = true;
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
    /// `E` on a wagon: its load into the pack; then uncouple it; then, empty and with nobody in it, it comes up itself.
    pub(crate) fn pick_up_wagon(&mut self, x: usize, y: usize) -> Option<Item> {
        let j = self.wagon_at(x, y)?; let kind = self.tr.wagons[j].kind; let it = kind.item();
        if !self.tr.wagons[j].cargo.is_empty() { let mut got = vec![];
            for (c, n) in std::mem::take(&mut self.tr.wagons[j].cargo) { self.inv[c as usize] += n; got.push(format!("{n} {}", c.name().to_lowercase())); }
            self.say(format!("Unloaded the {} wagon: {}.", kind.name(), got.join(", "))); return Some(it); }
        if self.tr.wagons[j].coupled {
            if let Some(k) = self.hauled_by(j) { let id = self.tr.wagons[j].id;
                if self.tr.borers[k].segs.last() != Some(&id) { self.say("Uncouple the wagons behind it first.".into()); return Some(it); }
                self.tr.borers[k].segs.pop(); }
            let w = &mut self.tr.wagons[j]; w.coupled = false; w.parted = true; w.ang = 0.;
            self.say(format!("Uncoupled the {} wagon.", kind.name())); self.events.push(Event::Click); return Some(it); }
        if self.tr.wagon_ride == Some(j) { return None; }
        self.tr.wagons.remove(j);
        if let Some(r) = self.tr.wagon_ride { if r > j { self.tr.wagon_ride = Some(r - 1); } }
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
            let mut w = self.tr.wagons[j].clone();
            // (a coupled wagon rides its borer's path: step_borers has moved it)
            if !hauled[j] { w.px = w.x; w.py = w.y; w.ang = 0.; if w.coupled { w.coupled = false; } }
            // falling
            if hauled[j] { w.vy = 0.; } else if !self.wagon_grounded(w.x, w.y) { w.vy = (w.vy + crate::world::TUNE.gravity * dt).min(crate::world::TUNE.terminal);
                let ny = w.y + w.vy * dt; if !self.wagon_hits(w.x, ny) { w.y = ny; } else { w.vy = 0.; w.y = w.y.floor(); } } else { w.vy = 0.; }
            // loose things falling in through its top (not a lit charge)
            if matches!(w.kind, WagonKind::Store | WagonKind::Coal) { let (x0, y0, x1, y1) = w.rect(); let mut i = 0;
                while i < self.drops.len() { let d = &self.drops[i];
                    if d.fuse == 0 && d.px >= x0 && d.px < x1 && d.py >= y0 - 1. && d.py < y1 && w.takes(d.item, d.n) { let d = self.drops.swap_remove(i); w.add(d.item, d.n); } else { i += 1; } } }
            self.shove_from_wagon(w.x, w.y);
            // a parted wagon may couple again once it's been pulled apart
            if w.parted && !hauled[j] && (0..self.tr.borers.len()).all(|k| { let (x, y) = self.next_seg(k); (x - w.x).powi(2) + (y - (w.y - WAGON_H / 2.)).powi(2) > (COUPLE_R + 4.).powi(2) }) { w.parted = false; }
            self.tr.wagons[j] = w;
        }
        // coupling on: a loose wagon whose middle has come up to where a train's next segment rides
        for k in 0..self.tr.borers.len() {
            let (x, y) = self.next_seg(k);
            let Some(j) = (0..self.tr.wagons.len()).find(|&j| { let w = &self.tr.wagons[j]; !hauled[j] && !w.parted && (x - w.x).powi(2) + (y - (w.y - WAGON_H / 2.)).powi(2) <= COUPLE_R * COUPLE_R }) else { continue };
            let n = self.tr.borers[k].segs.len(); let (px, py, a) = self.tr.borers[k].at(crate::borer::seg_dist(n));
            let w = &mut self.tr.wagons[j]; w.x = px; w.y = py + WAGON_H / 2.; w.ang = a; w.coupled = true; let (kind, id) = (w.kind, w.id);
            self.tr.borers[k].segs.push(id);
            self.say(format!("Clank: the {} wagon couples on.", kind.name())); self.events.push(Event::Place);
        }
        // a coal wagon feeds the borer it's coupled to
        for (k, t) in trains.iter().enumerate() { for &j in t {
            match self.tr.wagons[j].kind {
                WagonKind::Coal => if self.tr.borers[k].fuel.is_empty() && self.tr.borers[k].burning.is_none() && self.tr.borers[k].steam < STOKE_BELOW * crate::borer::STEAM_MAX { let w = &mut self.tr.wagons[j];
                    if let Some(c) = w.cargo.first_mut() { let it = c.0; c.1 -= 1; if c.1 == 0 { w.cargo.remove(0); } self.tr.borers[k].fuel.push((it, 1)); } },
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

    /// Where borer `k`'s next segment would ride (the middle of a wagon coupling on).
    fn next_seg(&self, k: usize) -> (f32, f32) { let b = &self.tr.borers[k]; let (x, y, _) = b.at(crate::borer::seg_dist(b.segs.len())); (x, y) }

    pub(crate) fn shift_wagons(&mut self, rows: i64, h: usize) {
        let dy = rows as f32; let riding = self.tr.wagon_ride; let mut keep = vec![]; let mut nr = None;
        for (j, mut w) in std::mem::take(&mut self.tr.wagons).into_iter().enumerate() { w.y -= dy; w.py -= dy; if w.y < WAGON_H + 2. || w.y >= h as f32 - 2. { continue; } if riding == Some(j) { nr = Some(keep.len()); } keep.push(w); }
        self.tr.wagons = keep; self.tr.wagon_ride = nr; if nr.is_none() { self.tr.bench = None; }
    }
    pub(crate) fn encode_wagons(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.wagons.len() as u32);
        for w in &self.tr.wagons { o.u8(w.kind as u8); o.f32(w.x); o.f32(w.y); o.u8(w.coupled as u8 | (w.parted as u8) << 1);
            o.u32(w.cargo.len() as u32); for &(it, n) in &w.cargo { o.u8(it as u8); o.u32(n); } o.u32(w.id); }
    }
    /// Since 14 each wagon is saved with its `id`; older ones are numbered as they load.
    pub(crate) fn decode_wagons(&mut self, r: &mut crate::band::Rd, ver: u8) -> Result<(), String> {
        let (gw, gh) = (self.g.w as f32, self.g.h as f32);
        for _ in 0..r.len(4096)? {
            let (k, x, y, fl) = (r.u8()?, r.f32()?, r.f32()?, r.u8()?);
            let mut cargo = vec![]; for _ in 0..r.len(4096)? { let (it, n) = (r.u8()?, r.u32()?); if let Some(it) = r.item(it) { cargo.push((it, n)); } }
            if r.old { r.f32()?; } // (before 13: the water wagon's water, now gone)
            let id = if ver >= 14 { r.u32()? } else { self.next_wagon_id() };
            if let Some(kind) = WagonKind::from_save(k, r.old) { if x > 2. && x < gw - 2. && y > WAGON_H + 1. && y < gh - 1. {
                self.tr.wagons.push(Wagon { kind, id, x, y, px: x, py: y, vy: 0., coupled: fl & 1 != 0, parted: fl & 2 != 0, cargo, ang: 0. }); } }
        }
        Ok(())
    }
}
