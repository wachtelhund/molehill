//! Seeded procedural world: layered strata by depth, x-varying biomes, CA + worm caves,
//! ore veins, aquifers, deep lava, crystal geodes, fungus, roots. Then a stability pass so
//! fresh caves obey the structural model.
use crate::grid::{hash3, Grid, F_WET};
use crate::mat::{Mat, CELL_M, CPM, S, SF};
use crate::stress::Stress;

/// Rows per chest band (32 m).
pub const CHEST_BAND: usize = 32 * CPM;

/// Ore seams by depth (metres below the holt, after the strata wobble): full strength between `lo` and
/// `hi`, fading to nothing over `fade` metres outside. Copper sits just below the holt (tier 2), tin a
/// little deeper, iron in the granite from about 35 m (tier 3), quartz and crystal in the deep basalt.
pub struct Band { pub lo: f32, pub hi: f32, pub fade: f32 }
pub const COPPER: Band = Band { lo: 2., hi: 30., fade: 10. };
pub const TIN: Band = Band { lo: 12., hi: 45., fade: 8. };
pub const IRON: Band = Band { lo: 40., hi: 85., fade: 8. };
pub const QUARTZ: Band = Band { lo: 50., hi: 1e9, fade: 10. };
impl Band { pub fn weight(&self, d: f32) -> f32 { if d < self.lo { (1. - (self.lo - d) / self.fade).max(0.) } else if d > self.hi { (1. - (d - self.hi) / self.fade).max(0.) } else { 1. } } }

/// Grottos: at most one per GROTTO_W × GROTTO_H cell of the world (40 m × 24 m), decided by a hash.
/// The near grotto (see `Gen::find_near_grotto`): its middle lies between NEAR_GROTTO.0 and .1 metres from
/// the burrow and no more than .2 metres above or below it, so digging out sideways finds it in the first
/// stretch of play. Only in windows wide enough for found places (`sites::SITE_MIN_W`).
pub const NEAR_GROTTO: (f32, f32, f32) = (14., 30., 14.);
pub const GROTTO_W: usize = 40 * CPM; pub const GROTTO_H: usize = 24 * CPM;
/// Chance that a grotto cell holds a grotto, by the depth (m) of its middle. Grottos follow the water:
/// commonest in the wet upper layers, thinning out with depth, none where the lava takes over.
pub fn grotto_chance(d: f32) -> f32 {
    if d < -20. { 0.2 } else if d < 10. { 0.2 - (d + 20.) / 30. * 0.1 } else if d < 50. { 0.1 - (d - 10.) / 40. * 0.08 } else if d < 60. { 0.02 } else { 0. }
}
/// Shell round the hollow: limestone where the host rock is too weak to hold a dome.
/// The starting burrow's footprint around (window middle, depth 0), in cells: half its width, how far it
/// reaches up (dome and roof) and down (floor and the dig-out skin). Grottos keep BURROW_MARGIN clear of it.
const BURROW_HALF_W: i64 = 6 * CPM as i64; const BURROW_TOP: i64 = 5 * CPM as i64; const BURROW_BOTTOM: i64 = 3 * CPM as i64;
const BURROW_MARGIN: i64 = 2 * CPM as i64;
/// The start zone: within START_SAFE m of the starting burrow (the window's middle column, depth 0) every
/// cell can be dug with the Survival Pickaxe (tier 1), so a new holt always has a way out and can reach the
/// copper just below it. Harder ground (granite, basalt, iron, quartz, crystal) starts beyond it, and its
/// fluid pockets are water, never lava.
pub const START_SAFE: f32 = 25.;
pub fn in_start_zone(w: usize, x: usize, wy: i64) -> bool { near_start(w, x, wy, START_SAFE) }
/// Within `r` metres of the starting burrow?
pub fn near_start(w: usize, x: usize, wy: i64, r: f32) -> bool {
    let (dx, dy) = ((x as f32 - (w / 2) as f32) / CPM as f32, wy as f32 / CPM as f32); dx * dx + dy * dy < r * r
}
/// A pickaxe-diggable stand-in for harder ground (strong stone stays strong: granite becomes limestone).
pub fn soften(m: Mat) -> Mat {
    if m == Mat::Bedrock || m.p().tier <= 1 { return m; }
    match m { Mat::Basalt => Mat::Rock, Mat::IronOre => Mat::TinOre, _ if m.kind() == crate::mat::Kind::Solid => Mat::Limestone, _ => m }
}
pub const GROTTO_SHELL: usize = 3 * CPM / 2;
/// Height (cells) a root-tree reaches from its root row `root_wy`, per 1 m column block: 2-3.5 m.
/// Regrowth uses the same rule, so a cut tree grows back to the height it had.
pub fn tree_height(x: usize, root_wy: i64, seed: u32) -> usize { let h = hash3((x / CPM) as u32, root_wy as u32, seed ^ 0x7EE5); 2 * CPM + h as usize % (3 * CPM / 2 + 1) }
/// One grotto: a domed hollow with a flat floor of moss and mycelium, a seep pool sunk into the floor,
/// and 1-3 root-trees. `cx` is a window column; `cy` and the floor are absolute rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grotto { pub id: u32, pub cx: usize, pub cy: i64, pub rx: usize, pub ry: usize,
    /// floor row offset below the centre (cells)
    pub floor: i64,
    /// pool centre (dx from cx) and half width (cells)
    pub pool: (i64, i64),
    /// trunk block starts (window columns; the trunk is cells [x + 2, x + 6)) and heights
    pub trees: [(usize, usize); 3], pub ntrees: usize }
