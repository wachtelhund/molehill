//! Random disasters (port of the v2/disasters design): calm → warning → active → calm.
//! Flood (water spring in the upper half, prefers existing water pockets), lava surge (bottom 40 %,
//! prefers lava pockets), earthquake (loosens soft overhangs near the epicentre; the stress solver
//! and falling-sand CA do the rest). Own RNG, so the world's determinism is unaffected.
//! Volumes and budgets are fractions of the world's cell count (resolution independent).
use crate::grid::Grid;
use crate::mat::{Kind, Mat, BUILT};
use crate::world::{Event, World};

pub const HZ: u32 = 60;
#[derive(Clone, Copy, PartialEq, Debug)] pub enum Phase { Calm, Warning, Active }
#[derive(Clone, Copy, PartialEq, Debug)] pub enum Kind2 { Flood, Lava, Quake }
impl Kind2 { pub fn name(self) -> &'static str { match self { Kind2::Flood => "Flood", Kind2::Lava => "Lava surge", Kind2::Quake => "Earthquake" } } }

pub struct Disasters {
    pub rng: u64, pub phase: Phase, pub kind: Kind2, pub x: usize, pub y: usize, pub ticks_left: u32,
    pub enabled: bool, budget: f32, rate: f32,
}
const FLOOD_VOL: f32 = 0.004; const LAVA_VOL: f32 = 0.002; const QUAKE_BUDGET: f32 = 0.0015; const SRC_W: f32 = 0.03;
const WARN_S: u32 = 10; const ACTIVE_S: u32 = 6;

impl Disasters {
    pub fn new(seed: u32) -> Self { Disasters { rng: (seed as u64).wrapping_mul(0x9E3779B97F4A7C15) | 1, phase: Phase::Calm, kind: Kind2::Quake, x: 0, y: 0, ticks_left: 150 * HZ, enabled: true, budget: 0., rate: 0. } }
    fn r(&mut self) -> u32 { self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17; (self.rng >> 32) as u32 }
    fn rf(&mut self) -> f32 { self.r() as f32 / u32::MAX as f32 }
    pub fn seconds_left(&self) -> f32 { self.ticks_left as f32 / HZ as f32 }

