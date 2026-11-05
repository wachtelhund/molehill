//! Procedural sites of the Deepholt: seven families of found places, not random loot rooms.
//!
//! Each site has a purpose, materials, layout variation, a depth/rarity band, and something a Delver
//! can *do* (salvage, drain, climb, excavate, harvest, ride). Real foreground (props, rails, water,
//! tubes) is ordinary cells and obeys the same physics as anything you build. Background lining comes
//! from `walls.rs` and holds no roof.
//!
//! Placement mirrors ruins: at most one site per SITE_W × SITE_H cell, from the seed. None near the
//! start burrow, none in a grotto, none overlapping a ruin or another site. Buried transport halts
//! sit next to an Ancient tube station when one is near, and never grow a second waygate.
use crate::craft::Item;
use crate::gen::{Gen, GROTTO_H, GROTTO_W, START_SAFE};
use crate::grid::hash3;
use crate::logic::Dev;
use crate::mat::{Mat, CELL_M, CPM, S};
use crate::ruins::{Kind as RuinKind, RUIN_H, RUIN_W};
use crate::walls::Wall;
use crate::world::{Farm, Furniture, World};

/// At most one site per 96 m × 48 m cell. Wider than a grotto cell, tighter than a ruin cell.
pub const SITE_W: usize = 96 * CPM;
pub const SITE_H: usize = 48 * CPM;
/// Small test windows (the arena, most scenarios) stay site-free so existing holts don't change shape.
pub const SITE_MIN_W: usize = 160 * CPM;

/// Chance a site cell holds a site, by the depth (m) of its middle. None in the far wet up or the
/// lava deep; a handful around the holt layers and the mid-granite.
pub fn site_chance(d: f32) -> f32 {
    if d < -28. || d > 100. { 0. } else if d < 10. { 0.10 } else if d < 55. { 0.13 } else { 0.09 }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind { Mine, Holt, Pumpworks, Fissure, Fossil, Crystal, Halt }
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Mine => "abandoned mine", Kind::Holt => "abandoned holt", Kind::Pumpworks => "flooded pumpworks",
            Kind::Fissure => "fissure", Kind::Fossil => "fossil cave", Kind::Crystal => "salt hall",
            Kind::Halt => "buried cargo halt",
        }
    }
    /// Width and height (cells).
    pub fn size(self) -> (usize, usize) {
        match self {
            Kind::Mine => (24 * CPM, 7 * CPM),
            Kind::Holt => (16 * CPM, 7 * CPM),
            Kind::Pumpworks => (22 * CPM, 10 * CPM),
            Kind::Fissure => (10 * CPM, 26 * CPM),
            Kind::Fossil => (20 * CPM, 10 * CPM),
            Kind::Crystal => (16 * CPM, 8 * CPM),
            Kind::Halt => (26 * CPM, 8 * CPM),
        }
    }
    /// Which depths this family belongs in (metres, the cell's middle).
    pub fn at_depth(self, d: f32) -> bool {
        match self {
            Kind::Mine => d > 0. && d < 56.,
            Kind::Holt => d >= -32. && d < 20.,
            Kind::Pumpworks => d > -12. && d < 40.,
            Kind::Fissure => d > 8. && d < 90.,
            Kind::Fossil => d >= 20. && d < 88.,
            Kind::Crystal => d > 36. && d < 100.,
            Kind::Halt => d > 12. && d < 84.,
        }
    }
    pub fn from_u8(v: u8) -> Kind {
        [Kind::Mine, Kind::Holt, Kind::Pumpworks, Kind::Fissure, Kind::Fossil, Kind::Crystal, Kind::Halt][v as usize % 7]
    }
    pub fn from_name(s: &str) -> Option<Kind> {
        Some(match s {
            "mine" => Kind::Mine, "holt" => Kind::Holt, "pump" | "pumpworks" => Kind::Pumpworks,
            "fissure" => Kind::Fissure, "fossil" => Kind::Fossil, "crystal" | "salt" => Kind::Crystal,
            "halt" => Kind::Halt, _ => return None,
        })
    }
}

