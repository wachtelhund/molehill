//! Transport: machines for getting about the Deepholt. Everything here runs on weight, slope, friction
//! and current, like the rest of the sim (see PROGRESSION.md, Transportation).
//!
//! **Rails and mine carts.** Rail is a built block (`Mat::Rail`, flag `TRACK`): copper straps on wooden
//! sleepers, laid with the block brush, held up like any built block. A cart is its own body:
//! * it sits on the top of the rail under its middle, follows the track's height, and rolls along it;
//! * on a slope gravity pulls it along (g·sinθ); rolling friction (`ROLL`) slows it; it stops dead at the
//!   end of the track (a bumper: a little bounce back);
//! * the mole pushes it by walking into it: the push is a force, so a loaded cart gets going slower;
//! * the mole rides it (right mouse on it, or drop into it from above; jump to get out);
//! * loose items that fall or are thrown into it are its cargo; `E` takes the cargo into the pack, and on
//!   an empty cart picks the cart up;
//! * off the rail (rail gone under it, or set down on bare ground) it falls and sits, wheels in the dirt.
//!
//! **Powered rails.** Copperline laid along the track (in the same 1 m blocks) with current in it drives a
//! cart on: toward `POWER_V` the way it is running, with `POWER_ACC` of pull, so it climbs slopes plain
//! track can't. A lever, knot or listener on that line switches the stretch, so a line can be automated
//! with the same logic as doors and pumps. At the end of a powered stretch a cart bounces off the bumper
//! and is driven back the other way: a powered line between two bumpers shuttles.
use crate::craft::{Item, NITEM};
use crate::mat::{CPM, S, SF, TRACK};
use crate::world::{Event, World, BODY_H, BODY_HW, TUNE};

/// A cart's body: 1 m long, 0.5 m tall (cells). Its position is the middle of its wheel line.
pub const CART_W: f32 = CPM as f32; pub const CART_H: f32 = (CPM / 2) as f32;
/// Gravity along the track (cells/s²): the real 9.81 m/s².
pub const G_CELLS: f32 = 9.81 * CPM as f32;
/// Rolling resistance coefficient (iron-shod wheels on copper straps, a gritty mine): a cart let go at walking pace
/// on the level rolls about 7 m.
pub const ROLL: f32 = 0.1;
/// Masses (kg): the empty cart, each item of cargo, a riding mole.
pub const CART_KG: f32 = 80.; pub const ITEM_KG: f32 = 5.; pub const RIDER_KG: f32 = 30.;
/// The push a mole leans into a cart with (N): an empty cart reaches walking pace in about a metre; a
/// full one takes a few.
pub const PUSH_N: f32 = 320.;
/// Top speed on the rails (cells/s), 10 m/s.
pub const CART_MAX: f32 = 10. * CPM as f32;
/// Powered rail: the speed its motor drives a cart to (cells/s, 5 m/s) and its pull (m/s²): enough to climb
/// a 1-in-2 slope (g·sinθ + rolling friction ≈ 5.3 m/s²) at a crawl, and a 1-in-4 at speed.
pub const POWER_V: f32 = 5. * CPM as f32; pub const POWER_ACC: f32 = 6.;
/// Cargo a cart holds (items).
pub const CART_LOAD: u32 = 40;

