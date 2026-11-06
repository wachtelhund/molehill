//! Mechanical power: drive trains of shafts, gears, water wheels and belts, turned by running water and stored
//! in a hanging weight. No current and no magic: power is a stream's push on paddles, carried by iron and
//! timber to belts that move real loose items.
//!
//! **Trains.** Machinery cells (flag `MECH`) that touch make one drive train, with one rule: a plain **iron
//! shaft** only passes turning along its own line (left-right, or up-down), so to turn a corner or branch, a
//! **gear** (or any `HUB`: gear, water wheel, belt, weight drum) has to sit where the lines meet. Trains are
//! found again every `MECH_EVERY` ticks over the active region (`step_power`).
//!
//! **Power.** Each train adds up what turns it and what it has to turn, in watts:
//! * a **water wheel** is undershot: wherever its lowest paddles dip into a running stream, the current (read
//!   beside the wheel, see sluice.rs; its bottom paddles reach a quarter metre down) pushes them with ½·ρ·A·v³·η (`WHEEL_K`·v³ per paddle cell), so a 2 m/s
//!   stream gives about 150 W a cell;
//! * a **belt** costs `BELT_W` a cell to keep rolling, and `ITEM_W` for each lump riding on it;
//! * a **shaker screen** beside a turning train is shaken by it (as copperline current does) for `SCREEN_W`;
//! * a **counterweight** (a drum with a hanging iron weight) makes up a shortfall by falling, m·g·v up to
//!   `WEIGHT_V`, and takes spare power by winding up the same way, until it hits the floor or its drum.
//!
//! The train turns at the share of its demand it can meet (1 at most): a belt runs at `BELT_V` times that.
//!
//! **Belts.** A loose item lying on a belt is gripped by it (its friction) and carried toward the belt's speed,
//! the way the belt was laid (`Mat::Belt` right, `Mat::BeltL` left; right mouse turns a run round). Lumps on a
//! belt **queue**: one can't pass the lump ahead of it, so a blocked end backs the line up, a full belt pushes
//! its tail off the back, and an open end drops them off.
use std::collections::HashMap;

use crate::craft::Item;
use crate::items::ITEM_HW;
use crate::mat::{Mat, BELT, CPM, HUB, MECH, SCREEN};
use crate::world::{Drop, Event, World};

/// Ticks between solving the trains (and moving counterweights).
pub const MECH_EVERY: u64 = 6;
/// A belt's speed with its train fully driven (cells/s, 1 m/s).
pub const BELT_V: f32 = CPM as f32;
/// Watts: a belt cell rolling, a lump carried, a screen cell shaken.
pub const BELT_W: f32 = 2.;
pub const ITEM_W: f32 = 5.;
pub const SCREEN_W: f32 = 3.;
/// Water wheel: W per paddle cell per (m/s)³ of stream: ½·1000 kg/m³ · (1/8 m × 1 m) · 30 % efficiency.
pub const WHEEL_K: f32 = 0.5 * 1000. * (1. / CPM as f32) * 0.3;
/// Counterweight: its iron mass (kg), the fastest it falls or winds (m/s), and the longest rope (m).
pub const WEIGHT_KG: f32 = 200.;
pub const WEIGHT_V: f32 = 0.25;
pub const ROPE_M: usize = 8;

#[derive(Clone, Debug)]
pub struct Weight {
    /// the drum's cell (it is a `Mat::Drum` cell of the train), and the weight's top below it (cells, f32)
    pub x: usize, pub drum: usize, pub y: f32, pub py: f32,
    /// the lowest the weight can go (the floor under it, or the end of the rope)
    pub bottom: f32,
}
impl Weight {
    /// How far it can still fall / be wound up (cells).
    fn room_down(&self) -> f32 { (self.bottom - self.y).max(0.) }
    fn room_up(&self) -> f32 { (self.y - (self.drum + 2) as f32).max(0.) }
}

/// The drive trains as last solved.
#[derive(Clone, Debug, Default)]
pub struct Power {
    /// machinery cell → its train
    pub train: HashMap<usize, u32>,
    /// per train: how fast it turns (0..1 of full), what it could give and what it has to do (W)
    pub speed: Vec<f32>, pub supply: Vec<f32>, pub demand: Vec<f32>,
    pub weights: Vec<Weight>,
    /// the machinery cells, found by a scan of the active region every `SCAN_EVERY` ticks (and added as they're
    /// built in between); checked each solve
    pub cells: Vec<usize>, pub scanned: u64,
}
/// Ticks between full scans for machinery (a second): built machinery joins at once, other changes within this.
pub const SCAN_EVERY: u64 = 60;