/// One site: its box, a variant nibble for layout, and the chest / Ancient-tube link if any.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub id: u32, pub kind: Kind, pub x0: usize, pub y0: i64, pub w: usize, pub h: usize,
    /// layout knobs packed from the seed (ore type, which bay collapsed, which way a halt faces, …)
    pub var: u32,
    /// a halt's wrecked tube runs off to the left
    pub left: bool,
    pub chest: Option<(usize, i64)>,
    /// an Ancient tube station this halt points at (the ruin's id)
    pub link: Option<u32>,
}

/// What a Delver reads the first time they come within 8 m. No cards, no banners: a line in the log.
pub fn notice(kind: Kind, link: bool) -> &'static str {
    match kind {
        Kind::Mine => "An old mine: timber still holding, the rails gone to rust. Someone followed the seam and left.",
        Kind::Holt => "A holt, empty. Nests cold, the fungus still trying.",
        Kind::Pumpworks => "Pumpworks, half drowned. The sluices still know their job.",
        Kind::Fissure => "A fissure splits the rock. Old bridges hang in pieces; a rope still reaches.",
        Kind::Fossil => "Bones in the stone, bigger than any Delver. The rock grew around them.",
        Kind::Crystal => "A hall of salt and crystal. Water lies still and throws the lamps back.",
        Kind::Halt if link => "The old cargo halt meets a line of the Ancients. Their tube runs on from here.",
        Kind::Halt => "A cargo halt, wrecked. Carts on their sides, the tubes torn. The line once ran further.",
    }
}