#[derive(Clone, Debug)]
pub struct Cart {
    /// middle of the wheel line (cells), and a tick ago (drawing)
    pub x: f32, pub y: f32, pub px: f32, pub py: f32,
    /// speed along the track (cells/s, + = right); fall speed off the rails
    pub v: f32, pub vy: f32,
    /// the way it last ran (a powered rail drives it on that way)
    pub dir: i8,
    pub on_rail: bool,
    pub cargo: Vec<(Item, u32)>,
    /// ticks since it began tipping its load at a bumper (0 = upright; see production.rs)
    pub tip: u16,
}
impl Cart {
    pub fn load(&self) -> u32 { self.cargo.iter().map(|c| c.1).sum() }
    pub fn mass(&self, rider: bool) -> f32 { CART_KG + self.load() as f32 * ITEM_KG + if rider { RIDER_KG } else { 0. } }
    /// The cart's box (x0, y0, x1, y1) in cells.
    pub fn rect(&self) -> (f32, f32, f32, f32) { (self.x - CART_W / 2., self.y - CART_H, self.x + CART_W / 2., self.y) }
    pub fn contains(&self, x: f32, y: f32) -> bool { let (x0, y0, x1, y1) = self.rect(); x >= x0 && x < x1 && y >= y0 - 1. && y < y1 }
    fn add(&mut self, it: Item, n: u32) { if let Some(c) = self.cargo.iter_mut().find(|c| c.0 == it) { c.1 += n; } else { self.cargo.push((it, n)); } }
}

/// Pack items that go down whole as a transport machine (`World::place_transport`).
pub fn is_machine(it: Item) -> bool { matches!(it, Item::Cart | Item::RopeLadder | Item::Lift | Item::TubeHopper | Item::OreHopper) }

/// Everything transport keeps between ticks.
#[derive(Clone, Debug, Default)]
pub struct Transport {
    pub carts: Vec<Cart>,
    /// the cart the mole rides, if any
    pub riding: Option<usize>,
    /// rope ladders and lifts (see lifts.rs), and whether the mole is on a ladder
    pub ladders: Vec<crate::lifts::Ladder>, pub lifts: Vec<crate::lifts::Lift>, pub climbing: bool,
    /// pressure-tube hoppers and capsules in the pipe (see tubes.rs), and the capsule the mole is sealed in
    pub hoppers: Vec<crate::tubes::Hopper>, pub capsules: Vec<crate::tubes::Capsule>, pub in_tube: Option<usize>,
    /// ore hoppers over the track (see production.rs), and how many times a cart has tipped at a bumper
    pub bins: Vec<crate::production::OreHopper>, pub tips: u32,
    /// rope-gun ropes fixed in the rock, the hook in flight, and the rope the mole hangs on with the length
    /// of rope above its hands (see ropes.rs)
    pub ropes: Vec<crate::ropes::Rope>, pub hook: Option<crate::ropes::Hook>, pub on_rope: Option<(usize, f32)>,
}

impl World {
    /// Is the cell at (x, y) track?
    pub fn is_rail(&self, x: i32, y: i32) -> bool { self.g.in_bounds(x, y) && self.g.get(x as usize, y as usize).has(TRACK) }
    /// The top of the track in column x, within `reach` cells of row y (the row a cart's wheels sit on).
    fn rail_top(&self, x: i32, y: f32, reach: i32) -> Option<f32> {
        let yc = y.round() as i32;
        (0..=reach).flat_map(|d| [yc + d, yc - d]).find(|&r| self.is_rail(x, r) && !self.is_rail(x, r - 1)).map(|r| r as f32)
    }
    /// Is any of the cart's body (above its wheels) in solid ground at x?
    fn cart_blocked(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = ((x - CART_W / 2.).floor() as i32, (x + CART_W / 2. - 1e-3).floor() as i32);
        let (y0, y1) = ((y - CART_H).floor() as i32, (y - 1. - 1e-3).floor() as i32);
        // (track never blocks it: its wheels ride over a step in the rail; nor do loose grains, which it shoves)
        (y0..=y1).any(|r| (x0..=x1).any(|c| !self.g.in_bounds(c, r) || !self.passable(c as usize, r as usize) && !self.is_rail(c, r) && !self.shovable(c, r)))
    }
    /// Loose ground (sand, gravel, rubble) a cart ploughs through.
    fn shovable(&self, x: i32, y: i32) -> bool { let i = self.g.idx(x as usize, y as usize); let m = self.g.m(i); m.kind() == crate::mat::Kind::Powder || self.g.loose(i) && !m.has(TRACK) }
    /// Grains in the cart's body are pushed up over it, to the nearest open cell above (never lost).
    fn shove_from_cart(&mut self, x: f32, y: f32) {
        let (x0, x1) = ((x - CART_W / 2.).floor() as i32, (x + CART_W / 2. - 1e-3).floor() as i32);
        let (y0, y1) = ((y - CART_H).floor() as i32, (y - 1. - 1e-3).floor() as i32);
        for r in y0..=y1 { for c in x0..=x1 { if !self.g.in_bounds(c, r) || !self.shovable(c, r) || self.passable(c as usize, r as usize) { continue; }
            let i = self.g.idx(c as usize, r as usize); let (m, f) = (self.g.mat[i], self.g.flags[i]);
            if let Some(t) = (1..3 * CPM as i32).map(|k| y0 - k).find(|&rr| self.g.in_bounds(c, rr) && self.g.get(c as usize, rr as usize) == crate::mat::Mat::Empty) {
                let j = self.g.idx(c as usize, t as usize); self.g.set(j, crate::mat::Mat::from_u8(m)); self.g.flags[j] = f; self.g.set(i, crate::mat::Mat::Empty);
            } } }
    }
    fn on_ground(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = ((x - CART_W / 2. + 1.).floor() as i32, (x + CART_W / 2. - 1.).floor() as i32);
        (x0..=x1).any(|c| !self.g.in_bounds(c, y as i32) || !self.passable(c as usize, y as usize))
    }

