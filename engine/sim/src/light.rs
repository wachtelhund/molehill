//! Per-cell RGB light. There is no sky: everything a Delver sees is lit by something down here.
//!
//! Two kinds of light, added together:
//! * **Glow** from glowing materials (lava, crystal, fungus, fire): seeded in the cells themselves and
//!   spread by a two-sweep 8-neighbour chamfer pass, L(i) = max(L(i), L(n)·t(i)), with t the cell's
//!   transmittance (open air 0.90 per legacy tile, rock about 0.3, so light just licks a wall face).
//! * **Surface sources** (`surface_sources`): a glowing face that touches open air casts rays too: hard and
//!   bright for lava and fire, soft and a little further-reaching for glowing rock and growth (mood light).
//! * **Sources** (`Source`): torches, lamps, kilns, the mole's headlamp, a torch in its paw. Each casts
//!   rays out to its radius. A ray stops at the first solid cell (lighting that face), dims in liquid,
//!   and fades smoothly, (1 − d/R)² for a point light (a bright core and a soft pool) and (1 − (d/R)²)²
//!   for a beam (it throws further), so the edge of a light pool is soft. A cone source (the
//!   headlamp) only casts inside its beam and softens toward the beam's edges. Sources add up, so
//!   colours mix. A short, steep bounce pass lets direct light round an edge a little without filling
//!   the shadows behind a pillar.
//!
//! Computed only over a rectangle (camera view + margin), so cost is bounded. Ray counts scale with
//! the radius, so the rim has no gaps at any cell size.
use crate::grid::{Grid, F_SEEN};
use crate::mat::{Kind, Mat, PROPS, SF, CPM};

/// A light source. Position and radius are in cells; `rgb` is the brightness at the centre (1 ≈ full).
#[derive(Clone, Copy, Debug)]
pub struct Source { pub x: f32, pub y: f32, pub rgb: [f32; 3], pub radius: f32, pub cone: Option<Cone> }
/// A beam: unit direction (y down) and half-angle in radians.
#[derive(Clone, Copy, Debug)]
pub struct Cone { pub dx: f32, pub dy: f32, pub half: f32 }

impl Source {
    pub fn point(x: f32, y: f32, rgb: [f32; 3], radius_m: f32) -> Source { Source { x, y, rgb, radius: radius_m * CPM as f32, cone: None } }
    pub fn beam(x: f32, y: f32, rgb: [f32; 3], radius_m: f32, dx: f32, dy: f32, half: f32) -> Source {
        let l = (dx * dx + dy * dy).sqrt().max(1e-6);
        Source { x, y, rgb, radius: radius_m * CPM as f32, cone: Some(Cone { dx: dx / l, dy: dy / l, half }) }
    }
}

/// Light recipes, in metres. One place to tune how far each thing reaches.
pub mod kinds {
    pub const TORCH: ([f32; 3], f32) = ([1.5, 1.08, 0.56], 7.5);
    pub const HELD_TORCH: ([f32; 3], f32) = ([1.35, 0.98, 0.5], 6.5);
    pub const LAMP: ([f32; 3], f32) = ([1.25, 1.2, 0.88], 9.);
    pub const KILN: ([f32; 3], f32) = ([1.35, 0.58, 0.2], 6.);
    /// The headlamp burns fungus oil: a warm light with a faint green cast.
    pub const HEADLAMP: ([f32; 3], f32) = ([1.15, 1.12, 0.74], 12.);
    pub const HEADLAMP_HALF: f32 = 0.5; // ≈ 29°
    /// The small glow round the lamp itself (and the mole's own faint glow when the lamp is out).
    pub const LAMP_GLOW: ([f32; 3], f32) = ([0.42, 0.40, 0.3], 3.);
    pub const MOLE_GLOW: ([f32; 3], f32) = ([0.26, 0.23, 0.2], 2.);
    /// A fungus planter: a soft green glow.
    pub const PLANTER: ([f32; 3], f32) = ([0.35, 0.75, 0.45], 3.);
    /// A glowmite drifting in a grotto: a small cold green-gold glow. A jar of one carried in the paw.
    pub const GLOWMITE: ([f32; 3], f32) = ([0.42, 0.62, 0.3], 2.5);
    pub const GLOWMITE_JAR: ([f32; 3], f32) = ([0.6, 0.85, 0.42], 4.);
    /// A charge going off: a short warm flash at the blast (peak colour, reach in metres), so a blast in an
    /// unlit drift shows the room and what it throws for a moment. It fades out as (1 − t/T)² over
    /// BLAST_FLASH_S and casts the same shadows as any light (it stops at the first wall).
    pub const BLAST_FLASH: ([f32; 3], f32) = ([1.5, 1.1, 0.7], 10.);
    pub const BLAST_FLASH_S: f32 = 0.35;
}