/// What a cell is in a grotto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part { Hollow, Pool, Trunk, Roots, Floor, Wall, Shell }
impl Grotto {
    pub fn floor_row(&self) -> i64 { self.cy + self.floor }
    fn in_hollow(&self, dx: i64, dy: i64) -> bool { (dx as f32 / self.rx as f32).powi(2) + (dy as f32 / self.ry as f32).powi(2) < 1. && dy < self.floor }
    /// The cell (x, wy)'s part of this grotto, or None outside its shell.
    pub fn part(&self, x: usize, wy: i64) -> Option<Part> {
        let (dx, dy) = (x as i64 - self.cx as i64, wy - self.cy);
        let t = GROTTO_SHELL as f32;
        if (dx as f32 / (self.rx as f32 + t)).powi(2) + (dy as f32 / (self.ry as f32 + t)).powi(2) >= 1. { return None; }
        let (f, tile, xi) = (self.floor, S as i64, x as i64);
        for &(bx, th) in &self.trees[..self.ntrees] { let t0 = bx as i64 + 2;
            if dy < f && dy >= f - th as i64 && xi >= t0 && xi < t0 + 4 { return Some(Part::Trunk); }
            // roots: under the trunk exactly at the top (so regrowth keeps it 4 cells wide), spreading deeper
            let sp = (dy - f) / 2; if dy >= f && dy < f + CPM as i64 && xi >= t0 - sp && xi < t0 + 4 + sp { return Some(Part::Roots); }
        }
        let (px, pw) = self.pool;
        if dy >= f && dy < f + (CPM / 2) as i64 && (dx - px).abs() < pw - (dy - f) { return Some(Part::Pool); }
        if self.in_hollow(dx, dy) { return Some(Part::Hollow); }
        if dy >= f && dy < f + tile && self.in_hollow(dx, f - 1) { return Some(Part::Floor); }
        if dy > 0 && dy < f && (self.in_hollow(dx - tile, dy) || self.in_hollow(dx + tile, dy)) { return Some(Part::Wall); }
        Some(Part::Shell)
    }
    /// Is (x, wy) open air or water of this grotto (stabilisation must not fill it)?
    pub fn is_void(&self, x: usize, wy: i64) -> bool { matches!(self.part(x, wy), Some(Part::Hollow | Part::Pool)) }
}
impl Gen {
    /// The grotto in grotto cell (gx, gy), if it has one.
    pub fn grotto_at(&self, gx: i64, gy: i64) -> Option<Grotto> {
        let w = self.biome.len(); if gx < 0 || (gx as usize + 1) * GROTTO_W > w { return None; }
        if let Some((ngx, ngy, g)) = self.near_grotto { if (ngx, ngy) == (gx, gy) { return Some(g); } }
        let h = hash3(gx as u32, gy as u32, self.seed ^ 0x6A07);
        let mid = gy * GROTTO_H as i64 + GROTTO_H as i64 / 2;
        if (h & 0xFFFF) as f32 / 65536. >= grotto_chance(mid as f32 * CELL_M) { return None; }
        self.grotto_shape(gx, gy, h, 1)
    }
    /// The one grotto every wide holt gets within a fair walk of the burrow (see NEAR_GROTTO), unless the seed
    /// already grew one with two or more trees that near: the first of a seeded list of tries, in empty grotto
    /// cells beside and around the start, whose shape lands at the right distance with at least two root-trees. Wood stays scarce (a grotto's trees give a handful), but a new
    /// Delver never has to hunt far for the first of it.
    fn find_near_grotto(&self) -> Option<(i64, i64, Grotto)> {
        let w = self.biome.len(); if w < crate::sites::SITE_MIN_W { return None; }
        let sgx = ((w / 2) / GROTTO_W) as i64; let c = CPM as f32;
        let at = |g: &Grotto| { let (dx, dy) = ((g.cx as f32 - (w / 2) as f32) / c, g.cy as f32 / c); ((dx * dx + dy * dy).sqrt(), dy.abs()) };
        // nature first: a grotto the seed already put in reach (two trees or more) is the near grotto
        for gy in -2..=1 { for gx in sgx - 1..=sgx + 1 {
            if let Some(g) = self.grotto_at(gx, gy) { let (d, _) = at(&g); if g.ntrees >= 2 && d <= NEAR_GROTTO.1 { return None; } }
        } }
        for k in 0..48u32 {
            let h = hash3(k, 0x5EED, self.seed ^ 0x6A07);
            let (gx, gy) = (sgx + (h % 3) as i64 - 1, -((h >> 2) as i64 & 1));
            if gx < 0 || (gx as usize + 1) * GROTTO_W > w { continue; }
            if self.grotto_at(gx, gy).is_some() { continue; } // never replace a grotto the seed grew
            let Some(g) = self.grotto_shape(gx, gy, h, 2) else { continue };
            let (d, ady) = at(&g);
            if g.ntrees >= 2 && d >= NEAR_GROTTO.0 && d <= NEAR_GROTTO.1 && ady <= NEAR_GROTTO.2 { return Some((gx, gy, g)); }
        }
        None
    }
    /// A grotto's shape in cell (gx, gy) from hash `h`, wanting at least `min_trees` trees; None if it would
    /// break into the starting burrow.
    fn grotto_shape(&self, gx: i64, gy: i64, h: u32, min_trees: usize) -> Option<Grotto> {
        let w = self.biome.len();
        let (gw, gh) = (GROTTO_W as i64, GROTTO_H as i64);
        let c = CPM as i64;
        let r = |k: u32, n: i64| (hash3(h, k, self.seed) as i64).rem_euclid(n.max(1));
        let (rx, ry) = (5 * c + r(1, 3 * c + 1), 3 * c + r(2, 3 * c / 2 + 1));
        let m = GROTTO_SHELL as i64 + c; // room for the shell inside the grotto cell
        let cx = gx * gw + rx + m + r(3, gw - 2 * (rx + m) + 1);
        let cy = gy * gh + ry + m + r(4, gh - 2 * (ry + m) + 1);
        // never into the starting burrow (depth 0, the window's middle): the grotto's box, shell included,
        // stays BURROW_MARGIN clear of the burrow's footprint (its dome, walls and floor), so one can open
        // up within sight of the holt but never breaks into it
        let sh = GROTTO_SHELL as i64; let (bx, by) = (BURROW_HALF_W + BURROW_MARGIN, (BURROW_TOP + BURROW_MARGIN, BURROW_BOTTOM + BURROW_MARGIN));
        if (cx - (w / 2) as i64).abs() < rx + sh + bx && cy + ry + sh > -by.0 && cy - ry - sh < by.1 { return None; }
        let floor = ry / 2;
        let pw = 3 * c / 2; let px = r(5, rx) - rx / 2;
        let mut trees = [(0usize, 0usize); 3]; let mut n = 0;
        let want = (1 + r(6, 3) as usize).max(min_trees);
        for k in 0..3i64 { if n >= want { break; }
            let dx = (k - 1) * rx * 11 / 20 + r(7 + k as u32, c) - c / 2;
            if (dx - px).abs() < pw + c { continue; }
            let bx = ((cx + dx) / c * c) as usize;
            trees[n] = (bx, tree_height(bx, cy + floor, self.seed)); n += 1; }
        Some(Grotto { id: h, cx: cx as usize, cy, rx: rx as usize, ry: ry as usize, floor, pool: (px, pw), trees, ntrees: n })
    }
    /// The grotto whose cell holds (x, wy), if any.
    pub fn grotto_of(&self, x: usize, wy: i64) -> Option<Grotto> { self.grotto_at((x / GROTTO_W) as i64, wy.div_euclid(GROTTO_H as i64)) }
    /// Every grotto touching rows [wy0, wy1).
    pub fn grottos_in_rows(&self, wy0: i64, wy1: i64) -> Vec<Grotto> {
        let gh = GROTTO_H as i64; let mut v = vec![];
        for gy in wy0.div_euclid(gh)..=(wy1 - 1).div_euclid(gh) { for gx in 0..(self.biome.len() / GROTTO_W) as i64 { if let Some(g) = self.grotto_at(gx, gy) { v.push(g); } } }
        v
    }
    /// Open air or water of a grotto (kept open by stabilisation).
    pub fn in_grotto_void(&self, x: usize, wy: i64) -> bool { self.grotto_of(x, wy).map_or(false, |g| g.is_void(x, wy)) || self.in_ruin_void(x, wy) || self.in_site_void(x, wy) } // (and a ruin's halls, and a site's carved rooms)
}

