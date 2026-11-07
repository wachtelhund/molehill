//! The steam borer: a late-game boring machine that goes through the rock like a worm. A round, toothed bore head on a
//! steam cab, with its wagons (wagon.rs) coupled behind as body segments. It steers freely in every direction, up, down,
//! diagonally and round in curves, and grinds a smooth round tunnel `2 * HEAD_R` across wherever its head goes. The cab
//! and every wagon follow the head's own path at their spacing, so the whole train winds through the tunnel it cut.
//!
//! * **Fuel:** right mouse with coal, charcoal or wood in the paw puts a lump from the pack into its firebox; a lump
//!   burns at a time (`steam::burn_ticks`) giving `FIRE_W`, raising steam. It runs on fuel alone: it has no water tank
//!   to fill (it used to; saves from then load with the tank's water gone). A coupled coal wagon stokes it.
//! * **Riding:** right mouse with an empty paw climbs into its cab; Space climbs out (beside the cab, in the tunnel).
//!   In the cab the walking keys steer: A/D/W/S and their diagonals point the way to go, and the head turns toward it at
//!   up to `TURN` (so it swings round in curves, never snapping). Held, it runs up to `BORER_V`; let go, it stops.
//! * **Boring:** the head cuts every cell in its round face (anything but bedrock). How fast it can go through rock is
//!   what its cutting power `GRIND_W` can grind: a cell costs `CUT_KJ` per unit of the rock's dig time, so soft ground
//!   goes at full speed and granite at about 2 m/s. Nothing cuts bedrock: the head stops against it (and can turn away).
//! * **Spoil:** its head grinds the rock fine and rams the grit into the tunnel wall round it, so the tunnel stays open
//!   behind it. What's worth keeping comes off its sieve: coal, ore, quartz and crystal go into the first wagon of the
//!   train with room (coal into a coal wagon first), or are dropped behind the cab if no wagon has room.
//! * **Grip:** it holds itself in its tunnel by its walls. Out in the open with nothing within reach of its head or cab
//!   (and no wagon standing on anything), it falls, and the train pours down its path after it.
//! * Ash from its firebox drops behind the cab as loose items.
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};
use crate::craft::Item;
use crate::light;
use crate::mat::{Mat, CPM, NMAT, S};
use crate::steam::{burn_ticks, BOILER_EFF, FIRE_W};
use crate::wagon::{GAP, WAGON_H, WAGON_W};
use crate::world::{Drop, Event, World, BODY_H};

/// The round tunnel's radius (cells): 1.5 m across.
pub const HEAD_R: f32 = (3 * CPM / 4) as f32;
/// The cab: its length and height (cells), and how far its middle rides behind the head's.
pub const BORER_W: f32 = (3 * CPM / 2) as f32;
pub const BORER_H: f32 = CPM as f32;
pub const CAB_D: f32 = HEAD_R + 1.5;
/// From one wagon's middle to the next along the path (cells).
pub const SEG_D: f32 = WAGON_W + GAP;
pub const STEAM_MAX: f32 = 300.;
/// Top speed (cells/s: 5 m/s), how fast it gets there and stops (cells/s²), how fast its head turns (rad/s: at top
/// speed a curve about 1.25 m round, tighter going slower).
pub const BORER_V: f32 = 5. * CPM as f32;
pub const ACCEL: f32 = 4. * BORER_V;
pub const TURN: f32 = 4.;
/// Driving at top speed (W, less going slower); the head's cutting power (W) and what a cell costs it (kJ per unit
/// of the rock's dig time).
pub const DRIVE_W: f32 = 600.;
pub const GRIND_W: f32 = 5000.;
pub const CUT_KJ: f32 = 0.0012;

/// Where the middle of wagon `n` of the train (0 the first behind the cab) rides, as path length behind the head.
pub fn seg_dist(n: usize) -> f32 { CAB_D + BORER_W / 2. + GAP + WAGON_W / 2. + n as f32 * SEG_D }

