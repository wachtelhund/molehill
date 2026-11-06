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
pub const TNT_KG: f32 = 0.5;
/// A material's blast strength (kPa), looked up by its blast resistance in the material table. A face
/// cracks into loose rubble above BLAST_CRACK × that, and shatters away above BLAST_SHATTER ×. Tuned so one
/// 0.5 kg charge in an open 0.75 m tunnel cracks limestone and soft rock out to about 2 m (a crater about a
/// metre deep), granite only within about half a metre, and basalt and iron hardly at all.
pub const BLAST_KPA: [f32; 12] = [20., 60., 150., 220., 300., 360., 700., 850., 1000., 2600., 6500., 9000.];
pub fn blast_strength(m: Mat) -> f32 { if m == Mat::Bedrock { f32::INFINITY } else { BLAST_KPA.get(m.p().blast as usize).copied().unwrap_or(f32::INFINITY) } }
pub const BLAST_CRACK: f32 = 1.0; pub const BLAST_SHATTER: f32 = 2.5;
/// The shock fades into rock as e^(−depth / λ) past the face it hits, λ = BLAST_PEN_M × (1 + 2·confinement),
/// confinement = rock the wave hit or shattered per open cell it filled (about 0.1 in a cave, 0.3 in a
/// tunnel, near 1 tamped into a pocket).
pub const BLAST_PEN_M: f32 = 0.8;
/// The push fades into a bed of loose ground (sand, gravel, rubble, ash) over this depth (m), and nothing
/// under LAUNCH_KPA is thrown.
pub const BLAST_LOOSE_M: f32 = 0.8; pub const LAUNCH_KPA: f32 = 8.;
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
        self.blasts_run += 1;
        let w3 = kg.cbrt(); let cell_v = CELL_M * CELL_M; // m³ per cell (1 m deep)
        // per reached cell: overpressure, and the direction the wave arrived from
        let mut p = vec![0f32; n]; let mut dir = vec![(0i8, 0i8); n];
        let mut dist = vec![f32::INFINITY; n];
        let mut heap = std::collections::BinaryHeap::new();
        let key = |d: f32| std::cmp::Reverse((d * 64.) as u32);
        let i0 = cy * w + cx; dist[i0] = 0.; heap.push((key(0.), i0 as u32));
        let mut vol = 0f32; let mut rep = BlastReport::default(); let mut wave = crate::debris::Wave { cx, cy, tick: self.tick, ..Default::default() };
        let mut faces: Vec<(usize, f32, (i8, i8))> = vec![]; // solids and loose ground the wave hit: (cell, P, direction)
        let mut done = vec![false; n]; let mut flying: Vec<(usize, f32, (i8, i8))> = vec![];
        let passes = |m: Mat| matches!(m.kind(), Kind::Empty | Kind::Gas | Kind::Liquid);
        while let Some((std::cmp::Reverse(_), i)) = heap.pop() {
            let i = i as usize; if done[i] { continue; } done[i] = true;
            let m = self.g.m(i);
            vol += cell_v * if m.kind() == Kind::Liquid { 0.2 } else { 1. };
            let r_eq = (3. * vol / (4. * std::f32::consts::PI)).cbrt().max(0.1);
            // diffraction: the further the wave has had to detour round rock to get here (path length over
            // the straight line), the more it has lost bending round edges, so a wall shadows what's behind it
            let (x, y) = (i % w, i / w);
            let straight = ((x as f32 - cx as f32).powi(2) + (y as f32 - cy as f32).powi(2)).sqrt();
            let detour_m = (dist[i] - straight).max(0.) * CELL_M;
            let pi = overpressure(r_eq / w3).max(TNT_KJ * GAS_SHARE * kg / vol.max(cell_v)) / (1. + detour_m / DIFFRACT_M); p[i] = pi; rep.cells += 1;
            if pi >= 5. && wave.cells.len() < crate::debris::WAVE_CELLS { wave.cells.push(crate::debris::WaveCell { i: i as u32, d: dist[i], p: pi, dx: dir[i].0, dy: dir[i].1 }); wave.reach = wave.reach.max(dist[i]); }
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
                        // a third of it flies off as rubble (thrown below), the rest is pulverised (no fire is left in the crater)
                        let h = hash3(nx as u32, ny as u32, self.tick as u32);
                        if h % 3 == 0 { let c = mj.p().crumbles_to; if c.kind() != Kind::Empty { self.g.set(j, c); if c.kind() == Kind::Solid { self.g.set_loose(j); } flying.push((j, pi, (dx as i8, dy as i8))); } else { self.g.set(j, Mat::Empty); } }
                        else { self.g.set(j, Mat::Empty); }
                        rep.shattered += 1;
                        let d = dist[i] + c; dist[j] = d; dir[j] = (dx as i8, dy as i8); heap.push((key(d), j as u32));
                    } else { faces.push((j, pi, (dx as i8, dy as i8))); }
                }
            }
        }
        // faces: crack rock that the load beats; loose ground is thrown instead. Each thrown cell carries
        // whether it is backed by rock (the far side of what it sits in is solid): backed ground can't be
        // driven on into the rock, so it is thrown back out of the surface (a floor sprays up)
        faces.sort_by_key(|f| f.0); faces.dedup_by(|a, b| { if a.0 == b.0 { b.1 = b.1.max(a.1); true } else { false } });
        let mut throw: Vec<(usize, f32, (i8, i8), bool, f32)> = flying.into_iter().map(|(j, pj, d)| (j, pj, d, true, 1.)).collect(); // (cell, P, direction, backed, speed factor)
        // confinement: how much rock the wave was boxed in by, for the air it filled. A tamped charge's gas
        // can't vent, so it cracks deeper into the rock round it; in a big cave the shock just spreads out.
        let confined = ((faces.len() + rep.shattered) as f32 / rep.cells.max(1) as f32).clamp(0., 1.);
        let pen = BLAST_PEN_M * (1. + 2. * confined);
        for &(j, pj, d) in &faces {
            let m = self.g.m(j); if m == Mat::Empty { continue; }
            let pj = pj * (1. + confined); // a boxed-in charge's gas keeps pushing on the walls
            let loose = m.kind() == Kind::Powder || self.g.loose(j);
            if loose {
                // a bed or pile of loose ground: the push runs on into it, fading over BLAST_LOOSE_M, and every
                // cell it still moves goes; it is backed if the pile ends against rock
                let (mut x, mut y) = ((j % w) as i32, (j / w) as i32); let mut run = vec![(j, pj)]; let mut backed = true;
                for k in 1..(2 * CPM) as i32 { x += d.0 as i32; y += d.1 as i32; if !self.g.in_bounds(x, y) { break; }
                    let t = y as usize * w + x as usize; let mt = self.g.m(t);
                    if matches!(mt.kind(), Kind::Empty | Kind::Gas) { backed = false; break; }
                    if !(mt.kind() == Kind::Powder || self.g.loose(t)) { break; }
                    let pk = pj * (-(k as f32) * CELL_M / BLAST_LOOSE_M).exp(); if pk < LAUNCH_KPA { break; } run.push((t, pk)); }
                for (t, pk) in run { throw.push((t, pk, d, backed, 1.)); }
                continue;
            }
            if pj > BLAST_CRACK * blast_strength(m) && m.kind() == Kind::Solid {
                let c = m.p().crumbles_to; self.g.set(j, c); if c.kind() == Kind::Solid { self.g.set_loose(j); } rep.cracked += 1;
                throw.push((j, pj, d, true, 0.9));
                // the shock runs on into the rock, fading with depth: it cracks (or shatters) on until it is too
                // weak for what it meets, so a crater has depth, not just a skin
                // (it runs radially, away from the charge, so a tamped charge breaks a round crater, not a cross)
                let (jx, jy) = ((j % w) as f32 + 0.5, (j / w) as f32 + 0.5); let (rx, ry) = (jx - cx as f32 - 0.5, jy - cy as f32 - 0.5);
                let rl = (rx * rx + ry * ry).sqrt(); let (ux, uy) = if rl < 0.5 { (d.0 as f32, d.1 as f32) } else { (rx / rl, ry / rl) };
                let mut last = j;
                for k in 1..(4 * CPM) as i32 { let (x, y) = ((jx + ux * k as f32).floor() as i32, (jy + uy * k as f32).floor() as i32); if !self.g.in_bounds(x, y) { break; }
                    let t = y as usize * w + x as usize; if t == last { continue; } last = t; let mt = self.g.m(t);
                    if mt.kind() == Kind::Empty { continue; } if mt.kind() != Kind::Solid || self.g.loose(t) { break; }
                    let pk = pj * (-(k as f32) * CELL_M / pen).exp(); let st = blast_strength(mt);
                    if pk > BLAST_SHATTER * st { self.g.set(t, Mat::Empty); rep.shattered += 1; }
                    else if pk > BLAST_CRACK * st { let c = mt.p().crumbles_to; self.g.set(t, c); if c.kind() == Kind::Solid { self.g.set_loose(t); } rep.cracked += 1; throw.push((t, pk, d, true, 0.9)); }
                    else { break; } }
            }
        }
        // liquids in the wave are thrown too: a pool is backed by its bed, so it heaves up in a plume
        for i in 0..n { if p[i] > LAUNCH_KPA && self.g.m(i).kind() == Kind::Liquid { throw.push((i, p[i], if dir[i] == (0, 0) { (0, 1) } else { dir[i] }, true, 0.8)); } }
        // throw: every cell leaves with a velocity from the push on it, away from the charge; backed ground
        // is thrown back out of its surface. A cell hash spreads the spray (deterministic).
        let (fx, fy) = (cx as f32 + 0.5, cy as f32 + 0.5);
        throw.sort_by_key(|t| t.0); throw.dedup_by_key(|t| t.0);
        for (j, pj, (ddx, ddy), backed, f) in throw {
            let mj = self.g.m(j);
            if mj.kind() == Kind::Solid && !self.g.loose(j) { continue; }
            let v = (pj / crate::debris::DEBRIS_KPA_PER_MPS).min(crate::debris::DEBRIS_MAX_MPS) * f; if v < 1.5 { continue; }
            let (x, y) = ((j % w) as f32 + 0.5, (j / w) as f32 + 0.5);
            let (mut ux, mut uy) = (x - fx, y - fy); let l = (ux * ux + uy * uy).sqrt();
            if l < 0.5 { ux = 0.; uy = -1.; } else { ux /= l; uy /= l; }
            let (nx, ny) = (ddx as f32, ddy as f32); let nl = (nx * nx + ny * ny).sqrt().max(1.); let (nx, ny) = (nx / nl, ny / nl);
            let dot = ux * nx + uy * ny; if backed && dot > 0. { ux -= 2. * dot * nx; uy -= 2. * dot * ny; }
            let hh = hash3(j as u32, self.tick as u32, 0xb1); let a = ((hh & 1023) as f32 / 1023. - 0.5) * 0.7; let sp = 0.75 + ((hh >> 10) & 255) as f32 / 255. * 0.5;
            let (ca, sa) = (a.cos(), a.sin()); let (ux, uy) = (ux * ca - uy * sa, ux * sa + uy * ca - 0.3);
            let l = (ux * ux + uy * uy).sqrt().max(1e-3);
            if self.launch(j, ux / l * v * sp, uy / l * v * sp) { rep.moved += 1; }
        }
        // loose items fly along the wave
        for d in self.drops.iter_mut() {
            let (x, y) = (d.px.floor() as i32, d.py.floor() as i32); if !self.g.in_bounds(x, y) { continue; }
            let i = y as usize * w + x as usize; let pi = p[i]; if pi < 5. { continue; }
            let (ux, uy) = dir[i]; let l = ((ux * ux + uy * uy) as f32).sqrt().max(1.);
            let k = (pi / 4.).min(90.) * SF; d.vx += ux as f32 / l * k; d.vy += uy as f32 / l * k - 0.25 * k; d.rest = 0;
        }
        // moles: hurt by the overpressure where they stand, and knocked along the wave
        let body = |x: usize, y: usize| -> (f32, (i8, i8)) { let mut best = (0f32, (0i8, 0i8)); for k in 0..3 * S { if y >= k { let i = (y - k) * w + x; if p[i] > best.0 { best = (p[i], dir[i]); } } } best };
        let hits: Vec<(f32, (i8, i8))> = self.moles.iter().map(|m| if m.alive { body(m.x, m.y) } else { (0., (0, 0)) }).collect();
        for (k, &(pm, d)) in hits.iter().enumerate() {
            if pm <= 0. { continue; }
            self.moles[k].hp -= ((pm - 40.) / 3.).max(0.); // a knock under 40 kPa; about 340 kPa is deadly
            if k == 0 && pm > 40. { self.pl.harm = crate::world::Harm::Blast; }
            if k == 0 { let l = ((d.0 as f32).powi(2) + (d.1 as f32).powi(2)).sqrt().max(1.); let kv = (pm / 8.).min(40.) * SF;
                self.pl.vx += d.0 as f32 / l * kv; self.pl.vy += d.1 as f32 / l * kv - 0.3 * kv; self.pl.lock = self.pl.lock.max(0.15); }
        }
        // devices: charges go off, fuse cords light, the rest can be wrecked
        self.logic.on_blast(&mut self.g, &p);
        // the knock carries through the rock to listeners
        self.logic.blast_heard(&self.g, cx, cy, 60. + 20. * (kg / 0.005).log10());
        let pm = hits.first().map_or(0., |h| h.0);
        if self.keep_blast_field { self.blast_field = p; }
        self.waves.push(wave); if self.waves.len() > 4 { self.waves.remove(0); }
        // feel: a hard jolt (bigger charges, and more the closer the mole), and the ground rumbles
        self.events.push(Event::Explode); self.events.push(Event::Quake); if rep.cracked + rep.shattered > 0 { self.events.push(Event::Crumble); }
        self.shake = self.shake.max((1.2 + w3 * 2.5).min(5.)).max((pm / 25.).min(6.));
        rep
    }
}