pub struct Noise { seed: u32 }
impl Noise {
    pub fn new(seed: u32) -> Self { Noise { seed } }
    fn v(&self, x: i32, y: i32) -> f32 { (hash3(x as u32, y as u32, self.seed) & 0xFFFF) as f32 / 65535. }
    pub fn at(&self, x: f32, y: f32) -> f32 {
        let (x0, y0) = (x.floor(), y.floor()); let (tx, ty) = (x - x0, y - y0);
        let (sx, sy) = (tx * tx * (3. - 2. * tx), ty * ty * (3. - 2. * ty)); let (xi, yi) = (x0 as i32, y0 as i32);
        let a = self.v(xi, yi) + (self.v(xi + 1, yi) - self.v(xi, yi)) * sx;
        let b = self.v(xi, yi + 1) + (self.v(xi + 1, yi + 1) - self.v(xi, yi + 1)) * sx;
        a + (b - a) * sy
    }
    pub fn fbm(&self, x: f32, y: f32, scale: f32, oct: u32) -> f32 {
        let (mut s, mut amp, mut f, mut norm) = (0., 1., 1. / scale, 0.);
        for o in 0..oct { s += amp * Noise { seed: self.seed.wrapping_add(o * 977) }.at(x * f, y * f); norm += amp; amp *= 0.5; f *= 2.; }
        s / norm
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Biome { Loam, Karst, Crags, Ashlands }
pub const BIOME_NAMES: [&str; 4] = ["Loamlands", "Karst caverns", "Granite crags", "Ashlands"];

pub struct GenInfo { pub surface: Vec<usize>, pub biome: Vec<Biome>, pub chests: Vec<(usize, usize)>, pub start: (usize, usize), pub seed: u32 }

pub fn biome_at(n: &Noise, x: usize) -> Biome {
    let v = n.fbm(x as f32 / SF, 0.5, 260., 2);
    if v < 0.36 { Biome::Loam } else if v < 0.52 { Biome::Karst } else if v < 0.66 { Biome::Crags } else { Biome::Ashlands }
}

/// Rows are generated on demand from an absolute row index `wy` (0 = the starting layer, + is
/// deeper). Pure per-cell function of (x, wy, seed): the same seed always yields the same Deepholt.
pub struct Gen { pub seed: u32, n0: Noise, n1: Noise, n2: Noise, n3: Noise, pub biome: Vec<Biome>,
    /// the grotto placed near the start (its grotto cell and shape), fixed by the seed
    pub near_grotto: Option<(i64, i64, Grotto)>,
    /// the first-hour found place placed in reach of the start (its site cell and site), fixed by the seed
    pub near_site: Option<(i64, i64, crate::sites::Site)> }
impl Gen {
    pub fn new(seed: u32, w: usize) -> Gen {
        let n3 = Noise::new(seed ^ 0x3C3C);
        let biome = (0..w).map(|x| biome_at(&n3, x)).collect();
        let mut g = Gen { seed, n0: Noise::new(seed), n1: Noise::new(seed ^ 0x51ED), n2: Noise::new(seed ^ 0xA77A), n3, biome, near_grotto: None, near_site: None };
        g.near_grotto = g.find_near_grotto(); g.near_site = g.find_near_site(); g
    }
    /// Open cave cell? Blobby caverns + thin winding tunnels.
    pub fn open(&self, x: usize, wy: i64) -> bool {
        let (fx, fy) = (x as f32 / SF, wy as f32 / SF); // noise runs in legacy 0.25 m tiles: features keep their size in metres
        let band = match self.biome[x] { Biome::Karst => 0.04, Biome::Loam => 0.0, Biome::Crags => -0.03, Biome::Ashlands => 0.01 };
        let c = self.n3.fbm(fx, fy * 1.6, 46., 3) + band;
        let t = (self.n1.fbm(fx + 5000., fy * 1.3, 110., 2) - 0.5).abs();
        c > 0.66 || t < 0.012
    }
    pub fn cell(&self, x: usize, wy: i64, w: usize) -> (u8, u8) { self.cell_with(x, wy, w, &|x, wy| self.open(x, wy)) }
    fn cell_with(&self, x: usize, wy: i64, w: usize, open: &dyn Fn(usize, i64) -> bool) -> (u8, u8) {
        if x < 2 * S || x >= w - 2 * S { return (Mat::Bedrock as u8, 0); }
        let (fx, fy) = (x as f32 / SF, wy as f32 / SF); let d = fy * 0.25; // fx, fy in legacy 0.25 m tiles; d = metres below the starting layer
        // ruins of the Ancients are stamped in whole (see ruins.rs); then grottos are carved: their hollow,
        // pool, trees and floor don't depend on the rock around
        if let Some(c) = self.ruin_cell(x, wy) { return c; }
        if let Some(c) = self.site_cell(x, wy) { return c; }
        let part = self.grotto_of(x, wy).and_then(|g| g.part(x, wy));
        let glow = || self.n0.at(fx / 6., fy / 6.) > 0.58;
        match part {
            Some(Part::Hollow) => return (0, 0),
            Some(Part::Pool) => return (Mat::Water as u8, 0),
            Some(Part::Trunk) => return (Mat::RootTree as u8, 0),
            Some(Part::Roots) => return (Mat::TreeRoot as u8, F_WET),
            Some(Part::Floor) => return (if glow() { Mat::Mycelium } else { Mat::Moss } as u8, F_WET),
            Some(Part::Wall) if glow() => return (Mat::Mycelium as u8, 0),
            _ => {}
        }
        let dd = d + (self.n1.fbm(fx, fy, 60., 3) - 0.5) * 10. + match self.biome[x] { Biome::Loam => -4., Biome::Karst => 2., Biome::Crags => 8., Biome::Ashlands => 4. };
        if part.is_none() && open(x, wy) {
            // fluid pockets: wetter up, hotter down
            let pk = self.n2.fbm(fx + 7000., fy * 1.4, 28., 2);
            if pk > 0.63 {
                let lava_p = ((dd - 10.) / 60.).clamp(0., 0.92); let water_p = ((40. - dd) / 60.).clamp(0.08, 1.);
                let r = (hash3((x / (24 * S)) as u32, (wy.div_euclid(24 * S as i64)) as u32, self.seed ^ 0xF1) & 1023) as f32 / 1024.;
                if r < lava_p { return (Mat::Lava as u8, 0); }
                if r < lava_p + water_p * (1. - lava_p) { return (Mat::Water as u8, 0); }
            }
            return (0, 0);
        }
        let (n1, n2) = (&self.n1, &self.n2); let mut fl = 0u8;
        let mut m = if dd < -30. { if n2.fbm(fx, fy * 2., 40., 2) > 0.55 { Mat::Clay } else { Mat::Dirt } }
            else if dd < 0. { if n2.fbm(fx, fy * 2., 40., 2) > 0.5 { Mat::Clay } else if n2.fbm(fx + 90., fy, 70., 2) > 0.62 { Mat::Limestone } else { Mat::Dirt } }
            else if dd < 30. { if self.biome[x] == Biome::Crags && dd >= 15. { Mat::Granite } else if n2.fbm(fx + 300., fy, 70., 3) > 0.55 { Mat::Rock } else { Mat::Limestone } }
            else if dd < 70. { if n2.fbm(fx + 300., fy, 70., 3) > 0.5 { Mat::Granite } else { Mat::Rock } }
            else if self.biome[x] == Biome::Ashlands || n2.at(fx / 50., fy / 50.) > 0.3 { Mat::Basalt } else { Mat::Granite }; // the deep is mostly basalt
        if dd < 35. { let l = n2.fbm(fx, fy * 3., 50., 3); if l > 0.67 { m = if n1.at(fx / 30., fy / 30.) > 0.5 { Mat::Gravel } else { Mat::Sand }; if l > 0.71 { fl |= F_WET; } } }
        let vein = |nn: &Noise, sc: f32, sal: f32| (nn.fbm(fx + sal, fy * 1.3, sc, 3) - 0.5).abs();
        if m != Mat::Sand && m != Mat::Gravel {
            if dd > -20. && dd < 9. && vein(&self.n1, 70., 11.) < 0.018 { m = Mat::Coal; } // coal never forms where lava runs
            if dd > -60. && dd < 9. && vein(&self.n0, 30., 900.) < 0.008 { m = Mat::Coal; }
            // seams by depth band (thinner toward a band's edges): copper, tin, iron, quartz
            if vein(&self.n2, 45., 77.) < 0.014 * COPPER.weight(dd) { m = Mat::CopperOre; }
            if vein(&self.n3, 40., 501.) < 0.009 * TIN.weight(dd) { m = Mat::TinOre; }
            if vein(&self.n0, 55., 1234.) < 0.013 * IRON.weight(dd) { m = Mat::IronOre; }
            if vein(&self.n1, 25., 4321.) < 0.006 * QUARTZ.weight(dd) { m = Mat::Quartz; }
        }
        if part.is_some() { // a grotto's shell (and the walls without mycelium): a dome strong enough to stand
            if m.p().tensile < Mat::Limestone.p().tensile || m.kind() != crate::mat::Kind::Solid { m = Mat::Limestone; }
            return (m as u8, fl & !F_WET);
        }
        // decoration on cave walls
        let (k, ki) = (S, S as i64); // a decoration skin one legacy tile thick
        let edge = open(x, wy - ki) || open(x, wy + ki) || open(x - k, wy) || open(x + k, wy);
        if edge && m != Mat::Sand {
            let r = self.n0.at(fx / 9., fy / 9.);
            if dd > 45. && r > 0.80 { m = Mat::Crystal; } else if dd > 35. && r < 0.10 { m = Mat::Quartz; }
            else if open(x, wy - ki) && dd < 9. && n1.at(fx / 14., fy / 14.) > 0.68 { m = Mat::Fungus; }
            else if dd < -10. && n2.at(fx / 7., fy / 7.) > 0.7 { m = Mat::Roots; }
        }
        (m as u8, fl)
    }
    /// Fill window rows [y0, y1) with generated content; `oy` = absolute row of window row 0.
    pub fn fill_rows(&self, g: &mut Grid, oy: i64, y0: usize, y1: usize) {
        let w = g.w;
        let rows = y1 - y0 + 2 * S; let mut op = vec![false; rows * w];
        for r in 0..rows { for x in 2 * S..w - 2 * S { op[r * w + x] = self.open(x, oy + y0 as i64 - S as i64 + r as i64); } }
        let base = oy + y0 as i64 - S as i64;
        let open = |x: usize, wy: i64| { let r = wy - base; if r >= 0 && (r as usize) < rows { op[r as usize * w + x] } else { self.open(x, wy) } };
        for y in y0..y1 { for x in 0..w { let (m, f) = self.cell_with(x, oy + y as i64, w, &open); let i = y * w + x; g.mat[i] = m; g.flags[i] = f; } }
        // lava pockets have long since baked their surroundings: nothing burnable within reach of lava
        let r = 4 * S;
        for y in y0..y1 { for x in 2 * S..w - 2 * S { if g.mat[y * w + x] != Mat::Lava as u8 { continue; }
            for yy in y.saturating_sub(r).max(y0)..(y + r + 1).min(y1) { for xx in x.saturating_sub(r)..(x + r + 1).min(w - 2 * S) {
                let j = yy * w + xx; if Mat::from_u8(g.mat[j]).p().ignite > 0 { g.mat[j] = Mat::Basalt as u8; }
            } }
        } }
        // the start zone is all pickaxe ground (see `in_start_zone`), whatever the bands or the lava put there;
        // and it's cool: a fluid pocket in it is water, not lava (which would crust into basalt)
        for y in y0..y1 { for x in 0..w { let i = y * w + x; let wy = oy + y as i64;
            if g.mat[i] == Mat::Lava as u8 && near_start(w, x, wy, START_SAFE + 10.) { g.mat[i] = Mat::Water as u8; } // (10 m spare: no lava can run in while the world settles)
            else if in_start_zone(w, x, wy) { g.mat[i] = soften(Mat::from_u8(g.mat[i])) as u8; } } }
        // unsupported sand at gen time turns to sandstone
        for y in (y0..y1).rev() { if y + 1 >= g.h { continue; } for x in 1..w - 1 {
            let i = y * w + x; if g.mat[i] != Mat::Sand as u8 { continue; }
            let ok = |j: usize| { let m = Mat::from_u8(g.mat[j]); m.kind() == crate::mat::Kind::Solid || m == Mat::Sand };
            if !(ok(i + w) && ok(i + w - 1) && ok(i + w + 1)) { g.mat[i] = Mat::Rock as u8; }
        } }
    }
    /// Chest sites in a CHEST_BAND-row (32 m) band (deterministic): (x, absolute row).
    pub fn chests_in_band(&self, band: i64, w: usize) -> Vec<(usize, i64)> {
        let mut v = vec![]; let bh = CHEST_BAND as i64;
        for k in 0..6u32 {
            let x = 8 * S + hash3(k, band as u32, self.seed ^ 0xC4) as usize % (w - 16 * S);
            for dy in 0..bh { let wy = band * bh + dy; if self.open(x, wy) && !self.open(x, wy + 1) && self.cell(x, wy, w).0 == 0 { v.push((x, wy)); break; } }
        }
        v
    }
}

/// Starting window: rows centred on the starting layer, a dry burrow carved in, then stabilised.
/// The generator and world info for a seed without filling any rows (a load overwrites the window anyway).
pub fn info_only(seed: u32, w: usize, h: usize) -> (Gen, GenInfo) {
    let gen = Gen::new(seed, w); let biome = gen.biome.clone();
    (gen, GenInfo { surface: vec![0; w], biome, chests: vec![], start: (w / 2 / CPM, h / 2 / CPM), seed })
}
pub fn generate(g: &mut Grid, seed: u32, oy: i64) -> (Gen, GenInfo) {
    let (w, h) = (g.w, g.h);
    let gen = Gen::new(seed, w);
    gen.fill_rows(g, oy, 0, h);
    let sx = w / 2; let cy1 = (-oy) as usize; // window row of the starting layer
    let (cx0, cx1, cy0) = (sx - 15 * S, sx + 15 * S, cy1 - 12 * S);
    for y in cy0 - 6 * S..cy1 + 6 * S { for x in cx0 - 8 * S..=cx1 + 8 * S { let i = y * w + x; if g.mat[i] == Mat::Water as u8 || g.mat[i] == Mat::Lava as u8 { g.mat[i] = Mat::Clay as u8; } } }
    // a domed burrow: the roof steps down toward the walls so no roof run spans more than a few cells
    let top = |x: usize| { let t = (x as f32 - sx as f32) / (15. * SF); cy0 + (12. * SF * t * t).round() as usize };
    for x in cx0 - 3 * S..=cx1 + 3 * S { let t = top(x.clamp(cx0, cx1)); for y in t.saturating_sub(5 * S)..t { g.mat[y * w + x] = Mat::Limestone as u8; } }
    for x in cx0..=cx1 { for y in top(x)..=cy1 { g.mat[y * w + x] = 0; } }
    // side walls tie the dome down to the floor (otherwise an open cavern beside it leaves the roof floating)
    for x in (cx0 - 3 * S..cx0).chain(cx1 + 1..=cx1 + 3 * S) { for y in top(x.clamp(cx0, cx1))..=cy1 { let i = y * w + x; if Mat::from_u8(g.mat[i]).kind() != crate::mat::Kind::Solid { g.mat[i] = Mat::Limestone as u8; } } }
    for y in cy1 + 1..cy1 + 4 * S { for x in cx0 - 4 * S..=cx1 + 4 * S { g.mat[y * w + x] = Mat::Clay as u8; } }
    // ...and the walls stand on rock: if a cavern lies under the burrow, drop granite legs down to the next solid ground
    let mut st = Stress::new(w * h); st.solve(g);
    for x in (cx0 - 4 * S..cx0).chain(cx1 + 1..=cx1 + 4 * S) { let mut y = cy1 + 4 * S; while y < h - 1 && !st.reach[y * w + x] { g.mat[y * w + x] = Mat::Limestone as u8; g.flags[y * w + x] = 0; y += 1; } } // diggable with claws
    let keep = |x: usize, y: usize| (x >= cx0 && x <= cx1 && y >= cy0 && y <= cy1) || gen.in_grotto_void(x, oy + y as i64);
    for y in cy0 - 6 * S..cy1 + 4 * S { for x in cx0 - 4 * S..=cx1 + 4 * S { g.flags[y * w + x] = 0; } } // built cells are fresh rock, not rubble
    let soft = |x: usize, y: usize| in_start_zone(w, x, oy + y as i64);
    stabilize_soft(g, &mut st, 0, usize::MAX, &keep, &soft);
    g.wake_all(); for _ in 0..240 { g.step_ca(); }
    // nothing the settling pass let flow in stays inside the burrow
    for y in cy0 - 8 * S..=cy1 { for x in cx0 - 3 * S..=cx1 + 3 * S { let i = y * w + x; if matches!(Mat::from_u8(g.mat[i]), Mat::Water | Mat::Lava | Mat::Steam) { g.mat[i] = if x >= cx0 && x <= cx1 && y >= cy0 && y <= cy1 { 0 } else { Mat::Clay as u8 }; } } }
    stabilize_soft(g, &mut st, 0, usize::MAX, &keep, &soft);
    // the burrow floor stays soil (farms need it), even where the stability pass reinforced it
    for x in cx0..=cx1 { for y in cy1 + 1..=cy1 + S { let i = y * w + x; if matches!(Mat::from_u8(g.mat[i]), Mat::Granite | Mat::Limestone) { g.mat[i] = Mat::Clay as u8; } } }
    // claws can always dig down out of the burrow: no granite just below its floor
    for y in cy1 + 1 + S..cy1 + 9 * S { for x in cx0 - 4 * S..=cx1 + 4 * S { let i = y * w + x; if g.mat[i] == Mat::Granite as u8 { g.mat[i] = Mat::Limestone as u8; } } }
    g.struct_dirty = Some((0, 0, w - 1, h - 1)); g.wake_all();
    let biome = gen.biome.clone();
    (gen, GenInfo { surface: vec![0; w], biome, chests: vec![], start: (sx / CPM, cy1 / CPM), seed })
}

pub fn stabilize(g: &mut Grid, st: &mut Stress) { stabilize_rows(g, st, 0, usize::MAX) }
/// Like `stabilize`, but only touches failing cells in rows [y0, y1).
pub fn stabilize_rows(g: &mut Grid, st: &mut Stress, y0: usize, y1: usize) { stabilize_keep(g, st, y0, y1, &|_, _| false) }
/// `kept(x, y)`: open space that must stay open (the starting burrow, grotto hollows and pools).
pub fn stabilize_keep(g: &mut Grid, st: &mut Stress, y0: usize, y1: usize, kept: &dyn Fn(usize, usize) -> bool) { stabilize_soft(g, st, y0, y1, kept, &|_, _| false) }
/// `stabilize_keep`, patching with limestone instead of granite where `soft(x, y)` (the start zone).
pub fn stabilize_soft(g: &mut Grid, st: &mut Stress, y0: usize, y1: usize, kept: &dyn Fn(usize, usize) -> bool, soft: &dyn Fn(usize, usize) -> bool) {
    let (w, h) = (g.w, g.h);
    let thr0 = st.thr; st.thr = 0.45; // gen-time safety factor: fresh caves must survive fluids shifting about
    // after one full solve, each round re-solves only around the cells the last round changed (every
    // failing cell is changed, so nothing elsewhere can be failing): exact, and far cheaper on a big window
    let mut changed: Vec<usize> = vec![];
    for it in 0..100 {
        if it == 40 { st.thr = 1.0; } // out of patience with the margin: just make sure nothing actually fails
        // the first round is a full solve, or with a solver that already knows the rest of the window
        // (a slide into fresh rows) just those rows
        let all = if it == 0 { if st.reach.len() == w * h && y1 <= h { st.solve_rect(g, Some((0, y0, w - 1, y1 - 1))) } else { st.solve(g) } } else { st.solve_cells(g, &changed) };
        let fail: Vec<u32> = all.into_iter().filter(|&i| (i as usize / w) >= y0 && (i as usize / w) < y1).collect();
        if fail.is_empty() { break; }
        changed.clear();
        for &i in &fail {
            let i = i as usize; let m = Mat::from_u8(g.mat[i]); changed.push(i); // (keep protects the burrow's air, not its roof)
            if !st.reach[i] { g.mat[i] = 0; continue; } // floating island: no load path
            if it >= 4 { // still failing: widen the pillar / thicken the roof by filling neighbouring voids,
                // further out the longer it keeps failing (a one-cell fill barely helps at 8 cells/m)
                let (x, y) = (i % w, i / w); let r = (1 + (it as i32 - 4) / 3).min(4 * S as i32);
                for (dx, dy) in (-r..=r).flat_map(|dx| (-r..=r + 1).map(move |dy| (dx, dy))) { let (nx, ny) = (x as i32 + dx, y as i32 + dy); if nx > 1 && ny >= 0 && (nx as usize) < w - 2 && (ny as usize) < h { let j = ny as usize * w + nx as usize; if kept(nx as usize, ny as usize) { continue; } if g.mat[j] == 0 || g.mat[j] == Mat::Water as u8 || g.mat[j] == Mat::Lava as u8 { g.mat[j] = if soft(nx as usize, ny as usize) { Mat::Limestone } else { Mat::Granite } as u8; changed.push(j); } } }
            }
            g.mat[i] = if (it < 3 && m.p().tensile < Mat::Limestone.p().tensile) || soft(i % w, i / w) { Mat::Limestone as u8 } else { Mat::Granite as u8 };
        }
    }
    st.thr = thr0;
}
