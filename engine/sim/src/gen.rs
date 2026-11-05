//! Seeded procedural world: layered strata by depth, x-varying biomes, CA + worm caves,
//! ore veins, aquifers, deep lava, crystal geodes, fungus, roots. Then a stability pass so
//! fresh caves obey the structural model.
use crate::grid::{hash3, Grid, F_WET};
use crate::mat::Mat;
use crate::stress::Stress;

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
    let v = n.fbm(x as f32, 0.5, 260., 2);
    if v < 0.36 { Biome::Loam } else if v < 0.52 { Biome::Karst } else if v < 0.66 { Biome::Crags } else { Biome::Ashlands }
}

/// Rows are generated on demand from an absolute row index `wy` (0 = the starting layer, + is
/// deeper). Pure per-cell function of (x, wy, seed): the same seed always yields the same Deepholt.
pub struct Gen { pub seed: u32, n0: Noise, n1: Noise, n2: Noise, n3: Noise, pub biome: Vec<Biome> }
impl Gen {
    pub fn new(seed: u32, w: usize) -> Gen {
        let n3 = Noise::new(seed ^ 0x3C3C);
        let biome = (0..w).map(|x| biome_at(&n3, x)).collect();
        Gen { seed, n0: Noise::new(seed), n1: Noise::new(seed ^ 0x51ED), n2: Noise::new(seed ^ 0xA77A), n3, biome }
    }
    /// Open cave cell? Blobby caverns + thin winding tunnels.
    pub fn open(&self, x: usize, wy: i64) -> bool {
        let (fx, fy) = (x as f32, wy as f32);
        let band = match self.biome[x] { Biome::Karst => 0.04, Biome::Loam => 0.0, Biome::Crags => -0.03, Biome::Ashlands => 0.01 };
        let c = self.n3.fbm(fx, fy * 1.6, 46., 3) + band;
        let t = (self.n1.fbm(fx + 5000., fy * 1.3, 110., 2) - 0.5).abs();
        c > 0.66 || t < 0.012
    }
    pub fn cell(&self, x: usize, wy: i64, w: usize) -> (u8, u8) { self.cell_with(x, wy, w, &|x, wy| self.open(x, wy)) }
    fn cell_with(&self, x: usize, wy: i64, w: usize, open: &dyn Fn(usize, i64) -> bool) -> (u8, u8) {
        if x < 2 || x >= w - 2 { return (Mat::Bedrock as u8, 0); }
        let (fx, fy) = (x as f32, wy as f32); let d = fy * 0.25; // metres below the starting layer
        let dd = d + (self.n1.fbm(fx, fy, 60., 3) - 0.5) * 10. + match self.biome[x] { Biome::Loam => -4., Biome::Karst => 2., Biome::Crags => 8., Biome::Ashlands => 4. };
        if open(x, wy) {
            // fluid pockets: wetter up, hotter down
            let pk = self.n2.fbm(fx + 7000., fy * 1.4, 28., 2);
            if pk > 0.63 {
                let lava_p = ((dd - 10.) / 60.).clamp(0., 0.92); let water_p = ((40. - dd) / 60.).clamp(0.08, 1.);
                let r = (hash3((x / 24) as u32, (wy.div_euclid(24)) as u32, self.seed ^ 0xF1) & 1023) as f32 / 1024.;
                if r < lava_p { return (Mat::Lava as u8, 0); }
                if r < lava_p + water_p * (1. - lava_p) { return (Mat::Water as u8, 0); }
            }
            return (0, 0);
        }
        let (n1, n2) = (&self.n1, &self.n2); let mut fl = 0u8;
        let mut m = if dd < -30. { if n2.fbm(fx, fy * 2., 40., 2) > 0.55 { Mat::Clay } else { Mat::Dirt } }
            else if dd < 0. { if n2.fbm(fx, fy * 2., 40., 2) > 0.5 { Mat::Clay } else if n2.fbm(fx + 90., fy, 70., 2) > 0.62 { Mat::Limestone } else { Mat::Dirt } }
            else if dd < 30. { if self.biome[x] == Biome::Crags { Mat::Granite } else if n2.fbm(fx + 300., fy, 70., 3) > 0.55 { Mat::Rock } else { Mat::Limestone } }
            else if dd < 70. { if n2.fbm(fx + 300., fy, 70., 3) > 0.5 { Mat::Granite } else { Mat::Rock } }
            else if self.biome[x] == Biome::Ashlands || n2.at(fx / 50., fy / 50.) > 0.4 { Mat::Basalt } else { Mat::Granite };
        if dd < 35. { let l = n2.fbm(fx, fy * 3., 50., 3); if l > 0.67 { m = if n1.at(fx / 30., fy / 30.) > 0.5 { Mat::Gravel } else { Mat::Sand }; if l > 0.71 { fl |= F_WET; } } }
        let vein = |nn: &Noise, sc: f32, sal: f32| (nn.fbm(fx + sal, fy * 1.3, sc, 3) - 0.5).abs();
        if m != Mat::Sand && m != Mat::Gravel {
            if dd > -20. && dd < 9. && vein(&self.n1, 70., 11.) < 0.018 { m = Mat::Coal; } // coal never forms where lava runs
            if dd > -60. && dd < 9. && vein(&self.n0, 30., 900.) < 0.008 { m = Mat::Coal; }
            if dd > 0. && vein(&self.n2, 45., 77.) < 0.012 { m = Mat::CopperOre; }
            if dd > 10. && vein(&self.n3, 40., 501.) < 0.009 { m = Mat::TinOre; }
            if dd > 35. && vein(&self.n0, 55., 1234.) < 0.012 { m = Mat::IronOre; }
            if dd > 45. && vein(&self.n1, 25., 4321.) < 0.006 { m = Mat::Quartz; }
        }
        // decoration on cave walls
        let edge = open(x, wy - 1) || open(x, wy + 1) || open(x - 1, wy) || open(x + 1, wy);
        if edge && m != Mat::Sand {
            let r = self.n0.at(fx / 9., fy / 9.);
            if dd > 45. && r > 0.80 { m = Mat::Crystal; } else if dd > 35. && r < 0.10 { m = Mat::Quartz; }
            else if open(x, wy - 1) && dd < 9. && n1.at(fx / 14., fy / 14.) > 0.68 { m = Mat::Fungus; }
            else if dd < -10. && n2.at(fx / 7., fy / 7.) > 0.7 { m = Mat::Roots; }
        }
        (m as u8, fl)
    }
    /// Fill window rows [y0, y1) with generated content; `oy` = absolute row of window row 0.
    pub fn fill_rows(&self, g: &mut Grid, oy: i64, y0: usize, y1: usize) {
        let w = g.w;
        let rows = y1 - y0 + 2; let mut op = vec![false; rows * w];
        for r in 0..rows { for x in 2..w - 2 { op[r * w + x] = self.open(x, oy + y0 as i64 - 1 + r as i64); } }
        let base = oy + y0 as i64 - 1;
        let open = |x: usize, wy: i64| { let r = wy - base; if r >= 0 && (r as usize) < rows { op[r as usize * w + x] } else { self.open(x, wy) } };
        for y in y0..y1 { for x in 0..w { let (m, f) = self.cell_with(x, oy + y as i64, w, &open); let i = y * w + x; g.mat[i] = m; g.flags[i] = f; } }
        // lava pockets have long since baked their surroundings: nothing burnable within reach of lava
        for y in y0..y1 { for x in 2..w - 2 { if g.mat[y * w + x] != Mat::Lava as u8 { continue; }
            for yy in y.saturating_sub(4).max(y0)..(y + 5).min(y1) { for xx in x.saturating_sub(4)..(x + 5).min(w - 2) {
                let j = yy * w + xx; if matches!(Mat::from_u8(g.mat[j]), Mat::Coal | Mat::Roots | Mat::Fungus) { g.mat[j] = Mat::Basalt as u8; }
            } }
        } }
        // unsupported sand at gen time turns to sandstone
        for y in (y0..y1).rev() { if y + 1 >= g.h { continue; } for x in 1..w - 1 {
            let i = y * w + x; if g.mat[i] != Mat::Sand as u8 { continue; }
            let ok = |j: usize| { let m = Mat::from_u8(g.mat[j]); m.kind() == crate::mat::Kind::Solid || m == Mat::Sand };
            if !(ok(i + w) && ok(i + w - 1) && ok(i + w + 1)) { g.mat[i] = Mat::Rock as u8; }
        } }
    }
    /// Chest sites in a 128-row band (deterministic): (x, absolute row).
    pub fn chests_in_band(&self, band: i64, w: usize) -> Vec<(usize, i64)> {
        let mut v = vec![];
        for k in 0..6u32 {
            let x = 8 + hash3(k, band as u32, self.seed ^ 0xC4) as usize % (w - 16);
            for dy in 0..128i64 { let wy = band * 128 + dy; if self.open(x, wy) && !self.open(x, wy + 1) && self.cell(x, wy, w).0 == 0 { v.push((x, wy)); break; } }
        }
        v
    }
}

