//! Blasts: a pressure wave instead of a fixed crater.
//!
//! A charge of `kg` of TNT-equivalent releases its energy at a point. The wave runs out through open
//! air, gas and liquid, shortest path first (Dijkstra over the 8 neighbours). Solids stop it, so
//! anything round a corner only gets what diffracts round the edge, and a sealed wall shadows
//! completely. Its peak overpressure follows the Kinney-Graham free-air curve
//!
//!   P / P_atm = 808 (1 + (Z/4.5)²) / √((1 + (Z/0.048)²)(1 + (Z/0.32)²)(1 + (Z/1.35)²)),   Z = R / W^⅓,
//!
//! with one change: R is not the distance travelled but the *equivalent radius of the volume the wave has
//! filled* by the time it gets there, R = (3V / 4π)^⅓. A tunnel fills slowly, so the pressure stays high
//! far along it; a big cave fills fast and soaks it up. Liquid barely yields, so a cell of water counts
//! as a fifth of a cell of air: a blast carries further through a flooded tunnel.
//!
//! In a closed space the hot gas also pushes: P = max(shock, E·GAS_SHARE / V), so a charge tamped into a
//! tight pocket breaks far more rock than one going off in the open.
//!
//! Each solid face the wave reaches is loaded with the local overpressure. Rock breaks when that beats its
//! blast strength (from the material table's blast resistance). Far over that it shatters into open space
//! and the wave goes on into the crater; just over it, it cracks into loose rubble. Loose ground and liquids in the wave's path are thrown away from the charge, loose items fly
//! along the wave, and moles take damage from the overpressure where they stand (behind a wall, little or
//! none). Nearby charges go off, fuse cords light, and the knock is heard by listeners like a thrum.
//! Deterministic, and bounded: at most `BLAST_CELLS` cells, and the wave stops below `BLAST_FLOOR`.
use crate::grid::hash3;
use crate::mat::{Kind, Mat, CELL_M, CPM, S, SF};
use crate::world::{Event, World};

/// A TNT charge, kg TNT-equivalent.
pub const TNT_KG: f32 = 0.25;
/// A material's blast strength (kPa) from its blast resistance in the material table: 15 kPa × 2^blast
/// (dirt 60, clay 120, limestone 480, brick 960, basalt 1920, granite 3840; bedrock never breaks).
/// A face cracks into loose rubble above BLAST_CRACK × that, and shatters away above BLAST_SHATTER ×.
pub fn blast_strength(m: Mat) -> f32 { if m == Mat::Bedrock { f32::INFINITY } else { 15. * 2f32.powi(m.p().blast as i32) } }
pub const BLAST_CRACK: f32 = 1.5; pub const BLAST_SHATTER: f32 = 3.;
/// TNT's energy (kJ/kg) and the share that stays as hot gas pushing on the walls of a closed space: in a
/// tight pocket that quasi-static gas pressure, E·share / V, outlasts and outweighs the shock.
pub const TNT_KJ: f32 = 4184.; pub const GAS_SHARE: f32 = 0.35;
/// Diffraction: overpressure falls by 1 / (1 + detour / DIFFRACT_M), detour = how much longer the wave's path
/// is than the straight line (m). A straight tunnel loses nothing; round a wall's edge it loses a lot.
pub const DIFFRACT_M: f32 = 0.5;
/// The wave stops spreading below this overpressure (kPa), or after this many cells.
pub const BLAST_FLOOR: f32 = 3.;
pub const BLAST_CELLS: usize = 400_000;
const P_ATM: f32 = 101.3;

/// Kinney-Graham peak overpressure (kPa) at scaled distance z = R / W^⅓ (m/kg^⅓).
pub fn overpressure(z: f32) -> f32 {
    let z = z.max(0.02);
    P_ATM * 808. * (1. + (z / 4.5).powi(2)) / ((1. + (z / 0.048).powi(2)) * (1. + (z / 0.32).powi(2)) * (1. + (z / 1.35).powi(2))).sqrt()
}

/// What a blast did, for the caller and the tests.
#[derive(Default, Debug, Clone)]
pub struct BlastReport { pub cells: usize, pub shattered: usize, pub cracked: usize, pub moved: usize, pub reach_m: f32 }