    /// Set down a transport item from the pack at (x, y). Ok(true) if it was one.
    pub fn place_transport(&mut self, it: Item, x: usize, y: usize) -> Result<bool, &'static str> {
        match it {
            Item::Cart => {
                if self.inv[it as usize] == 0 { return Err("none left"); }
                if !self.in_reach(x, y) { return Err("out of reach"); }
                // on the track near the cursor, else on the floor under it
                let xf = x as f32 + 0.5;
                let y = self.rail_top(x as i32, y as f32, 3 * S as i32).or_else(|| (y..(y + 4 * S).min(self.g.h - 1)).find(|&r| !self.passable(x, r)).map(|r| r as f32)).ok_or("needs track or a floor under it")?;
                if self.cart_blocked(xf, y) { return Err("no room for it"); }
                if self.tr.carts.iter().any(|c| (c.x - xf).abs() < CART_W && (c.y - y).abs() < CART_H) { return Err("there's a cart there"); }
                self.inv[it as usize] -= 1;
                self.tr.carts.push(Cart { x: xf, y, px: xf, py: y, v: 0., vy: 0., dir: self.moles.first().map_or(1, |m| m.facing), on_rail: false, cargo: vec![], tip: 0 });
                self.events.push(Event::Place); Ok(true)
            }
            Item::RopeLadder | Item::Lift | Item::TubeHopper | Item::OreHopper => {
                if self.inv[it as usize] == 0 { return Err("none left"); }
                match it { Item::RopeLadder => self.place_ladder(x, y)?, Item::Lift => self.place_lift(x, y)?, Item::OreHopper => self.place_bin(x, y)?, _ => self.place_hopper(x, y)? }
                self.inv[it as usize] -= 1; Ok(true)
            }
            _ => Ok(false),
        }
    }
    pub(crate) fn cart_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.carts.iter().position(|c| c.contains(xf, yf)) }
    /// Right mouse on a transport machine: get in or out of a cart. True if there was one.
    pub fn use_transport(&mut self, x: usize, y: usize) -> bool {
        if let Some(k) = self.cart_at(x, y) {
            if self.tr.riding == Some(k) { self.leave_cart(); } else { self.tr.riding = Some(k); self.say("In the cart. Space to jump out.".into()); }
            self.events.push(Event::Click); return true;
        }
        self.use_bin(x, y) || self.use_hopper(x, y) || self.use_lift(x, y)
    }
    fn leave_cart(&mut self) {
        if let Some(k) = self.tr.riding.take() { let c = &self.tr.carts[k];
            self.pl.px = c.x; self.pl.py = c.y - CART_H; self.pl.vx = c.v; self.pl.vy = -TUNE.jump_v * 0.6; self.pl.sync = None;
            let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); }
    }
    /// `E` on a cart: take its cargo into the pack; an empty cart (nobody in it) comes up itself.
    pub fn pick_up_transport(&mut self, x: usize, y: usize) -> Option<Item> {
        let Some(k) = self.cart_at(x, y) else { return self.pick_up_bin(x, y).or_else(|| self.pick_up_hopper(x, y)).or_else(|| self.pick_up_lift(x, y)).or_else(|| self.pick_up_rope(x, y)) };
        // unloading says what came home ("Unloaded the cart: 12 stone, 3 copper ore.")
        if !self.tr.carts[k].cargo.is_empty() { let mut got = vec![];
            for (it, n) in std::mem::take(&mut self.tr.carts[k].cargo) { self.inv[it as usize] += n; got.push(format!("{n} {}", it.name().to_lowercase())); }
            self.say(format!("Unloaded the cart: {}.", got.join(", "))); return Some(Item::Cart); }
        if self.tr.riding == Some(k) { return None; }
        self.tr.carts.remove(k); if let Some(r) = self.tr.riding { if r > k { self.tr.riding = Some(r - 1); } }
        self.inv[Item::Cart as usize] += 1; Some(Item::Cart)
    }

    /// The mole in a cart: carried with it, hands on the rim. Space jumps out; A/D lean (a little push).
    /// Returns true when the mole is riding (the normal movement is skipped).
    pub(crate) fn ride_step(&mut self) -> bool {
        if self.tube_ride_step() { return true; }
        let Some(k) = self.tr.riding else { return false };
        if k >= self.tr.carts.len() || !self.moles[0].alive { self.tr.riding = None; return false; }
        let jump = self.input.jump && !self.pl.jump_was; self.pl.jump_was = self.input.jump;
        if jump { self.leave_cart(); return true; }
        let lean = self.input.right as i32 - self.input.left as i32;
        if lean != 0 { self.moles[0].facing = lean as i8; let c = &mut self.tr.carts[k]; if c.on_rail { c.v += lean as f32 * 0.6 * CPM as f32 / 60.; } }
        let c = &self.tr.carts[k];
        self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py;
        self.pl.px = c.x; self.pl.py = c.y - CART_H * 0.5; self.pl.vx = c.v; self.pl.vy = 0.; self.pl.grounded = true; self.pl.cling = 0;
        let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); self.pl.fall_from = y;
        true
    }

    /// One tick of every cart.
    pub(crate) fn step_transport(&mut self) {
        let dt = 1. / 60.;
        for k in 0..self.tr.carts.len() {
            let mut c = self.tr.carts[k].clone(); c.px = c.x; c.py = c.y;
            let rider = self.tr.riding == Some(k);
            // tipping at a bumper: held there, its load going out over the front (production.rs)
            if c.tip > 0 && self.step_tip(&mut c) { self.tr.carts[k] = c; continue; }
            let top = self.rail_top(c.x.floor() as i32, c.y, 2 * S as i32);
            c.on_rail = top.is_some() && c.vy >= 0.;
            if let Some(t) = top.filter(|_| c.on_rail) {
                c.y = t; c.vy = 0.;
                // the track's slope under the cart: heights half a cart either side (y grows downward)
                let h = |dx: f32| self.rail_top((c.x + dx).floor() as i32, t, 2 * S as i32);
                let s = match (h(-CART_W / 2.), h(CART_W / 2.)) { (Some(a), Some(b)) => (b - a) / CART_W, (Some(a), None) => (t - a) / (CART_W / 2.), (None, Some(b)) => (b - t) / (CART_W / 2.), _ => 0. };
                let (sin, cos) = (s / (1. + s * s).sqrt(), 1. / (1. + s * s).sqrt());
                let mut a = G_CELLS * sin;
                // powered: copperline with current laid along this stretch drives the cart on (step 2 hook)
                a += self.rail_drive(&c);
                // rolling friction opposes motion (and holds a cart still on a slope too gentle to roll)
                let f = ROLL * G_CELLS * cos;
                if c.v.abs() > f * dt { c.v -= c.v.signum() * f * dt; c.v += a * dt; } else if a.abs() > f { c.v += (a - a.signum() * f) * dt; } else { c.v = 0.; }
                c.v = c.v.clamp(-CART_MAX, CART_MAX);
                if c.v.abs() > 0.05 { c.dir = c.v.signum() as i8; }
                // a loaded cart standing against a bumper tips too (eased up to it, or reloaded mid-tip: the tip isn't saved)
                if c.v == 0. && c.load() > 0 && self.bumper_ahead(&c, c.dir) { c.tip = 1; self.tr.tips += 1; self.tr.carts[k] = c; continue; }
                // roll along, cell by cell, following the track's height; stop at its end or at a wall
                let dx = c.v * dt; let n = (dx.abs().ceil() as i32).max(1);
                for _ in 0..n {
                    let nx = c.x + dx / n as f32;
                    match self.rail_top(nx.floor() as i32, c.y, S as i32) {
                        Some(ny) if !self.cart_blocked(nx, ny) => { c.x = nx; c.y = ny; self.shove_from_cart(nx, ny); }
                        _ => {
                            // a buffer (a Bumper block) ahead: a loaded cart tips its load out over it
                            if c.load() > 0 && self.bumper_ahead(&c, c.dir) { c.tip = 1; c.v = 0.; self.tr.tips += 1; self.events.push(Event::Click); break; }
                            if c.v.abs() > 2. * CPM as f32 { self.events.push(Event::Click); self.shake = self.shake.max(0.1); } c.v = -c.v * 0.15; break; } // the bumper
                    }
                }
            } else {
                // off the rails: it falls, and sits where it lands (wheels in the dirt: no rolling)
                c.vy = (c.vy + TUNE.gravity * dt).min(TUNE.terminal); c.v *= 0.8;
                let dy = c.vy * dt; let n = (dy.abs().ceil() as i32).max(1);
                for _ in 0..n { let ny = c.y + dy / n as f32; if self.on_ground(c.x, ny) || self.rail_top(c.x.floor() as i32, ny, 0).is_some() { c.vy = 0.; c.y = ny.floor(); break; } c.y = ny; }
                if c.v.abs() > 0.1 && !self.cart_blocked(c.x + c.v * dt, c.y) { c.x += c.v * dt; }
            }
            // cargo: loose items that fall into it (not a lit charge: that rides along and goes off)
            let (x0, y0, x1, y1) = c.rect();
            let mut j = 0;
            while j < self.drops.len() {
                let d = &self.drops[j];
                if d.fuse == 0 && d.px >= x0 && d.px < x1 && d.py >= y0 - SF && d.py < y1 && c.load() + d.n <= CART_LOAD { let d = self.drops.swap_remove(j); c.add(d.item, d.n); } else { j += 1; }
            }
            let _ = rider;
            self.tr.carts[k] = c;
        }
        // carts meet: they swap their speeds along the track (a fair knock between equals)
        for a in 0..self.tr.carts.len() { for b in a + 1..self.tr.carts.len() {
            let (ca, cb) = (&self.tr.carts[a], &self.tr.carts[b]);
            if (ca.x - cb.x).abs() < CART_W && (ca.y - cb.y).abs() < CART_H && (cb.x - ca.x) * (ca.v - cb.v) > 0. {
                let (va, vb) = (ca.v, cb.v); let (ma, mb) = (ca.mass(self.tr.riding == Some(a)), cb.mass(self.tr.riding == Some(b)));
                let (na, nb) = (((ma - mb) * va + 2. * mb * vb) / (ma + mb), ((mb - ma) * vb + 2. * ma * va) / (ma + mb));
                self.tr.carts[a].v = na * 0.9; self.tr.carts[b].v = nb * 0.9; self.events.push(Event::Click);
            } } }
        self.push_carts();
        self.step_bins();
        self.step_lifts();
        self.step_tubes();
        self.step_ropes();
    }

    /// The mole walking into a cart pushes it: a force on the cart's mass, and the mole is held at its side
    /// (so a heavy cart slows the mole down). Dropping into one from above gets in.
    fn push_carts(&mut self) {
        if self.tr.riding.is_some() || self.moles.is_empty() || !self.moles[0].alive { return; }
        let dt = 1. / 60.; let (px, py) = (self.pl.px, self.pl.py);
        for k in 0..self.tr.carts.len() {
            let c = &self.tr.carts[k]; let (x0, y0, x1, y1) = c.rect();
            let overlap_y = py > y0 && py - BODY_H < y1; let overlap_x = px + BODY_HW > x0 && px - BODY_HW < x1;
            if !overlap_x || !overlap_y { continue; }
            // in from above: feet came down over the rim
            if self.pl.ppy <= y0 + 0.5 && self.pl.vy >= 0. && (px - c.x).abs() < CART_W / 2. - 1. { self.tr.riding = Some(k); self.events.push(Event::Place); return; }
            let side = if px < c.x { -1. } else { 1. }; // the mole is on this side of it
            let mass = c.mass(false); let on = c.on_rail;
            let c = &mut self.tr.carts[k];
            if self.pl.vx * -side > 0. && on {
                // leaning in: the cart is pushed toward the mole's speed, as hard as a mole can push
                let want = self.pl.vx; let acc = PUSH_N / mass * CPM as f32;
                if (want - c.v) * -side > 0. { c.v += (want - c.v).clamp(-acc * dt, acc * dt); }
                c.dir = (-side) as i8;
            }
            // and held at its side, moving no faster than it does
            self.pl.px = if side < 0. { x0 - BODY_HW } else { x1 + BODY_HW };
            if self.pl.vx * -side > 0. { self.pl.vx = c.v; }
            let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y));
        }
    }
    /// Is the track at (x, y) powered: copperline laid along it (in the same 1 m block) that carries current?
    pub fn rail_powered(&self, x: usize, y: usize) -> bool { x < self.g.w && y < self.g.h && self.logic.net_lit(self.logic.block_of(x, y)) }
    /// Extra drive along the track (cells/s²) for a cart on powered rail: the motor pulls it toward
    /// `POWER_V` the way it is running (or last ran), at most `POWER_ACC`. Without current it's plain track.
    pub(crate) fn rail_drive(&self, c: &Cart) -> f32 {
        let (x, y) = (c.x.floor().max(0.) as usize, c.y.max(0.) as usize);
        if !self.rail_powered(x, y) { return 0.; }
        let want = c.dir as f32 * POWER_V; let k = CPM as f32;
        ((want - c.v) * 4.).clamp(-POWER_ACC * k, POWER_ACC * k)
    }

    /// Slide everything by `rows` (the window moved deeper by that many rows); what leaves the window is lost.
    pub(crate) fn shift_transport(&mut self, rows: i64, h: usize) {
        let dy = rows as f32; let riding = self.tr.riding.map(|k| k as i64);
        let mut keep = vec![]; let mut new_ride = None;
        for (k, mut c) in std::mem::take(&mut self.tr.carts).into_iter().enumerate() {
            c.y -= dy; c.py -= dy; if c.y < 2. || c.y >= h as f32 - 2. { continue; }
            if riding == Some(k as i64) { new_ride = Some(keep.len()); } keep.push(c);
        }
        self.tr.carts = keep; self.tr.riding = new_ride;
        self.shift_tubes(rows); self.shift_bins(rows, h); self.shift_ropes(rows);
        let (dr, hh) = (rows, h as i64);
        self.tr.ladders.retain_mut(|l| { let t = l.top as i64 - dr; let a = l.ay as i64 - dr; if t < 1 || a < 0 || t + l.len as i64 >= hh { return false; } l.top = t as usize; l.ay = a as usize; true });
        self.tr.lifts.retain_mut(|l| { let hd = l.head as i64 - dr; let b = l.bottom as i64 - dr; if hd < 1 || b >= hh - 1 { return false; } l.head = hd as usize; l.bottom = b as usize; l.y -= dy; l.py -= dy; true });
    }
    /// Save: the carts (window-relative cells) and their cargo.
    pub fn encode_transport(&self) -> Vec<u8> {
        let mut o = crate::band::Wr(vec![]); o.u8(5);
        o.u32(self.tr.carts.len() as u32);
        for c in &self.tr.carts { o.f32(c.x); o.f32(c.y); o.f32(c.v); o.u8((c.dir + 1) as u8); o.u32(c.cargo.len() as u32); for &(it, n) in &c.cargo { o.u8(it as u8); o.u32(n); } }
        // v2: ladders and lifts
        o.u32(self.tr.ladders.len() as u32); for l in &self.tr.ladders { o.u32(l.x as u32); o.u32(l.top as u32); o.u32(l.len as u32); o.u32(l.ax as u32); o.u32(l.ay as u32); }
        o.u32(self.tr.lifts.len() as u32); for l in &self.tr.lifts { o.u32(l.x as u32); o.u32(l.head as u32); o.f32(l.y); o.u32(l.bottom as u32); o.u32(l.ballast); }
        // v3: pressure tubes
        self.encode_tubes(&mut o);
        // v4: ore hoppers
        self.encode_bins(&mut o);
        // v5: rope-gun ropes
        self.encode_ropes(&mut o);
        o.0
    }
    pub fn decode_transport(&mut self, b: &[u8]) -> Result<(), String> {
        let mut r = crate::band::Rd { b, p: 0 }; let ver = r.u8()?;
        let (gw, gh) = (self.g.w as f32, self.g.h as f32);
        for _ in 0..r.len(4096)? {
            let (x, y, v, dir) = (r.f32()?, r.f32()?, r.f32()?, r.u8()? as i8 - 1);
            let mut cargo = vec![]; for _ in 0..r.len(NITEM)? { let (it, n) = (r.u8()?, r.u32()?); if (it as usize) < NITEM { cargo.push((crate::craft::ITEMS[it as usize], n)); } }
            if x > 1. && x < gw - 1. && y > 1. && y < gh - 1. { self.tr.carts.push(Cart { x, y, px: x, py: y, v, vy: 0., dir: dir.signum().max(-1), on_rail: false, cargo, tip: 0 }); }
        }
        if ver >= 2 {
            let (w, h) = (self.g.w, self.g.h);
            for _ in 0..r.len(4096)? { let (x, top, len, ax, ay) = (r.u32()? as usize, r.u32()? as usize, r.u32()? as usize, r.u32()? as usize, r.u32()? as usize); if x < w && ax < w && top > 0 && top + len < h && ay < h { self.tr.ladders.push(crate::lifts::Ladder { x, top, len, ax, ay }); } }
            for _ in 0..r.len(4096)? { let (x, head, y, bottom, ballast) = (r.u32()? as usize, r.u32()? as usize, r.f32()?, r.u32()? as usize, r.u32()?);
                if x < w && head > 0 && bottom < h - 1 && y.is_finite() { self.tr.lifts.push(crate::lifts::Lift { x, head, y, py: y, v: 0., bottom, ballast }); } }
        }
        if ver >= 3 { self.decode_tubes(&mut r)?; }
        if ver >= 4 { self.decode_bins(&mut r)?; }
        if ver >= 5 { self.decode_ropes(&mut r)?; }
        Ok(())
    }
}
