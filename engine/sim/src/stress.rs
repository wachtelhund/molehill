//! Structural solver. Physically-motivated, cheap enough to run on 0.5M cells every few ticks.
//!
//! Units: cell edge s = 0.25 m, unit depth b = 1 m, g = 9.81 m/s², ρ in kg/m³, stresses in kPa.
//! 1. Overburden: P(x,y) = Σ_{cells above, not empty/gas} ρ·g·s / 1000          [kPa]
//! 2. Connectivity: static solids not 4-connected to bedrock have no load path → fail. Loose
//!    ground (powder, loose rubble) passes load straight down: whatever rests on a supported pile
//!    is supported, so placed pillars and bricks can stand on a sand floor.
//! 3. Bending (roofs): every maximal horizontal run of static cells with no support directly
//!    below is a beam of span L = n·s. Its depth h = min static thickness above the run, capped
//!    at the span (arching / Voussoir effect: only ~L of rock acts as the beam). Superimposed
//!    load q = overburden on top of the beam (loose rubble, water, etc.).
//!      simply supported (both ends rest on supported cells): M = (ρ g h + q)·L²/8
//!      cantilever (one end):                                M = (ρ g h + q)·L²/2
//!      section modulus Z = b·h²/6  →  σ_t = M / Z, compared with the weakest tensile
//!      strength along the bottom fibre (the run itself). Wood/iron roof linings raise it.
//! 4. Compression (walls/pillars): every horizontal run of static cells of width w carries its
//!    own overburden plus half of the overburden of the voids beside it (tributary area):
//!      σ_c = (Σ_run P + ½Σ_left void P + ½Σ_right void P) / w, compared with compressive
//!      each cell's own compressive strength (a weak cell in a wide wall crushes alone).
//! Utilisation u = σ / strength. u > 1 → the cells crack (visible) and crumble into the
//! falling-sand sim after a short delay unless the load is relieved (e.g. a pillar is built).
use crate::grid::Grid;
use crate::mat::{Kind, Mat, CELL_M, G, PROPS};

const ARCH: usize = 64;
pub struct Stress {
    pub util: Vec<u8>,      // utilisation ×128 (128 = at capacity, 255 = ≥2× or no load path)
    pub pending: Vec<u8>,   // crack countdown
    pub cracking: Vec<u32>, // indices with pending > 0
    pub overburden: Vec<f32>,
    pub last_fail: usize,
    pub reach: Vec<bool>,
    /// failure threshold on utilisation (1.0 at runtime; gen uses a safety factor)
    pub thr: f32,
}
pub const CRACK_TICKS: u8 = 40;

impl Stress {
    pub fn new(n: usize) -> Self { Stress { util: vec![0; n], pending: vec![0; n], cracking: vec![], overburden: vec![0.; n], last_fail: 0, reach: vec![], thr: 1.0 } }

