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

pub fn generate(g: &mut Grid, seed: u32) -> GenInfo {
    let (w, h) = (g.w, g.h);
    let n0 = Noise::new(seed); let n1 = Noise::new(seed ^ 0x51ED); let n2 = Noise::new(seed ^ 0xA77A); let n3 = Noise::new(seed ^ 0x3C3C);
    let base = h / 8; // ~16 m of sky on a 512-high map
    let surface: Vec<usize> = (0..w).map(|x| (base as f32 + (n0.fbm(x as f32, 3., 90., 3) - 0.5) * 40.) as usize).collect();
    let biome: Vec<Biome> = (0..w).map(|x| biome_at(&n3, x)).collect();
    let dm = |cells: f32| cells * 0.25; // metres
    for x in 0..w {
        let s = surface[x]; let b = biome[x];
        for y in 0..h {
            let i = y * w + x;
            if y < s { g.mat[i] = 0; continue; }
            let d = dm((y - s) as f32); // depth in m
            let (fx, fy) = (x as f32, y as f32);
            let wob = (n1.fbm(fx, fy, 60., 3) - 0.5) * 8.; // layer boundary wobble (m)
            let dd = d + wob;
            let shift = match b { Biome::Loam => 4., Biome::Karst => -2., Biome::Crags => -8., Biome::Ashlands => -4. };
            let dd = dd - shift;
            let mut m = if y == s { Mat::Grass }
                else if dd < 6. { Mat::Dirt }
                else if dd < 20. { if n2.fbm(fx, fy * 2., 40., 2) > 0.55 { Mat::Clay } else { Mat::Dirt } }
                else if dd < 55. { if b == Biome::Crags { Mat::Granite } else { Mat::Limestone } }
                else if dd < 90. { if n2.fbm(fx + 300., fy, 70., 3) > 0.5 { Mat::Granite } else { Mat::Rock } }
                else { if b == Biome::Ashlands || n2.at(fx / 50., fy / 50.) > 0.4 { Mat::Basalt } else { Mat::Granite } };
            // lenses: sand / gravel aquifers in the soil band
            if dd > 3. && dd < 35. {
                let l = n2.fbm(fx, fy * 3., 50., 3);
                if l > 0.66 { m = if n1.at(fx / 30., fy / 30.) > 0.5 { Mat::Gravel } else { Mat::Sand }; if dd > 10. && l > 0.7 { g.flags[i] |= F_WET; } }
            }
            // ore veins: thin ridges of noise
            let vein = |nn: &Noise, sc: f32, sal: f32| (nn.fbm(fx + sal, fy * 1.3, sc, 3) - 0.5).abs();
            if m != Mat::Grass && m != Mat::Sand {
                if dd > 12. && dd < 70. && vein(&n1, 70., 11., ) < 0.018 && m != Mat::Dirt { m = Mat::Coal; }
                if dd > 8. && dd < 25. && vein(&n0, 30., 900.) < 0.008 { m = Mat::Coal; }
                if dd > 18. && dd < 80. && vein(&n2, 45., 77.) < 0.012 { m = Mat::CopperOre; }
                if dd > 25. && dd < 90. && vein(&n3, 40., 501.) < 0.009 { m = Mat::TinOre; }
                if dd > 50. && vein(&n0, 55., 1234.) < 0.012 { m = Mat::IronOre; }
                if dd > 60. && vein(&n1, 25., 4321.) < 0.006 { m = Mat::Quartz; }
            }
            if y >= h - 4 || x < 2 || x >= w - 2 { m = Mat::Bedrock; }
            g.mat[i] = m as u8;
        }
    }
    // ---- caves: cellular automata on a 2-cell lattice, density by depth band and biome ----
    let (lw, lh) = (w / 2, h / 2);
    let mut cave = vec![false; lw * lh];
    for ly in 0..lh { for lx in 0..lw {
        let (x, y) = (lx * 2, ly * 2); let s = surface[x];
        if y < s + 40 || y >= h - 8 || x < 6 || x >= w - 6 { continue; }
        let d = dm((y - s) as f32);
        let band = match biome[x] { Biome::Karst => 0.50, Biome::Loam => 0.40, Biome::Crags => 0.36, Biome::Ashlands => 0.42 };
        let region = n3.fbm(x as f32, y as f32, 80., 2);
        let dens = band + (region - 0.5) * 0.35 + if d > 25. { 0.04 } else { -0.06 };
        cave[ly * lw + lx] = (hash3(lx as u32, ly as u32, seed ^ 99) & 1023) as f32 / 1024. < dens;
    } }
    for _ in 0..5 {
        let prev = cave.clone();
        for ly in 1..lh - 1 { for lx in 1..lw - 1 {
            let mut c = 0; for dy in -1i32..=1 { for dx in -1i32..=1 { if (dx, dy) != (0, 0) && prev[(ly as i32 + dy) as usize * lw + (lx as i32 + dx) as usize] { c += 1; } } }
            cave[ly * lw + lx] = if prev[ly * lw + lx] { c >= 4 } else { c >= 5 };
        } }
    }
    let carve = |g: &mut Grid, x: usize, y: usize| { let i = y * g.w + x; if g.mat[i] != Mat::Bedrock as u8 { g.mat[i] = 0; } };
    for ly in 0..lh { for lx in 0..lw { if cave[ly * lw + lx] { for dy in 0..2 { for dx in 0..2 { carve(g, lx * 2 + dx, ly * 2 + dy); } } } } }
    // ---- worms: long winding tunnels ----
    for k in 0..(w / 60) as u32 {
        let mut x = (hash3(k, 1, seed) as usize % (w - 40)) as f32 + 20.; let mut y = surface[x as usize] as f32 + 30. + (hash3(k, 2, seed) % (h as u32 / 2)) as f32;
        let mut a = (hash3(k, 3, seed) % 628) as f32 / 100.;
        for step in 0..260 {
            a += (n1.at(k as f32 * 10. + step as f32 * 0.05, 0.) - 0.5) * 0.9; let r = 2.0 + 2.5 * n2.at(step as f32 * 0.03, k as f32);
            x += a.cos() * 1.5; y += a.sin() * 0.8;
            if x < 8. || x >= (w - 8) as f32 || y >= (h - 10) as f32 { break; }
            if (y as usize) < surface[x as usize] + 24 { a = a.abs(); continue; }
            let ri = r.ceil() as i32;
            for dy in -ri..=ri { for dx in -ri..=ri { if ((dx * dx + dy * dy) as f32) <= r * r { carve(g, (x as i32 + dx) as usize, (y as i32 + dy) as usize); } } }
        }
    }
    // ---- fluids: aquifer pools, deep lava ----
    let water_table: Vec<f32> = (0..w).map(|x| surface[x] as f32 + 140. + (n0.fbm(x as f32, 9., 120., 2) - 0.5) * 120.).collect();
    let lava_level = h as f32 - 90.;
    for x in 2..w - 2 { for y in (0..h - 4).rev() {
        let i = y * w + x; if g.mat[i] != 0 || y < surface[x] + 30 { continue; }
        let d = dm((y - surface[x]) as f32);
        if (y as f32) > lava_level + (n1.at(x as f32 / 40., 3.) - 0.5) * 30. && d > 80. { g.mat[i] = Mat::Lava as u8; }
        else if (y as f32) > water_table[x] && d < 50. && n2.at(x as f32 / 35., y as f32 / 35.) > 0.42 { g.mat[i] = Mat::Water as u8; }
    } }
    for k in 0..(w / 18) as u32 { // root systems
        let x0 = 6 + hash3(k, 5, seed) as usize % (w - 12); if biome[x0] == Biome::Ashlands { continue; }
        for r in 0..3u32 {
            let (mut x, mut y) = (x0 as f32, surface[x0] as f32 + 1.); let mut a = 1.57 + (hash3(k, r, seed ^ 7) % 100) as f32 / 100. - 0.5;
            for s in 0..(20 + hash3(k, r, 3) % 40) {
                a += (n2.at(k as f32 + s as f32 * 0.2, r as f32) - 0.5) * 0.6; x += a.cos(); y += a.sin().abs().max(0.3);
                let (xi, yi) = (x as usize, y as usize); if xi < 3 || xi >= w - 3 || yi >= h - 5 { break; }
                let i = yi * w + xi; if matches!(Mat::from_u8(g.mat[i]), Mat::Dirt | Mat::Clay | Mat::Grass | Mat::Gravel) { g.mat[i] = Mat::Roots as u8; }
            }
        }
    }
    // ---- starting burrow: shaft from the surface into a chamber ----
    let sx = w / 2; let sy = surface[sx];
    let (cx0, cx1, cy0, cy1) = (sx - 15, sx + 15, sy + 14, sy + 24);
    for y in cy0..=cy1 { for x in cx0..=cx1 { g.mat[y * w + x] = 0; } }
    for y in sy.saturating_sub(2)..cy0 { for x in sx - 3..=sx + 2 { g.mat[y * w + x] = 0; } }
    for y in cy1 + 1..cy1 + 4 { for x in cx0 - 4..=cx1 + 4 { g.mat[y * w + x] = Mat::Clay as u8; } }
    // never leave unsupported powders hanging at gen: sand that could flow becomes sandstone (bottom-up)
    {
        let mut ok = vec![false; w * h];
        for y in (0..h - 1).rev() { for x in 1..w - 1 {
            let i = y * w + x; if g.mat[i] != Mat::Sand as u8 { continue; }
            let base = |j: usize| { let m = Mat::from_u8(g.mat[j]); (m.kind() == crate::mat::Kind::Solid) || ok[j] };
            if base(i + w) && base(i + w - 1) && base(i + w + 1) { ok[i] = true; } else { g.mat[i] = Mat::Rock as u8; }
        } }
    }
    // ---- stability pass: anything the structural model rejects is reinforced or removed ----
    let mut st = Stress::new(w * h);
    stabilize(g, &mut st);
    // ---- decoration: crystals & quartz in deep walls, fungus on cave floors, roots under surface ----
    for y in 1..h - 1 { for x in 1..w - 1 {
        let i = y * w + x; let m = g.mat[i]; if m == 0 || m == Mat::Bedrock as u8 || m >= Mat::Water as u8 { continue; }
        let d = dm(y.saturating_sub(surface[x]) as f32);
        let open_up = g.mat[i - w] == 0; let open_any = open_up || g.mat[i + w] == 0 || g.mat[i - 1] == 0 || g.mat[i + 1] == 0;
        if !open_any { continue; }
        let r = n3.at(x as f32 / 9., y as f32 / 9.);
        if d > 55. && r > 0.80 { g.mat[i] = Mat::Crystal as u8; }
        else if d > 45. && r < 0.10 { g.mat[i] = Mat::Quartz as u8; }
        else if open_up && d > 12. && d < 90. && n1.at(x as f32 / 14., y as f32 / 14.) > 0.68 { g.mat[i] = Mat::Fungus as u8; }
    } }
    g.wake_all(); for _ in 0..240 { g.step_ca(); }
    stabilize(g, &mut st);
    // chests: on cave floors and buried
    let mut chests = vec![];
    let mut k = 0u32;
    while chests.len() < 24 && k < 20000 {
        k += 1;
        let x = 8 + hash3(k, 11, seed) as usize % (w - 16); let y = surface[x] + 30 + hash3(k, 12, seed) as usize % (h - surface[x] - 40);
        let i = y * w + x;
        let floor = g.mat[i] == 0 && g.mat[i + w] != 0 && g.mat[i + w] < Mat::Water as u8;
        let buried = k % 3 == 0 && g.mat[i] != 0 && g.mat[i] < Mat::Water as u8 && g.mat[i] != Mat::Bedrock as u8;
        if (floor || buried) && chests.iter().all(|&(cx, cy): &(usize, usize)| (cx as i32 - x as i32).abs() + (cy as i32 - y as i32).abs() > 60) { chests.push((x / 4, y / 4)); }
    }
    g.struct_dirty = Some((0, 0, w - 1, h - 1)); g.wake_all();
    GenInfo { surface, biome, chests, start: (sx / 4, cy1 / 4), seed }
}

fn stabilize(g: &mut Grid, st: &mut Stress) {
    let (w, h) = (g.w, g.h);
    for it in 0..80 {
        let fail = st.solve(g);
        if fail.is_empty() { break; }
        for &i in &fail {
            let i = i as usize; let m = Mat::from_u8(g.mat[i]);
            if !st.reach[i] { g.mat[i] = 0; continue; } // floating island: no load path
            if it >= 4 { // still failing: widen the pillar / thicken the roof by filling neighbouring voids
                let (x, y) = (i % w, i / w);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, 1), (-1, 1), (1, 1), (0, 2)] { let (nx, ny) = (x as i32 + dx, y as i32 + dy); if nx > 1 && ny > 0 && (nx as usize) < w - 2 && (ny as usize) < h - 4 { let j = ny as usize * w + nx as usize; if g.mat[j] == 0 || g.mat[j] == Mat::Water as u8 || g.mat[j] == Mat::Lava as u8 { g.mat[j] = Mat::Granite as u8; } } }
            }
            g.mat[i] = if it < 3 && m.p().tensile < Mat::Limestone.p().tensile { Mat::Limestone as u8 } else { Mat::Granite as u8 };
        }
    }
}