/// Seed of the local glow flood per unit of emission (the rays carry the real light).
const GLOW: f32 = 0.45 / 170.;
/// Glowing rock and growth as a source: brightness per unit of emission (lava and fire: 0.8) and extra reach (m).
/// A hall studded with crystal glows cold and even instead of blowing out.
pub const SOFT_GLOW: f32 = 0.32; pub const SOFT_REACH_M: f32 = 1.5;

/// Is the glowing cell that lights (x, y) lava or fire (it hardly matters which neighbour: a pool's air)?
fn near_fluid_glow(g: &Grid, x: usize, y: usize) -> bool {
    let i = y * g.w + x; [i + g.w, i.wrapping_sub(1), i + 1, i.wrapping_sub(g.w)].into_iter().filter(|&j| j < g.mat.len())
        .any(|j| { let m = g.m(j); m.p().emit != [0; 3] && matches!(m.kind(), Kind::Liquid | Kind::Gas) })
}

/// Glowing surfaces as ray-cast sources: every 2 m square (in `r`, grown by `margin` cells) holding a
/// glowing cell (lava, crystal, fungus, fire) that touches open air gets one point source in that
/// air, coloured by the material, reaching further the brighter it glows (2–5 m). So a lava pool lights
/// a cave with real shadows, like a torch does.
pub fn surface_sources(g: &Grid, r: (usize, usize, usize, usize), margin: usize) -> Vec<Source> {
    use std::collections::HashMap;
    let (x0, y0) = (r.0.saturating_sub(margin), r.1.saturating_sub(margin)); let (x1, y1) = ((r.2 + margin).min(g.w - 1), (r.3 + margin).min(g.h - 1));
    let b = 2 * CPM; let mut best: HashMap<(usize, usize), (u32, usize, usize, [u8; 3])> = HashMap::new();
    let open = |i: usize| matches!(g.m(i).kind(), Kind::Empty | Kind::Gas);
    for y in y0.max(1)..y1 { for x in x0.max(1)..x1 {
        let i = y * g.w + x; let e = PROPS[g.mat[i] as usize].emit; let sum = e[0] as u32 + e[1] as u32 + e[2] as u32; if sum == 0 { continue; }
        // the open neighbour the light leaves through (above first: pools and floors light upward)
        let Some(o) = [i - g.w, i - 1, i + 1, i + g.w].into_iter().find(|&j| open(j)) else { continue };
        let k = (x / b, y / b); if best.get(&k).is_none_or(|v| v.0 < sum) { best.insert(k, (sum, o % g.w, o / g.w, e)); }
    } }
    best.into_values().map(|(sum, x, y, e)| {
        // molten and burning things (lava, fire) throw a hard bright light; glowing rock and growth (crystal,
        // mycelium, fungus) a softer one that carries a little further: mood light, not a lamp
        let soft = !matches!(g.m(y * g.w + x).kind(), Kind::Liquid | Kind::Gas) && !near_fluid_glow(g, x, y);
        let (k, r) = if soft { (SOFT_GLOW / 170., (sum as f32 / 100.).clamp(2., 5.) + SOFT_REACH_M) } else { (0.8 / 170., (sum as f32 / 100.).clamp(2., 5.)) };
        Source::point(x as f32 + 0.5, y as f32 + 0.5, [e[0] as f32 * k, e[1] as f32 * k, e[2] as f32 * k], r) }).collect()
}

pub struct Light {
    pub w: usize, pub h: usize, pub rgb: Vec<[f32; 3]>, pub rect: (usize, usize, usize, usize),
    trans: [f32; crate::mat::NMAT], diag: f32, bounce: f32, liquid: f32,
    direct: Vec<[f32; 3]>, scratch: Vec<f32>,
}

