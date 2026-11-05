//! World = grid + structure + light + colony (moles, jobs, items, stations, chests, quests).
use crate::craft::*;
use crate::gen::{self, Gen, GenInfo};
use crate::grid::{hash3, Grid, F_SEEN, F_WET};
use crate::light::{self, Light};
use crate::mat::{Kind, Mat, NMAT};
use crate::stress::Stress;
use crate::logic::{Dev, Logic};
use crate::disasters::Disasters;
use crate::air::{self, Air};
use std::collections::VecDeque;
use std::sync::atomic::Ordering::Relaxed;

pub const D_DIG: u8 = 1; pub const D_WOOD: u8 = 2; pub const D_BRICK: u8 = 3; pub const D_IRON: u8 = 4;
pub const MOLE_H: usize = 3;
const MOVE_TICKS: u32 = 3;
const REACH: i32 = 3;

#[derive(Clone, Debug, PartialEq)]
pub enum Job { Dig, Build, Haul(usize), Deliver, Craft(usize), Station(usize), Flee, Eat, Sleep }

#[derive(Clone)]
pub struct Mole {
    pub name: &'static str, pub x: usize, pub y: usize, pub hp: f32, pub breath: f32,
    pub path: VecDeque<(usize, usize)>, pub job: Option<Job>, pub work: u32, pub work_cell: Option<usize>,
    pub carry: Option<(Item, u32)>, pub hunger: f32, pub sleep: f32, pub facing: i8, pub fall: u32, pub idle: u32, pub anim: u32, pub alive: bool,
}
#[derive(Clone)]
pub struct Drop { pub x: usize, pub y: usize, pub item: Item, pub n: u32, pub claimed: bool }
/// A mushroom plot: the open cell where the mushroom stands, on soil. `growth` 0..1, `why` = what's
/// holding it back (bit 1 too bright, bit 2 too dry).
#[derive(Clone)]
pub struct Farm { pub x: usize, pub y: usize, pub growth: f32, pub why: u8 }
#[derive(Clone)]
pub struct Chest { pub x: usize, pub y: usize, pub opened: bool }
#[derive(Clone)]
pub struct Building { pub kind: Station, pub x: usize, pub y: usize, pub built: bool, pub claimed: bool }

pub struct World {
    pub g: Grid, pub st: Stress, pub light: Light, pub info: GenInfo, pub tick: u64,
    pub inv: [u32; NITEM], dig_acc: [u16; NMAT], place_acc: [u16; 8], pub desig: Vec<u8>,
    pub moles: Vec<Mole>, pub drops: Vec<Drop>, pub stockpile: (usize, usize, usize, usize),
    pub buildings: Vec<Building>, pub torches: Vec<(usize, usize)>, pub chests: Vec<Chest>,
    pub quest: usize, pub log: Vec<(u64, String)>, pub discovered: [bool; NMAT], pub chests_opened: u32, pub pillars: u32,
    pub max_depth: f32, pub craft_queue: Vec<usize>, pub day: f32, pub crumbled: u64, pub light_rect: (usize, usize, usize, usize),
    pub events: Vec<Event>, pub react_pending: bool,
    pub logic: Logic, pub disasters: Disasters, pub shake: f32,
    pub air: Air, pub farms: Vec<Farm>, pub harvested: u32,
    /// Endless vertical world: the grid is a window; `oy` = absolute row of window row 0.
    pub gen: Gen, pub oy: i64, pub cache: std::collections::HashMap<i64, (Vec<u8>, Vec<u8>)>, pub opened: std::collections::HashSet<(usize, i64)>,
}
/// For audio / UI feedback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event { Dig, Crumble, Crack, Place, Chest, Quest, Explode, Discover, Hurt, Craft, Eat, Death, Warning, Quake, Click, Harvest, Gasp }

const NAMES: [&str; 8] = ["Mossy", "Pip", "Bramble", "Tuck", "Clover", "Dusty", "Fern", "Nib"];