/// Starting window: rows centred on the starting layer, a dry burrow carved in, then stabilised.
pub fn generate(g: &mut Grid, seed: u32, oy: i64) -> (Gen, GenInfo) {
    let (w, h) = (g.w, g.h);
    let gen = Gen::new(seed, w);
    gen.fill_rows(g, oy, 0, h);
    let sx = w / 2; let cy1 = (-oy) as usize; // window row of the starting layer
    let (cx0, cx1, cy0) = (sx - 15, sx + 15, cy1 - 12);
    for y in cy0 - 6..cy1 + 6 { for x in cx0 - 8..=cx1 + 8 { let i = y * w + x; if g.mat[i] == Mat::Water as u8 || g.mat[i] == Mat::Lava as u8 { g.mat[i] = Mat::Clay as u8; } } }
    // a domed burrow: the roof steps down toward the walls so no roof run spans more than a few cells
    let top = |x: usize| { let t = (x as f32 - sx as f32) / 15.; cy0 + (12. * t * t).round() as usize };
    for x in cx0 - 3..=cx1 + 3 { let t = top(x.clamp(cx0, cx1)); for y in t.saturating_sub(5)..t { g.mat[y * w + x] = Mat::Limestone as u8; } }
    for x in cx0..=cx1 { for y in top(x)..=cy1 { g.mat[y * w + x] = 0; } }
    // side walls tie the dome down to the floor (otherwise an open cavern beside it leaves the roof floating)
    for x in (cx0 - 3..cx0).chain(cx1 + 1..=cx1 + 3) { for y in top(x.clamp(cx0, cx1))..=cy1 { let i = y * w + x; if Mat::from_u8(g.mat[i]).kind() != crate::mat::Kind::Solid { g.mat[i] = Mat::Limestone as u8; } } }
    for y in cy1 + 1..cy1 + 4 { for x in cx0 - 4..=cx1 + 4 { g.mat[y * w + x] = Mat::Clay as u8; } }
    // ...and the walls stand on rock: if a cavern lies under the burrow, drop granite legs down to the next solid ground
    let mut st = Stress::new(w * h); st.solve(g);
    for x in (cx0 - 4..cx0).chain(cx1 + 1..=cx1 + 4) { let mut y = cy1 + 4; while y < h - 1 && !st.reach[y * w + x] { g.mat[y * w + x] = Mat::Limestone as u8; g.flags[y * w + x] = 0; y += 1; } } // diggable with claws
    let keep = (cx0, cy0, cx1, cy1);
    for y in cy0 - 6..cy1 + 4 { for x in cx0 - 4..=cx1 + 4 { g.flags[y * w + x] = 0; } } // built cells are fresh rock, not rubble
    stabilize_keep(g, &mut st, 0, usize::MAX, Some(keep));
    g.wake_all(); for _ in 0..240 { g.step_ca(); }
    // nothing the settling pass let flow in stays inside the burrow
    for y in cy0 - 8..=cy1 { for x in cx0 - 3..=cx1 + 3 { let i = y * w + x; if matches!(Mat::from_u8(g.mat[i]), Mat::Water | Mat::Lava | Mat::Steam) { g.mat[i] = if x >= cx0 && x <= cx1 && y >= cy0 && y <= cy1 { 0 } else { Mat::Clay as u8 }; } } }
    stabilize_keep(g, &mut st, 0, usize::MAX, Some(keep));
    // the burrow floor stays soil (farms need it), even where the stability pass reinforced it
    for x in cx0..=cx1 { let i = (cy1 + 1) * w + x; if matches!(Mat::from_u8(g.mat[i]), Mat::Granite | Mat::Limestone) { g.mat[i] = Mat::Clay as u8; } }
    g.struct_dirty = Some((0, 0, w - 1, h - 1)); g.wake_all();
    let biome = gen.biome.clone();
    (gen, GenInfo { surface: vec![0; w], biome, chests: vec![], start: (sx / 4, cy1 / 4), seed })
}

