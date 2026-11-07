//! Coal power: a boiler burns real lumps of fuel, boils real water out of the ground, and its steam drives an
//! engine on a drive train (power.rs). What burns leaves ash and slag at the boiler's foot as loose items that
//! have to be taken away; what the engine has used goes up its stack as steam.
//!
//! **Boiler** (`Item::Boiler`, a machine 1.5 m wide and 1.25 m tall standing on a floor; its body is solid
//! `Mat::Boiler` shell cells on the tile grid, so water stays out, an engine can stand on it, and a broken shell
//! wrecks it):
//! * **Fuel:** coal, charcoal or wood dropped on its top is fed through the hatch onto the
//!   grate; one lump burns at a time (`burn_ticks`: coal and charcoal 40 s, wood 15 s) giving `FIRE_W` of heat.
//! * **Water:** it draws water cells from the ground it stands in (any touching its body, one a solve while its
//!   drum has room for `TANK` cells), so it wants a sump or a channel beside it. Each `WATER_KJ` of steam it
//!   raises boils one cell away. Dry, the fire only heats iron.
//! * **Steam:** heat into steam at `BOILER_EFF`, held up to `STEAM_MAX` kJ; past that its safety valve lifts
//!   and the excess goes up as steam cells.
//! * **Ash and slag:** each lump burnt drops a lump of ash out of the ash door (its right side at the floor, or its
//!   left if rock stands against the right; walled in on both, the grate is choked),
//!   every third lump of coal a lump of slag as well. With `ASH_CHOKE` of them lying against the door the grate
//!   chokes and the fire is smothered until they're hauled away.
//! * A lit firebox breathes like a few torches (air.rs).
//!
//! **Steam engine** (`Mat::Engine`, machinery: it joins a drive train like a gear) and **steam pipe**
//! (`Mat::SteamPipe`): engine cells touching the boiler, or joined to it by pipe, take its steam. Each engine
//! cell can give `ENGINE_CELL_W`; an engine covers what its train is short of after any water wheel, drawing
//! steam at `1/ENGINE_EFF` of what it gives, and puffs a steam cell from its top every `EXHAUST_KJ` it uses.
use crate::craft::Item;
use crate::items::ITEM_HW;
use crate::mat::{Mat, CPM, STEAMLINE};
use crate::world::{Drop, Event, World};

pub const BOILER_W: usize = 3 * CPM / 2;
pub const BOILER_H: usize = 5 * CPM / 4;
/// Heat of a lit grate (W), and how much of it raises steam.
pub const FIRE_W: f32 = 6000.;
pub const BOILER_EFF: f32 = 0.7;
/// The boiler drum's water (cells), the steam it holds before its valve lifts (kJ), and the steam a cell of water
/// makes (kJ; a game measure: a drum of water lasts a few minutes of hard firing).
pub const TANK: f32 = 32.;
pub const STEAM_MAX: f32 = 600.;
pub const WATER_KJ: f32 = 40.;
/// What an engine cell can give (W), its efficiency from steam, and the steam per exhaust puff (kJ).
pub const ENGINE_CELL_W: f32 = 25.;
pub const ENGINE_EFF: f32 = 0.15;
pub const EXHAUST_KJ: f32 = 30.;
/// Ash and slag lying against the ash door that choke the grate.
pub const ASH_CHOKE: u32 = 10;

/// How long a lump burns on the grate (ticks), if it's fuel.
pub fn burn_ticks(it: Item) -> Option<u32> { match it { Item::Coal | Item::Charcoal => Some(40 * 60), Item::Wood => Some(15 * 60), _ => None } }

#[derive(Clone, Debug)]
pub struct Boiler {
    /// its middle column and the floor row it stands on (its body is the rows above)
    pub x: usize, pub y: usize,
    /// lumps waiting on the grate; the one burning and its ticks left
    pub fuel: Vec<(Item, u32)>, pub burning: Option<Item>, pub burn: u32,
    /// water in the drum (cells), steam held (kJ), lumps of coal burnt (for slag), whether the grate is choked
    pub water: f32, pub steam: f32, pub burnt: u32, pub choked: bool,
    /// steam towards the next exhaust or valve puff (kJ); the engine cells it feeds (found each solve)
    pub puff: f32, pub engines: Vec<usize>,
}
impl Boiler {
    /// Its body (x0, y0, x1, y1) in cells.
    pub fn rect(&self) -> (f32, f32, f32, f32) { let h = (BOILER_W / 2) as f32; (self.x as f32 - h, (self.y - BOILER_H) as f32, self.x as f32 + h, self.y as f32) }
    pub fn contains(&self, x: f32, y: f32) -> bool { let (x0, y0, x1, y1) = self.rect(); x >= x0 && x < x1 && y >= y0 && y < y1 }
    pub fn lumps(&self) -> u32 { self.fuel.iter().map(|f| f.1).sum::<u32>() + self.burning.is_some() as u32 }
    fn add(&mut self, it: Item, n: u32) { if let Some(f) = self.fuel.iter_mut().find(|f| f.0 == it) { f.1 += n; } else { self.fuel.push((it, n)); } }
}