impl Gen {
    /// The site in site cell (gx, gy), if it has one.
    ///
    /// `fill_rows` asks this once per cell; the answer depends only on the site cell, so the last
    /// `(seed, gx, gy)` is memoised and a 2 M-cell window does the expensive overlap work a handful of times.
    pub fn site_at(&self, gx: i64, gy: i64) -> Option<Site> {
        thread_local! { static LAST: std::cell::RefCell<Option<(u32, i64, i64, Option<Site>)>> = std::cell::RefCell::new(None); }
        LAST.with(|slot| {
            if let Some((seed, x, y, s)) = *slot.borrow() {
                if seed == self.seed && x == gx && y == gy { return s; }
            }
            let s = self.site_at_fresh(gx, gy);
            *slot.borrow_mut() = Some((self.seed, gx, gy, s));
            s
        })
    }
    fn site_at_fresh(&self, gx: i64, gy: i64) -> Option<Site> {
        let w = self.biome.len(); if w < SITE_MIN_W { return None; }
        if gx < 0 || (gx as usize + 1) * SITE_W > w { return None; }
        let h = hash3(gx as u32, gy as u32, self.seed ^ 0x51FE);
        let (sw, sh) = (SITE_W as i64, SITE_H as i64); let mid = gy * sh + sh / 2;
        let d = mid as f32 * CELL_M;
        if (h & 0xFFFF) as f32 / 65536. >= site_chance(d) { return None; }
        let mut ks = [Kind::Mine; 7]; let mut n = 0usize;
        for k in 0..7 { let kd = Kind::from_u8(k); if kd.at_depth(d) { ks[n] = kd; n += 1; } }
        if n == 0 { return None; }
        let kind = ks[(h as usize >> 16) % n];
        let c = CPM as i64; let r = |k: u32, n: i64| (hash3(h, k, self.seed) as i64).rem_euclid(n.max(1));
        let (bw, bh) = (kind.size().0 as i64, kind.size().1 as i64);
        let m = 5 * c;
        let x0 = gx * sw + m + r(2, (sw - bw - 2 * m).max(1));
        let y0 = gy * sh + m + r(3, (sh - bh - 2 * m).max(1));
        if (x0 + bw / 2 - (w / 2) as i64).abs() < (START_SAFE as i64 + 20) * c && (y0 + bh / 2).abs() < 40 * c { return None; }
        let (gw, gh) = (GROTTO_W as i64, GROTTO_H as i64);
        for gy2 in (y0 - 6 * c).div_euclid(gh)..=(y0 + bh + 6 * c).div_euclid(gh) {
            for gx2 in ((x0 - 6 * c).max(0)).div_euclid(gw)..=(x0 + bw + 6 * c).div_euclid(gw) {
                if let Some(g) = self.grotto_at(gx2, gy2) {
                    let (gx0, gx1, gy0, gy1) = (g.cx as i64 - g.rx as i64 - 3 * c, g.cx as i64 + g.rx as i64 + 3 * c, g.cy - g.ry as i64 - 3 * c, g.cy + g.ry as i64 + 3 * c);
                    if gx1 >= x0 && gx0 <= x0 + bw && gy1 >= y0 && gy0 <= y0 + bh { return None; }
                }
            }
        }
        let (rw, rh) = (RUIN_W as i64, RUIN_H as i64);
        for rgy in (y0 - 6 * c).div_euclid(rh)..=(y0 + bh + 6 * c).div_euclid(rh) {
            for rgx in ((x0 - 6 * c).max(0)).div_euclid(rw)..=(x0 + bw + 6 * c).div_euclid(rw) {
                if let Some(ru) = self.ruin_at(rgx, rgy) {
                    let (rx0, rx1, ry0, ry1) = (ru.x0 as i64, ru.x0 as i64 + ru.w as i64, ru.y0, ru.y0 + ru.h as i64);
                    if rx1 > x0 && rx0 < x0 + bw && ry1 > y0 && ry0 < y0 + bh { return None; }
                }
            }
        }
        let mut link = None; let mut left = r(4, 2) == 0;
        if kind == Kind::Halt {
            for rgy in (y0 - 50 * c).div_euclid(rh)..=(y0 + bh + 50 * c).div_euclid(rh) {
                for rgx in ((x0 - 50 * c).max(0)).div_euclid(rw)..=(x0 + bw + 50 * c).div_euclid(rw) {
                    if let Some(ru) = self.ruin_at(rgx, rgy) {
                        if ru.kind != RuinKind::TubeStation { continue; }
                        let (rx0, rx1, ry0, ry1) = (ru.x0 as i64, ru.x0 as i64 + ru.w as i64, ru.y0, ru.y0 + ru.h as i64);
                        let (cx, cy) = ((x0 + bw / 2 - (rx0 + rx1) / 2).abs(), (y0 + bh / 2 - (ry0 + ry1) / 2).abs());
                        if cx < 50 * c && cy < 40 * c { link = Some(ru.id); left = (rx0 + rx1) / 2 < x0 + bw / 2; }
                    }
                }
            }
        }
        Some(Site::new(h, kind, x0 as usize, y0, r(5, 1 << 20) as u32, left, link))
    }
    pub fn site_of(&self, x: usize, wy: i64) -> Option<Site> { self.site_at((x / SITE_W) as i64, wy.div_euclid(SITE_H as i64)) }
    pub fn sites_in_rows(&self, wy0: i64, wy1: i64) -> Vec<Site> {
        let sh = SITE_H as i64; let mut v = vec![];
        if self.biome.len() < SITE_MIN_W { return v; }
        for gy in wy0.div_euclid(sh)..=(wy1 - 1).div_euclid(sh) {
            for gx in 0..(self.biome.len() / SITE_W) as i64 {
                if let Some(s) = self.site_at(gx, gy) { if s.y0 < wy1 && s.y0 + s.h as i64 > wy0 { v.push(s); } }
            }
        }
        v
    }
    /// What a site puts at (x, wy): Some((material, flags)) inside its carved cells, None to leave the host.
    pub fn site_cell(&self, x: usize, wy: i64) -> Option<(u8, u8)> {
        self.site_of(x, wy).and_then(|s| s.cell(x, wy)).map(|m| (m as u8, 0))
    }
    pub fn site_wall(&self, x: usize, wy: i64) -> Option<Wall> { self.site_of(x, wy).and_then(|s| s.wall(x, wy)) }
    /// Open air or water a site carved (stabilisation must not fill it).
    pub fn in_site_void(&self, x: usize, wy: i64) -> bool {
        self.site_of(x, wy).and_then(|s| s.cell(x, wy)).is_some_and(|m| matches!(m, Mat::Empty | Mat::Water | Mat::Rail | Mat::Tube))
    }
}

