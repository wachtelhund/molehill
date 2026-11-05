//! Structural solver. Physically-motivated; a full solve covers the whole window, and the world
//! re-solves only the region around each change (`solve_rect`) so digging stays cheap.
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
use crate::mat::{Kind, Mat, CELL_M, CPM, G, PROPS, S};

const ARCH: usize = 16 * CPM; // 16 m
/// Side margin of a region solve. Runs, voids, roof rows and overburden reach past it exactly (they read
/// the stored whole-window state); only the load-path search is cut at the region's edge, where cells
/// that had a path last time anchor it.
pub const SIDE: usize = 8 * CPM;
pub struct Stress {
    pub util: Vec<u8>,      // utilisation ×128 (128 = at capacity, 255 = ≥2× or no load path)
    pub pending: Vec<u8>,   // crack countdown
    pub cracking: Vec<u32>, // indices with pending > 0
    pub overburden: Vec<f32>,
    pub last_fail: usize,
    pub reach: Vec<bool>,
    /// per cell: for static cells the top row of their unbroken static column; for open cells the row of
    /// the roof above them. Kept between solves; a region solve updates the columns it touches.
    ctop: Vec<u32>, vtop: Vec<u32>,
    /// failure threshold on utilisation (1.0 at runtime; gen uses a safety factor)
    pub thr: f32,
}
pub const CRACK_TICKS: u8 = 40;
/// Merge cell boxes (x0, y0, x1, y1) that lie within `gap` cells of each other into clusters.
pub fn merge_boxes(mut boxes: Vec<(usize, usize, usize, usize)>, gap: usize) -> Vec<(usize, usize, usize, usize)> {
    let near = |a: &(usize, usize, usize, usize), b: &(usize, usize, usize, usize)| a.0 <= b.2 + gap && b.0 <= a.2 + gap && a.1 <= b.3 + gap && b.1 <= a.3 + gap;
    let mut merged = true;
    while merged { merged = false;
        'o: for i in 0..boxes.len() { for j in i + 1..boxes.len() { if near(&boxes[i], &boxes[j]) { let b = boxes.swap_remove(j); let a = &mut boxes[i];
            *a = (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)); merged = true; break 'o; } } } }
    boxes
}
pub const LOOSE_TICKS: u8 = 8;

impl Stress {
    pub fn new(n: usize) -> Self { Stress { util: vec![0; n], pending: vec![0; n], cracking: vec![], overburden: vec![0.; n], last_fail: 0, reach: vec![], ctop: vec![0; n], vtop: vec![0; n], thr: 1.0 } }