#[derive(Clone, Debug)]
pub struct Borer {
    /// the head's middle (cells), last tick's (for drawing), the way it heads (radians; 0 east, +y is down), its speed
    /// (cells/s) and fall speed
    pub x: f32, pub y: f32, pub px: f32, pub py: f32, pub ang: f32, pub v: f32, pub vy: f32,
    /// where the head has been, newest first (a point each cell or so; as long as the train needs)
    pub path: VecDeque<(f32, f32)>,
    /// the wagons coupled behind it, front to back (`Wagon::id`)
    pub segs: Vec<u32>,
    pub fuel: Vec<(Item, u32)>, pub burning: Option<Item>, pub burn: u32,
    /// steam held (kJ); what the rock in front has been costing to cut (dig time per cell of advance, smoothed); the
    /// cuttings toward the next item of each material
    pub steam: f32, pub hard: f32, pub acc: Vec<u16>,
    /// the cab's middle and angle (from the path; for drawing, clicks and the rider)
    pub cx: f32, pub cy: f32, pub cang: f32,
}
impl Borer {
    pub fn new(x: f32, y: f32, ang: f32) -> Borer {
        let mut b = Borer { x, y, px: x, py: y, ang, v: 0., vy: 0., path: VecDeque::new(), segs: vec![], fuel: vec![], burning: None, burn: 0, steam: 0., hard: 0., acc: vec![0; NMAT], cx: x, cy: y, cang: ang };
        b.straighten(); b
    }
    /// A straight path behind the head, back the way it heads (a borer just set down, or loaded from an old save).
    pub fn straighten(&mut self) {
        let (c, s) = (self.ang.cos(), self.ang.sin()); self.path.clear();
        for k in 1..=(self.need() as usize + 4) { self.path.push_back((self.x - c * k as f32, self.y - s * k as f32)); }
        self.place_cab();
    }
    /// How much path the train needs behind the head.
    fn need(&self) -> f32 { seg_dist(self.segs.len()) + SEG_D }
    /// The point `d` along the path behind the head, and the way the path runs there (toward the head).
    pub fn at(&self, d: f32) -> (f32, f32, f32) {
        let (mut ax, mut ay) = (self.x, self.y); let mut left = d;
        for &(bx, by) in &self.path {
            let l = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
            if l >= left && l > 1e-4 { let t = left / l; return (ax + (bx - ax) * t, ay + (by - ay) * t, (ay - by).atan2(ax - bx)); }
            left -= l; ax = bx; ay = by;
        }
        // past the end of the path: straight on, the way its last bit ran
        let a = match (self.path.len(), self.path.back()) { (n, Some(&(lx, ly))) if n >= 2 => { let (qx, qy) = self.path[n - 2]; (qy - ly).atan2(qx - lx) } _ => self.ang };
        (ax - a.cos() * left, ay - a.sin() * left, a)
    }
    fn place_cab(&mut self) { let (x, y, _) = self.at(CAB_D); let (x0, y0, _) = self.at(CAB_D - 3.); let (x1, y1, _) = self.at(CAB_D + 3.); self.cx = x; self.cy = y; self.cang = (y0 - y1).atan2(x0 - x1); }
    /// The head's middle moved to (x, y): note it in the path and drop what the train no longer needs.
    fn advance(&mut self, x: f32, y: f32) {
        let (ox, oy) = (self.x, self.y); self.x = x; self.y = y;
        let last = self.path.front().copied().unwrap_or((ox, oy));
        if (x - last.0).powi(2) + (y - last.1).powi(2) >= 1. { self.path.push_front((ox, oy)); }
        let need = self.need() + 4.; let (mut d, mut keep) = (0., 0); let (mut ax, mut ay) = (x, y);
        for &(bx, by) in &self.path { d += ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt(); keep += 1; ax = bx; ay = by; if d > need { break; } }
        self.path.truncate(keep.max(2));
    }
    /// Is (x, y) on the borer (its head or its cab)?
    pub fn contains(&self, x: f32, y: f32) -> bool {
        if (x - self.x).powi(2) + (y - self.y).powi(2) < (HEAD_R - 1.).powi(2) { return true; }
        let (dx, dy) = (x - self.cx, y - self.cy); let (c, s) = (self.cang.cos(), self.cang.sin());
        (dx * c + dy * s).abs() < BORER_W / 2. && (-dx * s + dy * c).abs() < BORER_H / 2.
    }
    pub fn lumps(&self) -> u32 { self.fuel.iter().map(|f| f.1).sum::<u32>() + self.burning.is_some() as u32 }
    /// Which way it faces east or west (for the rider and the old one-way rules).
    pub fn dir(&self) -> i8 { if self.ang.cos() < 0. { -1 } else { 1 } }
    pub(crate) fn shift(&mut self, dx: f32, dy: f32) {
        self.x -= dx; self.y -= dy; self.px -= dx; self.py -= dy; self.cx -= dx; self.cy -= dy;
        for p in self.path.iter_mut() { p.0 -= dx; p.1 -= dy; }
    }
}