impl Light {
    pub fn new(w: usize, h: usize) -> Self {
        let mut trans = [0.; crate::mat::NMAT];
        for (i, p) in PROPS.iter().enumerate() {
            trans[i] = match p.kind { Kind::Empty | Kind::Gas => 0.90, Kind::Liquid => 0.80, _ => (1. - p.opacity as f32 / 255.) * 0.55 };
        }
        trans[Mat::Glass as usize] = 0.88;
        // the factors above are per legacy 0.25 m tile: per cell they're the SF-th root, so light reaches as far in metres
        for t in trans.iter_mut() { *t = t.powf(1. / SF); }
        Light { w, h, rgb: vec![[0.; 3]; w * h], rect: (0, 0, 0, 0), trans, diag: 0.93f32.powf(1. / SF), bounce: 0.5f32.powf(1. / SF), liquid: 0.9f32.powf(1. / SF),
            direct: vec![[0.; 3]; w * h], scratch: vec![] }
    }
    #[inline] pub fn at(&self, i: usize) -> [f32; 3] { self.rgb[i] }

    /// Recompute inside rect (x0,y0)-(x1,y1) inclusive from glowing materials and `sources`.
    pub fn compute(&mut self, g: &Grid, r: (usize, usize, usize, usize), sources: &[Source]) {
        let (x0, y0, x1, y1) = (r.0, r.1, r.2.min(g.w - 1), r.3.min(g.h - 1));
        self.rect = (x0, y0, x1, y1);
        let w = self.w;
        // glow: seed the glowing cells, then spread
        // (a faint local glow only: the light a glowing surface throws into the open is cast as rays, see `surface_sources`)
        for y in y0..=y1 { for x in x0..=x1 { let i = y * w + x; let e = PROPS[g.mat[i] as usize].emit; self.rgb[i] = [e[0] as f32 * GLOW, e[1] as f32 * GLOW, e[2] as f32 * GLOW]; self.direct[i] = [0.; 3]; } }
        let t = self.trans; let dg = self.diag;
        Self::spread(&mut self.rgb, g, (x0, y0, x1, y1), w, |m| t[m], dg, 2);
        // direct light from every source that reaches the rect
        for s in sources {
            let rr = s.radius;
            if s.x + rr < x0 as f32 || s.x - rr > x1 as f32 + 1. || s.y + rr < y0 as f32 || s.y - rr > y1 as f32 + 1. { continue; }
            self.cast(g, s, (x0, y0, x1, y1));
        }
        // bounce: direct light creeps a little round edges (steep falloff keeps shadows)
        let b = self.bounce; let tr = self.trans;
        Self::spread(&mut self.direct, g, (x0, y0, x1, y1), w, |m| if tr[m] > 0.5 { b } else { tr[m] * 0.6 }, dg, 1);
        for y in y0..=y1 { for x in x0..=x1 { let i = y * w + x; for k in 0..3 { self.rgb[i][k] = (self.rgb[i][k] + self.direct[i][k]).min(2.2); } } }
    }

    /// Chamfer max-propagation in place over `r`, `iters` forward+backward sweep pairs.
    fn spread(buf: &mut [[f32; 3]], g: &Grid, r: (usize, usize, usize, usize), w: usize, tf: impl Fn(usize) -> f32, dg: f32, iters: usize) {
        let (x0, y0, x1, y1) = r;
        for _ in 0..iters {
            for y in y0..=y1 { for x in x0..=x1 {
                let i = y * w + x; let t = tf(g.mat[i] as usize); let mut l = buf[i];
                let mut take = |j: usize, f: f32| { let s = buf[j]; for k in 0..3 { let v = s[k] * f; if v > l[k] { l[k] = v; } } };
                if x > x0 { take(i - 1, t); }
                if y > y0 { take(i - w, t); if x > x0 { take(i - w - 1, t * dg); } if x < x1 { take(i - w + 1, t * dg); } }
                buf[i] = l;
            } }
            for y in (y0..=y1).rev() { for x in (x0..=x1).rev() {
                let i = y * w + x; let t = tf(g.mat[i] as usize); let mut l = buf[i];
                let mut take = |j: usize, f: f32| { let s = buf[j]; for k in 0..3 { let v = s[k] * f; if v > l[k] { l[k] = v; } } };
                if x < x1 { take(i + 1, t); }
                if y < y1 { take(i + w, t); if x < x1 { take(i + w + 1, t * dg); } if x > x0 { take(i + w - 1, t * dg); } }
                buf[i] = l;
            } }
        }
    }

