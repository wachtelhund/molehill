//! Stale air. CO2 concentration (0 = fresh, 1 = lethal) on an 8×8-cell block grid.
//! * Volume of a block = its open (non-solid, non-liquid) cells. Solid blocks hold no air.
//! * Sources (per update, in cell-units): breathing moles, torches, fire/lava cells, mushroom farms.
//! * Exchange: conservative diffusion between 4-adjacent open blocks, scaled by the smaller volume.
//! * Sinks: vents (strong, around the vent block) and slow seepage into porous soil (dirt/sand/gravel/
//!   roots/clay faces). Since the surface is sealed, a closed tunnel system slowly goes stale.
use crate::grid::Grid;
use crate::mat::{Kind, S, POROUS};

pub const AB: usize = 8 * S; // 2 m blocks
pub const UPDATE_TICKS: u64 = 15;
/// Point sources are in legacy cell-units of CO2 per update; a block holds S² times the cells now.
const PT: f32 = (S * S) as f32;
pub const MOLE_CO2: f32 = 0.08 * PT;
pub const TORCH_CO2: f32 = 0.05 * PT;
pub const FIRE_CO2: f32 = 0.03;
pub const FARM_CO2: f32 = 0.01 * PT;
const DIFF: f32 = 0.22;
const SEEP: f32 = 0.0016;

pub struct Air { pub bw: usize, pub bh: usize, pub co2: Vec<f32>, pub vol: Vec<f32>, porous: Vec<f32>, src: Vec<f32> }

impl Air {
    pub fn new(w: usize, h: usize) -> Air { let (bw, bh) = (w / AB, h / AB); let n = bw * bh; Air { bw, bh, co2: vec![0.; n], vol: vec![0.; n], porous: vec![0.; n], src: vec![0.; n] } }
    #[inline] pub fn block(&self, x: usize, y: usize) -> usize { (y / AB).min(self.bh - 1) * self.bw + (x / AB).min(self.bw - 1) }
    #[inline] pub fn at(&self, x: usize, y: usize) -> f32 { self.co2[self.block(x, y)] }
    pub fn add(&mut self, x: usize, y: usize, amount: f32) { let b = self.block(x, y); self.src[b] += amount; }
    /// Recount block volumes / porosity and add fire sources from the grid.
    fn survey(&mut self, g: &Grid) {
        for b in 0..self.bw * self.bh {
            let (x0, y0) = ((b % self.bw) * AB, (b / self.bw) * AB); let (mut v, mut p, mut f) = (0f32, 0f32, 0f32);
            // one sample per legacy 0.25 m tile, each standing for its S×S cells: air moves at block
            // scale, so this is the same answer in metres for a quarter (at S = 2) of the reads
            let k = (S * S) as f32;
            for y in (y0..y0 + AB).step_by(S) { for x in (x0..x0 + AB).step_by(S) {
                let m = g.get(x, y);
                if matches!(m.kind(), Kind::Empty | Kind::Gas) { v += k; }
                if m.p().heat > 0 { f += k; } // burning or molten
                if m.has(POROUS) { p += k; }
            } }
            self.vol[b] = v; self.porous[b] = if v > 0. { p.min(v * 2.) } else { 0. };
            self.src[b] += f * FIRE_CO2;
            if v == 0. { self.co2[b] = 0.; }
        }
    }
    /// One update. `vents` are cell coordinates of working vents.
    pub fn update(&mut self, g: &Grid, vents: &[(usize, usize)]) {
        self.survey(g);
        let (bw, bh) = (self.bw, self.bh);
        // sources
        for b in 0..bw * bh { if self.vol[b] > 0. { self.co2[b] += self.src[b] / self.vol[b]; } self.src[b] = 0.; }
        // diffusion (two half-sweeps keep it symmetric enough)
        for pass in 0..2 { for by in 0..bh { for bx in 0..bw {
            let a = by * bw + bx; if self.vol[a] == 0. { continue; }
            let nb = if pass == 0 { if bx + 1 < bw { a + 1 } else { continue } } else { if by + 1 < bh { a + bw } else { continue } };
            if self.vol[nb] == 0. { continue; }
            let (va, vb) = (self.vol[a], self.vol[nb]);
            let flux = DIFF * (self.co2[a] - self.co2[nb]) * va.min(vb) * 0.5;
            self.co2[a] -= flux / va; self.co2[nb] += flux / vb;
        } } }
        // sinks
        for b in 0..bw * bh { if self.vol[b] > 0. { let k = (SEEP * self.porous[b] / self.vol[b]).min(0.2); self.co2[b] *= 1. - k; } }
        for &(x, y) in vents {
            let c = self.block(x, y); let (cx, cy) = (c % bw, c / bw);
            for dy in -1i32..=1 { for dx in -1i32..=1 {
                let (nx, ny) = (cx as i32 + dx, cy as i32 + dy); if nx < 0 || ny < 0 || nx as usize >= bw || ny as usize >= bh { continue; }
                let b = ny as usize * bw + nx as usize; self.co2[b] *= if dx == 0 && dy == 0 { 0.5 } else { 0.8 };
            } }
        }
        for v in self.co2.iter_mut() { *v = v.clamp(0., 1.5); }
    }
    pub fn shift_rows(&mut self, br: i64) {
        let (bw, bh) = (self.bw, self.bh); let s = br.unsigned_abs() as usize; if s == 0 { return; }
        if s >= bh { self.co2.iter_mut().for_each(|c| *c = 0.); return; }
        let (d, n) = (s * bw, bw * bh);
        if br > 0 { self.co2.copy_within(d.., 0); self.co2[n - d..].iter_mut().for_each(|c| *c = 0.); } else { self.co2.copy_within(..n - d, d); self.co2[..d].iter_mut().for_each(|c| *c = 0.); }
    }
    pub fn to_bytes(&self) -> Vec<u8> { self.co2.iter().map(|&c| (c.clamp(0., 1.5) / 1.5 * 255.) as u8).collect() }
    pub fn from_bytes(&mut self, b: &[u8]) { for (c, &v) in self.co2.iter_mut().zip(b) { *c = v as f32 / 255. * 1.5; } }
}