impl Site {
    pub fn new(id: u32, kind: Kind, x0: usize, y0: i64, var: u32, left: bool, link: Option<u32>) -> Site {
        let (w, h) = kind.size();
        let mut s = Site { id, kind, x0, y0, w, h, var, left, chest: None, link };
        let c = CPM as i64;
        s.chest = match kind {
            Kind::Mine => Some((x0 + w - 3 * CPM, y0 + h as i64 - 2 * c + c / 2 - 1)),
            Kind::Holt => Some((x0 + w / 2, y0 + h as i64 - 2 * c + c / 2 - 1)),
            Kind::Pumpworks => Some((x0 + w - 4 * CPM, y0 + h as i64 - 2 * c + c / 2 - 1)),
            Kind::Fossil => Some((x0 + 3 * CPM, y0 + h as i64 - 2 * c + c / 2 - 1)),
            Kind::Crystal => Some((x0 + w / 2, y0 + h as i64 - 2 * c + c / 2 - 1)),
            Kind::Halt => Some((x0 + if left { w - 4 * CPM } else { 4 * CPM }, y0 + h as i64 - 2 * c + c / 2 - 1)),
            Kind::Fissure => None,
        };
        s
    }
    pub fn centre(&self) -> (usize, i64) { (self.x0 + self.w / 2, self.y0 + self.h as i64 / 2) }
    /// Material this site stamps at (x, wy), or None to leave the host rock.
    pub fn cell(&self, x: usize, wy: i64) -> Option<Mat> {
        let (dx, dy) = (x as i64 - self.x0 as i64, wy - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 { return None; }
        let c = CPM as i64; let (w, h) = (self.w as i64, self.h as i64);
        use Mat::*;
        Some(match self.kind {
            Kind::Mine => {
                // a timbered drift with rail fragments, ore pockets and one collapsed side bay
                let ore = if self.var & 1 == 0 { CopperOre } else { Coal };
                let bay = 14 * c + (self.var as i64 % 4) * c;
                if dy < c / 2 { Limestone }
                else if dy >= h - c {
                    let along = dx / c;
                    if along % 5 != 4 && dy < h - c / 2 { Rail } else { Rock }
                } else if dx < c / 2 || dx >= w - c / 2 { Rock }
                else if (dx + c / 4) % (4 * c) < c / 2 { WoodBeam }
                else if dx >= 6 * c && dx < 8 * c && dy >= h - 3 * c { ore }
                else if dx >= 16 * c && dx < 18 * c && dy >= h - 3 * c { ore }
                else if dx >= bay && dx < bay + 2 * c && dy >= h - 3 * c { Gravel }
                else { Empty }
            }
            Kind::Holt => {
                let t = (dx as f32 - w as f32 / 2.) / (w as f32 / 2.);
                let top = (2. * c as f32 * t * t).round() as i64;
                if dy < top + c / 2 { Limestone }
                else if dy >= h - c { Clay }
                else if dx < c / 2 || dx >= w - c / 2 { Limestone }
                else if dx >= 2 * c && dx < 2 * c + c / 2 && dy >= 2 * c { WoodBeam }
                else if dx >= w - 3 * c && dx < w - 3 * c + c / 2 && dy >= 2 * c { WoodBeam }
                else { Empty }
            }
            Kind::Pumpworks => {
                let split = 11 * c;
                if dy < c / 2 || dy >= h - c { StoneBrick }
                else if dx < c / 2 || dx >= w - c / 2 { StoneBrick }
                else if (dx - 5 * c).abs() < c / 4 || (dx - 17 * c).abs() < c / 4 { IronBeam }
                else if dx >= split && dx < split + c {
                    if dy >= h - 3 * c && dy < h - 2 * c { Door }
                    else if dy >= h - 2 * c && dy < h - c { Clay }
                    else { StoneBrick }
                } else if dx < split {
                    if dy >= h - 3 * c { Water } else { Empty }
                } else { Empty }
            }
            Kind::Fissure => {
                let inner = dx >= 2 * c && dx < w - 2 * c;
                if !inner { return None; }
                let ledge = |at: i64| dy >= at && dy < at + c / 2 && (dx < 4 * c || dx >= w - 4 * c);
                if ledge(7 * c) || ledge(14 * c) || ledge(20 * c) { return Some(if (dx / c) % 2 == 0 { WoodBeam } else { IronBeam }); }
                if dy >= 14 * c && dy < 14 * c + c / 2 && (dx < 3 * c || dx >= w - 3 * c) { IronBeam }
                else if dy >= h - c { Gravel }
                else { Empty }
            }
            Kind::Fossil => {
                let (cx, cy) = (w as f32 / 2., h as f32 * 0.55);
                let (rx, ry) = (w as f32 * 0.46, h as f32 * 0.42);
                let nx = (dx as f32 - cx) / rx; let ny = (dy as f32 - cy) / ry;
                if nx * nx + ny * ny > 1. { return None; }
                let spine = dy >= h - 2 * c && dy < h - c && (dx - w / 2).abs() < 8 * c;
                let rib = (0..8).any(|k| {
                    let px = 2 * c + k * 2 * c;
                    (dx - px).abs() < c / 3 && dy >= h - 5 * c && dy < h - c
                });
                let skull = dx >= w - 5 * c && dy >= h - 5 * c && dy < h - c && (dx - (w - 3 * c)).abs() + (dy - (h - 3 * c)).abs() < 2 * c;
                let roots = dy < 2 * c && (dx / c) % 3 == 1;
                if spine || rib { Carved } else if skull { Quartz } else if roots { Roots } else if dy >= h - c / 2 { Rock } else { Empty }
            }
            Kind::Crystal => {
                let (cx, cy) = (w as f32 / 2., h as f32 * 0.55);
                let (rx, ry) = (w as f32 * 0.46, h as f32 * 0.40);
                let nx = (dx as f32 - cx) / rx; let ny = (dy as f32 - cy) / ry;
                if nx * nx + ny * ny > 1. { return None; }
                let pool = dy >= h - c && ((dx - 4 * c).abs() < 2 * c || (dx - w + 4 * c).abs() < 2 * c);
                let cluster = (dx / c + dy / c + (self.var as i64 & 3)) % 5 == 0 && dy >= h - 3 * c;
                let col = (dx - 3 * c).abs() < c / 4 || (dx - w + 3 * c).abs() < c / 4;
                if pool { Water } else if col && dy >= 2 * c { Quartz } else if cluster { Crystal } else if dy >= h - c / 2 { Quartz } else { Empty }
            }
            Kind::Halt => {
                let hall = 14 * c;
                let in_hall = if self.left { dx >= w - hall } else { dx < hall };
                if in_hall {
                    let lx = if self.left { dx - (w - hall) } else { dx };
                    if dy < c / 2 { StoneBrick } else if dy >= h - c {
                        if lx >= 2 * c && lx < hall - 2 * c && dy < h - c / 2 { Rail } else { StoneBrick }
                    } else if lx < c / 2 || lx >= hall - c / 2 { StoneBrick }
                    else if (lx - 5 * c).abs() < c / 4 || (lx - 9 * c).abs() < c / 4 { IronBeam }
                    else { Empty }
                } else {
                    let (t0, t1) = (2 * c, 5 * c);
                    if dy < t0 - c / 2 || dy >= t1 + c / 2 { return None; }
                    if dy < t0 || dy >= t1 { StoneBrick }
                    else if dy < t0 + c / 4 || dy >= t1 - c / 4 { IronBeam }
                    else if (dx / c) % 4 == 3 { Gravel }
                    else { Tube }
                }
            }
        })
    }
    /// Lining this site wants at (x, wy). None = leave whatever `line_back` would put.
    pub fn wall(&self, x: usize, wy: i64) -> Option<Wall> {
        let (dx, dy) = (x as i64 - self.x0 as i64, wy - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 { return None; }
        if self.cell(x, wy).is_none() { return None; }
        let h = hash3(x as u32, wy as u32, self.id);
        Some(match self.kind {
            Kind::Mine => if (h & 7) == 0 { Wall::Gap } else if (h & 7) == 1 { Wall::Beams } else if (h & 7) == 2 { Wall::Crack } else { Wall::Timber },
            Kind::Holt => if dy > self.h as i64 - 2 * CPM as i64 && (h & 3) == 0 { Wall::Damp } else if (h & 5) == 0 { Wall::Masonry } else { Wall::Timber },
            Kind::Pumpworks => if (h & 7) == 0 { Wall::Pipes } else if (h & 7) == 1 { Wall::Damp } else { Wall::Masonry },
            Kind::Fissure => if dy > self.h as i64 - 3 * CPM as i64 { Wall::Damp } else if (h & 3) == 0 { Wall::Crack } else { Wall::Rock },
            Kind::Fossil => if (h & 3) == 0 { Wall::Fossil } else if (h & 3) == 1 { Wall::Crack } else { Wall::Rock },
            Kind::Crystal => if (h & 3) == 0 { Wall::Salt } else if (h & 3) == 1 { Wall::Crystal } else { Wall::Damp },
            Kind::Halt => if (h & 7) == 0 { Wall::Pipes } else if (h & 7) == 1 { Wall::Gap } else { Wall::Masonry },
        })
    }
}

/// Loot in a site chest: old kit and what the place was for, never relics (those stay in Ancient vaults).
pub fn site_loot(kind: Kind, h: u32) -> Vec<(Item, u32)> {
    match kind {
        Kind::Mine => vec![(Item::Coal, 4 + h % 3), (Item::CopperOre, 2 + h % 2), (Item::WoodPillar, 4), (Item::Rail, 6)],
        Kind::Holt => vec![(Item::Torch, 3), (Item::Grub, 2), (Item::Wood, 3), (Item::Fungus, 2)],
        Kind::Pumpworks => vec![(Item::CopperWire, 6), (Item::IronIngot, 1 + h % 2), (Item::Clay, 4)],
        Kind::Fissure => vec![(Item::RopeLadder, 1), (Item::Wood, 2)],
        Kind::Fossil => vec![(Item::Quartz, 3 + h % 2), (Item::Stone, 6), (Item::Crystal, 1)],
        Kind::Crystal => vec![(Item::Crystal, 3 + h % 2), (Item::Quartz, 2), (Item::Chalk, 2)],
        Kind::Halt => vec![(Item::Rail, 8), (Item::CopperIngot, 1), (Item::Tube, 4), (Item::Stone, 6)],
    }
}

impl World {
    /// Refresh the sites touching the window (with the chests).
    pub fn refresh_sites(&mut self) {
        self.sites = self.gen.sites_in_rows(self.oy, self.oy + self.g.h as i64);
        self.stamp_site_things();
    }
    /// Devices, furniture, farms, carts and ladders a site owns. Idempotent: a thing already there is left.
    pub fn stamp_site_things(&mut self) {
        let sites = self.sites.clone();
        for s in &sites { self.stamp_one(s); }
    }
    /// Scripted: stamp one site of `kind` with its top-left at window (x0, y0). Used by scenarios.
    pub fn stamp_site(&mut self, kind: Kind, x0: usize, y0: usize) {
        let (w, h) = kind.size();
        if x0 + w >= self.g.w || y0 + h >= self.g.h { return; }
        let s = Site::new(0x51FE ^ (x0 as u32).wrapping_mul(131) ^ (y0 as u32), kind, x0, self.oy + y0 as i64, 0, false, None);
        for y in 0..s.h { for x in 0..s.w {
            if let Some(m) = s.cell(s.x0 + x, s.y0 + y as i64) {
                let i = (y0 + y) * self.g.w + s.x0 + x; self.g.set(i, m); self.g.flags[i] = 0;
            }
            if let Some(wl) = s.wall(s.x0 + x, s.y0 + y as i64) { self.walls.set_at(s.x0 + x, y0 + y, wl); }
        } }
        if let Some((cx, cwy)) = s.chest {
            let cy = (cwy - self.oy) as usize;
            if cy < self.g.h && !self.chests.iter().any(|c| c.x == cx && c.y == cy) {
                self.chests.push(crate::world::Chest { x: cx, y: cy, opened: false, ancient: None });
            }
        }
        self.stamp_one(&s);
        self.sites.push(s);
    }
    fn stamp_one(&mut self, s: &Site) {
        let c = CPM; let y0 = (s.y0 - self.oy) as i64;
        if y0 < 4 || y0 + s.h as i64 >= self.g.h as i64 - 4 { return; }
        let (x0, y0) = (s.x0, y0 as usize);
        match s.kind {
            Kind::Holt => {
                self.try_furn(Item::Nest, x0 + 6 * c, y0 + s.h - 2 * c);
                self.try_furn(Item::Table, x0 + s.w / 2, y0 + s.h - 2 * c);
                self.try_furn(Item::Stool, x0 + s.w / 2 + 4 * S, y0 + s.h - 2 * c);
                self.try_furn(Item::Banner, x0 + s.w / 2, y0 + 3 * c);
                self.try_farm(x0 + s.w - 5 * c, y0 + s.h - 2 * c);
                self.try_torch(x0 + 5 * c, y0 + s.h - 3 * c);
            }
            Kind::Mine => {
                self.try_torch(x0 + 3 * c, y0 + s.h - 3 * c);
                self.try_torch(x0 + s.w - 4 * c, y0 + s.h - 3 * c);
            }
            Kind::Pumpworks => {
                let gx = x0 + 11 * c + c / 2; let gy = y0 + s.h - 3 * c + c / 2;
                self.try_dev(Dev::Floodgate, gx, gy);
                let lx = x0 + 10 * c; let ly = y0 + s.h - 4 * c;
                self.try_dev(Dev::Lever, lx, ly);
                self.try_wire(lx, ly); self.try_wire(gx, gy); self.try_wire((lx + gx) / 2, ly); self.try_wire((lx + gx) / 2, gy);
                if let Some(b) = self.try_block(lx, ly) { self.logic.dev_s[b] = 1; self.logic.invalidate(); }
                self.try_dev(Dev::Pump, x0 + s.w - 5 * c, y0 + s.h - 3 * c);
            }
            Kind::Fissure => {
                let lx = x0 + 3 * c; let top = y0 + 7 * c + c / 2;
                self.try_ladder(lx, top, x0 + 2 * c, top.saturating_sub(1));
            }
            Kind::Halt => {
                let cx = x0 + if s.left { s.w - 6 * c } else { 6 * c }; let cy = y0 + s.h - c;
                self.try_cart(cx as f32 + 0.5, cy as f32);
            }
            Kind::Fossil | Kind::Crystal => { self.try_torch(x0 + s.w / 2, y0 + s.h - 3 * c); }
        }
    }
    fn try_block(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.g.w || y >= self.g.h { return None; }
        Some(self.logic.block_of(x, y))
    }
    fn try_dev(&mut self, d: Dev, x: usize, y: usize) {
        let Some(b) = self.try_block(x, y) else { return };
        if self.logic.dev[b] != 0 { return; }
        if !self.logic.place(&mut self.g, d, b) {
            self.logic.dev[b] = d as u8; self.logic.invalidate();
        }
    }
    fn try_wire(&mut self, x: usize, y: usize) {
        let Some(b) = self.try_block(x, y) else { return };
        let _ = self.logic.place_wire(b);
    }
    fn try_furn(&mut self, it: Item, x: usize, y: usize) {
        if x >= self.g.w || y >= self.g.h { return; }
        if self.furniture.iter().any(|f| f.contains(x, y)) { return; }
        if let Some(fu) = self.furniture_spot(it, x, y) {
            let (x0, y0, x1, y1) = fu.rect();
            if (y0..=y1).any(|cy| (x0..=x1).any(|cx| self.g.is_solidish(cy * self.g.w + cx))) { return; }
            self.furniture.push(fu);
        } else if let Some(f) = it.furn() {
            let (fw, fh) = (f.w * S, f.h * S); if x + fw >= self.g.w || y < fh { return; }
            self.furniture.push(Furniture { item: it, x, y, tint: 0 });
        }
    }
    fn try_farm(&mut self, x: usize, y: usize) {
        if x >= self.g.w || y + 1 >= self.g.h { return; }
        if self.farms.iter().any(|f| f.x.abs_diff(x) < 2 * S && f.y.abs_diff(y) < 2 * S) { return; }
        if !self.g.m(y * self.g.w + x).passable() { return; }
        self.farms.push(Farm { x, y, growth: 0.4, why: 0 });
    }
    fn try_torch(&mut self, x: usize, y: usize) {
        if x >= self.g.w || y >= self.g.h { return; }
        if self.torches.iter().any(|&(tx, ty)| tx.abs_diff(x) < 2 * S && ty.abs_diff(y) < 2 * S) { return; }
        if self.g.is_solidish(y * self.g.w + x) { return; }
        self.torches.push((x, y));
    }
    fn try_ladder(&mut self, x: usize, top: usize, ax: usize, ay: usize) {
        if x >= self.g.w || top >= self.g.h { return; }
        if self.tr.ladders.iter().any(|l| l.x.abs_diff(x) < S && l.top.abs_diff(top) < S) { return; }
        let len = (top..(top + crate::lifts::LADDER_M).min(self.g.h - 1)).take_while(|&r| self.passable(x, r)).count();
        if len < 2 * S { return; }
        self.tr.ladders.push(crate::lifts::Ladder { x, top, len, ax: ax.min(self.g.w - 1), ay: ay.min(self.g.h - 1) });
    }
    fn try_cart(&mut self, x: f32, y: f32) {
        if x < 2. || y < 2. || x >= self.g.w as f32 - 2. || y >= self.g.h as f32 - 2. { return; }
        if self.tr.carts.iter().any(|c| (c.x - x).abs() < crate::transport::CART_W && (c.y - y).abs() < crate::transport::CART_H) { return; }
        self.tr.carts.push(crate::transport::Cart {
            x, y, px: x, py: y, v: 0., vy: 0., dir: 1, on_rail: self.is_rail(x as i32, y as i32),
            cargo: vec![(Item::Stone, 6 + (x as u32) % 5)],
        });
    }
    /// First visit within 8 m: a line in the log. No cards.
    pub(crate) fn step_sites(&mut self) {
        let Some(m) = self.moles.first().filter(|m| m.alive) else { return };
        let (mx, my) = (m.x as i64, self.oy + m.y as i64); let reach = 8 * CPM as i64;
        let near: Vec<(u32, Kind, bool)> = self.sites.iter().filter(|s| !self.sites_seen.contains(&s.id)
            && mx >= s.x0 as i64 - reach && mx < (s.x0 + s.w) as i64 + reach && my >= s.y0 - reach && my < s.y0 + s.h as i64 + reach)
            .map(|s| (s.id, s.kind, s.link.is_some())).collect();
        for (id, kind, link) in near {
            self.sites_seen.insert(id); self.events.push(crate::world::Event::Discover);
            self.say(notice(kind, link).into());
        }
    }
    /// The site whose chest sits at window (x, y), if any.
    pub fn site_at_chest(&self, x: usize, y: usize) -> Option<Kind> {
        let wy = self.oy + y as i64;
        self.sites.iter().find(|s| s.chest == Some((x, wy))).map(|s| s.kind)
    }
}

/// Overlap test used by the rarity/non-overlap tests.
pub fn boxes_overlap(a: (i64, i64, i64, i64), b: (i64, i64, i64, i64)) -> bool {
    a.2 > b.0 && a.0 < b.2 && a.3 > b.1 && a.1 < b.3
}