impl World {
    /// The train under the cell and how fast it turns (0..1), if it is machinery.
    pub fn turning(&self, i: usize) -> f32 { self.tr.power.train.get(&i).map_or(0., |&t| self.tr.power.speed.get(t as usize).copied().unwrap_or(0.)) }
    /// A belt cell's speed along x (cells/s), if (x, y) is a belt.
    pub(crate) fn belt_speed(&self, x: usize, y: usize) -> Option<f32> {
        let m = self.g.get(x, y); if !m.has(BELT) { return None; }
        let dir = if m == Mat::BeltL { -1. } else { 1. };
        Some(dir * BELT_V * self.turning(y * self.g.w + x))
    }
    /// The belt under a loose item lying at (px, py), if any: its speed (cells/s).
    pub(crate) fn belt_under(&self, px: f32, py: f32) -> Option<f32> {
        let r = (py + ITEM_HW + 0.05).floor() as i32; let (x0, x1) = ((px - ITEM_HW).floor() as i32, (px + ITEM_HW - 1e-3).floor() as i32);
        let mid = px.floor() as i32;
        std::iter::once(mid).chain(x0..=x1).filter(|&c| self.g.in_bounds(c, r)).find_map(|c| self.belt_speed(c as usize, r as usize))
    }
    /// Is a screen cell shaken by a turning train right beside it?
    /// (its knocker reaches the grating within a tile of the turning machinery)
    pub(crate) fn mech_shakes(&self, x: usize, y: usize) -> bool {
        let r = crate::mat::S as i32 + 1; let (gw, gh) = (self.g.w as i32, self.g.h as i32);
        (-r..=r).any(|dy| (-r..=r).any(|dx| { let (nx, ny) = (x as i32 + dx, y as i32 + dy); nx >= 0 && ny >= 0 && nx < gw && ny < gh && self.turning(ny as usize * self.g.w + nx as usize) > 0.05 }))
    }

    /// Right mouse on a belt: the whole run (the belt cells joined to it) turns round.
    pub(crate) fn turn_belt(&mut self, x: usize, y: usize) -> bool {
        if x >= self.g.w || y >= self.g.h || !self.g.get(x, y).has(BELT) { return false; }
        let mut seen = vec![y * self.g.w + x]; let mut k = 0;
        while k < seen.len() && seen.len() < 4096 { let i = seen[k]; k += 1;
            for j in self.g.n4(i % self.g.w, i / self.g.w).collect::<Vec<_>>() { if self.g.m(j).has(BELT) && !seen.contains(&j) { seen.push(j); } } }
        for &i in &seen { let m = if self.g.m(i) == Mat::Belt { Mat::BeltL } else { Mat::Belt }; self.g.mat[i] = m as u8; }
        self.say(format!("Turned the belt round: it runs {} now.", if self.g.m(seen[0]) == Mat::Belt { "right" } else { "left" }));
        self.events.push(Event::Click); true
    }

    /// Hang a counterweight: its drum under the rock (or a beam) above the cursor, the weight on its rope below it
    /// down to the floor (at most `ROPE_M`).
    pub(crate) fn place_weight(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let roof = (1..4 * CPM).map(|k| y.saturating_sub(k)).take_while(|&r| r > 0).find(|&r| !self.passable(x, r)).ok_or("needs a roof above to hang from")?;
        let drum = roof + 1;
        if !self.passable(x, drum) || !self.passable(x, drum + 1) { return Err("needs a drop under the roof"); }
        let floor = (drum + 1..(drum + 1 + ROPE_M * CPM).min(self.g.h - 1)).find(|&r| !self.passable(x, r)).unwrap_or(drum + ROPE_M * CPM);
        if floor < drum + 3 * CPM / 2 { return Err("needs at least a metre and a half of drop"); }
        let i = self.g.idx(x, drum); self.g.set(i, Mat::Drum); self.tr.power.cells.push(i);
        let y0 = (drum + 2) as f32;
        self.tr.power.weights.push(Weight { x, drum, y: y0, py: y0, bottom: (floor - CPM / 2) as f32 });
        self.events.push(Event::Place); Ok(())
    }
    pub fn weight_at(&self, x: usize, y: usize) -> Option<usize> {
        self.tr.power.weights.iter().position(|w| x.abs_diff(w.x) <= CPM / 4 && y >= w.drum && (y as f32) < w.y + (CPM / 2) as f32)
    }
    /// `E` on a counterweight: take it down (drum and weight).
    pub(crate) fn pick_up_weight(&mut self, x: usize, y: usize) -> Option<Item> {
        let k = self.weight_at(x, y)?; let w = self.tr.power.weights.remove(k);
        let i = self.g.idx(w.x, w.drum); if self.g.m(i) == Mat::Drum { self.g.set(i, Mat::Empty); }
        self.inv[Item::Counterweight as usize] += 1; Some(Item::Counterweight)
    }