    /// Cast one source's rays into `direct`. Within a source the brightest ray through a cell wins
    /// (a per-source scratch square); across sources light adds.
    fn cast(&mut self, g: &Grid, s: &Source, r: (usize, usize, usize, usize)) {
        let rr = s.radius.max(1.); let side = (2. * rr).ceil() as usize + 3; let ox = s.x.floor() as i32 - side as i32 / 2; let oy = s.y.floor() as i32 - side as i32 / 2;
        self.scratch.clear(); self.scratch.resize(side * side, 0.);
        let (a0, span) = match s.cone { Some(c) => (c.dy.atan2(c.dx) - c.half, 2. * c.half), None => (0., std::f32::consts::TAU) };
        // one ray per rim cell is gap-free for short lights; long ones (torches, the beam) get 1.5
        let dens = if rr <= 6. * CPM as f32 { 1.0 } else { 1.5 }; let rays = ((span * rr * dens).ceil() as usize).max(8);
        let step = 0.7f32; let n = (rr / step) as usize;
        for k in 0..rays {
            let a = a0 + span * (k as f32 + 0.5) / rays as f32; let (dx, dy) = (a.cos(), a.sin());
            // a beam softens over the outer third of its half-angle
            let edge = match s.cone { Some(c) => { let off = ((a - a0) - c.half).abs() / c.half.max(1e-3); let e = ((1. - off) / 0.35).clamp(0., 1.); e * e * (3. - 2. * e) } None => 1. };
            let (mut x, mut y) = (s.x, s.y); let mut t = edge;
            for st in 0..=n {
                let (xi, yi) = (x.floor() as i32, y.floor() as i32);
                if !g.in_bounds(xi, yi) { break; }
                let d = st as f32 * step / rr; let v = t * if s.cone.is_some() { let f = (1. - d * d).max(0.); f * f } else { let f = (1. - d).max(0.); f * f };
                let (sx, sy) = (xi - ox, yi - oy);
                let m = g.m(yi as usize * g.w + xi as usize); let solid = matches!(m.kind(), Kind::Solid | Kind::Powder);
                // a rock face returns about half the light that reaches it
                let v = if solid && st > 0 { v * 0.5 } else { v };
                if sx >= 0 && sy >= 0 && (sx as usize) < side && (sy as usize) < side { let c = &mut self.scratch[sy as usize * side + sx as usize]; if v > *c { *c = v; } }
                match m.kind() { Kind::Solid | Kind::Powder => { if st > 0 { break; } } Kind::Liquid => t *= self.liquid, _ => {} }
                if t < 0.01 { break; }
                x += dx * step; y += dy * step;
            }
        }
        let (x0, y0, x1, y1) = r;
        for sy in 0..side { let yy = oy + sy as i32; if yy < y0 as i32 || yy > y1 as i32 { continue; }
            for sx in 0..side { let xx = ox + sx as i32; if xx < x0 as i32 || xx > x1 as i32 { continue; }
                let v = self.scratch[sy * side + sx]; if v <= 0. { continue; }
                let i = yy as usize * self.w + xx as usize; for k in 0..3 { self.direct[i][k] += s.rgb[k] * v; } } }
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
/// Reveal along a beam: like `reveal`, but only inside the cone (the headlamp shows you further ahead).
pub fn reveal_cone(g: &mut Grid, cx: f32, cy: f32, c: Cone, r: usize) -> usize {
    let mut n = 0; let rays = ((2. * c.half * r as f32 * 1.2).ceil() as usize).max(4); let a0 = c.dy.atan2(c.dx) - c.half;
    for k in 0..rays {
        let a = a0 + 2. * c.half * (k as f32 + 0.5) / rays as f32; let (dx, dy) = (a.cos(), a.sin());
        let (mut x, mut y) = (cx, cy);
        for _ in 0..r {
            let (xi, yi) = (x.floor() as i32, y.floor() as i32); if !g.in_bounds(xi, yi) { break; }
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
