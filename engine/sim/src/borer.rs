//! The steam borer: a rideable boring machine. It carries its own firebox and water tank (as a boiler does, steam.rs),
//! drives on its wheels, and grinds the rock in front of its bore head. What it cuts goes back along its spoil belt and
//! is dropped as loose rubble in the open tunnel behind it, so a borer can't dig into a dead end with nowhere to put
//! the spoil: it jams. Nothing is stored in it but its fuel, its water and its steam.
//!
//! * **Fuel:** right mouse with coal, charcoal or wood in the paw puts a lump from the pack into its firebox; a lump
//!   burns at a time (`steam::burn_ticks`) giving `FIRE_W`, raising steam while it has water.
//! * **Water:** it draws water cells touching it into its tank (`TANK` cells), so it fills up standing in or by water.
//! * **Riding:** right mouse with an empty paw gets in; Space gets out. A/D drive it (`DRIVE_W`, up to `BORER_V`), and it
//!   climbs a step of up to a tile. It bores the way it faces (set when it's put down): driving into rock, its head grinds
//!   the face (`GRIND_W`; a column of the face in `GRIND_S`·the rock's dig time seconds, so dirt goes fast and basalt
//!   slowly; nothing cuts bedrock).
//! * **Spoil:** each cell cut becomes loose rubble of what it crumbles to (ore stays ore) in the nearest open cell up to
//!   `SPOIL_M` behind it; with no room there it jams and says so.
//! * Ash from its firebox drops behind it as loose items.
use crate::craft::Item;
use crate::mat::{Kind, Mat, CPM, S};
use crate::steam::{burn_ticks, BOILER_EFF, FIRE_W, WATER_KJ};
use crate::world::{Drop, Event, World, BODY_H};

pub const BORER_W: f32 = (3 * CPM / 2) as f32;
pub const BORER_H: f32 = CPM as f32;
pub const TANK: f32 = 16.;
pub const STEAM_MAX: f32 = 300.;
/// Driving (W) and its top speed (cells/s, 0.5 m/s); grinding (W) and seconds per column of face per unit of dig time.
pub const DRIVE_W: f32 = 300.; pub const BORER_V: f32 = 0.5 * CPM as f32;
pub const GRIND_W: f32 = 1500.; pub const GRIND_S: f32 = 0.15;
pub const SPOIL_M: usize = 4;

#[derive(Clone, Debug)]
pub struct Borer {
    /// its middle and the row its wheels stand on (cells), last tick's (for drawing), speed (cells/s), fall speed
    pub x: f32, pub y: f32, pub px: f32, pub py: f32, pub vx: f32, pub vy: f32,
    /// the way its bore head faces
    pub dir: i8,
    pub fuel: Vec<(Item, u32)>, pub burning: Option<Item>, pub burn: u32,
    /// water in its tank (cells), steam held (kJ), progress through the face column in front (0..1), jammed
    pub water: f32, pub steam: f32, pub grind: f32, pub jammed: bool,
}
impl Borer {
    pub fn rect(&self) -> (f32, f32, f32, f32) { (self.x - BORER_W / 2., self.y - BORER_H, self.x + BORER_W / 2., self.y) }
    pub fn contains(&self, x: f32, y: f32) -> bool { let (x0, y0, x1, y1) = self.rect(); x >= x0 && x < x1 && y >= y0 && y < y1 }
    pub fn lumps(&self) -> u32 { self.fuel.iter().map(|f| f.1).sum::<u32>() + self.burning.is_some() as u32 }
}

impl World {
    fn borer_cell_blocks(&self, c: i32, r: i32) -> bool { !self.g.in_bounds(c, r) || !self.passable(c as usize, r as usize) && !self.borer_shoves(c, r) }
    /// Loose grains (sand, rubble, its own spoil) don't stop it: its wheels shove them back.
    fn borer_shoves(&self, c: i32, r: i32) -> bool { let i = self.g.idx(c as usize, r as usize); let m = self.g.m(i); m.kind() == Kind::Powder || m.kind() == Kind::Solid && self.g.loose(i) }
    /// Cells of its body at (x, y) that are blocked.
    fn borer_hits(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = ((x - BORER_W / 2.).floor() as i32, (x + BORER_W / 2. - 1e-3).floor() as i32);
        let (y0, y1) = ((y - BORER_H).floor() as i32, (y - 1e-3).floor() as i32);
        (y0..=y1).any(|r| (x0..=x1).any(|c| self.borer_cell_blocks(c, r)))
    }
    /// Is there ground under any of its wheels?
    fn borer_grounded(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = ((x - BORER_W / 2. + 1.).floor() as i32, (x + BORER_W / 2. - 1.).floor() as i32);
        (x0..=x1).any(|c| self.borer_cell_blocks(c, y.floor() as i32))
    }