    /// Find the trains, balance each one's power, move the counterweights.
    pub(crate) fn step_power(&mut self) {
        if self.tick % MECH_EVERY != 0 { return; }
        self.step_boilers(MECH_EVERY as f32 / 60.);
        self.step_works(MECH_EVERY as f32 / 60.);
        let (gw, gh) = (self.g.w, self.g.h);
        // weights whose drum is gone fall as an item
        let mut k = 0;
        while k < self.tr.power.weights.len() { let w = &self.tr.power.weights[k];
            if self.g.m(self.g.idx(w.x, w.drum)) != Mat::Drum { let w = self.tr.power.weights.remove(k);
                self.drops.push(Drop::at(w.x as f32 + 0.5, w.y, Item::Counterweight, 1)); self.say("A counterweight's drum broke: the weight fell.".into()); continue; }
            k += 1; }
        // the machinery in the active region (the window if the CA has none)
        let (x0, y0, x1, y1) = self.g.active.map_or((0, 0, gw, gh), |(a, b, c, d)| (a * crate::grid::CHUNK, b * crate::grid::CHUNK, ((c + 1) * crate::grid::CHUNK).min(gw), ((d + 1) * crate::grid::CHUNK).min(gh)));
        let rescan = self.tr.power.scanned == 0 || self.tick >= self.tr.power.scanned + SCAN_EVERY;
        let mut cells = std::mem::take(&mut self.tr.power.cells);
        if rescan { cells.clear(); self.tr.power.scanned = self.tick.max(1);
            for y in y0.max(1)..y1.min(gh - 1) { let row = &self.g.mat[y * gw..(y + 1) * gw];
                for x in x0.max(1)..x1.min(gw - 1) { if crate::mat::PROPS[row[x] as usize].flags & MECH != 0 { cells.push(y * gw + x); } } } }
        else { cells.retain(|&i| i < self.g.mat.len() && self.g.m(i).has(MECH)); cells.sort_unstable(); cells.dedup(); }
        // trains: flood over (cell, axis) states; a shaft carries along its axis only (0 across, 1 up-down), a hub
        // every way (its state's axis is 2)
        let mut train: HashMap<usize, u32> = HashMap::with_capacity(cells.len());
        let mut seen: std::collections::HashSet<(usize, u8)> = std::collections::HashSet::with_capacity(cells.len() * 2);
        let state = |w: &World, i: usize, a: u8| if w.g.m(i).has(HUB) { (i, 2u8) } else { (i, a) };
        let mut n = 0u32;
        for &s in &cells {
            if train.contains_key(&s) || self.g.m(s) == Mat::Clutch && !self.pulled(s) { continue; }
            let mut todo: Vec<(usize, u8)> = if self.g.m(s).has(HUB) { vec![(s, 2)] } else { vec![(s, 0), (s, 1)] };
            for &st in &todo { seen.insert(st); }
            while let Some((i, ax)) = todo.pop() {
                train.insert(i, n);
                let (x, y) = (i % gw, i / gw);
                let dirs: &[(i32, i32, u8)] = match ax { 2 => &[(1, 0, 0), (-1, 0, 0), (0, 1, 1), (0, -1, 1)], 0 => &[(1, 0, 0), (-1, 0, 0)], _ => &[(0, 1, 1), (0, -1, 1)] };
                for &(dx, dy, a) in dirs {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(nx, ny) { continue; }
                    let j = ny as usize * gw + nx as usize; if !self.g.m(j).has(MECH) { continue; }
                    // a clutch joins the drive only while its pull-rod is pulled (linkage.rs)
                    if self.g.m(j) == Mat::Clutch && !self.pulled(j) { continue; }
                    let st = state(self, j, a); if seen.insert(st) { todo.push(st); }
                }
            }
            n += 1;
        }
        let nt = n as usize;
        let (mut supply, mut demand) = (vec![0f32; nt], vec![0f32; nt]);
        // (a clutch let out is in no train)
        for &i in &cells { let Some(&t) = train.get(&i) else { continue }; let t = t as usize; let m = self.g.m(i); let (x, y) = (i % gw, i / gw);
            if m.has(BELT) { demand[t] += BELT_W; }
            // a bottom paddle (open or water under it) reaches down a quarter metre into a stream running under the wheel
            if m == Mat::Wheel && y + 3 < gh && self.g.m(i + gw) != Mat::Wheel {
                if let Some(k) = (1..=2).find(|&k| self.g.m(i + k * gw) == Mat::Water) { supply[t] += WHEEL_K * self.stream_by_wheel(x, y + k).powi(3); } }
            for j in self.g.n4(x, y) { if self.g.m(j).has(SCREEN) { demand[t] += SCREEN_W; } }
        }
        // stamp mills and washers at work, and the bellows of a lit smelter (refine.rs)
        self.works_demand(&train, &mut demand);
        // what rides the belts
        for d in &self.drops { let r = (d.py + ITEM_HW + 0.05).floor() as usize; let c = d.px.floor() as usize;
            if r < gh && c < gw { if let Some(&t) = train.get(&(r * gw + c)) { if self.g.m(r * gw + c).has(BELT) { demand[t as usize] += ITEM_W * d.n as f32; } } } }
        // a counterweight's floor is what lies under it now (something built under it stops it; cleared, it goes on)
        for k in 0..self.tr.power.weights.len() { let w = &self.tr.power.weights[k];
            // (down its own line, as it was hung: the rope runs plumb under the drum)
            let (top, end) = ((w.drum + 2) as i32, (w.drum + 1 + ROPE_M * CPM).min(gh - 1) as i32);
            let blocked = |r: i32| w.x >= gw || !self.passable(w.x, r as usize);
            let floor = (top..end).find(|&r| blocked(r)).unwrap_or(end);
            let b = (floor - (CPM / 2) as i32).max(top) as f32;
            let w = &mut self.tr.power.weights[k]; w.bottom = b; if w.y > b { w.y = b; } }
        // steam engines cover what the water wheels leave short, drawing steam from their boilers
        let dt0 = MECH_EVERY as f32 / 60.;
        for t in 0..nt { let short = demand[t] - supply[t]; if short <= 0. { continue; }
            let cap = self.engine_cap(&train, t as u32, dt0); if cap <= 0. { continue; }
            let w = short.min(cap); supply[t] += w; self.engine_draw(&train, t as u32, w, dt0); }
        // counterweights make up the difference: falling to give, winding to take
        let dt = MECH_EVERY as f32 / 60.; let g = 9.81; let cpm = CPM as f32;
        let mut wsup = vec![0f32; nt];
        for w in self.tr.power.weights.iter_mut() {
            w.py = w.y;
            let Some(&t) = train.get(&(w.drum * gw + w.x)) else { continue }; let t = t as usize;
            // (what the weights before it on this train already give or take is already counted)
            let short = demand[t] - supply[t] - wsup[t];
            let v = if short > 0. { (short / (WEIGHT_KG * g)).min(WEIGHT_V).min(w.room_down() / cpm / dt) } else { -((-short) / (WEIGHT_KG * g)).min(WEIGHT_V).min(w.room_up() / cpm / dt) };
            w.y += v * cpm * dt; wsup[t] += v * WEIGHT_KG * g;
        }
        let speed: Vec<f32> = (0..nt).map(|t| { let s = supply[t] + wsup[t]; if demand[t] <= 0. { if s > 0. { 1. } else { 0. } } else { (s / demand[t]).clamp(0., 1.) } }).collect();
        for t in 0..nt { supply[t] += wsup[t].max(0.); }
        self.tr.power.cells = cells;
        self.tr.power.train = train; self.tr.power.speed = speed; self.tr.power.supply = supply; self.tr.power.demand = demand;
    }
    /// The stream's speed (m/s) at a water wheel's foot: read just upstream or downstream of the wheel, where the
    /// water's surface is free (under the wheel it's capped by the paddles).
    fn stream_by_wheel(&self, x: usize, y: usize) -> f32 {
        let gw = self.g.w; let row = y * gw;
        let probe = |dir: i32| -> f32 {
            let mut c = x as i32;
            for _ in 0..2 * CPM { c += dir; if c < 1 || c as usize >= gw - 1 { return 0.; } if self.g.m(row + c as usize) != Mat::Wheel { break; } }
            for _ in 0..CPM / 2 { if c < 1 || c as usize >= gw - 1 { break; }
                if let Some((v, _)) = crate::sluice::current(self, c as f32 + 0.5, y as f32 + 0.5) { return v.abs(); } c += dir; }
            0.
        };
        probe(-1).max(probe(1))
    }

