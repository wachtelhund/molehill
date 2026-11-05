//! Ruins of the Ancients (see LORE.md and PROGRESSION.md): rare, deep remains of facilities built by
//! Delvers who dug deeper and built finer. Three kinds:
//! * a **pumping hall**: a long brick hall roofed in brick on iron pillars, pump blocks and a flooded
//!   sump; one pillar has already given way, and an old iron girder carries its bay;
//! * a **sealed vault**: thick brick on every side, no way in but through it, and a chest inside;
//! * a **tube station**: a platform hall with a broken **waygate** (an iron frame round a sealed gate) and
//!   a 40 m capsule tube running off into the rock, choked by an old cave-in. It's the hook for the Ancients'
//!   fast lines: found, not yet mendable.
//! Ruins are ordinary cells (brick, iron, a door, gravel), so the stress solver treats them like anything
//! else: dig out a pillar and the roof comes down. Generation only stamps them in (`cell`); their open air
//! is kept open by stabilisation like a grotto's.
use crate::craft::Item;
use crate::gen::{Gen, GROTTO_H, GROTTO_W};
use crate::grid::hash3;
use crate::mat::{Mat, CELL_M, CPM};

/// At most one ruin per RUIN_W × RUIN_H cell of the world (160 m × 80 m), decided by a hash.
pub const RUIN_W: usize = 160 * CPM;
pub const RUIN_H: usize = 80 * CPM;
/// Chance a ruin cell holds a ruin, by the depth (m) of its middle: none near the holt layers, rare at
/// first, commonest deep down (the Ancients dug deeper than anyone since).
pub fn ruin_chance(d: f32) -> f32 { if d < 15. { 0. } else if d < 60. { 0.05 + (d - 15.) / 45. * 0.2 } else { 0.25 } }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind { PumpingHall, SealedVault, TubeStation }
impl Kind { pub fn name(self) -> &'static str { match self { Kind::PumpingHall => "pumping hall", Kind::SealedVault => "sealed vault", Kind::TubeStation => "tube station" } } }

/// One ruin: its bounding box (window columns, world rows), and where its chest and waygate are.
#[derive(Clone, Debug)]
pub struct Ruin {
    pub id: u32, pub kind: Kind, pub x0: usize, pub y0: i64, pub w: usize, pub h: usize,
    /// tube station: the tube runs off to the left (true) or right of the hall
    pub left: bool,
    /// pumping hall: which interior pillar has given way (0..4)
    pub broken: usize,
    /// tube station: where the old cave-in plugs the tube (metres along it)
    pub plug: usize,
    /// the vault's chest (column, world row of the open cell it sits in)
    pub chest: Option<(usize, i64)>,
    /// the tube station's waygate (column, world row of the gate's middle)
    pub gate: Option<(usize, i64)>,
}

/// Lore fragments: carvings and knocked patterns found in ruins. Plain, earthy, no magic; they point upward.
pub const FRAGMENTS: [&str; 8] = [
    "A carving of Delvers in a long line, every one of them digging upward.",
    "Marks knocked into the wall in a pattern: rest, rest, a long knock. Then: up.",
    "A plate of fine metal, scored with layers: wet above, hot below, and above them all a band left blank.",
    "A worn carving of a shaft with no top. The Delvers in it carry lamps, and none of them look down.",
    "Count-marks by the hundred, and beside them the sign for water, then for cool, then one no Delver uses now.",
    "A tally of fast lines: twelve stations, each a day's walk apart, each one an hour by capsule.",
    "Scratched beside the gate, newer than the rest: the pumps are failing, seal the lines.",
    "A wide carving of the holt layers, and at its very top a thin line, worn smooth by many paws.",
];

impl Gen {
    /// The ruin in ruin cell (gx, gy), if it has one.
    pub fn ruin_at(&self, gx: i64, gy: i64) -> Option<Ruin> {
        let w = self.biome.len(); if gx < 0 || (gx as usize + 1) * RUIN_W > w { return None; }
        let h = hash3(gx as u32, gy as u32, self.seed ^ 0xA9C1E7);
        let (rw, rh) = (RUIN_W as i64, RUIN_H as i64); let mid = gy * rh + rh / 2;
        if (h & 0xFFFF) as f32 / 65536. >= ruin_chance(mid as f32 * CELL_M) { return None; }
        let c = CPM as i64; let r = |k: u32, n: i64| (hash3(h, k, self.seed) as i64).rem_euclid(n.max(1));
        let kind = [Kind::PumpingHall, Kind::SealedVault, Kind::TubeStation][r(1, 3) as usize];
        let (bw, bh) = (kind.size().0 as i64, kind.size().1 as i64);
        let m = 6 * c; // keep clear of the cell's edges
        let x0 = gx * rw + m + r(2, rw - bw - 2 * m + 1); let y0 = gy * rh + m + r(3, rh - bh - 2 * m + 1);
        // never near the starting burrow (depth 0, the window's middle), never into a grotto
        if (x0 + bw / 2 - (w / 2) as i64).abs() < 60 * c && (y0 + bh / 2).abs() < 40 * c { return None; }
        let (gw, gh) = (GROTTO_W as i64, GROTTO_H as i64);
        for gy2 in (y0 - 8 * c).div_euclid(gh)..=(y0 + bh + 8 * c).div_euclid(gh) { for gx2 in ((x0 - 8 * c).max(0)).div_euclid(gw)..=(x0 + bw + 8 * c).div_euclid(gw) {
            if let Some(g) = self.grotto_at(gx2, gy2) {
                let (gx0, gx1, gy0, gy1) = (g.cx as i64 - g.rx as i64 - 3 * c, g.cx as i64 + g.rx as i64 + 3 * c, g.cy - g.ry as i64 - 3 * c, g.cy + g.ry as i64 + 3 * c);
                if gx1 >= x0 && gx0 <= x0 + bw && gy1 >= y0 && gy0 <= y0 + bh { return None; }
            } } }
        Some(Ruin::new(h, kind, x0 as usize, y0, r(4, 2) == 0, r(5, 4) as usize, (12 + r(6, 20)) as usize))
    }
    /// The ruin whose cell holds (x, wy), if any.
    pub fn ruin_of(&self, x: usize, wy: i64) -> Option<Ruin> { self.ruin_at((x / RUIN_W) as i64, wy.div_euclid(RUIN_H as i64)) }
    /// Every ruin touching rows [wy0, wy1).
    pub fn ruins_in_rows(&self, wy0: i64, wy1: i64) -> Vec<Ruin> {
        let rh = RUIN_H as i64; let mut v = vec![];
        for gy in wy0.div_euclid(rh)..=(wy1 - 1).div_euclid(rh) { for gx in 0..(self.biome.len() / RUIN_W) as i64 { if let Some(r) = self.ruin_at(gx, gy) { if r.y0 < wy1 && r.y0 + r.h as i64 > wy0 { v.push(r); } } } }
        v
    }
    /// What a ruin puts at (x, wy): Some((material, flags)) inside a ruin's box (0 = open air), None outside.
    pub fn ruin_cell(&self, x: usize, wy: i64) -> Option<(u8, u8)> { self.ruin_of(x, wy).and_then(|r| r.cell(x, wy)).map(|m| (m as u8, 0)) }
    /// Open air or water of a ruin (stabilisation keeps it open, like a grotto's hollow).
    pub fn in_ruin_void(&self, x: usize, wy: i64) -> bool { self.ruin_of(x, wy).map_or(false, |r| matches!(r.cell(x, wy), Some(Mat::Empty | Mat::Water))) }
}

impl Kind {
    /// Width and height of this kind of ruin (cells).
    pub fn size(self) -> (usize, usize) { match self { Kind::PumpingHall => (28 * CPM, 11 * CPM), Kind::SealedVault => (9 * CPM, 6 * CPM), Kind::TubeStation => (56 * CPM, 8 * CPM) } }
}
impl Ruin {
    /// A ruin of `kind` with its top-left at (x0, y0): its chest and waygate placed. `left`: a tube station's
    /// tube runs left; `broken`: the hall's fallen pillar; `plug`: where the tube's old cave-in lies (m).
    pub fn new(id: u32, kind: Kind, x0: usize, y0: i64, left: bool, broken: usize, plug: usize) -> Ruin {
        let (w, h) = kind.size(); let c = CPM as i64;
        let mut ru = Ruin { id, kind, x0, y0, w, h, left, broken: broken % 4, plug, chest: None, gate: None };
        match kind {
            Kind::SealedVault => ru.chest = Some((x0 + w / 2, y0 + h as i64 - 2 * c + c / 2 - 1)),
            Kind::TubeStation => { let hx = if left { x0 + w - 14 * CPM } else { x0 }; let gx = if left { hx + CPM / 2 } else { hx + 13 * CPM + CPM / 2 }; ru.gate = Some((gx, y0 + 3 * c + c / 2)); }
            _ => {}
        }
        ru
    }
    /// The ruin's material at (x, wy), or None outside its box (the host rock stays).
    pub fn cell(&self, x: usize, wy: i64) -> Option<Mat> {
        let (dx, dy) = (x as i64 - self.x0 as i64, wy - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 { return None; }
        let c = CPM as i64; let (w, h) = (self.w as i64, self.h as i64);
        use Mat::*;
        Some(match self.kind {
            Kind::PumpingHall => {
                // walls and floor 1 m of brick, the roof 1.5 m of brick on iron pillars; over the bay where a
                // pillar has given way the Ancients slung an iron girder under the roof (an old repair)
                let px = [6 * c + c / 2, 12 * c, 17 * c + c / 2, 23 * c];
                let b = self.broken;
                let (ba, bb) = (if b == 0 { c } else { px[b - 1] }, if b == 3 { w - c } else { px[b + 1] + c / 2 });
                if dy < 3 * c / 2 { StoneBrick } else if dy < 2 * c && dx >= ba && dx < bb { IronBeam }
                else if dy >= h - c { StoneBrick } else if dx < c || dx >= w - c { StoneBrick }
                else {
                    // the sump: a flooded pit in the floor, 2 to 7 m in
                    if dy >= h - c - 3 * c / 4 && dx >= 2 * c && dx < 7 * c { return Some(Water); }
                    // the pumps: iron blocks beside the sump
                    if dy >= h - c - 3 * c / 2 && (8 * c..10 * c).contains(&dx) { return Some(IronBeam); }
                    // four iron pillars; the broken one is a stump with rubble round its foot
                    for (k, px) in px.into_iter().enumerate() {
                        if dx >= px && dx < px + c / 2 {
                            if k != self.broken { return Some(IronBeam); }
                            return Some(if dy >= h - c - c { IronBeam } else { Empty });
                        }
                        if k == self.broken && dy >= h - c - c / 2 && (dx - px).abs() < 2 * c && (dx - px - c / 4).abs() * 2 < 2 * c - (h - c - dy) * 4 { return Some(Gravel); }
                    }
                    Empty
                }
            }
            Kind::SealedVault => {
                // a box of brick 1.5 m thick, an iron lintel under the roof, nothing open to the outside
                let t = 3 * c / 2;
                if dx < t || dx >= w - t || dy < t || dy >= h - t { return Some(if dy >= t - c / 2 && dy < t && dx >= t - c / 2 && dx < w - t + c / 2 { IronBeam } else { StoneBrick }); }
                Empty
            }
            Kind::TubeStation => {
                // the platform hall (14 m) at one end, the tube (42 m) running off from it
                let (hx0, hx1) = if self.left { (w - 14 * c, w) } else { (0, 14 * c) };
                if dx >= hx0 && dx < hx1 {
                    let (lx, ly) = (dx - hx0, dy);
                    if ly < c / 2 { return Some(StoneBrick); } if ly < c { return Some(IronBeam); }
                    if ly >= h - c { return Some(StoneBrick); }
                    let inner = if self.left { lx < c } else { lx >= 13 * c }; // the wall the tube meets
                    let outer = if self.left { lx >= 13 * c } else { lx < c };
                    if outer { return Some(StoneBrick); }
                    if inner {
                        // the waygate: an iron frame round a sealed gate 2.5 m tall
                        if ly >= 2 * c && ly < 4 * c + c / 2 { return Some(Door); }
                        return Some(IronBeam);
                    }
                    return Some(Empty);
                }
                // the tube: 2.5 m bore inside a quarter-metre iron skin, set in a metre of brick
                let along = if self.left { (hx0 - 1 - dx) / c } else { (dx - hx1) / c };
                let (t0, t1) = (2 * c - c / 4, 4 * c + c / 2 + c / 4);
                if dy < t0 - c || dy >= t1 + c { return None; } // host rock above and below the tube's casing
                if dy < t0 || dy >= t1 { return Some(StoneBrick); }
                if dy < t0 + c / 4 || dy >= t1 - c / 4 { return Some(IronBeam); }
                // an old cave-in has choked the tube for 3 m
                if along >= self.plug as i64 && along < self.plug as i64 + 3 { return Some(Gravel); }
                Empty
            }
        })
    }
    /// The lore fragment carved here.
    pub fn fragment(&self) -> &'static str { FRAGMENTS[(self.id >> 7) as usize % FRAGMENTS.len()] }
    /// Middle of the ruin (column, world row).
    pub fn centre(&self) -> (usize, i64) { (self.x0 + self.w / 2, self.y0 + self.h as i64 / 2) }
}

/// The relics a vault can hold, in the order they turn up. Each is unique: once found, later vaults give
/// old materials instead.
pub const RELICS: [Item; 2] = [Item::AncientBit, Item::GroundLens];
/// What an ancient vault holds besides its relic (or instead, once the relics are found).
pub fn vault_goods(h: u32) -> Vec<(Item, u32)> { vec![(Item::IronIngot, 3 + h % 3), (Item::CopperWire, 6 + h % 5), (Item::Crystal, 1 + h % 2)] }

impl crate::world::World {
    /// What an Ancient vault holds: the next relic not yet found (each turns up once), and old materials.
    pub(crate) fn vault_loot(&mut self, h: u32) -> Vec<(Item, u32)> {
        let mut v = vault_goods(h);
        if let Some(k) = (0..RELICS.len()).find(|&k| self.relics & (1 << k) == 0) { self.relics |= 1 << k; v.insert(0, (RELICS[k], 1)); }
        v
    }
    /// Every half second: a ruin first come within 8 m of reads out its carving (and a tube station tells of
    /// its waygate). Each ruin speaks once.
    pub(crate) fn step_ruins(&mut self) {
        let Some(m) = self.moles.first().filter(|m| m.alive) else { return };
        let (mx, my) = (m.x as i64, self.oy + m.y as i64); let reach = 8 * CPM as i64;
        let near: Vec<(u32, Kind, &'static str)> = self.ruins.iter().filter(|r| !self.ruins_seen.contains(&r.id)
            && mx >= r.x0 as i64 - reach && mx < (r.x0 + r.w) as i64 + reach && my >= r.y0 - reach && my < r.y0 + r.h as i64 + reach)
            .map(|r| (r.id, r.kind, r.fragment())).collect();
        for (id, kind, frag) in near {
            self.ruins_seen.insert(id); self.events.push(crate::world::Event::Discover);
            self.say(format!("Ruins of the Ancients: a {}. {}", kind.name(), frag));
            if kind == Kind::TubeStation { self.say("A waygate: a sealed capsule line runs off into the rock from here, cracked and choked, its pumps long dead. The Ancients crossed the deep by lines like this.".into()); }
        }
    }
}
