//! Structural solver. Physically-motivated, cheap enough to run on 0.5M cells every few ticks.
//!
//! Units: cell edge s = 0.25 m, unit depth b = 1 m, g = 9.81 m/s², ρ in kg/m³, stresses in kPa.
//! 1. Overburden: P(x,y) = Σ_{cells above, not empty/gas} ρ·g·s / 1000          [kPa]
//! 2. Connectivity: static solids not 4-connected to bedrock have no load path → fail.
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
//!      strength of the weakest cell. Thin pillars under deep overburden crush.
//! Utilisation u = σ / strength. u > 1 → the cells crack (visible) and crumble into the
//! falling-sand sim after a short delay unless the load is relieved (e.g. a pillar is built).
use crate::grid::Grid;
use crate::mat::{Kind, Mat, CELL_M, G, PROPS};

pub struct Stress {
    pub util: Vec<u8>,      // utilisation ×128 (128 = at capacity, 255 = ≥2× or no load path)
    pub pending: Vec<u8>,   // crack countdown
    pub cracking: Vec<u32>, // indices with pending > 0
    pub overburden: Vec<f32>,
    pub last_fail: usize,
    pub reach: Vec<bool>,
}
pub const CRACK_TICKS: u8 = 40;

impl Stress {
    pub fn new(n: usize) -> Self { Stress { util: vec![0; n], pending: vec![0; n], cracking: vec![], overburden: vec![0.; n], last_fail: 0, reach: vec![] } }

    /// Full solve. Returns indices with u > 1.
    pub fn solve(&mut self, g: &Grid) -> Vec<u32> {
        let (w, h) = (g.w, g.h);
        let n = w * h;
        let st = |i: usize| g.is_static(i);
        let supports = |i: usize| { let k = g.m(i).kind(); k == Kind::Powder || (k == Kind::Solid) };
        // 1. overburden
        let p = &mut self.overburden;
        for x in 0..w {
            let mut acc = 0f32;
            for y in 0..h { let i = y * w + x; p[i] = acc; let m = g.m(i); if !matches!(m.kind(), Kind::Empty | Kind::Gas) { acc += PROPS[m as usize].density * G * CELL_M / 1000.; } }
        }
        let p = &self.overburden;
        let mut util = vec![0f32; n];
        // 2. connectivity (BFS from bedrock)
        let mut reach = vec![false; n]; let mut q: Vec<u32> = Vec::with_capacity(n / 4);
        for i in 0..n { if g.mat[i] == Mat::Bedrock as u8 { reach[i] = true; q.push(i as u32); } }
        let mut head = 0;
        while head < q.len() {
            let i = q[head] as usize; head += 1; let (x, y) = (i % w, i / w);
            for j in g.n4(x, y) { if !reach[j] && st(j) { reach[j] = true; q.push(j as u32); } }
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
                    if k < cap && y + 1 >= k { qtop = qtop.max(p[(y + 1 - k) * w + xx]); } // rubble/water resting on the beam
                }
                let cnt: usize = (xa..=xb).map(|xx| { let mut k = 0; while k < cap && y >= k && st((y - k) * w + xx) { k += 1; } k }).sum();
                rho /= cnt.max(1) as f32;
                let hd = (hmin.max(1) as f32 * CELL_M).min(span.max(CELL_M));
                let wload = rho * G * hd / 1000. + qtop; // kPa (kN/m² over b = 1 m)
                let k = if left && right { 1. / 8. } else if left || right { 0.5 } else { 0.5 };
                let span_eff = if left || right { span } else { span.max(CELL_M) };
                let sigma = 6. * k * wload * span_eff * span_eff / (hd * hd);
                let u = sigma / tmin.max(1e-3);
                for xx in xa..=xb { let j = y * w + xx; if u > util[j] { util[j] = u; } }
            }
        }
        // 4. compression of horizontal solid runs (pillars / walls)
        for y in 1..h {
            let mut x = 0;
            while x < w {
                if !st(y * w + x) { x += 1; continue; }
                let xa = x; while x < w && st(y * w + x) { x += 1; } let xb = x - 1;
                let mut load: f32 = (xa..=xb).map(|xx| p[y * w + xx]).sum();
                                // left void run
                let mut l = xa; let mut vl = 0f32; let mut nl = 0; while l > 0 && !st(y * w + l - 1) { l -= 1; vl += p[y * w + l]; nl += 1; if nl > 400 { break; } }
                let mut r = xb; let mut vr = 0f32; let mut nr = 0; while r + 1 < w && !st(y * w + r + 1) { r += 1; vr += p[y * w + r]; nr += 1; if nr > 400 { break; } }
                load += 0.5 * (vl + vr);
                let wid = (xb - xa + 1) as f32;
                let sigma = load / wid;
                for xx in xa..=xb { let j = y * w + xx; let u = sigma / PROPS[g.mat[j] as usize].compressive.max(1e-3); if u > util[j] { util[j] = u; } }
            }
        }
        self.reach = reach;
        let mut fail = vec![];
        for i in 0..n {
            let u = util[i];
            self.util[i] = (u * 128.).min(255.) as u8;
            if u > 1. && g.mat[i] != Mat::Bedrock as u8 { fail.push(i as u32); }
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
