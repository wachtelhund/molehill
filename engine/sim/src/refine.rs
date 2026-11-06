//! Refinement: ore is crushed under falling stamps, washed apart by weight, smelted in a fire blown by bellows, and
//! poured into moulds where it sets into ingots. Every lump goes in and comes out as a loose item; the power comes off
//! drive trains (power.rs), the heat from burning real fuel.
//!
//! The three works stand on a level floor on the tile grid, their bodies solid shell cells (like a boiler's), fed by
//! dropping lumps on their tops and emptied through a spout at the floor on one side (the other if rock is against it):
//! * **Stamp mill** (`Mat::StampMill`, machinery: it joins the train it touches). A cam lifts its iron stamps and drops
//!   them on what lies in its mortar. Each blow lifts `STAMP_KG` by `STAMP_LIFT` (m·g·h: the train pays for it as
//!   `STAMP_W` while it works), and `BLOWS` break a lump: an ore lump comes out as its **concentrate** and a lump of
//!   **sand** (the rock it was in); stone and slag crush to gravel. Anything else stays on its top.
//! * **Washer** (`Mat::Washer`, machinery). A tub of water a train rocks (`WASHER_W`): it needs water touching it, of
//!   which it uses a cell every `WASH_PER_CELL` lumps. What's dropped in comes out sorted by weight: as dense as
//!   `HEAVY_KG_M3` or more (ore, concentrate, slag) out of its left spout, lighter (sand, gravel, coal, ash) out of its
//!   right.
//! * **Smelter** (`Mat::Smelter`, not machinery: it's blown by bellows). Coal or charcoal dropped in burns a lump at a time
//!   (`steam::burn_ticks`). Its heat tends to 800 °C, and up to `HOT_C` with full air from **bellows** (`Mat::Bellows`,
//!   machinery; the whole block of them touching it counts): they take `BELLOWS_W` a cell from their train while it's lit, and blow in proportion to how
//!   fast it turns, `BELLOWS_CELLS` turning cells being full air. Hot enough for a metal (copper and tin `1100`, iron
//!   `1250` °C) a lump of its ore or concentrate melts in `SMELT_TICKS`: ore gives `ORE_YIELD` of an ingot and half a lump
//!   of slag, washed concentrate `FINES_YIELD` and a quarter. Slag comes out of its spout.
//! * **Foundry moulds** (`Mat::Mould`): a mould (a tile of mould cells) built against a smelter is filled when it holds
//!   an ingot of molten metal: the cells glow (`Mat::MouldHot`) for `COOL_TICKS`, then the mould is empty again and the
//!   ingot lies on top of it.
use crate::craft::Item;
use crate::items::ITEM_HW;
use crate::mat::{Mat, CPM, S};
use crate::world::{Drop, Event, World};

pub const STAMP_KG: f32 = 50.; pub const STAMP_LIFT: f32 = 0.3; pub const BLOWS: u32 = 6;
/// A stamp mill at work: two stamps each falling once a second at full speed (m·g·h·2/s ≈ 300 W).
pub const STAMP_W: f32 = 2. * STAMP_KG * 9.81 * STAMP_LIFT;
pub const WASHER_W: f32 = 80.; pub const WASH_TICKS: u32 = 120; pub const WASH_PER_CELL: u32 = 4; pub const HEAVY_KG_M3: f32 = 2500.;
pub const BELLOWS_W: f32 = 10.; pub const BELLOWS_CELLS: f32 = 8.;
pub const COLD_C: f32 = 800.; pub const HOT_C: f32 = 1350.; pub const SMELT_TICKS: u32 = 600;
pub const ORE_YIELD: f32 = 0.5; pub const FINES_YIELD: f32 = 0.75; pub const COOL_TICKS: u32 = 900;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkKind { Stamp, Washer, Smelter }
impl WorkKind {
    pub fn size(self) -> (usize, usize) { match self { WorkKind::Stamp => (CPM, 3 * CPM / 2), WorkKind::Washer => (3 * CPM / 2, 3 * CPM / 4), WorkKind::Smelter => (CPM, 3 * CPM / 2) } }
    pub fn mat(self) -> Mat { match self { WorkKind::Stamp => Mat::StampMill, WorkKind::Washer => Mat::Washer, WorkKind::Smelter => Mat::Smelter } }
    pub fn item(self) -> Item { match self { WorkKind::Stamp => Item::StampMill, WorkKind::Washer => Item::Washer, WorkKind::Smelter => Item::Smelter } }
    pub fn name(self) -> &'static str { match self { WorkKind::Stamp => "stamp mill", WorkKind::Washer => "washer", WorkKind::Smelter => "smelter" } }
}