pub fn stabilize(g: &mut Grid, st: &mut Stress) { stabilize_rows(g, st, 0, usize::MAX) }
/// Like `stabilize`, but only touches failing cells in rows [y0, y1).
pub fn stabilize_rows(g: &mut Grid, st: &mut Stress, y0: usize, y1: usize) { stabilize_keep(g, st, y0, y1, None) }
/// `keep`: a rectangle (x0, y0, x1, y1) of open space that must stay open (the starting burrow).
pub fn stabilize_keep(g: &mut Grid, st: &mut Stress, y0: usize, y1: usize, keep: Option<(usize, usize, usize, usize)>) {
    let kept = |x: usize, y: usize| keep.map_or(false, |(a, b, c, d)| x >= a && x <= c && y >= b && y <= d);
    let (w, h) = (g.w, g.h);
    let thr0 = st.thr; st.thr = 0.45; // gen-time safety factor: fresh caves must survive fluids shifting about
    for it in 0..100 {
        if it == 40 { st.thr = 1.0; } // out of patience with the margin: just make sure nothing actually fails
        let fail: Vec<u32> = st.solve(g).into_iter().filter(|&i| (i as usize / w) >= y0 && (i as usize / w) < y1).collect();
        if fail.is_empty() { break; }
        for &i in &fail {
            let i = i as usize; let m = Mat::from_u8(g.mat[i]); // (keep protects the burrow's air, not its roof)
            if !st.reach[i] { g.mat[i] = 0; continue; } // floating island: no load path
            if it >= 4 { // still failing: widen the pillar / thicken the roof by filling neighbouring voids
                let (x, y) = (i % w, i / w);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, 1), (-1, 1), (1, 1), (0, 2), (0, -1), (-1, -1), (1, -1)] { let (nx, ny) = (x as i32 + dx, y as i32 + dy); if nx > 1 && ny >= 0 && (nx as usize) < w - 2 && (ny as usize) < h { let j = ny as usize * w + nx as usize; if kept(nx as usize, ny as usize) { continue; } if g.mat[j] == 0 || g.mat[j] == Mat::Water as u8 || g.mat[j] == Mat::Lava as u8 { g.mat[j] = Mat::Granite as u8; } } }
            }
            g.mat[i] = if it < 3 && m.p().tensile < Mat::Limestone.p().tensile { Mat::Limestone as u8 } else { Mat::Granite as u8 };
        }
    }
    st.thr = thr0;
}