/// `a` turned toward `to` by at most `step` (radians), the short way round.
fn turn_toward(a: f32, to: f32, step: f32) -> f32 { let d = (to - a + PI).rem_euclid(TAU) - PI; (a + d.clamp(-step, step)).rem_euclid(TAU) }

/// What its sieve keeps of what it cuts (the rest is rammed into the walls).
fn kept(m: Mat) -> Option<Item> { crate::craft::drop_of(m).filter(|it| matches!(it, Item::Coal | Item::CopperOre | Item::TinOre | Item::IronOre | Item::Quartz | Item::Crystal)) }

impl World {
    /// The cells of a round face of radius `r` round (x, y).
    fn disc(&self, x: f32, y: f32, r: f32) -> impl Iterator<Item = (i32, i32)> {
        let (c0, c1, r0, r1) = ((x - r).floor() as i32, (x + r).ceil() as i32, (y - r).floor() as i32, (y + r).ceil() as i32);
        (r0..=r1).flat_map(move |rr| (c0..=c1).map(move |c| (c, rr))).filter(move |&(c, rr)| (c as f32 + 0.5 - x).powi(2) + (rr as f32 + 0.5 - y).powi(2) <= r * r)
    }
    fn solid_at(&self, c: i32, r: i32) -> bool { !self.g.in_bounds(c, r) || !self.passable(c as usize, r as usize) }
    fn uncuttable(&self, c: i32, r: i32) -> bool { !self.g.in_bounds(c, r) || self.g.get(c as usize, r as usize) == Mat::Bedrock }
    /// The head moved to (x, y), pushed back off what it can't cut (bedrock, the window's edge) to a cell inside its rim,
    /// so it slides along bedrock it grazes; it stops only where it is pointed into it.
    fn off_bedrock(&self, mut x: f32, mut y: f32) -> (f32, f32) {
        let keep = HEAD_R - 1.;
        for _ in 0..4 {
            let hits: Vec<(i32, i32)> = self.disc(x, y, keep).filter(|&(c, r)| self.uncuttable(c, r)).collect();
            if hits.is_empty() { break; }
            for (c, r) in hits { let (dx, dy) = (x - (c as f32 + 0.5), y - (r as f32 + 0.5)); let d = (dx * dx + dy * dy).sqrt().max(1e-3); let push = keep + 0.01 - d; if push > 0. { x += dx / d * push; y += dy / d * push; } }
        }
        (x, y)
    }
    /// Does anything hold it: rock within a cell of its head's or cab's sides, or a wagon of its train standing on something?
    fn borer_held(&self, b: &Borer, train: &[usize]) -> bool {
        let ring = |x: f32, y: f32, r: f32| (0..24).any(|k| { let a = k as f32 * TAU / 24.; let (c, rr) = ((x + a.cos() * r).floor() as i32, (y + a.sin() * r).floor() as i32); self.solid_at(c, rr) });
        ring(b.x, b.y, HEAD_R + 1.) || ring(b.cx, b.cy, BORER_W / 2. + 1.)
            || train.iter().any(|&j| { let w = &self.tr.wagons[j]; let r = w.y.floor() as i32; ((w.x - WAGON_W / 2.).floor() as i32..(w.x + WAGON_W / 2.).ceil() as i32).any(|c| self.solid_at(c, r)) })
    }