/// The metal an ore or concentrate gives, the heat it needs (°C), and whether it's washed concentrate.
pub fn charge(it: Item) -> Option<(usize, f32, bool)> {
    Some(match it { Item::CopperOre => (0, 1100., false), Item::TinOre => (1, 1100., false), Item::IronOre => (2, 1250., false),
        Item::CopperFines => (0, 1100., true), Item::TinFines => (1, 1100., true), Item::IronFines => (2, 1250., true), _ => return None })
}
pub const METAL_INGOT: [Item; 3] = [Item::CopperIngot, Item::TinIngot, Item::IronIngot];
/// What a stamp mill makes of a lump, if it crushes it.
pub fn crushed(it: Item) -> Option<&'static [Item]> {
    Some(match it { Item::CopperOre => &[Item::CopperFines, Item::Sand], Item::TinOre => &[Item::TinFines, Item::Sand], Item::IronOre => &[Item::IronFines, Item::Sand],
        Item::Stone | Item::Slag => &[Item::Gravel], Item::Quartz => &[Item::Sand], _ => return None })
}

#[derive(Clone, Debug)]
pub struct Works {
    pub kind: WorkKind,
    /// its middle column (on the tile grid) and the floor row it stands on
    pub x: usize, pub y: usize,
    /// what's been fed in, waiting; ticks into the one at work (blows for a stamp mill)
    pub input: Vec<(Item, u32)>, pub work: u32,
    /// stamp mill: where its stamps are in their stroke (0..1, for drawing); washer: lumps since it last drew water
    pub stroke: f32, pub washed: u32, pub water: f32,
    /// smelter: fuel waiting, the lump burning and its ticks left, heat (°C), air (0..1), molten metal (ingots: copper,
    /// tin, iron), slag toward the next lump
    pub fuel: Vec<(Item, u32)>, pub burning: Option<Item>, pub burn: u32, pub temp: f32, pub air: f32, pub molten: [f32; 3], pub slag: f32,
}
impl Works {
    pub fn rect(&self) -> (f32, f32, f32, f32) { let (w, h) = self.kind.size(); (self.x as f32 - (w / 2) as f32, (self.y - h) as f32, self.x as f32 + (w / 2) as f32, self.y as f32) }
    pub fn contains(&self, x: f32, y: f32) -> bool { let (x0, y0, x1, y1) = self.rect(); x >= x0 && x < x1 && y >= y0 && y < y1 }
    pub fn load(&self) -> u32 { self.input.iter().map(|c| c.1).sum() }
}
fn push(v: &mut Vec<(Item, u32)>, it: Item, n: u32) { if let Some(c) = v.iter_mut().find(|c| c.0 == it) { c.1 += n; } else { v.push((it, n)); } }
fn pop(v: &mut Vec<(Item, u32)>) -> Option<Item> { let c = v.first_mut()?; let it = c.0; c.1 -= 1; if c.1 == 0 { v.remove(0); } Some(it) }

/// A cast setting in a mould: its cells, the ingot it makes, ticks left.
#[derive(Clone, Debug)]
pub struct Cast { pub cells: Vec<usize>, pub ingot: Item, pub left: u32 }