    /// A mole standing on a turning belt is carried along with it (a moving floor: walking adds to it).
    pub(crate) fn carry_on_belt(&mut self) {
        if !self.pl.grounded || self.tr.riding.is_some() || !self.moles.first().is_some_and(|m| m.alive) { return; }
        let (px, py) = (self.pl.px, self.pl.py); let r = py.floor() as i32;
        let (x0, x1) = ((px - crate::world::BODY_HW).floor() as i32, (px + crate::world::BODY_HW - 1e-3).floor() as i32);
        let Some(vb) = std::iter::once(px.floor() as i32).chain(x0..=x1).filter(|&c| self.g.in_bounds(c, r)).find_map(|c| self.belt_speed(c as usize, r as usize)) else { return };
        if vb == 0. { return; }
        let nx = px + vb / 60.;
        if self.box_hits(nx, py) { return; }
        self.pl.px = nx; let (x, y) = (nx as usize, (py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y));
    }

    /// Lumps on a belt queue: none passes the one ahead of it on the same belt row.
    pub(crate) fn belt_queue(&mut self) {
        if self.drops.is_empty() { return; }
        let gw = self.g.w;
        // (row, belt dir) -> indices of the items riding there
        let mut rows: HashMap<(usize, i8), Vec<usize>> = HashMap::new();
        for (k, d) in self.drops.iter().enumerate() {
            let r = (d.py + ITEM_HW + 0.05).floor(); if r < 0. || r as usize >= self.g.h { continue; }
            let Some(v) = self.belt_under(d.px, d.py) else { continue }; let _ = gw;
            let dir = if v > 0. { 1 } else if v < 0. { -1 } else { 0 }; if dir == 0 { continue; }
            rows.entry((r as usize, dir)).or_default().push(k);
        }
        for ((_, dir), mut ks) in rows {
            let df = dir as f32;
            // front first: the one furthest along the way the belt runs
            ks.sort_by(|&a, &b| (self.drops[b].px * df).total_cmp(&(self.drops[a].px * df)));
            for w in 1..ks.len() {
                let (ahead, k) = (ks[w - 1], ks[w]);
                let (ax, avx) = (self.drops[ahead].px, self.drops[ahead].vx);
                let gap = (ax - self.drops[k].px) * df; let min = 2. * ITEM_HW;
                if gap < min && (self.drops[ahead].py - self.drops[k].py).abs() < ITEM_HW {
                    // held back behind it (if there's room behind: else it stays wedged, a jam)
                    let nx = ax - df * min; let (it, py) = (self.drops[k].item, self.drops[k].py);
                    if self.item_box_hits_pub(nx, py, it) { continue; }
                    let d = &mut self.drops[k];
                    d.px = nx; d.x = nx.max(0.) as usize; if d.vx * df > avx * df { d.vx = avx; }
                }
            }
        }
    }

    pub(crate) fn shift_power(&mut self, rows: i64, h: usize) {
        let dy = rows as f32; let hh = h as i64;
        self.tr.power.weights.retain_mut(|w| { let d = w.drum as i64 - rows; if d < 1 || (w.bottom - dy) as i64 >= hh - 1 { return false; }
            w.drum = d as usize; w.y -= dy; w.py -= dy; w.bottom -= dy; true });
        self.tr.power.train.clear(); self.tr.power.cells.clear(); self.tr.power.scanned = 0;
    }
    pub(crate) fn encode_power(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.power.weights.len() as u32);
        for w in &self.tr.power.weights { o.u32(w.x as u32); o.u32(w.drum as u32); o.f32(w.y); o.f32(w.bottom); }
    }
    pub(crate) fn decode_power(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (gw, gh) = (self.g.w, self.g.h);
        for _ in 0..r.len(4096)? { let (x, drum, y, bottom) = (r.u32()? as usize, r.u32()? as usize, r.f32()?, r.f32()?);
            if x < gw && drum + 2 < gh && y.is_finite() && bottom.is_finite() && bottom < gh as f32 { self.tr.power.weights.push(Weight { x, drum, y, py: y, bottom }); } }
        Ok(())
    }
}
