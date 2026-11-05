//! Per-cell RGB light. Seeds: sky light down open columns (scaled by day factor), material
//! emitters (lava, crystal, fungus, fire), and point lights (torches, lamps). Propagation is a
//! two-sweep 8-neighbour chamfer pass: L(i) = max(L(i), L(n) * t(i) * d), where t is the cell's
//! transmittance (open air 0.90/cell, rock ~0.3 so light just licks the wall face) and d = 0.93
//! for diagonals. Computed only over a rectangle (camera view + margin) so cost is bounded.
use crate::grid::{Grid, F_SEEN};
use crate::mat::{Kind, Mat, PROPS, SF};

pub struct Light { pub w: usize, pub h: usize, pub rgb: Vec<[f32; 3]>, pub rect: (usize, usize, usize, usize), trans: [f32; 32], diag: f32 }

impl Light {
    pub fn new(w: usize, h: usize) -> Self {
        let mut trans = [0.; 32];
        for (i, p) in PROPS.iter().enumerate() {
            trans[i] = match p.kind { Kind::Empty | Kind::Gas => 0.90, Kind::Liquid => 0.80, _ => (1. - p.opacity as f32 / 255.) * 0.55 };
        }
        trans[Mat::Glass as usize] = 0.88;
        // the factors above are per legacy 0.25 m tile: per cell they're the SF-th root, so light reaches as far in metres
        for t in trans.iter_mut() { *t = t.powf(1. / SF); }
        Light { w, h, rgb: vec![[0.; 3]; w * h], rect: (0, 0, 0, 0), trans, diag: 0.93f32.powf(1. / SF) }
    }
    #[inline] pub fn at(&self, i: usize) -> [f32; 3] { self.rgb[i] }

    /// Recompute inside rect (x0,y0)-(x1,y1) inclusive. `points`: (x, y, rgb 0..1.5, radius cells).
    pub fn compute(&mut self, g: &Grid, r: (usize, usize, usize, usize), day: f32, points: &[(usize, usize, [f32; 3])]) {
        let (x0, y0, x1, y1) = (r.0, r.1, r.2.min(g.w - 1), r.3.min(g.h - 1));
        self.rect = (x0, y0, x1, y1);
        let w = self.w;
        let sky = if day <= 0. { [0.; 3] } else { [1.0 * day + 0.05, 0.97 * day + 0.06, 0.9 * day + 0.10] };
        // seed
        for x in x0..=x1 {
            // sky transmission down the column from y=0
            let mut t = 1.0f32;
            let step = |m: usize, t: f32, tr: &[f32; 32]| if t > 0.002 { t * match PROPS[m].kind { Kind::Empty | Kind::Gas => 1.0, Kind::Liquid => 0.85, _ => tr[m] * 0.5 } } else { 0. };
            for y in 0..y0 { t = step(g.mat[y * w + x] as usize, t, &self.trans); if t == 0. { break; } }
            for y in y0..=y1 {
                let i = y * w + x; let m = g.mat[i] as usize; let e = PROPS[m].emit;
                self.rgb[i] = [sky[0] * t + e[0] as f32 / 170., sky[1] * t + e[1] as f32 / 170., sky[2] * t + e[2] as f32 / 170.];
                t = step(m, t, &self.trans);
            }
        }
        for &(px, py, c) in points {
            if px >= x0 && px <= x1 && py >= y0 && py <= y1 { let i = py * w + px; for k in 0..3 { self.rgb[i][k] = self.rgb[i][k].max(c[k]); } }
        }
        // propagate: forward then backward, twice
        let dg = self.diag;
        for _ in 0..2 {
            for y in y0..=y1 { for x in x0..=x1 {
                let i = y * w + x; let t = self.trans[g.mat[i] as usize]; let mut l = self.rgb[i];
                let mut take = |j: usize, f: f32| { let s = self.rgb[j]; for k in 0..3 { let v = s[k] * f; if v > l[k] { l[k] = v; } } };
                if x > x0 { take(i - 1, t); }
                if y > y0 { take(i - w, t); if x > x0 { take(i - w - 1, t * dg); } if x < x1 { take(i - w + 1, t * dg); } }
                self.rgb[i] = l;
            } }
            for y in (y0..=y1).rev() { for x in (x0..=x1).rev() {
                let i = y * w + x; let t = self.trans[g.mat[i] as usize]; let mut l = self.rgb[i];
                let mut take = |j: usize, f: f32| { let s = self.rgb[j]; for k in 0..3 { let v = s[k] * f; if v > l[k] { l[k] = v; } } };
                if x < x1 { take(i + 1, t); }
                if y < y1 { take(i + w, t); if x < x1 { take(i + w + 1, t * dg); } if x > x0 { take(i + w - 1, t * dg); } }
                self.rgb[i] = l;
            } }
        }
    }
}

/// Fog of war: raycast from (cx, cy) in 96 directions up to `r` cells; cells along the ray are
/// revealed until (and including) the first opaque cell.
pub fn reveal(g: &mut Grid, cx: usize, cy: usize, r: usize) -> usize {
    let mut n = 0;
    let rays = 96 * crate::mat::S; // enough rays that their spacing at the rim stays the same in cells
    for k in 0..rays {
        let a = k as f32 / rays as f32 * std::f32::consts::TAU; let (dx, dy) = (a.cos(), a.sin());
        let (mut x, mut y) = (cx as f32 + 0.5, cy as f32 + 0.5);
        for _ in 0..r {
            let (xi, yi) = (x as i32, y as i32); if !g.in_bounds(xi, yi) { break; }
            let i = yi as usize * g.w + xi as usize;
            if g.flags[i] & F_SEEN == 0 { g.flags[i] |= F_SEEN; n += 1; }
            if matches!(g.m(i).kind(), Kind::Solid | Kind::Powder) { break; }
            x += dx; y += dy;
        }
    }
    n
}
pub fn reveal_box(g: &mut Grid, x0: usize, y0: usize, x1: usize, y1: usize) {
    for y in y0..=y1.min(g.h - 1) { for x in x0..=x1.min(g.w - 1) { let i = y * g.w + x; g.flags[i] |= F_SEEN; } }
}