impl World {
    /// Stand a works on the floor under (x, y): on the tile grid, its body shell cells.
    pub(crate) fn place_works(&mut self, kind: WorkKind, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let (w, h) = kind.size(); let hw = w / 2;
        if x < hw + 2 || x + hw + 2 >= self.g.w { return Err("too near the edge"); }
        let x = (x - hw) / S * S + hw;
        let floor = (y..(y + 3 * CPM).min(self.g.h - 1)).find(|&r| (x - hw..x + hw).any(|c| !self.passable(c, r))).ok_or("needs a floor under it")?;
        if floor < h + 2 { return Err("too near the edge"); }
        if (x - hw..x + hw).any(|c| self.passable(c, floor)) { return Err("needs a level floor under all of it"); }
        if !(floor - h..floor).all(|r| (x - hw..x + hw).all(|c| self.g.get(c, r) == Mat::Empty)) { return Err("needs room for it"); }
        for r in floor - h..floor { for c in x - hw..x + hw { let i = self.g.idx(c, r); self.g.set(i, kind.mat()); if kind != WorkKind::Smelter { self.tr.power.cells.push(i); } } }
        self.tr.works.push(Works { kind, x, y: floor, input: vec![], work: 0, stroke: 0., washed: 0, water: 0., fuel: vec![], burning: None, burn: 0, temp: 20., air: 0., molten: [0.; 3], slag: 0. });
        self.events.push(Event::Place); Ok(())
    }
    pub fn works_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.works.iter().position(|w| w.contains(xf, yf)) }
    fn clear_works(&mut self, wk: &Works) {
        let (x0, y0, x1, y1) = wk.rect();
        for r in y0 as usize..y1 as usize { for c in x0 as usize..x1 as usize { let i = self.g.idx(c, r); if self.g.m(i) == wk.kind.mat() { self.g.set(i, Mat::Empty); } } }
    }
    /// `E` on a works: what's in it into the pack; an empty, cold one comes up itself.
    pub(crate) fn pick_up_works(&mut self, x: usize, y: usize) -> Option<Item> {
        let k = self.works_at(x, y)?;
        let wk = &mut self.tr.works[k]; let mut got: Vec<(Item, u32)> = std::mem::take(&mut wk.input); got.extend(std::mem::take(&mut wk.fuel));
        if !got.is_empty() { let mut s = vec![]; for (it, n) in got { self.inv[it as usize] += n; s.push(format!("{n} {}", it.name().to_lowercase())); }
            let kind = self.tr.works[k].kind; self.say(format!("Emptied the {}: {}.", kind.name(), s.join(", "))); return Some(kind.item()); }
        if self.tr.works[k].burning.is_some() || self.tr.works[k].temp > 200. { self.say("The smelter is still hot.".into()); return None; }
        let wk = self.tr.works.remove(k); self.clear_works(&wk); self.inv[wk.kind.item() as usize] += 1; Some(wk.kind.item())
    }

    /// The spout at its floor: right, or left if rock stands against the right (or the side asked for). None: walled in.
    fn spout(&self, wk: &Works, prefer: f32) -> Option<(f32, f32)> {
        let (x0, _, x1, y1) = wk.rect(); let ay = y1 - ITEM_HW - 0.05;
        let side = |s: f32| (s, if s > 0. { x1 + ITEM_HW + 0.2 } else { x0 - ITEM_HW - 0.2 });
        [side(prefer), side(-prefer)].into_iter().find(|&(_, ax)| !self.item_box_hits_pub(ax, ay, Item::Sand))
    }
    fn spill(&mut self, wk: &Works, it: Item, prefer: f32) -> bool {
        // (a washer's two spouts are its sort: each lump only ever leaves by its own side)
        let Some((s, ax)) = self.spout(wk, prefer).filter(|&(s, _)| wk.kind != WorkKind::Washer || s == prefer) else { return false };
        let (_, _, _, y1) = wk.rect(); let mut d = Drop::at(ax, y1 - ITEM_HW - 0.05, it, 1); d.vx = s * 0.8 * CPM as f32; self.drops.push(d); true
    }
    /// Say (once every 10 s) that a works can't empty: rock against its spout.
    fn walled(&mut self, wk: &Works) {
        let s = format!("The {}'s spout is walled in: it needs room at its foot to let out what it's done.", wk.kind.name());
        if !self.log.iter().rev().take(20).any(|(t, l)| *l == s && self.tick < t + 600) { self.say(s); }
    }
    /// How fast the train turning a works' shell turns (0..1): its fastest cell.
    fn works_turning(&self, wk: &Works) -> f32 {
        let (x0, y0, x1, y1) = wk.rect(); let gw = self.g.w;
        (y0 as usize..y1 as usize).flat_map(|r| (x0 as usize..x1 as usize).map(move |c| r * gw + c)).map(|i| self.turning(i)).fold(0., f32::max)
    }
    /// The ring of cells round a works.
    fn ring(&self, wk: &Works) -> Vec<usize> {
        let (x0, y0, x1, y1) = wk.rect(); let (gw, gh) = (self.g.w, self.g.h); let (x0, y0, x1, y1) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);
        (x0 - 1..=x1).flat_map(|c| [(c, y0 - 1), (c, y1)]).chain((y0..y1).flat_map(|r| [(x0 - 1, r), (x1, r)])).filter(|&(c, r)| c < gw && r < gh).map(|(c, r)| r * gw + c).collect()
    }

    /// What the works want from their trains this solve (W), by train: stamps and washers at work, bellows of a lit smelter.
    pub(crate) fn works_demand(&self, train: &std::collections::HashMap<usize, u32>, demand: &mut [f32]) {
        for wk in &self.tr.works {
            let (x0, y0, _, _) = wk.rect(); let gw = self.g.w; let first = y0 as usize * gw + x0 as usize;
            match wk.kind {
                WorkKind::Stamp => if wk.load() > 0 { if let Some(&t) = train.get(&first) { demand[t as usize] += STAMP_W; } },
                WorkKind::Washer => if wk.load() > 0 { if let Some(&t) = train.get(&first) { demand[t as usize] += WASHER_W; } },
                WorkKind::Smelter => if wk.burning.is_some() {
                    let mut bl: Vec<usize> = self.ring(wk).into_iter().filter(|&i| self.g.m(i) == Mat::Bellows).collect(); let mut q = 0;
                    while q < bl.len() && bl.len() < 256 { let i = bl[q]; q += 1; for n in self.g.n4(i % self.g.w, i / self.g.w).collect::<Vec<_>>() { if self.g.m(n) == Mat::Bellows && !bl.contains(&n) { bl.push(n); } } }
                    for i in bl { if let Some(&t) = train.get(&i) { demand[t as usize] += BELLOWS_W; } } },
            }
        }
    }

    /// One solve of every works (`dt` s), using how fast their trains turned at the last solve.
    pub(crate) fn step_works(&mut self, dt: f32) {
        let ticks = (dt * 60.).round() as u32;
        // moulds cool
        let mut k = 0;
        while k < self.tr.casts.len() { let c = &mut self.tr.casts[k]; c.left = c.left.saturating_sub(ticks);
            if c.left == 0 { let c = self.tr.casts.remove(k); let top = *c.cells.iter().min_by_key(|&&i| i / self.g.w).unwrap();
                for &i in &c.cells { if self.g.m(i) == Mat::MouldHot { self.g.set(i, Mat::Mould); } }
                let (x, y) = ((top % self.g.w) as f32 + 0.5, (top / self.g.w) as f32 - ITEM_HW - 0.1);
                self.drops.push(Drop::at(x, y, c.ingot, 1)); self.events.push(Event::Click); continue; }
            k += 1; }
        let mut k = 0;
        while k < self.tr.works.len() {
            let mut wk = self.tr.works[k].clone(); let (x0, y0, x1, y1) = wk.rect(); let (w, h) = wk.kind.size();
            // a broken shell wrecks it
            let shell = (y0 as usize..y1 as usize).map(|r| (x0 as usize..x1 as usize).filter(|&c| self.g.get(c, r) == wk.kind.mat()).count()).sum::<usize>();
            if shell * 4 < w * h * 3 {
                for (it, n) in wk.input.drain(..).chain(wk.fuel.drain(..)) { for _ in 0..n { self.drops.push(Drop::at(wk.x as f32, y0 - 2., it, 1)); } }
                self.clear_works(&wk); self.tr.works.remove(k); self.say(format!("A {} was wrecked.", wk.kind.name())); continue;
            }
            // what's dropped on its top goes in, if it takes it
            let takes = |it: Item| match wk.kind { WorkKind::Stamp => crushed(it).is_some(), WorkKind::Washer => it.size_m() <= 0.12, WorkKind::Smelter => charge(it).is_some() || matches!(it, Item::Coal | Item::Charcoal) };
            let mut j = 0;
            while j < self.drops.len() { let d = &self.drops[j];
                if d.fuse == 0 && takes(d.item) && d.px >= x0 && d.px < x1 && d.py >= y0 - 3. * ITEM_HW && d.py < y0 && d.hold == 0 {
                    let d = self.drops.swap_remove(j); if wk.kind == WorkKind::Smelter && charge(d.item).is_none() { push(&mut wk.fuel, d.item, d.n); } else { push(&mut wk.input, d.item, d.n); } } else { j += 1; } }
            match wk.kind {
                WorkKind::Stamp => {
                    let sp = self.works_turning(&wk);
                    if wk.load() > 0 && sp > 0.05 {
                        // a blow a second at full speed, each stamp lifted and let fall
                        let before = wk.stroke; wk.stroke += sp * dt; if wk.stroke >= 1. { wk.stroke -= 1.; wk.work += 1; if before < 1. { self.events.push(Event::Click); self.shake = self.shake.max(0.03); } }
                        // a lump broken goes out of its spout; with no room at either side it waits, unbroken
                        if wk.work >= BLOWS { wk.work = BLOWS;
                            if self.spout(&wk, 1.).is_some() { wk.work = 0; if let Some(it) = pop(&mut wk.input) { for &o in crushed(it).unwrap_or(&[]) { self.spill(&wk, o, 1.); } } }
                            else { self.walled(&wk); } }
                    }
                }
                WorkKind::Washer => {
                    let sp = self.works_turning(&wk);
                    // water from what touches it
                    if wk.water < 1. { let ring = self.ring(&wk); if let Some(&i) = ring.iter().find(|&&i| self.g.m(i) == Mat::Water) { self.g.set(i, Mat::Empty); wk.water += 1.; } }
                    if wk.load() > 0 && sp > 0.05 && wk.water > 0. {
                        wk.work += (sp * ticks as f32).round() as u32;
                        if wk.work >= WASH_TICKS { wk.work = WASH_TICKS;
                            // out of its own side only: with that spout walled in it waits
                            let it = wk.input[0].0; let side = if it.density() >= HEAVY_KG_M3 { -1. } else { 1. };
                            if self.spill(&wk, it, side) { wk.work = 0; pop(&mut wk.input); wk.washed += 1; if wk.washed >= WASH_PER_CELL { wk.washed = 0; wk.water -= 1.; } }
                            else { self.walled(&wk); } }
                    }
                }
                WorkKind::Smelter => {
                    // the fire, a lump at a time
                    if wk.burning.is_none() { if let Some(it) = pop(&mut wk.fuel) { wk.burning = Some(it); wk.burn = crate::steam::burn_ticks(it).unwrap_or(0); } }
                    if wk.burning.is_some() { wk.burn = wk.burn.saturating_sub(ticks); if wk.burn == 0 { wk.burning = None; } }
                    // air from the bellows touching it
                    let ring = self.ring(&wk);
                    // (the whole bellows: every bellows cell joined to one touching it)
                    let mut bl: Vec<usize> = ring.iter().copied().filter(|&i| self.g.m(i) == Mat::Bellows).collect(); let mut q = 0;
                    while q < bl.len() && bl.len() < 256 { let i = bl[q]; q += 1; for n in self.g.n4(i % self.g.w, i / self.g.w).collect::<Vec<_>>() { if self.g.m(n) == Mat::Bellows && !bl.contains(&n) { bl.push(n); } } }
                    wk.air = (bl.iter().map(|&i| self.turning(i)).sum::<f32>() / BELLOWS_CELLS).min(1.);
                    let target = if wk.burning.is_some() { COLD_C + (HOT_C - COLD_C) * wk.air } else { 20. };
                    wk.temp += (target - wk.temp) * (dt / 15.).min(1.);
                    // a lump of charge at a time, if it's hot enough for it
                    if let Some(&(it, _)) = wk.input.first() { let (m, need, fines) = charge(it).unwrap();
                        if wk.temp >= need { wk.work += ticks; if wk.work >= SMELT_TICKS { wk.work = 0; pop(&mut wk.input);
                            wk.molten[m] += if fines { FINES_YIELD } else { ORE_YIELD }; wk.slag += if fines { 0.25 } else { 0.5 };
                            if wk.slag >= 1. && self.spill(&wk, Item::Slag, 1.) { wk.slag -= 1.; } } } }
                    // tap a whole ingot into an empty mould against it
                    for m in 0..3 { if wk.molten[m] < 1. { continue; }
                        let Some(&start) = ring.iter().find(|&&i| self.g.m(i) == Mat::Mould) else { break };
                        let mut cells = vec![start]; let mut q = 0;
                        while q < cells.len() && cells.len() < S * S { let i = cells[q]; q += 1; for n in self.g.n4(i % self.g.w, i / self.g.w).collect::<Vec<_>>() { if self.g.m(n) == Mat::Mould && !cells.contains(&n) && cells.len() < S * S { cells.push(n); } } }
                        for &i in &cells { self.g.set(i, Mat::MouldHot); }
                        self.tr.casts.push(Cast { cells, ingot: METAL_INGOT[m], left: COOL_TICKS }); wk.molten[m] -= 1.; self.events.push(Event::Click);
                    }
                }
            }
            self.tr.works[k] = wk; k += 1;
        }
    }

    pub(crate) fn shift_works(&mut self, rows: i64, h: usize) {
        self.tr.works.retain_mut(|w| { let y = w.y as i64 - rows; let (_, hh) = w.kind.size(); if y < (hh + 2) as i64 || y >= h as i64 - 1 { return false; } w.y = y as usize; true });
        let (w, n) = (self.g.w as i64, (self.g.w * self.g.h) as i64);
        self.tr.casts.retain_mut(|c| { for i in c.cells.iter_mut() { let j = *i as i64 - rows * w; if j < 0 || j >= n { return false; } *i = j as usize; } true });
    }
    pub(crate) fn encode_works(&self, o: &mut crate::band::Wr) {
        let items = |o: &mut crate::band::Wr, v: &[(Item, u32)]| { o.u32(v.len() as u32); for &(it, n) in v { o.u8(it as u8); o.u32(n); } };
        o.u32(self.tr.works.len() as u32);
        for w in &self.tr.works { o.u8(w.kind as u8); o.u32(w.x as u32); o.u32(w.y as u32); items(o, &w.input); items(o, &w.fuel);
            o.u8(w.burning.map_or(255, |it| it as u8)); o.u32(w.burn); o.f32(w.temp); for m in w.molten { o.f32(m); } o.f32(w.slag); o.f32(w.water); }
        o.u32(self.tr.casts.len() as u32);
        for c in &self.tr.casts { o.u32(c.cells.len() as u32); for &i in &c.cells { o.u32(i as u32); } o.u8(c.ingot as u8); o.u32(c.left); }
    }
    pub(crate) fn decode_works(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (gw, gh) = (self.g.w, self.g.h); let ni = crate::craft::NITEM;
        let items = |r: &mut crate::band::Rd| -> Result<Vec<(Item, u32)>, String> { let mut v = vec![]; for _ in 0..r.len(ni)? { let (it, n) = (r.u8()?, r.u32()?); if (it as usize) < ni { v.push((crate::craft::ITEMS[it as usize], n)); } } Ok(v) };
        for _ in 0..r.len(4096)? {
            let kind = match r.u8()? { 0 => WorkKind::Stamp, 1 => WorkKind::Washer, _ => WorkKind::Smelter };
            let (x, y) = (r.u32()? as usize, r.u32()? as usize); let (input, fuel) = (items(r)?, items(r)?);
            let bi = r.u8()?; let burning = ((bi as usize) < ni).then(|| crate::craft::ITEMS[bi as usize]);
            let (burn, temp) = (r.u32()?, r.f32()?); let molten = [r.f32()?, r.f32()?, r.f32()?]; let (slag, water) = (r.f32()?, r.f32()?);
            let (w, h) = kind.size();
            if x > w && x + w < gw && y > h + 1 && y < gh - 1 && temp.is_finite() { self.tr.works.push(Works { kind, x, y, input, work: 0, stroke: 0., washed: 0, water, fuel, burning, burn, temp, air: 0., molten, slag }); }
        }
        let n = gw * gh;
        for _ in 0..r.len(4096)? { let mut cells = vec![]; for _ in 0..r.len(64)? { let i = r.u32()? as usize; if i < n { cells.push(i); } }
            let (it, left) = (r.u8()?, r.u32()?); if (it as usize) < ni && !cells.is_empty() { self.tr.casts.push(Cast { cells, ingot: crate::craft::ITEMS[it as usize], left }); } }
        Ok(())
    }
}