impl World {
    pub fn new(w: usize, h: usize, seed: u32) -> World {
        let mut g = Grid::new(w, h);
        let oy = -(h as i64 / 2); let (gn, info) = gen::generate(&mut g, seed, oy);
        let mut wd = World {
            st: Stress::new(w * h), light: Light::new(w, h), tick: 0, inv: [0; NITEM], dig_acc: [0; NMAT], place_acc: [0; 8], desig: vec![0; w * h],
            moles: vec![], drops: vec![], stockpile: (0, 0, 0, 0), buildings: vec![], torches: vec![], chests: vec![],
            quest: 0, log: vec![], discovered: [false; NMAT], chests_opened: 0, pillars: 0, max_depth: 0., craft_queue: vec![], day: 0., crumbled: 0,
            light_rect: (0, 0, w - 1, h - 1), events: vec![], react_pending: false, logic: Logic::new(w, h), disasters: Disasters::new(seed), shake: 0., air: Air::new(w, h), farms: vec![], harvested: 0, gen: gn, oy, cache: Default::default(), opened: Default::default(), info, g,
        };
        wd.refresh_chests();
        let (sx, sy) = (wd.info.start.0 * 4, wd.info.start.1 * 4);
        // floor of the start chamber: the first support below
        let mut fy = sy.min(h - 6); while fy < h - 5 && !wd.g.is_solidish(wd.g.idx(sx, fy + 1)) { fy += 1; }
        wd.stockpile = (sx + 6, fy - 2, sx + 16, fy);
        for k in 0..4 { wd.moles.push(Mole { name: NAMES[k], x: sx - 8 + k * 4, y: fy, hp: 100., breath: 100., path: VecDeque::new(), job: None, work: 0, work_cell: None, carry: None, hunger: 80. + k as f32 * 4., sleep: 70. + k as f32 * 7., facing: 1, fall: 0, idle: 0, anim: k as u32 * 7, alive: true }); }
        light::reveal_box(&mut wd.g, sx - 60, fy.saturating_sub(42), sx + 60, (fy + 52).min(h - 1)); // the colony knows its neighbourhood
        { let b = wd.logic.block_of(sx, fy - 5); let gg = &mut wd.g; wd.logic.place(gg, Dev::Vent, b); } // the burrow's air shaft
        wd.torches.push((sx - 12, fy - 2)); wd.torches.push((sx + 12, fy - 2));
        for (it, n) in [(Item::Wood, 8), (Item::Torch, 6), (Item::WoodPillar, 12), (Item::Grub, 12), (Item::CopperWire, 40), (Item::Gate, 6), (Item::Thrummer, 3), (Item::Spore, 6), (Item::Vent, 1)] { wd.inv[it as usize] = n; }
        for k in 0..NMAT { if wd.g.mat.iter().zip(&wd.g.flags).any(|(&m, &f)| m as usize == k && f & F_SEEN != 0) { wd.discovered[k] = true; } }
        // settle the structure once so crack timers start from the real state
        let f = wd.st.solve(&wd.g); wd.st.crack(&f); wd.g.struct_dirty = None;
        wd.say("A new colony digs in. Seed ".to_string() + &seed.to_string());
        wd
    }
    pub fn say(&mut self, s: String) { self.log.push((self.tick, s)); if self.log.len() > 60 { self.log.remove(0); } }
    pub fn depth_m(&self, _x: usize, y: usize) -> f32 { (self.oy + y as i64) as f32 * 0.25 }
    /// Chests for every 128-row band overlapping the window (opened ones stay opened).
    pub fn refresh_chests(&mut self) {
        let h = self.g.h as i64; self.chests.clear();
        for band in self.oy.div_euclid(128)..=(self.oy + h).div_euclid(128) {
            for (x, wy) in self.gen.chests_in_band(band, self.g.w) {
                let y = wy - self.oy; if y < 4 || y >= h - 4 { continue; }
                self.chests.push(Chest { x, y: y as usize, opened: self.opened.contains(&(x, wy)) });
            }
        }
    }
    /// Slide the window by `rows` (multiple of 64; + = deeper). Rows leaving are cached; rows entering
    /// come from the cache or the generator. Everything with a y coordinate moves with the world.
    pub fn shift(&mut self, rows: i64) {
        let (w, h) = (self.g.w, self.g.h); let s = rows.unsigned_abs() as usize; if s == 0 || s >= h || s % 64 != 0 { return; }
        for c in &self.chests { if c.opened { self.opened.insert((c.x, self.oy + c.y as i64)); } }
        let leaving: Vec<usize> = if rows > 0 { (0..s).collect() } else { (h - s..h).collect() };
        for &y in &leaving { self.cache.insert(self.oy + y as i64, (self.g.mat[y * w..(y + 1) * w].to_vec(), self.g.flags[y * w..(y + 1) * w].to_vec())); }
        let d = s * w;
        if rows > 0 {
            self.g.mat.copy_within(d.., 0); self.g.flags.copy_within(d.., 0); self.g.shade.copy_within(d.., 0); self.desig.copy_within(d.., 0);
        } else {
            self.g.mat.copy_within(..w * h - d, d); self.g.flags.copy_within(..w * h - d, d); self.g.shade.copy_within(..w * h - d, d); self.desig.copy_within(..w * h - d, d);
        }
        self.oy += rows;
        let entering: Vec<usize> = if rows > 0 { (h - s..h).collect() } else { (0..s).collect() };
        let (e0, e1) = (entering[0], entering[entering.len() - 1] + 1);
        for y in e0..e1 { for x in 0..w { self.desig[y * w + x] = 0; } }
        let fresh: Vec<usize> = entering.iter().copied().filter(|&y| !self.cache.contains_key(&(self.oy + y as i64))).collect();
        if !fresh.is_empty() { let (a, b) = (fresh[0], fresh[fresh.len() - 1] + 1); self.gen.fill_rows(&mut self.g, self.oy, a, b); let mut st = Stress::new(w * h); gen::stabilize_rows(&mut self.g, &mut st, a.saturating_sub(8), (b + 8).min(h)); }
        for &y in &entering { if let Some((m, f)) = self.cache.remove(&(self.oy + y as i64)) { self.g.mat[y * w..(y + 1) * w].copy_from_slice(&m); self.g.flags[y * w..(y + 1) * w].copy_from_slice(&f); } }
        // entities
        let dy = |y: usize| -> Option<usize> { let v = y as i64 - rows; if v >= 0 && v < h as i64 { Some(v as usize) } else { None } };
        for m in self.moles.iter_mut() { if let Some(v) = dy(m.y) { m.y = v; } else { m.alive = false; } m.path.clear(); m.job = None; m.work_cell = None; }
        self.drops = std::mem::take(&mut self.drops).into_iter().filter_map(|mut d| dy(d.y).map(|v| { d.y = v; d })).collect();
        self.torches = std::mem::take(&mut self.torches).into_iter().filter_map(|(x, y)| dy(y).map(|v| (x, v))).collect();
        self.buildings = std::mem::take(&mut self.buildings).into_iter().filter_map(|mut b| dy(b.y).filter(|&v| v >= Station::H).map(|v| { b.y = v; b.claimed = false; b })).collect();
        self.farms = std::mem::take(&mut self.farms).into_iter().filter_map(|mut f| dy(f.y).map(|v| { f.y = v; f })).collect();
        let sp = self.stockpile; if let (Some(a), Some(b)) = (dy(sp.1), dy(sp.3)) { self.stockpile = (sp.0, a, sp.2, b); }
        self.logic.shift_rows(rows / crate::logic::B as i64);
        self.air.shift_rows(rows / air::AB as i64);
        self.disasters.y = (self.disasters.y as i64 - rows).clamp(8, h as i64 - 8) as usize;
        self.refresh_chests();
        self.g.life.iter_mut().for_each(|v| *v = 0); self.g.wake_all();
        self.st = Stress::new(w * h); let f = self.st.solve(&self.g); self.st.crack(&f); self.g.struct_dirty = None;
        self.light = Light::new(w, h);
    }
    /// Keep the colony inside the window: slide when living moles crowd the top or bottom.
    fn follow_colony(&mut self) {
        let h = self.g.h; let ys: Vec<usize> = self.moles.iter().filter(|m| m.alive).map(|m| m.y).collect(); if ys.is_empty() { return; }
        let (lo, hi) = (*ys.iter().min().unwrap(), *ys.iter().max().unwrap());
        if hi > h - 100 && lo > 160 { self.shift(128); } else if lo < 100 && hi < h - 160 { self.shift(-128); }
    }