impl World {
    /// Stand a boiler on the floor under (x, y).
    pub(crate) fn place_boiler(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let h = BOILER_W / 2;
        if x < h + 2 || x + h + 2 >= self.g.w { return Err("too near the edge"); }
        // on the tile grid, so engines, pipes and pools meet its sides
        let s = crate::mat::S; let x = (x - h) / s * s + h;
        let floor = (y..(y + 3 * CPM).min(self.g.h - 1)).find(|&r| !self.passable(x - h, r) || !self.passable(x + h - 1, r) || !self.passable(x, r)).ok_or("needs a floor under it")?;
        if floor < BOILER_H + 2 { return Err("too near the edge"); }
        if (x - h..x + h).any(|c| self.passable(c, floor)) { return Err("needs a level floor under all of it"); }
        if !(floor - BOILER_H..floor).all(|r| (x - h..x + h).all(|c| self.g.get(c, r) == Mat::Empty || self.g.get(c, r) == Mat::Water)) { return Err("needs 1.5 m by 1.25 m of room"); }
        if self.tr.boilers.iter().any(|b| b.x.abs_diff(x) < BOILER_W && b.y.abs_diff(floor) < BOILER_H) { return Err("there's a boiler there"); }
        // water standing where it goes is taken into its drum; its body is shell cells
        let mut water = 0.;
        for r in floor - BOILER_H..floor { for c in x - h..x + h { let i = self.g.idx(c, r); if self.g.m(i) == Mat::Water { water += 1.; } self.g.set(i, Mat::Boiler); } }
        self.tr.boilers.push(Boiler { x, y: floor, fuel: vec![], burning: None, burn: 0, water: (water as f32).min(TANK), steam: 0., burnt: 0, choked: false, puff: 0., engines: vec![] });
        self.events.push(Event::Place); Ok(())
    }
    pub fn boiler_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.boilers.iter().position(|b| b.contains(xf, yf)) }
    /// `E` on a boiler: its unburnt fuel into the pack; a cold, empty one comes up itself (its water is lost).
    pub(crate) fn pick_up_boiler(&mut self, x: usize, y: usize) -> Option<Item> {
        let k = self.boiler_at(x, y)?;
        if !self.tr.boilers[k].fuel.is_empty() { let mut got = vec![];
            for (it, n) in std::mem::take(&mut self.tr.boilers[k].fuel) { self.inv[it as usize] += n; got.push(format!("{n} {}", it.name().to_lowercase())); }
            self.say(format!("Raked the fuel out of the boiler: {}.", got.join(", "))); return Some(Item::Boiler); }
        if self.tr.boilers[k].burning.is_some() { self.say("The boiler's fire is still burning.".into()); return None; }
        let b = self.tr.boilers.remove(k); self.clear_shell(&b); self.inv[Item::Boiler as usize] += 1; Some(Item::Boiler)
    }

    fn clear_shell(&mut self, b: &Boiler) {
        let (x0, y0, x1, y1) = b.rect();
        for r in y0 as usize..y1 as usize { for c in x0 as usize..x1 as usize { let i = self.g.idx(c, r); if self.g.m(i) == Mat::Boiler { self.g.set(i, Mat::Empty); } } }
    }
    /// Put a puff of steam into the open cell nearest above (x, y) (up to 1 m), if there is one.
    fn puff_steam(&mut self, x: usize, y: usize) {
        for r in (y.saturating_sub(CPM)..=y).rev() { let i = self.g.idx(x, r); if self.g.m(i) == Mat::Empty { self.g.set(i, Mat::Steam); return; } }
    }

    /// One solve of every boiler (`dt` s): fuel in, the fire, water in, steam up, ash out, and which engine cells it
    /// feeds. Called from `step_power` before the trains are balanced.
    pub(crate) fn step_boilers(&mut self, dt: f32) {
        let (gw, gh) = (self.g.w, self.g.h);
        for k in 0..self.tr.boilers.len() {
            let mut b = self.tr.boilers[k].clone(); let (x0, y0, x1, y1) = b.rect();
            // its shell broken up (dug, blasted, fallen in): it's wrecked, its fuel spilling, and what's left of the
            // shell crumbles
            let shell = (y0 as usize..y1 as usize).map(|r| (x0 as usize..x1 as usize).filter(|&c| self.g.get(c, r) == Mat::Boiler).count()).sum::<usize>();
            if shell * 4 < BOILER_W * BOILER_H * 3 {
                for (it, n) in std::mem::take(&mut b.fuel) { for _ in 0..n { self.drops.push(Drop::at(b.x as f32, y0 - 2., it, 1)); } }
                self.clear_shell(&b); self.tr.boilers.remove(k); self.say("A boiler's shell broke up: it's wrecked.".into()); self.events.push(Event::Click); return;
            }
            // fuel dropped on its top goes in through the hatch
            let mut j = 0;
            while j < self.drops.len() { let d = &self.drops[j];
                if d.fuse == 0 && burn_ticks(d.item).is_some() && d.px >= x0 && d.px < x1 && d.py >= y0 - 3. * ITEM_HW && d.py < y0 { let d = self.drops.swap_remove(j); b.add(d.item, d.n); } else { j += 1; } }
            // the ash door opens on the right, or on the left if rock stands against the right; ash and slag piled
            // against it choke the grate, and with rock against both sides it can't be raked out at all
            let ay = y1 - ITEM_HW - 0.05;
            let door = [(1f32, x1 + ITEM_HW + 0.2), (-1f32, x0 - ITEM_HW - 0.2)].into_iter().find(|&(_, ax)| !self.item_box_hits_pub(ax, ay, Item::Ash));
            let heap = door.map_or(0, |(s, _)| { let (dx0, dx1) = if s > 0. { (x1, x1 + CPM as f32) } else { (x0 - CPM as f32, x0) };
                self.drops.iter().filter(|d| matches!(d.item, Item::Ash | Item::Slag) && d.px >= dx0 && d.px < dx1 && d.py > y1 - CPM as f32 && d.py <= y1).map(|d| d.n).sum::<u32>() });
            let was = b.choked; b.choked = door.is_none() || heap >= ASH_CHOKE;
            if b.choked && !was { self.say(if door.is_none() { "The boiler's ash door is walled in: it needs room at one side to rake the ash out.".into() } else { "The boiler's ash door is choked: haul the ash and slag away.".to_string() }); }
            // the fire: a lump at a time
            let ticks = (dt * 60.) as u32;
            if !b.choked {
                if b.burning.is_none() { if let Some(f) = b.fuel.first_mut() { let it = f.0; f.1 -= 1; if f.1 == 0 { b.fuel.remove(0); } b.burning = Some(it); b.burn = burn_ticks(it).unwrap_or(0); } }
                if let Some(it) = b.burning {
                    b.burn = b.burn.saturating_sub(ticks);
                    // heat into steam while there's water to boil
                    if b.water > 0. { let kj = FIRE_W * BOILER_EFF * dt / 1000.; b.steam += kj; b.water = (b.water - kj / WATER_KJ).max(0.); }
                    if b.burn == 0 {
                        b.burning = None;
                        // what's left of the lump falls through the grate and out of the ash door
                        let (s, ax) = door.unwrap_or((1., x1 + ITEM_HW + 0.2));
                        let mut out = vec![Item::Ash];
                        if it == Item::Coal { b.burnt += 1; if b.burnt % 3 == 0 { out.push(Item::Slag); } }
                        for o in out { let mut d = Drop::at(ax, ay, o, 1); d.vx = s * 0.8 * CPM as f32; self.drops.push(d); }
                    }
                }
            }
            // the safety valve: steam past what the drum holds goes up as steam
            if b.steam > STEAM_MAX { b.puff += b.steam - STEAM_MAX; b.steam = STEAM_MAX; }
            while b.puff >= EXHAUST_KJ { b.puff -= EXHAUST_KJ; self.puff_steam(b.x, (y0 as usize).saturating_sub(1)); }
            // water drawn in from the ground touching it
            if b.water + 1. <= TANK {
                let ring: Vec<usize> = (x0 as usize - 1..=x1 as usize).flat_map(|c| [(c, y0 as usize - 1), (c, y1 as usize - 1)])
                    .chain((y0 as usize..y1 as usize).flat_map(|r| [(x0 as usize - 1, r), (x1 as usize, r)])).filter(|&(c, r)| c < gw && r < gh).map(|(c, r)| r * gw + c).collect();
                if let Some(&i) = ring.iter().find(|&&i| self.g.m(i) == Mat::Water) { self.g.set(i, Mat::Empty); b.water += 1.; }
            }
            // the engine cells it feeds: touching it, or along steam pipe from it
            let mut seen: Vec<usize> = (x0 as usize - 1..=x1 as usize).flat_map(|c| [(y0 as usize - 1) * gw + c, y1 as usize * gw + c])
                .chain((y0 as usize..y1 as usize).flat_map(|r| [r * gw + x0 as usize - 1, r * gw + x1 as usize])).filter(|&i| i < gw * gh && self.g.m(i).has(STEAMLINE)).collect();
            let mut q = 0;
            while q < seen.len() && seen.len() < 4096 { let i = seen[q]; q += 1;
                // steam fills the whole engine it reaches, but goes no further through it into more pipe
                let eng = self.g.m(i) == Mat::Engine;
                for n in self.g.n4(i % gw, i / gw).collect::<Vec<_>>() { let mn = self.g.m(n); if mn.has(STEAMLINE) && (!eng || mn == Mat::Engine) && !seen.contains(&n) { seen.push(n); } } }
            b.engines = seen.into_iter().filter(|&i| self.g.m(i) == Mat::Engine).collect();
            self.tr.boilers[k] = b;
        }
    }

    /// The engines' share of a train's power: what the boilers feeding train `t`'s engine cells can give (W).
    pub(crate) fn engine_cap(&self, train: &std::collections::HashMap<usize, u32>, t: u32, dt: f32) -> f32 {
        self.tr.boilers.iter().map(|b| { let n = b.engines.iter().filter(|i| train.get(i) == Some(&t)).count();
            if n == 0 { 0. } else { (n as f32 * ENGINE_CELL_W).min(b.steam * 1000. / dt * ENGINE_EFF) } }).sum()
    }
    /// Draw the steam for `w` W of engine work on train `t` for `dt` s from its boilers, and puff the exhaust.
    pub(crate) fn engine_draw(&mut self, train: &std::collections::HashMap<usize, u32>, t: u32, w: f32, dt: f32) {
        let mut need = w * dt / 1000. / ENGINE_EFF;
        for k in 0..self.tr.boilers.len() { if need <= 0. { break; }
            let feeds: Vec<usize> = self.tr.boilers[k].engines.iter().copied().filter(|i| train.get(i) == Some(&t)).collect(); if feeds.is_empty() { continue; }
            let b = &mut self.tr.boilers[k]; let take = need.min(b.steam); b.steam -= take; need -= take; b.puff += take;
            // exhaust from the top of an engine cell
            let top = *feeds.iter().min_by_key(|&&i| i / self.g.w).unwrap(); let gw = self.g.w;
            while self.tr.boilers[k].puff >= EXHAUST_KJ { self.tr.boilers[k].puff -= EXHAUST_KJ; self.puff_steam(top % gw, (top / gw).saturating_sub(1)); }
        }
    }

    pub(crate) fn shift_boilers(&mut self, rows: i64, h: usize) {
        self.tr.boilers.retain_mut(|b| { let y = b.y as i64 - rows; if y < (BOILER_H + 2) as i64 || y >= h as i64 - 1 { return false; } b.y = y as usize; b.engines.clear(); true });
    }
    pub(crate) fn encode_boilers(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.boilers.len() as u32);
        for b in &self.tr.boilers { o.u32(b.x as u32); o.u32(b.y as u32); o.u32(b.fuel.len() as u32); for &(it, n) in &b.fuel { o.u8(it as u8); o.u32(n); }
            o.u8(b.burning.map_or(255, |it| it as u8)); o.u32(b.burn); o.f32(b.water); o.f32(b.steam); o.u32(b.burnt); }
    }
    pub(crate) fn decode_boilers(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (gw, gh) = (self.g.w, self.g.h);
        for _ in 0..r.len(4096)? {
            let (x, y) = (r.u32()? as usize, r.u32()? as usize);
            let mut fuel = vec![]; for _ in 0..r.len(4096)? { let (it, n) = (r.u8()?, r.u32()?); if let Some(it) = r.item(it) { fuel.push((it, n)); } }
            let bi = r.u8()?; let burning = r.item(bi);
            let (burn, water, steam, burnt) = (r.u32()?, r.f32()?, r.f32()?, r.u32()?);
            if x > BOILER_W && x + BOILER_W < gw && y > BOILER_H + 1 && y < gh - 1 && water.is_finite() && steam.is_finite() {
                self.tr.boilers.push(Boiler { x, y, fuel, burning, burn, water: water.clamp(0., TANK), steam: steam.clamp(0., STEAM_MAX), burnt, choked: false, puff: 0., engines: vec![] }); }
        }
        Ok(())
    }
}