    /// Full solve. Returns indices with u > 1.
    /// Height of the load-bearing column above any cell (16 m), the pressure-arch height.
    pub fn _arch() -> usize { ARCH }
    /// The window slid by `rows` (+ = deeper; rows that stay move up by `rows`): carry this solver's state
    /// with the rock (stored roof rows are row numbers, so they move too); the entering rows start blank.
    /// Afterwards only the entering rows (and the edge the window left) need a region solve.
    pub fn shift_rows(&mut self, rows: i64, w: usize) {
        let n = self.util.len(); if self.reach.len() != n { return; }
        let s = rows.unsigned_abs() as usize; let d = s * w; if d >= n { *self = Stress::new(n); return; }
        fn mv<T: Copy>(v: &mut [T], d: usize, up: bool, blank: T) { let n = v.len(); if up { v.copy_within(d.., 0); v[n - d..].fill(blank); } else { v.copy_within(..n - d, d); v[..d].fill(blank); } }
        let up = rows > 0;
        mv(&mut self.util, d, up, 0); mv(&mut self.pending, d, up, 0); mv(&mut self.overburden, d, up, 0.); mv(&mut self.reach, d, up, false);
        mv(&mut self.ctop, d, up, 0); mv(&mut self.vtop, d, up, 0);
        let s32 = s as u32; for v in self.ctop.iter_mut().chain(self.vtop.iter_mut()) { *v = if up { v.saturating_sub(s32) } else { *v + s32 }; }
        self.cracking = std::mem::take(&mut self.cracking).into_iter().filter_map(|i| { let i = i as usize; if up { i.checked_sub(d) } else { (i + d < n).then_some(i + d) } }).map(|i| i as u32).collect();
    }
    pub fn solve(&mut self, g: &Grid) -> Vec<u32> { self.solve_rect(g, None) }
    /// Re-solve around the chunks holding `cells` (each one's 64-cell chunk; nearby chunks cluster into one
    /// region). Exact when nothing else changed since the last solve. Falls back to a full solve when the
    /// clusters would cover most of the window anyway.
    pub fn solve_cells(&mut self, g: &Grid, cells: &[usize]) -> Vec<u32> {
        use crate::grid::CHUNK;
        let mut seen = std::collections::HashSet::new(); let mut boxes = vec![];
        for &i in cells { let (cx, cy) = ((i % g.w) / CHUNK, (i / g.w) / CHUNK); if seen.insert((cx, cy)) { boxes.push((cx * CHUNK, cy * CHUNK, cx * CHUNK + CHUNK - 1, cy * CHUNK + CHUNK - 1)); } }
        let boxes = merge_boxes(boxes, 16 * CPM);
        let area: usize = boxes.iter().map(|b| (b.2 - b.0 + 1 + 2 * SIDE + 8 * CPM).min(g.w) * (b.3 - b.1 + 1 + 28 * CPM).min(g.h)).sum();
        if area * 10 > g.w * g.h * 6 { return self.solve(g); }
        let mut f = vec![]; for b in boxes { f.extend(self.solve_rect(g, Some(b))); } f
    }
    /// Re-solve only around `dirty` (cells x0, y0, x1, y1 inclusive): the work region is that box plus
    /// SIDE (8 m) to each side, the 16 m arch height above and 8 m below; results are written back only for
    /// the box plus 4 m (cells further out keep their last state). The region's edge cells that had
    /// a load path last time anchor it, so the 2M-cell window isn't re-solved for every bite.
    /// `None` (or no previous solve) is a full solve. Returns failing cells (u > thr) in the written area.
    pub fn solve_rect(&mut self, g: &Grid, dirty: Option<(usize, usize, usize, usize)>) -> Vec<u32> {
        let (w, h) = (g.w, g.h);
        let n = w * h;
        let dirty = if self.reach.len() == n { dirty } else { None };
        let (rx0, ry0, rx1, ry1, cx0, cy0, cx1, cy1) = match dirty {
            None => (0, 0, w - 1, h - 1, 0, 0, w - 1, h - 1),
            Some((a, b, c, d)) => {
                let (side, up, down, keep) = (SIDE, ARCH, 8 * CPM, 4 * CPM);
                let (cx0, cy0, cx1, cy1) = (a.saturating_sub(keep), b.saturating_sub(keep), (c + keep).min(w - 1), (d + keep).min(h - 1));
                (cx0.saturating_sub(side), cy0.saturating_sub(up), (cx1 + side).min(w - 1), (cy1 + down).min(h - 1), cx0, cy0, cx1, cy1)
            }
        };
        let (rw, rh) = (rx1 - rx0 + 1, ry1 - ry0 + 1); let rn = rw * rh;
        let li = |x: usize, y: usize| (y - ry0) * rw + (x - rx0);
        let st = |i: usize| g.is_static(i);
        let supports = |i: usize| { let k = g.m(i).kind(); k == Kind::Powder || (k == Kind::Solid) };
        let open = |i: usize| matches!(g.m(i).kind(), Kind::Empty | Kind::Gas);
        // 1. overburden (sliding ARCH-cell column, started ARCH rows above the region)
        let p = &mut self.overburden;
        for x in rx0..=rx1 {
            let mut acc = 0f32;
            // Ground arching (Terzaghi): load deeper than ARCH cells above is carried by the rock arch
            // around the opening, so only a sliding column of ARCH cells bears down on a cell.
            let cw = |y: usize| { let m = g.m(y * w + x); if matches!(m.kind(), Kind::Empty | Kind::Gas) { 0. } else { PROPS[m as usize].density * G * CELL_M / 1000. } };
            let ys = ry0.saturating_sub(ARCH);
            for y in ys..=ry1 { let i = y * w + x; if y >= ry0 { p[i] = acc; } acc += cw(y); if y >= ARCH && y - ARCH >= ys { acc -= cw(y - ARCH); } }
        }
        let p = &self.overburden;
        // ctop: top row of the unbroken static column a static cell belongs to (does it reach the roof?);
        // vtop: for an open cell, the row of the first non-open cell above it (the roof over that void).
        // The region's top row looks up through the grid for its true values.
        // Stored between solves. A full solve fills them; a region solve redoes its own columns from the
        // region top down, past the region's bottom until nothing changes (a column's top can move below it).
        let full = dirty.is_none();
        { let (ct, vt) = (&mut self.ctop, &mut self.vtop);
          for x in rx0..=rx1 { for y in ry0..h { let i = y * w + x;
            let (c, v) = if st(i) { (if y > 0 && st(i - w) { ct[i - w] } else { y as u32 }, 0) }
                else if open(i) { (0, if y == 0 { 0 } else if open(i - w) { vt[i - w] } else { (y - 1) as u32 }) } else { (0, 0) };
            if !full && y > ry1 && ct[i] == c && vt[i] == v { break; }
            ct[i] = c; vt[i] = v;
          } } }
        let (ctop, vtop) = (&self.ctop, &self.vtop);
        let inr = |x: usize, y: usize| x >= rx0 && x <= rx1 && y >= ry0 && y <= ry1;
        let ctop_at = |x: usize, y: usize| ctop[y * w + x];
        let vtop_at = |x: usize, y: usize| vtop[y * w + x];
        let mut util = vec![0f32; rn];
        // 2. connectivity (BFS from bedrock)
        let mut reach = vec![false; rn]; let mut q: Vec<u32> = Vec::with_capacity(rn / 4);
        // anchors: bedrock walls, the window's top and bottom rows (the Deepholt continues beyond them),
        // and for a partial solve the region's edge cells that had a load path last time
        let edge = |x: usize, y: usize| (x == rx0 && rx0 > 0) || (x == rx1 && rx1 + 1 < w) || (y == ry0 && ry0 > 0) || (y == ry1 && ry1 + 1 < h);
        for y in ry0..=ry1 { for x in rx0..=rx1 { let i = y * w + x;
            if g.mat[i] == Mat::Bedrock as u8 || ((i < w || i >= n - w) && st(i)) || (dirty.is_some() && edge(x, y) && self.reach[i] && supports(i)) { reach[li(x, y)] = true; q.push(i as u32); } } }
        let mut head = 0;
        while head < q.len() {
            let i = q[head] as usize; head += 1; let (x, y) = (i % w, i / w);
            // static cells carry each other in all four directions; loose ground (sand, rubble) only
            // carries what rests directly on top of it, so a pillar built on a sand floor has a load path
            let carried = g.is_static(i);
            if carried { for j in g.n4(x, y) { let (jx, jy) = (j % w, j / w); if inr(jx, jy) && !reach[li(jx, jy)] && st(j) { reach[li(jx, jy)] = true; q.push(j as u32); } } }
            if y > ry0 { let j = i - w; if !reach[li(x, y - 1)] && supports(j) { reach[li(x, y - 1)] = true; q.push(j as u32); } }
        }
        for y in ry0..=ry1 { for x in rx0..=rx1 { if st(y * w + x) && !reach[li(x, y)] { util[li(x, y)] = 9.; } } }
        // 3. bending of roof runs
        for y in ry0..=ry1.min(h - 2) {
            let beam = |x: usize| st(y * w + x) && !supports(y * w + x + w);
            let mut x = rx0; while x > 0 && beam(x) && beam(x - 1) { x -= 1; } // a run reaching in from the left
            while x <= rx1 {
                if !beam(x) { x += 1; continue; }
                let xa = x; while x < w && beam(x) { x += 1; } let xb = x - 1;
                let left = xa > 0 && st(y * w + xa - 1) && supports(y * w + xa - 1 + w);
                let right = xb + 1 < w && st(y * w + xb + 1) && supports(y * w + xb + 1 + w);
                let span = (xb - xa + 1) as f32 * CELL_M;
                let mut hmin = usize::MAX; let mut rho = 0f32; let mut tmin = f32::MAX; let mut qtop = 0f32;
                let cap = ((xb - xa + 1) + 1).min(80 * S);
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
                    if !inr(xx, y) { continue; } let j = li(xx, y); let uu = u * f.min(1.); if uu > util[j] { util[j] = uu; }
                }
            }
        }
        // 4. compression of horizontal solid runs (pillars / walls)
        for y in ry0.max(1)..=ry1 {
            let mut x = rx0; while x > 0 && st(y * w + x) && st(y * w + x - 1) { x -= 1; } // a run reaching in from the left
            while x <= rx1 {
                if !st(y * w + x) { x += 1; continue; }
                let xa = x; while x < w && st(y * w + x) { x += 1; } let xb = x - 1;
                let mut load: f32 = (xa..=xb).map(|xx| p[y * w + xx]).sum();
                // Tributary load from the open space beside the run, but only if the run's static column
                // actually reaches up to that void's roof (a free-standing stub carries no roof). Above
                // an opening of width n the rock arches, so only the loosened zone (≈ n+1 cells of rock)
                // bears down on the supports: a lone prop in a sound hall isn't crushed.
                let rt = (xa..=xb).map(|xx| ctop_at(xx, y)).min().unwrap_or(0);
                let cell_w = PROPS[Mat::Granite as usize].density * G * CELL_M / 1000.;
                let mut nl = 0; while xa > nl && !st(y * w + xa - nl - 1) && nl < 400 * S { nl += 1; }
                let mut nr = 0; while xb + nr + 1 < w && !st(y * w + xb + nr + 1) && nr < 400 * S { nr += 1; }
                let (cl, cr) = ((nl + 1) as f32 * cell_w, (nr + 1) as f32 * cell_w);
                let trib = |xx: usize, cap: f32| { let j = y * w + xx; if open(j) && rt > vtop_at(xx, y) { 0. } else { p[j].min(cap) } };
                let vl: f32 = (1..=nl).map(|k| trib(xa - k, cl)).sum();
                let vr: f32 = (1..=nr).map(|k| trib(xb + k, cr)).sum();
                load += 0.5 * (vl + vr);
                let wid = (xb - xa + 1) as f32;
                let sigma = load / wid;
                // Confinement (Mohr-Coulomb, σ3 ≈ K0·σv): a cell held in on both sides by the rest
                // of the run can't crush; only cells at a free face (the run's ends) see the
                // unconfined strength. So an overloaded body spalls at its faces instead of the
                // whole mass turning to rubble at once.
                for xx in [xa, xb] { if !inr(xx, y) { continue; } let j = li(xx, y); let u = sigma / PROPS[g.mat[y * w + xx] as usize].compressive.max(1e-3); if u > util[j] { util[j] = u; } }
                if xb > xa + 1 { for xx in (xa + 1).max(rx0)..xb.min(rx1 + 1) { let j = li(xx, y); let u = 0.5 * sigma / PROPS[g.mat[y * w + xx] as usize].compressive.max(1e-3); if u > util[j] { util[j] = u.min(0.9); } } }
            }
        }
        if self.reach.len() != n { self.reach = vec![false; n]; }
        let mut fail = vec![];
        for y in cy0..=cy1 { for x in cx0..=cx1 {
            let (i, l) = (y * w + x, li(x, y)); let u = util[l];
            self.reach[i] = reach[l];
            self.util[i] = (u * 128.).min(255.) as u8;
            if u > self.thr && g.mat[i] != Mat::Bedrock as u8 { fail.push(i as u32); }
        } }
        fail
    }

    /// Start crack timers on failing cells.
    pub fn crack(&mut self, fail: &[u32]) {
        self.last_fail = fail.len();
        // cells with no load path at all (a clump left hanging on water or air) give way almost at
        // once; overloaded ones creak for CRACK_TICKS first so there's time to react
        for &i in fail { let i = i as usize; if self.pending[i] == 0 { self.pending[i] = if self.reach.get(i) == Some(&false) { LOOSE_TICKS } else { CRACK_TICKS }; self.cracking.push(i as u32); } }
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