    #[inline] fn passable(&self, x: usize, y: usize) -> bool { let m = self.g.get(x, y); matches!(m, Mat::Empty | Mat::Steam | Mat::Water) && !self.is_building_cell(x, y, false) }
    fn is_building_cell(&self, _x: usize, _y: usize, _solid: bool) -> bool { false }
    pub fn standable(&self, x: usize, y: usize) -> bool {
        let g = &self.g; if x < 1 || x + 1 >= g.w || y < MOLE_H || y + 1 >= g.h { return false; }
        for k in 0..MOLE_H { if !self.passable(x, y - k) { return false; } }
        if g.is_solidish(g.idx(x, y + 1)) { return true; }
        for k in 0..MOLE_H { if g.is_static(g.idx(x - 1, y - k)) || g.is_static(g.idx(x + 1, y - k)) { return true; } }
        g.is_static(g.idx(x, y - MOLE_H))
    }
    fn bfs(&self, from: (usize, usize), max: usize, goal: impl Fn(usize, usize) -> bool) -> Option<VecDeque<(usize, usize)>> {
        let w = self.g.w; let mut prev: std::collections::HashMap<usize, usize> = Default::default();
        let s = from.1 * w + from.0; prev.insert(s, s); let mut q = VecDeque::from([s]);
        while let Some(c) = q.pop_front() {
            let (x, y) = (c % w, c / w);
            if goal(x, y) { let mut p = VecDeque::new(); let mut k = c; while k != s { p.push_front((k % w, k / w)); k = prev[&k]; } return Some(p); }
            if prev.len() > max { break; }
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, -1), (0, 1), (1, -1), (-1, -1), (1, 1), (-1, 1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy); if nx < 1 || ny < 1 || nx as usize >= w - 1 || ny as usize >= self.g.h - 1 { continue; }
                let n = ny as usize * w + nx as usize; if prev.contains_key(&n) { continue; }
                if !self.standable(nx as usize, ny as usize) { continue; }
                let step_up = dy < 0 && self.g.is_solidish(self.g.idx(nx as usize, y)) && y >= MOLE_H && self.passable(x, y - MOLE_H); // hop onto a one-cell ledge
                if dx != 0 && dy != 0 && !step_up && !(self.standable(nx as usize, y) || self.standable(x, ny as usize)) { continue; }
                prev.insert(n, c); q.push_back(n);
            }
        }
        None
    }
    fn tier_ok(&self, m: Mat) -> bool { m != Mat::Bedrock && m.p().tier <= tool_tier(&self.inv).max(1).min(3) + if m.p().tier <= 1 { 1 } else { 0 } }
    fn diggable(&self, i: usize) -> bool { let m = self.g.m(i); matches!(m.kind(), Kind::Solid | Kind::Powder) && m != Mat::Bedrock && self.tier_ok(m) }
    fn dig_target_near(&self, x: usize, y: usize) -> Option<usize> {
        let w = self.g.w; let mut best = None; let mut bd = i32::MAX;
        for dy in -REACH - 1..=REACH - 1 { for dx in -REACH..=REACH {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(nx, ny) { continue; }
            let i = ny as usize * w + nx as usize;
            if self.desig[i] == D_DIG && self.diggable(i) && self.g.n4(nx as usize, ny as usize).any(|j| !self.g.is_solidish(j)) {
                let d = dx * dx + (dy + 1) * (dy + 1); if d < bd { bd = d; best = Some(i); }
            }
        } }
        best
    }
    fn build_target_near(&self, x: usize, y: usize) -> Option<usize> {
        let w = self.g.w;
        for dy in -REACH - 1..=REACH - 1 { for dx in -REACH..=REACH {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(nx, ny) { continue; }
            let (ux, uy) = (nx as usize, ny as usize); let i = uy * w + ux; let d = self.desig[i];
            if d >= D_WOOD && self.g.mat[i] == 0 && self.has_build_item(d) && (ux as i32 - x as i32).abs() + (uy as i32 - y as i32).abs() > 0
                && self.g.n4(ux, uy).any(|j| self.g.is_solidish(j)) && !self.moles.iter().any(|m| m.x == ux && m.y >= uy && m.y < uy + MOLE_H) { return Some(i); }
        } }
        None
    }
    fn has_build_item(&self, d: u8) -> bool { let it = build_item(d); self.inv[it as usize] > 0 || self.place_acc[d as usize] > 0 }

    pub fn designate(&mut self, cx: usize, cy: usize, r: usize, code: u8) {
        let r2 = (r * r) as i32;
        for dy in -(r as i32)..=r as i32 { for dx in -(r as i32)..=r as i32 {
            if dx * dx + dy * dy > r2 { continue; }
            let (x, y) = (cx as i32 + dx, cy as i32 + dy); if !self.g.in_bounds(x, y) { continue; }
            let i = y as usize * self.g.w + x as usize;
            if code == 0 { self.desig[i] = 0; }
            else if code == D_DIG { if self.g.is_solidish(i) && self.g.m(i) != Mat::Bedrock { self.desig[i] = D_DIG; } }
            else if self.g.mat[i] == 0 || self.g.m(i).kind() != Kind::Solid { self.desig[i] = code; }
        } }
    }
    pub fn place_torch(&mut self, x: usize, y: usize) -> bool {
        if self.inv[Item::Torch as usize] == 0 || self.g.is_solidish(self.g.idx(x, y)) { return false; }
        self.inv[Item::Torch as usize] -= 1; self.torches.push((x, y)); self.events.push(Event::Place); true
    }
    pub fn place_building(&mut self, kind: Station, x: usize, y: usize) -> Result<(), &'static str> {
        if self.buildings.iter().any(|b| b.kind == kind) { return Err("already have one"); }
        for &(it, n) in kind.cost() { if self.inv[it as usize] < n { return Err("missing materials"); } }
        for yy in y + 1 - Station::H..=y { for xx in x..x + Station::W { if xx >= self.g.w || self.g.is_solidish(self.g.idx(xx, yy)) { return Err("needs open space"); } } }
        if !(x..x + Station::W).all(|xx| self.g.is_solidish(self.g.idx(xx, y + 1))) { return Err("needs a flat floor"); }
        self.buildings.push(Building { kind, x, y, built: false, claimed: false }); Ok(())
    }
    pub fn queue_craft(&mut self, r: usize) { if self.craft_queue.len() < 12 { self.craft_queue.push(r); } }
    pub fn explode(&mut self, cx: usize, cy: usize, r: f32, power: u8) {
        let ri = r.ceil() as i32;
        for dy in -ri..=ri { for dx in -ri..=ri {
            let d = ((dx * dx + dy * dy) as f32).sqrt(); if d > r { continue; }
            let (x, y) = (cx as i32 + dx, cy as i32 + dy); if !self.g.in_bounds(x, y) { continue; }
            let i = y as usize * self.g.w + x as usize; let m = self.g.m(i);
            if m == Mat::Bedrock || m.p().blast > power { continue; }
            if d < r * 0.6 { self.g.set(i, if hash3(x as u32, y as u32, self.tick as u32) % 9 == 0 { Mat::Fire } else { Mat::Empty }); }
            else if m.kind() == Kind::Solid { let c = m.p().crumbles_to; self.g.set(i, c); self.g.set_loose(i); }
        } }
        for m in self.moles.iter_mut() { let d = ((m.x as f32 - cx as f32).powi(2) + (m.y as f32 - cy as f32).powi(2)).sqrt(); if d < r * 1.5 { m.hp -= 80. * (1. - d / (r * 1.5)); } }
        self.logic.on_explosion(&mut self.g, cx, cy, r);
        self.events.push(Event::Explode); self.shake = self.shake.max(r / 3.);
    }

    pub fn step(&mut self) {
        self.tick += 1; self.events.clear();
        self.g.step_ca();
        if self.g.reactions.swap(0, Relaxed) != 0 { self.react_pending = true; }
        if (self.tick % 6 == 0 && self.g.struct_dirty.is_some()) || (self.tick % 24 == 0 && self.react_pending) {
            self.react_pending = false;
            self.g.struct_dirty = None; let f = self.st.solve(&self.g);
            let before = self.st.cracking.len(); self.st.crack(&f); if self.st.cracking.len() > before { self.events.push(Event::Crack); }
        }
        let crumble = self.st.tick(&self.g);
        if !crumble.is_empty() { self.events.push(Event::Crumble); }
        for &i in &crumble { let i = i as usize; let c = self.g.m(i).p().crumbles_to; self.g.set(i, c); if c.kind() == Kind::Solid { self.g.set_loose(i); } self.crumbled += 1; }
        if self.tick % 2 == 0 { self.step_moles(); }
        // wiring / logic / Thrumming
        let mp: Vec<(usize, usize)> = self.moles.iter().filter(|m| m.alive).map(|m| (m.x, m.y)).collect();
        self.logic.step(&mut self.g, &mp, self.tick);
        for s in std::mem::take(&mut self.logic.msgs) { self.say(s); }
        let booms = std::mem::take(&mut self.logic.booms);
        for b in booms { if self.logic.dev[b] == Dev::Tnt as u8 { self.logic.dev[b] = 0; let (x, y) = self.logic.centre(b); self.explode(x, y, 9., 6); } }
        // disasters
        let mut dz = std::mem::replace(&mut self.disasters, Disasters::new(0));
        dz.step(self); self.disasters = dz;
        if self.tick % air::UPDATE_TICKS == 0 { self.step_air(); }
        if self.tick % 30 == 0 { self.step_farms(); }
        if self.tick % 120 == 0 { self.follow_colony(); }
        self.shake *= 0.93;
        if self.tick % 30 == 0 { self.check_quests(); self.check_chests(); }
        self.day = 0.; // no sky in the Deepholt
    }
    pub fn vents(&self) -> Vec<(usize, usize)> { self.logic.dev_list.iter().filter(|&&b| self.logic.dev[b] == Dev::Vent as u8).map(|&b| self.logic.centre(b)).collect() }
    fn step_air(&mut self) {
        for m in &self.moles { if m.alive { self.air.add(m.x, m.y - 1, if m.job == Some(Job::Sleep) { air::MOLE_CO2 * 0.5 } else { air::MOLE_CO2 }); } }
        for &(x, y) in &self.torches { self.air.add(x, y, air::TORCH_CO2); }
        for f in &self.farms { self.air.add(f.x, f.y, air::FARM_CO2); }
        let v = self.vents(); self.air.update(&self.g, &v);
    }
    /// Plant a spore on the open cell (x, y) standing on soil.
    pub fn plant(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if self.inv[Item::Spore as usize] == 0 { return Err("no spores (dig fungus to find some)"); }
        if x < 1 || y + 1 >= self.g.h || x + 1 >= self.g.w { return Err("out of bounds"); }
        // snap to the floor: climb out of rock or drop through air (up to 6 cells)
        let mut y = y; for _ in 0..6 { if y > 1 && self.g.mat[self.g.idx(x, y)] != 0 { y -= 1; } else if y + 2 < self.g.h && self.g.mat[self.g.idx(x, y + 1)] == 0 { y += 1; } else { break; } }
        if self.g.mat[self.g.idx(x, y)] != 0 { return Err("needs an open cell"); }
        if !matches!(self.g.get(x, y + 1), Mat::Dirt | Mat::Clay | Mat::Roots | Mat::Gravel | Mat::Fungus | Mat::Grass) || self.g.loose(self.g.idx(x, y + 1)) { return Err("needs a soil floor (dirt, clay, roots)"); }
        if self.farms.iter().any(|f| f.x.abs_diff(x) < 3 && f.y.abs_diff(y) < 3) { return Err("too close to another mushroom"); }
        self.inv[Item::Spore as usize] -= 1; self.farms.push(Farm { x, y, growth: 0., why: 0 }); self.events.push(Event::Place); Ok(())
    }
    /// Growth check for one plot: (bright, dry).
    pub fn farm_conditions(&self, x: usize, y: usize) -> (bool, bool) {
        let near = |px: usize, py: usize, r: usize| px.abs_diff(x) <= r && py.abs_diff(y) <= r;
        let bright = self.torches.iter().any(|&(tx, ty)| near(tx, ty, 10)) || self.logic.dev_list.iter().any(|&b| self.logic.dev[b] == Dev::Lamp as u8 && self.logic.dev_s[b] != 0 && { let (cx, cy) = self.logic.centre(b); near(cx, cy, 10) })
            || self.buildings.iter().any(|b| b.built && matches!(b.kind, Station::Kiln | Station::Furnace) && near(b.x + Station::W / 2, b.y, 12));
        let mut wet = 0;
        for dy in -4i32..=4 { for dx in -4i32..=4 { let (nx, ny) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(nx, ny) { continue; } let i = self.g.idx(nx as usize, ny as usize);
            if self.g.mat[i] == Mat::Water as u8 { wet += 3; } else if self.g.flags[i] & F_WET != 0 || self.g.mat[i] == Mat::Clay as u8 { wet += 1; } } }
        (bright, wet < 4)
    }
    fn step_farms(&mut self) {
        let mut k = 0;
        while k < self.farms.len() {
            let (x, y) = (self.farms[k].x, self.farms[k].y);
            let soil = self.g.get(x, y + 1);
            if self.g.mat[self.g.idx(x, y)] != 0 || !matches!(soil, Mat::Dirt | Mat::Clay | Mat::Roots | Mat::Gravel | Mat::Fungus | Mat::Grass) { self.farms.swap_remove(k); continue; }
            let (bright, dry) = self.farm_conditions(x, y);
            self.farms[k].why = bright as u8 | (dry as u8) << 1;
            // dark + moist: ~60 s to fruit; one condition missing: 4× slower; neither: stalls
            let rate = match (bright, dry) { (false, false) => 1. / 120., (true, true) => 0., _ => 1. / 480. };
            self.farms[k].growth += rate;
            if self.farms[k].growth >= 1. {
                self.farms[k].growth = 0.;
                self.drop_item(x, y, Item::Mushroom, 2);
                if hash3(x as u32, y as u32, self.tick as u32) % 3 == 0 { self.drop_item(x, y, Item::Spore, 1); }
                self.harvested += 2; self.events.push(Event::Harvest);
            }
            k += 1;
        }
    }
    pub fn compute_light(&mut self, rect: (usize, usize, usize, usize)) {
        let mut pts: Vec<(usize, usize, [f32; 3])> = self.torches.iter().map(|&(x, y)| (x, y, [1.6, 1.15, 0.6])).collect();
        for m in &self.moles { if m.alive { pts.push((m.x, m.y - 1, [0.35, 0.30, 0.25])); } }
        for b in &self.buildings { if b.built && matches!(b.kind, Station::Kiln | Station::Furnace) { pts.push((b.x + Station::W / 2, b.y - 1, [1.4, 0.6, 0.2])); } }
        for &b in &self.logic.dev_list { if self.logic.dev[b] == Dev::Lamp as u8 && self.logic.dev_s[b] != 0 { let (x, y) = self.logic.centre(b); pts.push((x, y, [1.3, 1.25, 0.9])); } }
        let day = self.day; self.light_rect = rect;
        self.light.compute(&self.g, rect, day, &pts);
    }

    fn step_moles(&mut self) {
        let n = self.moles.len();
        for k in 0..n {
            if !self.moles[k].alive { continue; }
            self.moles[k].anim += 1;
            self.mole_physics(k);
            if !self.moles[k].alive { continue; }
            if (self.tick / 2 + k as u64) % 4 == 0 { let (x, y) = (self.moles[k].x, self.moles[k].y - 1); let before = self.g.flags.clone(); light::reveal(&mut self.g, x, y, 28); self.note_discoveries(&before, x, y); }
            self.mole_ai(k);
            let d = self.depth_m(self.moles[k].x, self.moles[k].y); if d > self.max_depth { self.max_depth = d; }
        }
    }
    fn note_discoveries(&mut self, before: &[u8], x: usize, y: usize) {
        let w = self.g.w; let (x0, y0, x1, y1) = (x.saturating_sub(29), y.saturating_sub(29), (x + 29).min(w - 1), (y + 29).min(self.g.h - 1));
        for yy in y0..=y1 { for xx in x0..=x1 {
            let i = yy * w + xx; if before[i] & F_SEEN == 0 && self.g.flags[i] & F_SEEN != 0 {
                let m = self.g.m(i);
                if !self.discovered[m as usize] && matches!(m, Mat::Coal | Mat::CopperOre | Mat::TinOre | Mat::IronOre | Mat::Quartz | Mat::Crystal | Mat::Lava | Mat::Water | Mat::Fungus | Mat::Limestone | Mat::Granite | Mat::Basalt | Mat::Clay) {
                    self.discovered[m as usize] = true; let d = self.depth_m(xx, yy);
                    self.inv[Item::Torch as usize] += 2; self.events.push(Event::Discover);
                    self.say(format!("Discovery: {} at {:.0} m (+2 torches)", m.p().name, d.max(0.)));
                }
            }
        } }
    }
    fn mole_physics(&mut self, k: usize) {
        let (x, y) = (self.moles[k].x, self.moles[k].y);
        let head = self.g.idx(x, y - 1); let hm = self.g.m(head);
        let in_lava = (0..MOLE_H).any(|j| matches!(self.g.get(x, y - j), Mat::Lava | Mat::Fire));
        let m = &mut self.moles[k];
        if in_lava { m.hp -= 4.; }
        // needs (rates per second at 60 tps; step_moles runs every 2nd tick → dt = 1/30 s)
        let dt = 1. / 30.; let busy = matches!(m.job, Some(Job::Dig) | Some(Job::Build) | Some(Job::Craft(_)) | Some(Job::Station(_)) | Some(Job::Haul(_)) | Some(Job::Deliver));
        let mult = if busy { 1.5 } else { 1. };
        m.hunger = (m.hunger - 0.33 * mult * dt).max(0.);
        if m.job == Some(Job::Sleep) { m.sleep = (m.sleep + 2.5 * dt).min(100.); } else { m.sleep = (m.sleep - 0.24 * mult * dt).max(0.); }
        if m.hunger <= 0. { m.hp -= 1. * dt; }
        if m.hunger > 60. && m.sleep > 50. && m.breath > 80. { m.hp = (m.hp + 0.5 * dt).min(100.); }
        let co2 = self.air.at(x, y - 1);
        if hm == Mat::Water { m.breath -= 0.6; if m.breath <= 0. { m.hp -= 1.; } }
        else if co2 > 0.45 { m.breath = (m.breath - (co2 - 0.45) * 1.2).max(0.); if m.breath <= 0. { m.hp -= 0.4; } }
        else { m.breath = (m.breath + 2.).min(100.); }
        let buried = (0..MOLE_H).any(|j| self.g.is_solidish(self.g.idx(x, y - j)));
        if buried {
            self.moles[k].hp -= 1.5; self.events.push(Event::Hurt);
            // try to wriggle up or sideways
            for (dx, dy) in [(0i32, -1i32), (-1, 0), (1, 0), (0, -2), (-2, 0), (2, 0), (0, -3)] { let (nx, ny) = ((x as i32 + dx) as usize, (y as i32 + dy) as usize); if self.standable(nx, ny) || (0..MOLE_H).all(|j| self.passable(nx, ny - j)) { self.moles[k].x = nx; self.moles[k].y = ny; break; } }
            self.moles[k].path.clear();
        } else if !self.standable(x, y) && y + 2 < self.g.h && !self.g.is_solidish(self.g.idx(x, y + 1)) {
            self.moles[k].y += 1; self.moles[k].fall += 1; self.moles[k].path.clear();
        } else if self.moles[k].fall > 0 { let f = self.moles[k].fall; if f > 14 { self.moles[k].hp -= (f - 14) as f32 * 3.; self.events.push(Event::Hurt); } self.moles[k].fall = 0; }
        if self.moles[k].hp <= 0. { self.moles[k].alive = false; let nm = self.moles[k].name; self.events.push(Event::Death); self.say(format!("{} has died.", nm)); }
    }
    fn mole_ai(&mut self, k: usize) {
        if self.moles[k].fall > 0 { return; }
        // follow path
        if !self.moles[k].path.is_empty() {
            if self.moles[k].anim % (MOVE_TICKS / 2).max(1) != 0 { return; }
            let (nx, ny) = *self.moles[k].path.front().unwrap();
            if !self.standable(nx, ny) { self.moles[k].path.clear(); self.moles[k].job = None; return; }
            self.moles[k].path.pop_front(); let m = &mut self.moles[k]; if nx != m.x { m.facing = if nx > m.x { 1 } else { -1 }; } m.x = nx; m.y = ny; return;
        }
        let (x, y) = (self.moles[k].x, self.moles[k].y);
        // needs override work: collapse asleep at 0, drop the job when starving
        { let m = &self.moles[k];
          if m.sleep <= 0. && m.job != Some(Job::Sleep) { self.release_job(k); self.moles[k].job = Some(Job::Sleep); let nm = self.moles[k].name; self.say(format!("{} collapsed asleep.", nm)); return; }
          if m.hunger < 15. && m.job.is_some() && !matches!(m.job, Some(Job::Eat) | Some(Job::Sleep)) && self.inv[Item::Grub as usize] + self.inv[Item::Fungus as usize] + self.inv[Item::Mushroom as usize] > 0 { self.release_job(k); } }
        let weak = self.moles[k].hunger < 20. || self.moles[k].sleep < 20.;
        if weak && self.tick % 4 == 0 && matches!(self.moles[k].job, Some(Job::Dig) | Some(Job::Build) | Some(Job::Craft(_)) | Some(Job::Station(_))) { return; } // half speed
        match self.moles[k].job.clone() {
            Some(Job::Dig) => {
                if let Some(i) = self.moles[k].work_cell.filter(|&i| self.desig[i] == D_DIG && self.diggable(i)).or_else(|| self.dig_target_near(x, y)) {
                    let m = self.g.m(i); let tier = tool_tier(&self.inv);
                    let need = (m.p().dig as u32 * 4 / (1 + tier as u32)).max(1);
                    self.moles[k].work_cell = Some(i); self.moles[k].work += 1; self.moles[k].facing = if i % self.g.w >= x { 1 } else { -1 };
                    if self.moles[k].work >= need { self.moles[k].work = 0; self.moles[k].work_cell = None; self.dig_cell(i, x, y); }
                } else { self.moles[k].job = None; self.moles[k].work_cell = None; }
            }
            Some(Job::Build) => {
                if let Some(i) = self.build_target_near(x, y) {
                    self.moles[k].work += 1;
                    if self.moles[k].work >= 6 { self.moles[k].work = 0; let d = self.desig[i]; self.place_cell(i, d); }
                } else { self.moles[k].job = None; }
            }
            Some(Job::Haul(di)) => {
                if di < self.drops.len() && self.drops[di].x.abs_diff(x) <= 2 && self.drops[di].y.abs_diff(y) <= 2 {
                    let d = self.drops.remove(di); self.moles[k].carry = Some((d.item, d.n)); self.fix_haul_indices(di);
                    let sp = self.stockpile;
                    match self.bfs((x, y), 60000, |xx, yy| xx >= sp.0 && xx <= sp.2 && yy >= sp.1 && yy <= sp.3) { Some(p) => { self.moles[k].path = p; self.moles[k].job = Some(Job::Deliver); } None => { self.drop_item(x, y, d.item, d.n); self.moles[k].carry = None; self.moles[k].job = None; } }
                } else { self.moles[k].job = None; }
            }
            Some(Job::Deliver) => {
                if let Some((it, n)) = self.moles[k].carry.take() { self.inv[it as usize] += n; }
                self.moles[k].job = None;
            }
            Some(Job::Station(bi)) => {
                let b = &self.buildings[bi]; if b.built { self.moles[k].job = None; return; }
                let kind = b.kind; let ok = kind.cost().iter().all(|&(it, n)| self.inv[it as usize] >= n);
                if !ok { self.buildings[bi].claimed = false; self.moles[k].job = None; return; }
                self.moles[k].work += 1;
                if self.moles[k].work >= 120 { self.moles[k].work = 0; for &(it, n) in kind.cost() { self.inv[it as usize] -= n; } self.buildings[bi].built = true; self.buildings[bi].claimed = false; self.moles[k].job = None; self.events.push(Event::Place); self.say(format!("Built a {}.", kind.name())); }
            }
            Some(Job::Craft(r)) => {
                let rec = &RECIPES[r];
                if !rec.inputs.iter().all(|&(it, n)| self.inv[it as usize] >= n) { self.moles[k].job = None; return; }
                self.moles[k].work += 2;
                if self.moles[k].work >= rec.ticks { self.moles[k].work = 0; for &(it, n) in rec.inputs { self.inv[it as usize] -= n; } self.inv[rec.out.0 as usize] += rec.out.1; self.moles[k].job = None; self.events.push(Event::Craft); self.say(format!("Crafted {}.", rec.name)); }
            }
            Some(Job::Eat) => {
                let food = if self.inv[Item::Grub as usize] > 0 { Some((Item::Grub, 45.)) } else if self.inv[Item::Mushroom as usize] > 0 { Some((Item::Mushroom, 35.)) } else if self.inv[Item::Fungus as usize] > 0 { Some((Item::Fungus, 25.)) } else { None };
                if let Some((it, v)) = food { self.inv[it as usize] -= 1; let m = &mut self.moles[k]; m.hunger = (m.hunger + v).min(100.); self.events.push(Event::Eat); }
                self.moles[k].job = if self.moles[k].hunger < 60. && food.is_some() { Some(Job::Eat) } else { None };
            }
            Some(Job::Sleep) => { if self.moles[k].sleep >= 95. { self.moles[k].job = None; } }
            Some(Job::Flee) | None => {
                self.moles[k].job = None;
                if self.air.at(x, y - 1) > 0.55 && self.moles[k].idle % 4 == 0 {
                    let a = &self.air;
                    if let Some(p) = self.bfs((x, y), 40000, |xx, yy| a.at(xx, yy - 1) < 0.3) { self.moles[k].path = p; self.moles[k].job = Some(Job::Flee); self.events.push(Event::Gasp); return; }
                }
                let (hu, sl) = (self.moles[k].hunger, self.moles[k].sleep);
                let has_food = self.inv[Item::Grub as usize] + self.inv[Item::Fungus as usize] + self.inv[Item::Mushroom as usize] > 0;
                if hu < 40. && has_food {
                    let sp = self.stockpile;
                    if let Some(p) = self.bfs((x, y), 60000, |xx, yy| xx + 1 >= sp.0 && xx <= sp.2 + 1 && yy + 2 >= sp.1 && yy <= sp.3 + 1) { self.moles[k].path = p; self.moles[k].job = Some(Job::Eat); return; }
                }
                if sl < 30. { self.moles[k].job = Some(Job::Sleep); return; }
                self.find_job(k);
            }
        }
    }
    fn release_job(&mut self, k: usize) {
        if let Some(Job::Station(bi)) = self.moles[k].job { if bi < self.buildings.len() { self.buildings[bi].claimed = false; } }
        if let Some(Job::Craft(r)) = self.moles[k].job { self.craft_queue.insert(0, r); }
        if let Some((it, n)) = self.moles[k].carry.take() { let (x, y) = (self.moles[k].x, self.moles[k].y); self.drop_item(x, y, it, n); }
        self.moles[k].job = None; self.moles[k].path.clear(); self.moles[k].work = 0;
    }
    fn fix_haul_indices(&mut self, removed: usize) {
        for m in self.moles.iter_mut() { if let Some(Job::Haul(d)) = m.job { if d > removed { m.job = Some(Job::Haul(d - 1)); } else if d == removed { m.job = None; m.path.clear(); } } }
    }
    fn find_job(&mut self, k: usize) {
        self.moles[k].idle += 1;
        if self.moles[k].idle % 8 != (k as u32 % 8) { return; } // stagger searches
        let (x, y) = (self.moles[k].x, self.moles[k].y);
        // 1. danger: cracking cells within 6 → step away (flee job)
        // 2. stations
        if let Some(bi) = self.buildings.iter().position(|b| !b.built && !b.claimed && b.kind.cost().iter().all(|&(it, n)| self.inv[it as usize] >= n)) {
            let (bx, by) = (self.buildings[bi].x, self.buildings[bi].y);
            if let Some(p) = self.bfs((x, y), 40000, |xx, yy| xx + 2 >= bx && xx <= bx + Station::W + 1 && yy.abs_diff(by) <= 2) { self.buildings[bi].claimed = true; self.moles[k].path = p; self.moles[k].job = Some(Job::Station(bi)); self.moles[k].idle = 0; return; }
        }
        // 3. crafting
        if let Some(&r) = self.craft_queue.first() {
            let rec = &RECIPES[r];
            if let Some(b) = self.buildings.iter().find(|b| b.built && b.kind == rec.station) {
                if rec.inputs.iter().all(|&(it, n)| self.inv[it as usize] >= n) && !self.moles.iter().any(|m| matches!(m.job, Some(Job::Craft(_)))) {
                    let (bx, by) = (b.x, b.y);
                    if let Some(p) = self.bfs((x, y), 40000, |xx, yy| xx + 1 >= bx && xx <= bx + Station::W && yy.abs_diff(by) <= 2) { self.craft_queue.remove(0); self.moles[k].path = p; self.moles[k].job = Some(Job::Craft(r)); self.moles[k].idle = 0; return; }
                }
            }
        }
        // 4. build designations
        if self.desig.iter().any(|&d| d >= D_WOOD) {
            if let Some(p) = self.bfs((x, y), 30000, |xx, yy| self.build_target_near(xx, yy).is_some()) { self.moles[k].path = p; self.moles[k].job = Some(Job::Build); self.moles[k].idle = 0; return; }
        }
        // 5. dig
        if let Some(p) = self.bfs((x, y), 30000, |xx, yy| self.dig_target_near(xx, yy).is_some()) { self.moles[k].path = p; self.moles[k].job = Some(Job::Dig); self.moles[k].idle = 0; return; }
        // 6. haul
        let sp = self.stockpile;
        let claimed: Vec<usize> = self.moles.iter().filter_map(|m| if let Some(Job::Haul(d)) = m.job { Some(d) } else { None }).collect();
        let targets: Vec<(usize, usize, usize)> = self.drops.iter().enumerate().filter(|(i, d)| !claimed.contains(i) && !(d.x >= sp.0 && d.x <= sp.2 && d.y >= sp.1 && d.y <= sp.3)).map(|(i, d)| (i, d.x, d.y)).collect();
        if !targets.is_empty() {
            if let Some(p) = self.bfs((x, y), 30000, |xx, yy| targets.iter().any(|&(_, dx, dy)| dx.abs_diff(xx) <= 2 && dy.abs_diff(yy) <= 2)) {
                let (ex, ey) = p.back().copied().unwrap_or((x, y));
                if let Some(&(i, _, _)) = targets.iter().find(|&&(_, dx, dy)| dx.abs_diff(ex) <= 2 && dy.abs_diff(ey) <= 2) { self.moles[k].path = p; self.moles[k].job = Some(Job::Haul(i)); self.moles[k].idle = 0; }
            }
        }
    }
    fn drop_item(&mut self, x: usize, y: usize, item: Item, n: u32) {
        if let Some(d) = self.drops.iter_mut().find(|d| d.item == item && d.x.abs_diff(x) <= 3 && d.y.abs_diff(y) <= 3 && d.n < 20) { d.n += n; return; }
        self.drops.push(Drop { x, y, item, n, claimed: false });
    }
    fn dig_cell(&mut self, i: usize, mx: usize, my: usize) {
        let m = self.g.m(i); let wet = self.g.flags[i] & F_WET != 0;
        self.g.set(i, if wet { Mat::Water } else { Mat::Empty }); self.desig[i] = 0; self.events.push(Event::Dig);
        if matches!(m, Mat::Dirt | Mat::Clay | Mat::Gravel | Mat::Roots | Mat::Grass) && hash3(i as u32, self.tick as u32, 0x6b) % 48 == 0 { self.drop_item(mx, my, Item::Grub, 1); }
        if m == Mat::Fungus && hash3(i as u32, self.tick as u32, 0x5b) % 2 == 0 { self.drop_item(mx, my, Item::Spore, 1); }
        if let Some(it) = drop_of(m) {
            self.dig_acc[m as usize] += 1;
            let per = if matches!(m, Mat::Crystal | Mat::Quartz) { 3 } else { CELLS_PER_ITEM };
            if self.dig_acc[m as usize] >= per { self.dig_acc[m as usize] = 0; self.drop_item(mx, my, it, 1); }
        }
        let (x, y) = (i % self.g.w, i / self.g.w); light::reveal(&mut self.g, x, y, 3);
    }
    fn place_cell(&mut self, i: usize, d: u8) {
        let it = build_item(d);
        if self.place_acc[d as usize] == 0 { if self.inv[it as usize] == 0 { return; } self.inv[it as usize] -= 1; self.place_acc[d as usize] = 4; }
        self.place_acc[d as usize] -= 1;
        self.g.set(i, build_mat(d)); self.desig[i] = 0; self.pillars += 1; self.events.push(Event::Place);
    }
    fn check_chests(&mut self) {
        for c in 0..self.chests.len() {
            if self.chests[c].opened { continue; }
            let (cx, cy) = (self.chests[c].x, self.chests[c].y);
            let near = self.moles.iter().any(|m| m.alive && m.x.abs_diff(cx) <= 4 && m.y.abs_diff(cy) <= 4);
            if !near || self.g.is_solidish(self.g.idx(cx, cy)) { continue; }
            self.chests[c].opened = true; self.chests_opened += 1; self.opened.insert((cx, self.oy + cy as i64));
            let h = hash3(cx as u32, cy as u32, self.info.seed);
            let d = self.depth_m(cx, cy);
            let loot: Vec<(Item, u32)> = if d > 50. { vec![(Item::Crystal, 2 + h % 3), (Item::IronIngot, 2), (Item::Tnt, 1 + h % 2)] }
                else if d > 25. { vec![(Item::CopperIngot, 2 + h % 3), (Item::TinIngot, 1), (Item::Torch, 6)] }
                else { vec![(Item::Coal, 4 + h % 4), (Item::Torch, 4), (Item::WoodPillar, 8)] };
            let mut s = String::new(); for &(it, n) in &loot { self.inv[it as usize] += n; s += &format!(" {}x{}", it.name(), n); }
            self.events.push(Event::Chest); self.say(format!("Treasure chest at {:.0} m:{}", d.max(0.), s));
        }
    }
    pub fn quest_done(&self, q: usize) -> bool {
        match QUESTS[q].goal {
            Goal::Depth(m) => self.max_depth >= m, Goal::Have(it, n) => self.inv[it as usize] >= n,
            Goal::Built(s) => self.buildings.iter().any(|b| b.built && b.kind == s), Goal::Chests(n) => self.chests_opened >= n, Goal::Pillars(n) => self.pillars >= n,
        }
    }
    fn check_quests(&mut self) {
        while self.quest < QUESTS.len() && self.quest_done(self.quest) {
            let q = &QUESTS[self.quest]; let mut s = String::new();
            for &(it, n) in q.reward { self.inv[it as usize] += n; s += &format!(" {}x{}", it.name(), n); }
            self.say(format!("Quest complete: {} - reward{}", q.text, s)); self.events.push(Event::Quest); self.quest += 1;
        }
    }
}
pub fn build_item(d: u8) -> Item { match d { D_BRICK => Item::Brick, D_IRON => Item::IronBeam, _ => Item::WoodPillar } }
pub fn build_mat(d: u8) -> Mat { match d { D_BRICK => Mat::StoneBrick, D_IRON => Mat::IronBeam, _ => Mat::WoodBeam } }
