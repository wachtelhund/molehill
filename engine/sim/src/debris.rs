//! Debris: cells a blast has torn loose and thrown (sand, rubble, gravel, ash, water). Each is out of the
//! grid while it flies: ballistic, with gravity and a little air drag, swept cell by cell so it never
//! tunnels through rock. It lands back as a cell in the last open cell before whatever it hits, so nothing
//! is lost and liquids splash down and flow on. Deterministic: no randomness here (the launch spread comes
//! from a cell hash in `blast.rs`).
//!
//! The blast also leaves a `Wave`: the cells its pressure reached, each with the path distance the front
//! travelled to get there, its peak overpressure and its direction. The sim never reads it back; the
//! front-end draws the front sweeping along it (slowed to `WAVE_SHOW_MPS` so the eye can follow it) and
//! raises dust that rides it.
use crate::mat::{Kind, Mat, CELL_M, CPM, S, SF};
use crate::world::{World, TUNE};

/// One flying cell. Position and velocity in cells and cells/s; (ppx, ppy) the position a tick ago.
#[derive(Clone, Debug)]
pub struct Debris { pub px: f32, pub py: f32, pub ppx: f32, pub ppy: f32, pub vx: f32, pub vy: f32, pub mat: u8, pub flags: u8, pub age: u16 }

/// A cell the wave reached: index, path distance from the charge (cells), peak overpressure (kPa), direction.
#[derive(Clone, Copy, Debug)]
pub struct WaveCell { pub i: u32, pub d: f32, pub p: f32, pub dx: i8, pub dy: i8 }
/// The record of one blast's pressure wave (for drawing; see the module doc).
#[derive(Clone, Debug, Default)]
pub struct Wave { pub cx: usize, pub cy: usize, pub tick: u64, pub reach: f32, pub cells: Vec<WaveCell> }

/// How fast the drawn front sweeps out (m/s). The real shock outruns a frame in a few metres; this is
/// slowed so it can be seen following the tunnels.
pub const WAVE_SHOW_MPS: f32 = 22.;
/// Half-width of the drawn front (m).
pub const WAVE_BAND_M: f32 = 0.35;
/// Cells recorded per wave (the strongest first are always kept: they are reached first).
pub const WAVE_CELLS: usize = 150_000;
/// Cells in flight at once; a blast past this leaves the rest where they are.
pub const DEBRIS_MAX: usize = 12_000;
/// Launch speed of a cell from the overpressure on it: about 1 m/s per DEBRIS_KPA_PER_MPS kPa, at most
/// DEBRIS_MAX_MPS (liquids a little slower: they are heavy and break into spray).
pub const DEBRIS_KPA_PER_MPS: f32 = 7.; pub const DEBRIS_MAX_MPS: f32 = 30.;

impl Wave {
    /// Seconds since the blast, at tick `now`.
    pub fn age(&self, now: u64) -> f32 { now.saturating_sub(self.tick) as f32 / 60. }
    /// Radius of the drawn front (path metres) at tick `now`.
    pub fn front_m(&self, now: u64) -> f32 { self.age(now) * WAVE_SHOW_MPS }
    /// Still being drawn (the front sweeps past the reach, then fades for half a second).
    pub fn live(&self, now: u64) -> bool { self.front_m(now) < self.reach * CELL_M + WAVE_SHOW_MPS * 0.5 }
    /// Cells in the drawn front at tick `now`.
    pub fn front(&self, now: u64) -> impl Iterator<Item = &WaveCell> + '_ {
        let r = self.front_m(now); self.cells.iter().filter(move |c| (c.d * CELL_M - r).abs() < WAVE_BAND_M)
    }
}

impl World {
    /// Throw the cell at `i` with velocity (vx, vy) in m/s: it leaves the grid and flies.
    pub(crate) fn launch(&mut self, i: usize, vx: f32, vy: f32) -> bool {
        if self.debris.len() >= DEBRIS_MAX { return false; }
        let m = self.g.m(i); if matches!(m.kind(), Kind::Empty | Kind::Gas) || m == Mat::Bedrock { return false; }
        let (x, y) = ((i % self.g.w) as f32 + 0.5, (i / self.g.w) as f32 + 0.5);
        let k = CPM as f32;
        self.debris.push(Debris { px: x, py: y, ppx: x, ppy: y, vx: vx * k, vy: vy * k, mat: m as u8, flags: self.g.flags[i], age: 0 });
        self.g.set(i, Mat::Empty); true
    }

    /// One tick of everything in flight.
    pub(crate) fn step_debris(&mut self) {
        if self.debris.is_empty() { return; }
        let dt = 1. / 60.; let (w, h) = (self.g.w as i32, self.g.h as i32);
        let open = |g: &crate::grid::Grid, x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && matches!(g.m(y as usize * w as usize + x as usize).kind(), Kind::Empty | Kind::Gas);
        let mut landed = vec![];
        for k in 0..self.debris.len() {
            let mut d = self.debris[k].clone(); d.ppx = d.px; d.ppy = d.py; d.age = d.age.saturating_add(1);
            let liquid = Mat::from_u8(d.mat).kind() == Kind::Liquid;
            d.vy = (d.vy + TUNE.gravity * dt).min(TUNE.terminal * 1.5);
            let drag = if liquid { 0.965 } else { 0.98 }; d.vx *= drag; d.vy *= drag;
            let (mx, my) = (d.vx * dt, d.vy * dt); let n = (mx.abs().max(my.abs()).ceil() as i32).max(1);
            let mut hit = false;
            for _ in 0..n {
                let (nx, ny) = (d.px + mx / n as f32, d.py + my / n as f32);
                if open(&self.g, nx.floor() as i32, ny.floor() as i32) { d.px = nx; d.py = ny; continue; }
                // something there: slide along it if only one axis is blocked (so spray runs along a floor)
                if open(&self.g, nx.floor() as i32, d.py.floor() as i32) && my > 0. { d.px = nx; d.vy = 0.; d.vx *= 0.6; }
                else if open(&self.g, d.px.floor() as i32, ny.floor() as i32) && mx.abs() > 0. { d.py = ny; d.vx = -d.vx * 0.15; }
                hit = true; break;
            }
            // it comes to rest once it has hit something and slowed, or after 4 s whatever happens
            if hit && (d.vx.abs() + d.vy.abs() < 6. * SF || d.age > 20) || d.age > 240 { landed.push(k); }
            self.debris[k] = d;
        }
        // land: in the cell it is in if that's open, else the nearest open cell above or beside it
        for &k in landed.iter().rev() {
            let d = self.debris.swap_remove(k); let (x, y) = (d.px.floor() as i32, d.py.floor() as i32);
            let r = 4 * S as i32;
            let spot = (0..=r).flat_map(|dy| [0, -1, 1, -2, 2].into_iter().map(move |dx| (x + dx, y - dy))).find(|&(x, y)| open(&self.g, x, y) && self.g.m(y as usize * w as usize + x as usize) == Mat::Empty);
            match spot {
                Some((x, y)) => { let i = y as usize * w as usize + x as usize; self.g.set(i, Mat::from_u8(d.mat)); self.g.flags[i] = d.flags; if self.g.loose(i) { self.g.set_loose(i); } }
                None => { let mut d = d; d.vx = 0.; d.vy = -2. * SF; if d.age < 600 { self.debris.push(d); } } // nowhere yet: hover a moment and try again
            }
        }
    }

    /// Cells of `m` in flight.
    pub fn debris_of(&self, m: Mat) -> usize { self.debris.iter().filter(|d| d.mat == m as u8).count() }
}