    /// Put a borer down on the floor under (x, y), its head facing the way the mole faces.
    pub(crate) fn place_borer(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let xf = x as f32 + 0.5;
        let floor = (y..(y + 2 * CPM).min(self.g.h - 1)).find(|&r| self.borer_grounded(xf, r as f32)).ok_or("needs a floor under it")? as f32;
        if self.borer_hits(xf, floor) { return Err("needs 1.5 m by 1 m of room"); }
        if self.tr.wagons.iter().any(|w| (w.x - xf).abs() < (BORER_W + crate::wagon::WAGON_W) / 2. && (w.y - floor).abs() < BORER_H) { return Err("there's a wagon there"); }
        let dir = self.moles.first().map_or(1, |m| m.facing);
        self.tr.borers.push(Borer { x: xf, y: floor, px: xf, py: floor, vx: 0., vy: 0., dir, fuel: vec![], burning: None, burn: 0, water: 0., steam: 0., grind: 0., jammed: false });
        self.events.push(Event::Place); Ok(())
    }
    pub fn borer_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.borers.iter().position(|b| b.contains(xf, yf)) }
    /// Right mouse on a borer: a lump of fuel from the pack (fuel in the paw), or get in / out (empty paw).
    pub(crate) fn use_borer(&mut self, x: usize, y: usize) -> bool {
        let Some(k) = self.borer_at(x, y) else { return false };
        if let Some(it) = self.gear.held.filter(|&it| burn_ticks(it).is_some() && self.inv[it as usize] > 0) {
            self.inv[it as usize] -= 1; let b = &mut self.tr.borers[k];
            if let Some(f) = b.fuel.iter_mut().find(|f| f.0 == it) { f.1 += 1; } else { b.fuel.push((it, 1)); }
            let n = self.tr.borers[k].lumps(); self.say(format!("Fed the borer's firebox: {n} lumps in it."));
        } else if self.tr.borer_ride == Some(k) { self.leave_borer(); }
        else { self.tr.borer_ride = Some(k); self.say("On the borer. A/D: drive and bore · Space: get off.".into()); }
        self.events.push(Event::Click); true
    }
    pub(crate) fn leave_borer(&mut self) {
        if let Some(k) = self.tr.borer_ride.take() { let b = &self.tr.borers[k]; let (x0, y0, _, _) = b.rect();
            self.pl.px = b.x; self.pl.py = y0; self.pl.vx = 0.; self.pl.vy = 0.; self.pl.sync = None;
            if self.box_hits(self.pl.px, self.pl.py) { self.pl.px = x0 - crate::world::BODY_HW - 0.5; self.pl.py = b.y; }
            let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); }
    }
    /// `E` on a borer: its unburnt fuel back into the pack; empty, cold and with nobody on it, it comes up itself.
    pub(crate) fn pick_up_borer(&mut self, x: usize, y: usize) -> Option<Item> {
        let k = self.borer_at(x, y)?;
        if !self.tr.borers[k].fuel.is_empty() { for (it, n) in std::mem::take(&mut self.tr.borers[k].fuel) { self.inv[it as usize] += n; } self.say("Raked the fuel out of the borer.".into()); return Some(Item::SteamBorer); }
        if self.tr.borer_ride == Some(k) || self.tr.borers[k].burning.is_some() { return None; }
        self.tr.borers.remove(k); if let Some(r) = self.tr.borer_ride { if r > k { self.tr.borer_ride = Some(r - 1); } }
        self.inv[Item::SteamBorer as usize] += 1; Some(Item::SteamBorer)
    }

    /// The mole on a borer: carried on its top; A/D drive it, Space gets off. True while riding.
    pub(crate) fn borer_ride_step(&mut self) -> bool {
        let Some(k) = self.tr.borer_ride else { return false };
        if k >= self.tr.borers.len() || !self.moles[0].alive { self.tr.borer_ride = None; return false; }
        let jump = self.input.jump && !self.pl.jump_was; self.pl.jump_was = self.input.jump;
        if jump { self.leave_borer(); return true; }
        let b = &self.tr.borers[k];
        self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py;
        self.pl.px = b.x - b.dir as f32 * BORER_W * 0.25; self.pl.py = b.y - BORER_H; self.pl.vx = b.vx; self.pl.vy = 0.; self.pl.grounded = true; self.pl.cling = 0;
        let _ = BODY_H;
        let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); self.pl.fall_from = y;
        true
    }

    /// Put a spoil cell of `m` (loose) in the nearest open cell behind a borer facing `dir` whose back is at `rear`,
    /// across the rows it spans. False if there's no room within `SPOIL_M`.
    fn spoil(&mut self, m: Mat, rear: f32, y0: i32, y1: i32, dir: i8) -> bool {
        // (half a metre back from its tail, clear of its wheels)
        let d = -(dir as i32); let start = if dir > 0 { rear.floor() as i32 - 1 - (CPM / 2) as i32 } else { rear.ceil() as i32 + (CPM / 2) as i32 };
        for k in 0..(SPOIL_M * CPM) as i32 { let c = start + d * k;
            for r in (y0..=y1).rev() { if self.g.in_bounds(c, r) && self.g.get(c as usize, r as usize) == Mat::Empty {
                let i = self.g.idx(c as usize, r as usize); self.g.set(i, m); if m.kind() == Kind::Solid { self.g.set_loose(i); } return true; } } }
        false
    }

    /// One tick of every borer.
    pub(crate) fn step_borers(&mut self) {
        let dt = 1. / 60.; let (gw, gh) = (self.g.w, self.g.h);
        for k in 0..self.tr.borers.len() {
            let mut b = self.tr.borers[k].clone(); b.px = b.x; b.py = b.y;
            let riding = self.tr.borer_ride == Some(k);
            // its train of wagons (wagon.rs): they cost more to drive, and its spoil belt runs back over them to the tail
            let train = self.train(k); let (tail, _) = self.train_tail(k, &train); let (ox, oy) = (b.x, b.y);
            // the firebox and the tank
            if b.burning.is_none() { if let Some(f) = b.fuel.first_mut() { let it = f.0; f.1 -= 1; if f.1 == 0 { b.fuel.remove(0); } b.burning = Some(it); b.burn = burn_ticks(it).unwrap_or(0); } }
            if let Some(_) = b.burning { b.burn = b.burn.saturating_sub(1);
                if b.water > 0. && b.steam < STEAM_MAX { let kj = FIRE_W * BOILER_EFF * dt / 1000.; b.steam = (b.steam + kj).min(STEAM_MAX); b.water = (b.water - kj / WATER_KJ).max(0.); }
                if b.burn == 0 { b.burning = None; let (x0, _, x1, y1) = b.rect(); let ax = if b.dir > 0 { x0 - 1.5 } else { x1 + 1.5 }; self.drops.push(Drop::at(ax, y1 - 1.5, Item::Ash, 1)); } }
            if b.water + 1. <= TANK && self.tick % 6 == 0 {
                // (round its body and along the floor under its wheels)
                let (x0, y0, x1, y1) = b.rect(); let (x0, y0, x1, y1) = (x0 as i32 - 1, y0 as i32 - 1, x1 as i32, y1 as i32);
                let ring = (x0..=x1).flat_map(|c| [(c, y0), (c, y1)]).chain((y0..=y1).flat_map(|r| [(x0, r), (x1, r)]));
                if let Some((c, r)) = ring.filter(|&(c, r)| self.g.in_bounds(c, r)).find(|&(c, r)| self.g.get(c as usize, r as usize) == Mat::Water) { let i = self.g.idx(c as usize, r as usize); self.g.set(i, Mat::Empty); b.water += 1.; }
            }
            // falling
            if !self.borer_grounded(b.x, b.y) { b.vy = (b.vy + crate::world::TUNE.gravity * dt).min(crate::world::TUNE.terminal);
                let ny = b.y + b.vy * dt; if !self.borer_hits(b.x, ny) { b.y = ny; } else { b.vy = 0.; b.y = b.y.floor(); } }
            // tipping: with nothing under its middle half and a real drop (deeper than a tile) on one side, its weight takes it
            // over the edge: it slides that way until it falls (a step under it is no drop: it rests on it)
            else if let Some(s) = self.borer_tips(&b) { let nx = b.x + s * BORER_V * 2. * dt; if !self.borer_hits(nx, b.y) { b.x = nx; } }
            else { b.vy = 0.; }
            // driving: the throttle from the rider's keys, paid in steam
            let throttle = if riding { self.input.right as i32 - self.input.left as i32 } else { 0 } as f32;
            let want = throttle * BORER_V;
            let drive_kj = (DRIVE_W + crate::wagon::WAGON_DRIVE_W * train.len() as f32) * dt / 1000.;
            b.vx = if throttle != 0. && b.steam >= drive_kj { b.steam -= drive_kj; want } else { 0. };
            if b.vx != 0. {
                let nx = b.x + b.vx * dt;
                if !self.borer_hits(nx, b.y) { b.x = nx; b.jammed = false; }
                // a step of up to a tile: it climbs it
                else if self.front_low_only(&b, nx) && !self.borer_hits(nx, b.y - S as f32) { b.x = nx; b.y -= S as f32; }
                else if (b.vx > 0.) == (b.dir > 0) {
                    // the head meets the face: grind the column in front, a column at a time
                    // (the column its front edge is about to move into)
                    let face = if b.dir > 0 { (nx + BORER_W / 2. - 1e-3).floor() as i32 } else { (nx - BORER_W / 2.).floor() as i32 };
                    let (y0, y1) = ((b.y - BORER_H).floor() as i32, (b.y - 1e-3).floor() as i32);
                    let col: Vec<(i32, Mat)> = (y0..=y1).filter(|&r| self.g.in_bounds(face, r)).map(|r| (r, self.g.get(face as usize, r as usize))).filter(|&(r, m)| self.borer_cell_blocks(face, r) && m != Mat::Empty).collect();
                    let hard = col.iter().map(|&(_, m)| if m == Mat::Bedrock { f32::INFINITY } else { m.p().dig as f32 }).fold(1f32, f32::max);
                    let grind_kj = GRIND_W * dt / 1000.;
                    if hard.is_finite() && b.steam >= grind_kj {
                        b.steam -= grind_kj; b.grind += dt / (GRIND_S * hard);
                        if self.tick % 10 == 0 { self.shake = self.shake.max(0.05); self.events.push(Event::Click); }
                        if b.grind >= 1. {
                            let rear = tail;
                            let mut cut_all = true;
                            for &(r, m) in &col { let i = self.g.idx(face as usize, r as usize);
                                let spoil = if m.kind() == Kind::Solid && !self.g.loose(i) { m.p().crumbles_to } else { m };
                                if self.spoil(spoil, rear, y0, y1, b.dir) { self.g.set(i, Mat::Empty); } else { cut_all = false; break; } }
                            if cut_all { b.grind = 0.; b.jammed = false; } else if !b.jammed { b.jammed = true; b.grind = 1.; self.say("The borer is jammed: no room behind it for its spoil.".into()); }
                        }
                    } else if !hard.is_finite() && self.tick % 120 == 0 && riding { self.say("The borer's head won't cut bedrock.".into()); }
                }
            }
            // loose grains it has rolled over are shoved back along its spoil belt
            { let (x0, y0, x1, y1) = b.rect(); let rear = tail;
                for r in y0.floor() as i32..y1.ceil() as i32 { for c in x0.floor() as i32..x1.ceil() as i32 {
                    if self.g.in_bounds(c, r) && !self.g.get(c as usize, r as usize).passable() && self.borer_shoves(c, r) {
                        let i = self.g.idx(c as usize, r as usize); let m = self.g.m(i);
                        if self.spoil(m, rear, y0 as i32, y1 as i32 - 1, b.dir) { self.g.set(i, Mat::Empty); } } } } }
            if b.x < 2. || b.x > gw as f32 - 2. || b.y > gh as f32 - 2. { b.x = b.x.clamp(2., gw as f32 - 2.); b.y = b.y.min(gh as f32 - 2.); }
            // the train follows on its coupling bars; if any wagon can't, the borer can't move either
            if !train.is_empty() && b.x != ox {
                match self.haul(b.x, b.dir, &train) {
                    Some(pos) => for (&j, (x, y)) in train.iter().zip(pos) { let w = &mut self.tr.wagons[j]; w.x = x; w.y = y; },
                    None => { b.x = ox; if b.y < oy { b.y = oy; } b.vx = 0.; if riding && self.tick % 120 == 0 { self.say("The train is stuck: a wagon behind can't follow.".into()); } }
                }
            }
            self.tr.borers[k] = b;
        }
    }
    /// Which way it tips, if its weight is over a drop: no ground under its middle half, and on that side open space more
    /// than a tile deep under its outer edge.
    fn borer_tips(&self, b: &Borer) -> Option<f32> {
        let r = b.y.floor() as i32; let ground = |c: i32| self.borer_cell_blocks(c, r);
        let (m0, m1) = ((b.x - BORER_W / 4.).floor() as i32, (b.x + BORER_W / 4. - 1e-3).floor() as i32);
        if (m0..=m1).any(ground) { return None; }
        [1f32, -1.].into_iter().find(|&s| { let edge = if s > 0. { (b.x + BORER_W / 2. - 1.).floor() as i32 } else { (b.x - BORER_W / 2.).floor() as i32 };
            !(0..=S as i32 + 1).any(|k| self.borer_cell_blocks(edge, r + k)) })
    }
    /// Is only the bottom tile of its front blocked at nx (a step it can climb)?
    fn front_low_only(&self, b: &Borer, nx: f32) -> bool {
        let (x0, x1) = ((nx - BORER_W / 2.).floor() as i32, (nx + BORER_W / 2. - 1e-3).floor() as i32);
        let (top, step) = ((b.y - BORER_H).floor() as i32, (b.y - S as f32).floor() as i32);
        (top..step).all(|r| (x0..=x1).all(|c| !self.borer_cell_blocks(c, r)))
    }

    pub(crate) fn shift_borers(&mut self, rows: i64, h: usize) {
        let dy = rows as f32; let riding = self.tr.borer_ride; let mut keep = vec![]; let mut nr = None;
        for (k, mut b) in std::mem::take(&mut self.tr.borers).into_iter().enumerate() { b.y -= dy; b.py -= dy; if b.y < BORER_H + 2. || b.y >= h as f32 - 2. { continue; } if riding == Some(k) { nr = Some(keep.len()); } keep.push(b); }
        self.tr.borers = keep; self.tr.borer_ride = nr;
    }
    pub(crate) fn encode_borers(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.borers.len() as u32);
        for b in &self.tr.borers { o.f32(b.x); o.f32(b.y); o.u8((b.dir + 1) as u8); o.u32(b.fuel.len() as u32); for &(it, n) in &b.fuel { o.u8(it as u8); o.u32(n); }
            o.u8(b.burning.map_or(255, |it| it as u8)); o.u32(b.burn); o.f32(b.water); o.f32(b.steam); }
    }
    pub(crate) fn decode_borers(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (gw, gh) = (self.g.w as f32, self.g.h as f32); let ni = crate::craft::NITEM;
        for _ in 0..r.len(4096)? {
            let (x, y, dir) = (r.f32()?, r.f32()?, r.u8()? as i8 - 1);
            let mut fuel = vec![]; for _ in 0..r.len(ni)? { let (it, n) = (r.u8()?, r.u32()?); if (it as usize) < ni { fuel.push((crate::craft::ITEMS[it as usize], n)); } }
            let bi = r.u8()?; let burning = ((bi as usize) < ni).then(|| crate::craft::ITEMS[bi as usize]);
            let (burn, water, steam) = (r.u32()?, r.f32()?, r.f32()?);
            if x > 2. && x < gw - 2. && y > BORER_H + 1. && y < gh - 1. && water.is_finite() && steam.is_finite() {
                self.tr.borers.push(Borer { x, y, px: x, py: y, vx: 0., vy: 0., dir: if dir < 0 { -1 } else { 1 }, fuel, burning, burn, water: water.clamp(0., TANK), steam: steam.clamp(0., STEAM_MAX), grind: 0., jammed: false }); }
        }
        Ok(())
    }
}