    /// Full solve. Returns indices with u > 1.
    /// Height of the load-bearing column above any cell (16 m), the pressure-arch height.
    pub fn _arch() -> usize { ARCH }
    pub fn solve(&mut self, g: &Grid) -> Vec<u32> {
        let (w, h) = (g.w, g.h);
        let n = w * h;
        let st = |i: usize| g.is_static(i);
        let supports = |i: usize| { let k = g.m(i).kind(); k == Kind::Powder || (k == Kind::Solid) };
        // 1. overburden
        let p = &mut self.overburden;
        for x in 0..w {
            let mut acc = 0f32;
            // Ground arching (Terzaghi): load deeper than ARCH cells above is carried by the rock arch
            // around the opening, so only a sliding column of ARCH cells bears down on a cell.
            let cw = |y: usize| { let m = g.m(y * w + x); if matches!(m.kind(), Kind::Empty | Kind::Gas) { 0. } else { PROPS[m as usize].density * G * CELL_M / 1000. } };
            for y in 0..h { let i = y * w + x; p[i] = acc; acc += cw(y); if y >= ARCH { acc -= cw(y - ARCH); } }
        }
        let p = &self.overburden;
        // ctop: top row of the unbroken static column a static cell belongs to (does it reach the roof?);
        // vtop: for an open cell, the row of the first non-open cell above it (the roof over that void)
        let open = |i: usize| matches!(g.m(i).kind(), Kind::Empty | Kind::Gas);
        let mut ctop = vec![0u32; n]; let mut vtop = vec![0u32; n];
        for y in 0..h { for x in 0..w { let i = y * w + x;
            if st(i) { ctop[i] = if y > 0 && st(i - w) { ctop[i - w] } else { y as u32 }; }
            else if open(i) { vtop[i] = if y == 0 { 0 } else if open(i - w) { vtop[i - w] } else { (y - 1) as u32 }; }
        } }
        let mut util = vec![0f32; n];
        // 2. connectivity (BFS from bedrock)
        let mut reach = vec![false; n]; let mut q: Vec<u32> = Vec::with_capacity(n / 4);
        // anchors: bedrock walls, plus the window's top and bottom rows (the Deepholt continues beyond them)
        for i in 0..n { if g.mat[i] == Mat::Bedrock as u8 || ((i < w || i >= n - w) && st(i)) { reach[i] = true; q.push(i as u32); } }
        let mut head = 0;
        while head < q.len() {
            let i = q[head] as usize; head += 1; let (x, y) = (i % w, i / w);
            // static cells carry each other in all four directions; loose ground (sand, rubble) only
            // carries what rests directly on top of it, so a pillar built on a sand floor has a load path
            let carried = g.is_static(i);
            if carried { for j in g.n4(x, y) { if !reach[j] && st(j) { reach[j] = true; q.push(j as u32); } } }
            if y > 0 { let j = i - w; if !reach[j] && supports(j) { reach[j] = true; q.push(j as u32); } }
        }
        for i in 0..n { if st(i) && !reach[i] { util[i] = 9.; } }
        // 3. bending of roof runs
        for y in 0..h - 1 {
            let mut x = 0;
            while x < w {
                let i = y * w + x;
                if !(st(i) && !supports(i + w)) { x += 1; continue; }
                let xa = x; while x < w && st(y * w + x) && !supports(y * w + x + w) { x += 1; } let xb = x - 1;
                let left = xa > 0 && st(y * w + xa - 1) && supports(y * w + xa - 1 + w);
                let right = xb + 1 < w && st(y * w + xb + 1) && supports(y * w + xb + 1 + w);
                let span = (xb - xa + 1) as f32 * CELL_M;
                let mut hmin = usize::MAX; let mut rho = 0f32; let mut tmin = f32::MAX; let mut qtop = 0f32;
                let cap = ((xb - xa + 1) + 1).min(80);
                for xx in xa..=xb {
                    let mut k = 0; while k < cap && y >= k && st((y - k) * w + xx) { rho += PROPS[g.mat[(y - k) * w + xx] as usize].density; k += 1; }
                    hmin = hmin.min(k); tmin = tmin.min(PROPS[g.mat[y * w + xx] as usize].tensile);
                    // superimposed load: only loose rubble / sand / water lying directly on the beam.
                    // Static rock above (beyond the arch cap) carries itself; an open cave above is
                    // carried by its own roof, so neither loads this beam.
                    if k < cap && y >= k { let mut yy = y - k; let mut n = 0; let mut qq = 0f32;
                        loop { let j = yy * w + xx; let m = g.m(j); if matches!(m.kind(), Kind::Empty | Kind::Gas) || st(j) || n >= ARCH { break; }
                            qq += PROPS[m as usize].density * G * CELL_M / 1000.; n += 1; if yy == 0 { break; } yy -= 1; }
                        qtop += qq; }
                }
                qtop /= (xb - xa + 1) as f32; // spread over the span (uniformly distributed load)
                let cnt: usize = (xa..=xb).map(|xx| { let mut k = 0; while k < cap && y >= k && st((y - k) * w + xx) { k += 1; } k }).sum();
                rho /= cnt.max(1) as f32;
                let hd = (hmin.max(1) as f32 * CELL_M).min(span.max(CELL_M));
                let wload = rho * G * hd / 1000. + qtop; // kPa (kN/m² over b = 1 m)
                let k = if left && right { 1. / 8. } else if left || right { 0.5 } else { 0.5 };
                let span_eff = if left || right { span } else { span.max(CELL_M) };
                let sigma = 6. * k * wload * span_eff * span_eff / (hd * hd);
                let u = sigma / tmin.max(1e-3);
                // the moment isn't uniform along the beam: M(x) ∝ x(L-x) when both ends rest
                // (peak at midspan), ∝ (distance from the free tip)² for a cantilever (peak at the
                // root). Overloaded roofs therefore break where M peaks and leave the rest standing,
                // so a cave-in sags out of the middle into a stable arch instead of peeling whole rows.
                let n = (xb - xa + 1) as f32;
                for xx in xa..=xb {
                    let t = (xx - xa) as f32 + 0.5;
                    let f = if left && right { 4. * t * (n - t) / (n * n) } else if left { ((n - t + 0.5) / n).powi(2) } else if right { ((t + 0.5) / n).powi(2) } else { 1. };
                    let j = y * w + xx; let uu = u * f.min(1.); if uu > util[j] { util[j] = uu; }
                }
            }
        }
        // 4. compression of horizontal solid runs (pillars / walls)
        for y in 1..h {
            let mut x = 0;
            while x < w {
                if !st(y * w + x) { x += 1; continue; }
                let xa = x; while x < w && st(y * w + x) { x += 1; } let xb = x - 1;
                let mut load: f32 = (xa..=xb).map(|xx| p[y * w + xx]).sum();
                // Tributary load from the open space beside the run, but only if the run's static column
                // actually reaches up to that void's roof (a free-standing stub carries no roof). Above
                // an opening of width n the rock arches, so only the loosened zone (≈ n+1 cells of rock)
                // bears down on the supports: a lone prop in a sound hall isn't crushed.
                let rt = (xa..=xb).map(|xx| ctop[y * w + xx]).min().unwrap_or(0);
                let cell_w = PROPS[Mat::Granite as usize].density * G * CELL_M / 1000.;
                let mut nl = 0; while xa > nl && !st(y * w + xa - nl - 1) && nl < 400 { nl += 1; }
                let mut nr = 0; while xb + nr + 1 < w && !st(y * w + xb + nr + 1) && nr < 400 { nr += 1; }
                let (cl, cr) = ((nl + 1) as f32 * cell_w, (nr + 1) as f32 * cell_w);
                let trib = |j: usize, cap: f32| if open(j) && rt > vtop[j] { 0. } else { p[j].min(cap) };
                let vl: f32 = (1..=nl).map(|k| trib(y * w + xa - k, cl)).sum();
                let vr: f32 = (1..=nr).map(|k| trib(y * w + xb + k, cr)).sum();
                load += 0.5 * (vl + vr);
                let wid = (xb - xa + 1) as f32;
                let sigma = load / wid;
                // Confinement (Mohr-Coulomb, σ3 ≈ K0·σv): a cell held in on both sides by the rest
                // of the run can't crush; only cells at a free face (the run's ends) see the
                // unconfined strength. So an overloaded body spalls at its faces instead of the
                // whole mass turning to rubble at once.
                for xx in [xa, xb] { let j = y * w + xx; let u = sigma / PROPS[g.mat[j] as usize].compressive.max(1e-3); if u > util[j] { util[j] = u; } }
                if xb > xa + 1 { for xx in xa + 1..xb { let j = y * w + xx; let u = 0.5 * sigma / PROPS[g.mat[j] as usize].compressive.max(1e-3); if u > util[j] { util[j] = u.min(0.9); } } }
            }
        }
        self.reach = reach;
        let mut fail = vec![];
        for i in 0..n {
            let u = util[i];
            self.util[i] = (u * 128.).min(255.) as u8;
            if u > self.thr && g.mat[i] != Mat::Bedrock as u8 { fail.push(i as u32); }
        }
        fail
    }

    /// Start crack timers on failing cells.
    pub fn crack(&mut self, fail: &[u32]) {
        self.last_fail = fail.len();
        for &i in fail { let i = i as usize; if self.pending[i] == 0 { self.pending[i] = CRACK_TICKS; self.cracking.push(i as u32); } }
    }
    /// Advance cracks; returns cells that crumble now.
    pub fn tick(&mut self, g: &Grid) -> Vec<u32> {
        let mut out = vec![]; let mut keep = vec![];
        for &i in &self.cracking {
            let iu = i as usize;
            if !g.is_static(iu) { self.pending[iu] = 0; continue; }
            if self.util[iu] < 128 { self.pending[iu] = 0; continue; } // relieved
            self.pending[iu] -= 1;
            if self.pending[iu] == 0 { out.push(i); } else { keep.push(i); }
        }
        self.cracking = keep; out
    }
}