impl World {
    /// Set off `kg` of TNT-equivalent at cell (cx, cy).
    pub fn blast(&mut self, cx: usize, cy: usize, kg: f32) -> BlastReport {
        let (w, h) = (self.g.w, self.g.h); let n = w * h;
        if cx >= w || cy >= h || kg <= 0. { return BlastReport::default(); }
        let w3 = kg.cbrt(); let cell_v = CELL_M * CELL_M; // m³ per cell (1 m deep)
        // per reached cell: overpressure, and the direction the wave arrived from
        let mut p = vec![0f32; n]; let mut dir = vec![(0i8, 0i8); n];
        let mut dist = vec![f32::INFINITY; n];
        let mut heap = std::collections::BinaryHeap::new();
        let key = |d: f32| std::cmp::Reverse((d * 64.) as u32);
        let i0 = cy * w + cx; dist[i0] = 0.; heap.push((key(0.), i0 as u32));
        let mut vol = 0f32; let mut rep = BlastReport::default();
        let mut faces: Vec<(usize, f32, (i8, i8))> = vec![]; // solids and loose ground the wave hit: (cell, P, direction)
        let mut done = vec![false; n];
        let passes = |m: Mat| matches!(m.kind(), Kind::Empty | Kind::Gas | Kind::Liquid);
        while let Some((std::cmp::Reverse(_), i)) = heap.pop() {
            let i = i as usize; if done[i] { continue; } done[i] = true;
            let m = self.g.m(i);
            vol += cell_v * if m.kind() == Kind::Liquid { 0.2 } else { 1. };
            let r_eq = (3. * vol / (4. * std::f32::consts::PI)).cbrt().max(0.1);
            // diffraction: the further the wave has had to detour round rock to get here (path length over
            // the straight line), the more it has lost bending round edges, so a wall shadows what's behind it
            let (x, y) = (i % w, i / w);
            let straight = (((x as f32 - cx as f32).powi(2) + (y as f32 - cy as f32).powi(2)).sqrt());
            let detour_m = (dist[i] - straight).max(0.) * CELL_M;
            let pi = overpressure(r_eq / w3).max(TNT_KJ * GAS_SHARE * kg / vol.max(cell_v)) / (1. + detour_m / DIFFRACT_M); p[i] = pi; rep.cells += 1;
            rep.reach_m = rep.reach_m.max(dist[i] * CELL_M);
            if pi < BLAST_FLOOR || rep.cells >= BLAST_CELLS { continue; }
            for (dx, dy, c) in [(1i32, 0i32, 1f32), (-1, 0, 1.), (0, 1, 1.), (0, -1, 1.), (1, 1, 1.414), (-1, 1, 1.414), (1, -1, 1.414), (-1, -1, 1.414)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy); if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h { continue; }
                let j = ny as usize * w + nx as usize; if done[j] { continue; }
                let mj = self.g.m(j);
                if passes(mj) || (j == i0) {
                    let d = dist[i] + c; if d < dist[j] { dist[j] = d; dir[j] = (dx as i8, dy as i8); heap.push((key(d), j as u32)); }
                } else if dx == 0 || dy == 0 {
                    // a solid (or loose) face: loaded with the overpressure here
                    if mj == Mat::Bedrock { continue; }
                    let brk = blast_strength(mj);
                    let loose = mj.kind() == Kind::Powder || self.g.loose(j);
                    if !loose && pi > BLAST_SHATTER * brk {
                        // shattered: it opens, and the wave runs on into the crater
                        let fire = pi > 2000. && hash3(nx as u32, ny as u32, self.tick as u32) % 9 == 0;
                        self.g.set(j, if fire { Mat::Fire } else { Mat::Empty }); rep.shattered += 1;
                        let d = dist[i] + c; dist[j] = d; dir[j] = (dx as i8, dy as i8); heap.push((key(d), j as u32));
                    } else { faces.push((j, pi, (dx as i8, dy as i8))); }
                }
            }
        }
        // faces: crack rock that the load beats; loose ground is thrown instead
        faces.sort_by_key(|f| f.0); faces.dedup_by(|a, b| { if a.0 == b.0 { b.1 = b.1.max(a.1); true } else { false } });
        let mut throw: Vec<(usize, f32, (i8, i8))> = vec![];
        for &(j, pj, d) in &faces {
            let m = self.g.m(j); if m == Mat::Empty { continue; }
            let loose = m.kind() == Kind::Powder || self.g.loose(j);
            if loose { throw.push((j, pj, d)); continue; }
            if pj > BLAST_CRACK * blast_strength(m) && m.kind() == Kind::Solid {
                let c = m.p().crumbles_to; self.g.set(j, c); if c.kind() == Kind::Solid { self.g.set_loose(j); } rep.cracked += 1;
                throw.push((j, pj, d));
            }
        }
        // liquids in the wave are thrown too (the charge's own cell outward and up)
        for i in 0..n { if p[i] > 20. && self.g.m(i).kind() == Kind::Liquid { throw.push((i, p[i], if dir[i] == (0, 0) { (0, -1) } else { dir[i] })); } }
        // throw: furthest from the charge first, so the near ones have somewhere to go
        let (fx, fy) = (cx as f32, cy as f32);
        throw.sort_by(|a, b| { let da = ((a.0 % w) as f32 - fx).powi(2) + ((a.0 / w) as f32 - fy).powi(2); let db = ((b.0 % w) as f32 - fx).powi(2) + ((b.0 / w) as f32 - fy).powi(2); db.total_cmp(&da).then(a.0.cmp(&b.0)) });
        for (j, pj, (ddx, ddy)) in throw {
            if ddx == 0 && ddy == 0 { continue; }
            let reach = ((pj / 25.) * SF).min(3. * CPM as f32) as i32; // about a tile per 25 kPa, at most 3 m
            let liquid = self.g.m(j).kind() == Kind::Liquid;
            // along the wave to the furthest open cell within reach; water can be driven through water (the
            // pool heaves) and, if that's blocked, it goes up in a plume
            let ray = |g: &crate::grid::Grid, ddx: i32, ddy: i32| -> Option<usize> {
                let (mut x, mut y) = ((j % w) as i32, (j / w) as i32); let mut best = None;
                for _ in 0..reach { x += ddx; y += ddy; if !g.in_bounds(x, y) { break; } let t = y as usize * w + x as usize; let k = g.m(t).kind();
                    if matches!(k, Kind::Empty | Kind::Gas) { best = Some(t); } else if !(liquid && k == Kind::Liquid) { break; } }
                best };
            let tgt = ray(&self.g, ddx as i32, ddy as i32).or_else(|| if liquid { ray(&self.g, ddx as i32, -1).or_else(|| ray(&self.g, 0, -1)) } else { None });
            let Some(t) = tgt else { continue };
            let (m, f) = (self.g.mat[j], self.g.flags[j]);
            if Mat::from_u8(m).kind() == Kind::Solid && f & crate::grid::F_LOOSE == 0 { continue; }
            self.g.set(t, Mat::from_u8(m)); self.g.flags[t] = f; self.g.set(j, Mat::Empty); rep.moved += 1;
        }
        // loose items fly along the wave
        for d in self.drops.iter_mut() {
            let (x, y) = (d.px.floor() as i32, d.py.floor() as i32); if !self.g.in_bounds(x, y) { continue; }
            let i = y as usize * w + x as usize; let pi = p[i]; if pi < 5. { continue; }
            let (ux, uy) = dir[i]; let l = ((ux * ux + uy * uy) as f32).sqrt().max(1.);
            let k = (pi / 5.).min(70.) * SF; d.vx += ux as f32 / l * k; d.vy += uy as f32 / l * k - 0.2 * k; d.rest = 0;
        }
        // moles: hurt by the overpressure where they stand, and knocked along the wave
        let body = |x: usize, y: usize| -> (f32, (i8, i8)) { let mut best = (0f32, (0i8, 0i8)); for k in 0..3 * S { if y >= k { let i = (y - k) * w + x; if p[i] > best.0 { best = (p[i], dir[i]); } } } best };
        let hits: Vec<(f32, (i8, i8))> = self.moles.iter().map(|m| if m.alive { body(m.x, m.y) } else { (0., (0, 0)) }).collect();
        for (k, &(pm, d)) in hits.iter().enumerate() {
            if pm <= 0. { continue; }
            self.moles[k].hp -= ((pm - 30.) / 2.5).max(0.); // a knock under 30 kPa; about 280 kPa is deadly
            if k == 0 { let l = ((d.0 as f32).powi(2) + (d.1 as f32).powi(2)).sqrt().max(1.); let kv = (pm / 8.).min(40.) * SF;
                self.pl.vx += d.0 as f32 / l * kv; self.pl.vy += d.1 as f32 / l * kv - 0.3 * kv; self.pl.lock = self.pl.lock.max(0.15); }
        }
        // devices: charges go off, fuse cords light, the rest can be wrecked
        self.logic.on_blast(&mut self.g, &p);
        // the knock carries through the rock to listeners
        self.logic.blast_heard(&self.g, cx, cy, 60. + 20. * (kg / 0.005).log10());
        let pm = hits.first().map_or(0., |h| h.0);
        if self.keep_blast_field { self.blast_field = p; }
        self.events.push(Event::Explode); self.shake = self.shake.max((0.5 + w3 * 2.).min(4.)).max((pm / 40.).min(4.));
        rep
    }
}