    /// Put a borer down with its head at the height of a mole on the floor under (x, y), heading the way the mole faces.
    pub(crate) fn place_borer(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let xf = x as f32 + 0.5;
        let floor = (y..(y + 2 * CPM).min(self.g.h - 1)).find(|&r| self.solid_at(x as i32, r as i32)).ok_or("needs a floor under it")? as f32;
        let dir = self.moles.first().map_or(1, |m| m.facing) as f32;
        let (hx, hy) = (xf, floor - HEAD_R);
        let b = Borer::new(hx, hy, if dir < 0. { PI } else { 0. });
        if self.disc(hx, hy, HEAD_R - 0.5).any(|(c, r)| self.solid_at(c, r)) || self.disc(b.cx, b.cy, BORER_H / 2. - 0.5).any(|(c, r)| self.solid_at(c, r)) { return Err("needs 1.5 m of room round it"); }
        if self.tr.wagons.iter().any(|w| b.contains(w.x, w.y - WAGON_H / 2.)) { return Err("there's a wagon there"); }
        self.tr.borers.push(b);
        self.events.push(Event::Place); Ok(())
    }
    pub fn borer_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.borers.iter().position(|b| b.contains(xf, yf)) }
    /// Right mouse on a borer: a lump of fuel from the pack (fuel in the paw), or climb in / out (empty paw).
    pub(crate) fn use_borer(&mut self, x: usize, y: usize) -> bool {
        let Some(k) = self.borer_at(x, y) else { return false };
        if let Some(it) = self.gear.held.filter(|&it| burn_ticks(it).is_some() && self.inv[it as usize] > 0) {
            self.inv[it as usize] -= 1; let b = &mut self.tr.borers[k];
            if let Some(f) = b.fuel.iter_mut().find(|f| f.0 == it) { f.1 += 1; } else { b.fuel.push((it, 1)); }
            let n = self.tr.borers[k].lumps(); self.say(format!("Fed the borer's firebox: {n} lumps in it."));
        } else if self.tr.borer_ride == Some(k) { self.leave_borer(); }
        else {
            if self.tr.wagon_ride.is_some() { self.leave_wagon(); }
            if self.tr.riding.is_some() || self.tr.in_tube.is_some() { return false; }
            self.tr.borer_ride = Some(k); self.say("In the borer's cab. A/D/W/S steer (diagonals too) · Space: climb out.".into());
        }
        self.events.push(Event::Click); true
    }
    /// Out of the cab: the mole stands in the tunnel beside it (the first open spot of the cab, the head, or just behind).
    pub(crate) fn leave_borer(&mut self) {
        let Some(k) = self.tr.borer_ride.take() else { return };
        let b = &self.tr.borers[k];
        let xy = |p: (f32, f32, f32)| (p.0, p.1);
        let spots = [(b.cx, b.cy), (b.x, b.y), xy(b.at(CAB_D + BORER_W / 2. + 2.)), xy(b.at(CAB_D - 2.))];
        let feet = |(x, y): (f32, f32)| (0..=CPM as i32).map(move |k| (x, y + BODY_H / 2. + k as f32 - CPM as f32 / 2.));
        let spot = spots.into_iter().flat_map(feet).find(|&(x, y)| !self.box_hits(x, y)).unwrap_or((b.cx, b.cy + BODY_H / 2.));
        self.pl.px = spot.0; self.pl.py = spot.1; self.pl.ppx = spot.0; self.pl.ppy = spot.1; self.pl.vx = 0.; self.pl.vy = 0.; self.pl.sync = None;
        let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); self.pl.fall_from = y;
        self.say("Out of the borer's cab.".into());
    }
    /// Does W steer (riding a borer), so that it mustn't jump?
    pub fn borer_takes_up(&self) -> bool { self.tr.borer_ride.is_some() }
    /// `E` on a borer: its unburnt fuel back into the pack; empty, cold and with nobody in it, it comes up itself (its
    /// wagons left standing uncoupled).
    pub(crate) fn pick_up_borer(&mut self, x: usize, y: usize) -> Option<Item> {
        let k = self.borer_at(x, y)?;
        if !self.tr.borers[k].fuel.is_empty() { for (it, n) in std::mem::take(&mut self.tr.borers[k].fuel) { self.inv[it as usize] += n; } self.say("Raked the fuel out of the borer.".into()); return Some(Item::SteamBorer); }
        if self.tr.borer_ride == Some(k) || self.tr.borers[k].burning.is_some() { return None; }
        for j in self.train(k) { self.tr.wagons[j].coupled = false; }
        self.tr.borers.remove(k); if let Some(r) = self.tr.borer_ride { if r > k { self.tr.borer_ride = Some(r - 1); } }
        self.inv[Item::SteamBorer as usize] += 1; Some(Item::SteamBorer)
    }

    /// The mole in a borer's cab: carried in it, the walking keys steering. Space climbs out. True while riding.
    pub(crate) fn borer_ride_step(&mut self) -> bool {
        let Some(k) = self.tr.borer_ride else { return false };
        if k >= self.tr.borers.len() || !self.moles[0].alive { self.tr.borer_ride = None; return false; }
        let jump = self.input.jump && !self.pl.jump_was; self.pl.jump_was = self.input.jump;
        if jump { self.leave_borer(); return true; }
        self.sit_in_cab(k);
        true
    }
    /// The rider's place: sitting in the cab (the camera follows it there).
    fn sit_in_cab(&mut self, k: usize) {
        let b = &self.tr.borers[k]; let (cx, cy, dir) = (b.cx, b.cy, b.dir());
        self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py;
        self.pl.px = cx; self.pl.py = cy + BODY_H / 2.; self.pl.vx = 0.; self.pl.vy = 0.; self.pl.grounded = true; self.pl.cling = 0;
        let (x, y) = (self.pl.px.max(0.) as usize, (self.pl.py - 0.001).max(0.) as usize); self.moles[0].x = x; self.moles[0].y = y; self.moles[0].facing = dir; self.pl.sync = Some((x, y)); self.pl.fall_from = y;
    }

    /// Cut a cell of the face: rock into the wall, what the sieve keeps into the train.
    fn worm_cut(&mut self, k: usize, i: usize, train: &[usize]) -> f32 {
        let m = self.g.m(i); let wet = self.g.flags[i] & crate::grid::F_WET != 0;
        let cost = m.p().dig as f32;
        self.leave_lining(i, m);
        self.g.set(i, if wet { Mat::Water } else { Mat::Empty }); self.desig[i] = 0;
        if let Some(it) = kept(m) {
            let b = &mut self.tr.borers[k]; b.acc[m as usize] += 1;
            let per = if matches!(m, Mat::Crystal | Mat::Quartz) { 3 * (S * S) as u16 } else { crate::craft::CELLS_PER_ITEM };
            if b.acc[m as usize] >= per { b.acc[m as usize] = 0; self.borer_keep(k, it, train); }
        }
        cost
    }
    /// An item off the sieve: into the first wagon with room (fuel into a coal wagon first), else behind the cab.
    fn borer_keep(&mut self, k: usize, it: Item, train: &[usize]) {
        let coal_first = |w: &crate::wagon::Wagon| (w.kind == crate::wagon::WagonKind::Coal) as u8;
        let mut order: Vec<usize> = train.to_vec(); order.sort_by_key(|&j| std::cmp::Reverse(coal_first(&self.tr.wagons[j])));
        if let Some(&j) = order.iter().find(|&&j| self.tr.wagons[j].takes(it, 1)) { self.tr.wagons[j].add(it, 1); return; }
        let b = &self.tr.borers[k]; let (x, y, _) = b.at(CAB_D + BORER_W / 2. + 1.); self.drops.push(Drop::at(x, y, it, 1));
    }

    /// One tick of every borer.
    pub(crate) fn step_borers(&mut self) {
        let dt = 1. / 60.;
        for k in 0..self.tr.borers.len() {
            let mut b = self.tr.borers[k].clone(); b.px = b.x; b.py = b.y;
            let riding = self.tr.borer_ride == Some(k);
            let train = self.train(k);
            b.segs.retain(|id| self.tr.wagons.iter().any(|w| w.id == *id && w.coupled));
            // the firebox
            if b.burning.is_none() { if let Some(f) = b.fuel.first_mut() { let it = f.0; f.1 -= 1; if f.1 == 0 { b.fuel.remove(0); } b.burning = Some(it); b.burn = burn_ticks(it).unwrap_or(0); } }
            if b.burning.is_some() { b.burn = b.burn.saturating_sub(1);
                if b.steam < STEAM_MAX { b.steam = (b.steam + FIRE_W * BOILER_EFF * dt / 1000.).min(STEAM_MAX); }
                if b.burn == 0 { b.burning = None; let (x, y, _) = b.at(CAB_D + BORER_W / 2.); self.drops.push(Drop::at(x, y, Item::Ash, 1)); } }
            // steering: the rider's keys give the way to go; the head turns toward it, and runs while a key is held
            let (sx, sy) = if riding { ((self.input.right as i32 - self.input.left as i32) as f32, (self.input.down as i32 - self.input.up as i32) as f32) } else { (0., 0.) };
            let steer = sx != 0. || sy != 0.;
            if steer { b.ang = turn_toward(b.ang, sy.atan2(sx), TURN * dt); }
            // how fast the rock lets it go: what its head can grind of what it has been cutting
            let cap = if b.hard > 1. { (GRIND_W / 1000. / (b.hard * CUT_KJ)).min(BORER_V) } else { BORER_V };
            let want = if steer { cap } else { 0. };
            b.v = if b.v < want { (b.v + ACCEL * dt).min(want) } else { (b.v - 2. * ACCEL * dt).max(want) };
            // driving is paid in steam; short of it, it goes only as far as the steam it has
            let drive_kj = (DRIVE_W * b.v / BORER_V + crate::wagon::WAGON_DRIVE_W * train.len() as f32 * (b.v > 0.) as u8 as f32) * dt / 1000.;
            if b.v > 0. && b.steam < drive_kj + 0.01 { b.v = 0.; if riding && self.tick % 120 == 0 { self.say("The borer has no steam: feed its firebox.".into()); } }
            // falling: nothing holds it
            let held = self.borer_held(&b, &train);
            if held { b.vy = 0.; } else { b.vy = (b.vy + crate::world::TUNE.gravity * dt).min(crate::world::TUNE.terminal); }
            let (mut nx, mut ny) = (b.x + b.ang.cos() * b.v * dt, b.y + b.ang.sin() * b.v * dt + b.vy * dt);
            if b.v > 0. { b.steam -= drive_kj; }
            if (nx, ny) != (b.x, b.y) {
                let (wx, wy) = (nx, ny); (nx, ny) = self.off_bedrock(nx, ny);
                let want_d = ((wx - b.x).powi(2) + (wy - b.y).powi(2)).sqrt(); let got = (nx - b.x) * (wx - b.x) + (ny - b.y) * (wy - b.y);
                if got <= 0.1 * want_d * want_d || self.disc(nx, ny, HEAD_R - 1.).any(|(c, r)| self.uncuttable(c, r)) {
                    // bedrock (or the window's edge) dead ahead: no further that way (it can still turn away)
                    if riding && b.v > 0. && self.tick % 120 == 0 { self.say("The borer's head won't cut bedrock.".into()); }
                    nx = b.x; ny = b.y; b.v = 0.; b.vy = 0.;
                } else if b.v == 0. && self.disc(nx, ny, HEAD_R).any(|(c, r)| self.solid_at(c, r)) {
                    // falling onto rock: it lands (it cuts only while driving)
                    nx = b.x; ny = b.y; b.vy = 0.;
                }
            }
            if (nx, ny) != (b.x, b.y) {
                // cut the face: every cell of the round head at its new place that isn't open
                let cells: Vec<usize> = self.disc(nx, ny, HEAD_R).filter(|&(c, r)| self.g.in_bounds(c, r) && !self.passable(c as usize, r as usize)).map(|(c, r)| self.g.idx(c as usize, r as usize)).filter(|&i| self.g.m(i) != Mat::Bedrock).collect();
                self.tr.borers[k] = b.clone();
                let mut dig = 0.; for &i in &cells { dig += self.worm_cut(k, i, &train); }
                let acc = std::mem::take(&mut self.tr.borers[k].acc); b.acc = acc;
                let step = ((nx - b.x).powi(2) + (ny - b.y).powi(2)).sqrt().max(1e-3);
                b.steam = (b.steam - dig * CUT_KJ).max(0.);
                b.hard += (dig / step - b.hard) * 0.2;
                if !cells.is_empty() {
                    light::reveal(&mut self.g, nx.max(0.) as usize, ny.max(0.) as usize, (HEAD_R as usize) + 3 * S);
                    if self.tick % 8 == 0 { self.shake = self.shake.max(0.04); self.events.push(Event::Dig); }
                }
                b.advance(nx, ny);
            } else { b.hard *= 0.9; }
            b.place_cab();
            // the train follows the head's path: each wagon at its place along it
            for (n, &j) in train.iter().enumerate() {
                let (x, y, a) = b.at(seg_dist(n)); let w = &mut self.tr.wagons[j];
                w.px = w.x; w.py = w.y; w.x = x; w.y = y + WAGON_H / 2.; w.ang = a; w.vy = 0.;
            }
            self.tr.borers[k] = b;
        }
        if let Some(k) = self.tr.borer_ride { if k < self.tr.borers.len() { self.sit_in_cab(k); } }
    }

    pub(crate) fn shift_borers(&mut self, rows: i64, h: usize) {
        let dy = rows as f32; let riding = self.tr.borer_ride; let mut keep = vec![]; let mut nr = None;
        for (k, mut b) in std::mem::take(&mut self.tr.borers).into_iter().enumerate() { b.shift(0., dy); if b.y < HEAD_R + 2. || b.y >= h as f32 - HEAD_R - 2. { continue; } if riding == Some(k) { nr = Some(keep.len()); } keep.push(b); }
        self.tr.borers = keep; self.tr.borer_ride = nr;
    }
    /// Saved (transport blob 14): the head, its heading, its fuel and steam, its path and its train.
    pub(crate) fn encode_borers(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.borers.len() as u32);
        for b in &self.tr.borers { o.f32(b.x); o.f32(b.y); o.f32(b.ang); o.u32(b.fuel.len() as u32); for &(it, n) in &b.fuel { o.u8(it as u8); o.u32(n); }
            o.u8(b.burning.map_or(255, |it| it as u8)); o.u32(b.burn); o.f32(b.steam);
            o.u32(b.path.len() as u32); for &(x, y) in &b.path { o.f32(x); o.f32(y); }
            o.u32(b.segs.len() as u32); for &id in &b.segs { o.u32(id); } }
    }
    /// Before 14 a borer was a cab on wheels boring east or west: (x, y) its middle and wheel row, a facing byte. It comes
    /// back as a worm heading that way with its head at the cab's front, a straight path behind it; its old train is
    /// coupled up by `adopt_old_trains` once the wagons are in.
    pub(crate) fn decode_borers(&mut self, r: &mut crate::band::Rd, ver: u8) -> Result<(), String> {
        let (gw, gh) = (self.g.w as f32, self.g.h as f32);
        for _ in 0..r.len(4096)? {
            let (x, y) = (r.f32()?, r.f32()?);
            let (ang, old_dir) = if ver >= 14 { (r.f32()?, 0) } else { let d = r.u8()? as i8 - 1; (if d < 0 { PI } else { 0. }, if d < 0 { -1 } else { 1 }) };
            let mut fuel = vec![]; for _ in 0..r.len(4096)? { let (it, n) = (r.u8()?, r.u32()?); if let Some(it) = r.item(it) { fuel.push((it, n)); } }
            let bi = r.u8()?; let burning = r.item(bi);
            let burn = r.u32()?; if r.old { r.f32()?; } let steam = r.f32()?; // (before 13: its tank's water, now gone)
            let (mut path, mut segs) = (VecDeque::new(), vec![]);
            if ver >= 14 {
                for _ in 0..r.len(1 << 16)? { let (px, py) = (r.f32()?, r.f32()?); if px.is_finite() && py.is_finite() { path.push_back((px, py)); } }
                for _ in 0..r.len(4096)? { segs.push(r.u32()?); }
            }
            // (old: the cab's middle was at x, y - BORER_H / 2; the head rides CAB_D ahead of it)
            let (hx, hy) = if ver >= 14 { (x, y) } else { (x + old_dir as f32 * CAB_D, y - BORER_H / 2.) };
            if hx > 2. && hx < gw - 2. && hy > HEAD_R + 1. && hy < gh - 1. && steam.is_finite() && ang.is_finite() {
                let mut b = Borer::new(hx, hy, ang.rem_euclid(TAU));
                b.fuel = fuel; b.burning = burning; b.burn = burn; b.steam = steam.clamp(0., STEAM_MAX); b.segs = segs;
                if ver >= 14 && path.len() >= 2 { b.path = path; b.place_cab(); } else if ver >= 14 { b.straighten(); }
                // (old: the cab's old middle and wheel row, for `adopt_old_trains`)
                if ver < 14 { b.cx = x; b.cy = y; }
                self.tr.borers.push(b);
            }
        }
        Ok(())
    }
    /// Old saves (before 14): the train was found by spacing behind each cab, on its wheel row (`Borer::cx`, `cy` hold
    /// those until now). Couple those wagons as its segments and set every train on its path.
    pub(crate) fn adopt_old_trains(&mut self) {
        for k in 0..self.tr.borers.len() {
            let (x, y, d) = { let b = &self.tr.borers[k]; (b.cx, b.cy, b.dir() as f32) };
            let (mut rear, mut ry) = (x - d * BORER_W / 2., y); let mut segs = vec![]; let mut used: Vec<usize> = vec![];
            loop {
                let want = rear - d * (GAP + WAGON_W / 2.);
                let Some(j) = (0..self.tr.wagons.len()).find(|&j| { let w = &self.tr.wagons[j]; w.coupled && !used.contains(&j) && !self.tr.borers.iter().any(|o| o.segs.contains(&w.id)) && (w.x - want).abs() < crate::wagon::COUPLE_D && (w.y - ry).abs() <= CPM as f32 }) else { break };
                used.push(j); segs.push(self.tr.wagons[j].id); let w = &self.tr.wagons[j]; rear = w.x - d * WAGON_W / 2.; ry = w.y;
            }
            let b = &mut self.tr.borers[k]; b.segs = segs; b.straighten();
            for (n, &j) in used.iter().enumerate() { let (px, py, a) = self.tr.borers[k].at(seg_dist(n)); let w = &mut self.tr.wagons[j]; w.x = px; w.y = py + WAGON_H / 2.; w.px = w.x; w.py = w.y; w.ang = a; }
        }
        // a wagon marked coupled that no borer took is left standing loose
        let taken: Vec<u32> = self.tr.borers.iter().flat_map(|b| b.segs.iter().copied()).collect();
        for w in self.tr.wagons.iter_mut() { if w.coupled && !taken.contains(&w.id) { w.coupled = false; } }
    }
}