    /// Force a warning now (debug key).
    pub fn schedule(&mut self, w: &mut World, kind: Kind2) -> bool {
        let Some((x, y)) = self.pick_site(&w.g, kind, w) else { return false };
        self.kind = kind; self.x = x; self.y = y; self.phase = Phase::Warning; self.ticks_left = WARN_S * HZ;
        w.say(format!("Warning: {} in {} s near {:.0} m!", kind.name(), WARN_S, w.depth_m(x, y).max(0.))); w.events.push(Event::Warning); true
    }
    fn pick_site(&mut self, g: &Grid, kind: Kind2, w: &World) -> Option<(usize, usize)> {
        let (gw, gh) = (g.w, g.h);
        let (y0, y1) = match kind { Kind2::Flood => (0, gh / 2), Kind2::Lava => (gh * 6 / 10, gh - 6), Kind2::Quake => (gh / 5, gh - 10) };
        let want = match kind { Kind2::Flood => Some(Mat::Water), Kind2::Lava => Some(Mat::Lava), _ => None };
        let ok = |x: usize, y: usize, strict: bool| {
            let i = y * gw + x; if y < 6 || y + 6 >= gh { return false; } let _ = w;
            match kind { Kind2::Quake => g.is_static(i), _ => { let m = g.m(i); (m.kind() == Kind::Empty || Some(m) == want) && (!strict || Some(m) == want) } }
        };
        for strict in [true, false] { for _ in 0..600 {
            let x = 8 + (self.r() as usize) % (gw - 16); let y = y0 + (self.r() as usize) % (y1 - y0).max(1);
            if ok(x, y, strict && want.is_some()) { return Some((x, y)); }
        } }
        let off = self.r() as usize;
        for k in 0..gw * (y1 - y0) { let j = (k + off) % (gw * (y1 - y0)); let (x, y) = (j % gw, y0 + j / gw); if x > 8 && x < gw - 8 && ok(x, y, false) { return Some((x, y)); } }
        None
    }
    pub fn step(&mut self, w: &mut World) {
        if self.ticks_left > 0 { self.ticks_left -= 1; }
        match self.phase {
            Phase::Calm => if self.ticks_left == 0 {
                if !self.enabled { self.ticks_left = 60 * HZ; return; }
                let kind = [Kind2::Flood, Kind2::Lava, Kind2::Quake][(self.r() % 3) as usize];
                if !self.schedule(w, kind) { self.ticks_left = 60 * HZ; }
            },
            Phase::Warning => if self.ticks_left == 0 {
                self.phase = Phase::Active; self.ticks_left = ACTIVE_S * HZ;
                let n = (w.g.w * w.g.h) as f32;
                self.budget = 0.; self.rate = n * match self.kind { Kind2::Flood => FLOOD_VOL, Kind2::Lava => LAVA_VOL, Kind2::Quake => QUAKE_BUDGET } / (ACTIVE_S * HZ) as f32;
                w.say(format!("{}!", self.kind.name())); if self.kind == Kind2::Quake { w.events.push(Event::Quake); }
            },
            Phase::Active => {
                self.budget += self.rate;
                match self.kind {
                    Kind2::Flood | Kind2::Lava => { let m = if self.kind == Kind2::Flood { Mat::Water } else { Mat::Lava }; self.spring(w, m); }
                    Kind2::Quake => { self.quake(w); w.shake = w.shake.max(3.); }
                }
                if self.ticks_left == 0 { self.phase = Phase::Calm; self.ticks_left = (90 + self.r() % 90) * HZ; w.say(format!("The {} subsides.", self.kind.name().to_lowercase())); }
            }
        }
    }
    /// Spawn whole fluid cells at the nearest free cell reachable through connected fluid (spring pressure).
    fn spring(&mut self, w: &mut World, m: Mat) {
        let g = &mut w.g; let gw = g.w; let half = ((gw as f32 * SRC_W) as usize / 2).max(2);
        while self.budget >= 1. {
            self.budget -= 1.;
            let sx = (self.x + self.r() as usize % (2 * half)).saturating_sub(half).clamp(1, gw - 2);
            let start = self.y * gw + sx;
            let mut q = std::collections::VecDeque::new(); let mut seen = std::collections::HashSet::new(); q.push_back(start); seen.insert(start);
            let mut placed = false;
            while let Some(i) = q.pop_front() {
                if seen.len() > 1024 { break; }
                if g.mat[i] == 0 { g.set(i, m); g.wake(i % gw, i / gw); placed = true; break; }
                if g.m(i) != m { continue; }
                let (x, y) = (i % gw, i / gw);
                for j in [i.wrapping_sub(gw), i - 1, i + 1, i + gw] { let (jx, jy) = (j % gw, j / gw); if jy == 0 || jy >= g.h - 2 || jx == 0 || jx >= gw - 1 || (jx as i64 - x as i64).abs() > 1 || (jy as i64 - y as i64).abs() > 1 { continue; } if seen.insert(j) { q.push_back(j); } }
            }
            if !placed { break; }
        }
    }
    fn quake(&mut self, w: &mut World) {
        let (gw, gh) = (w.g.w, w.g.h); let half = (gw as f32 * 0.3) as usize; let samples = ((gw * gh) as f32 * 0.003) as usize;
        for _ in 0..samples {
            if self.budget < 1. { break; }
            let x = (self.x + self.r() as usize % (2 * half)).saturating_sub(half).clamp(1, gw - 2);
            let y = (self.y + self.r() as usize % (2 * half)).saturating_sub(half).clamp(1, gh - 3);
            let i = y * gw + x; let m = w.g.m(i);
            // soft = static structural solid that is weak (not rock/anchor/built), with open space below
            let soft = w.g.is_static(i) && m.p().tensile < 100. && m != Mat::Bedrock && !m.has(BUILT);
            if soft && w.g.mat[i + gw] == 0 { w.g.set_loose(i); self.budget -= 1.; let _ = self.rf(); }
        }
    }
}
