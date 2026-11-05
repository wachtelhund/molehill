//! Cell grid + Noita-style falling-sand CA on 64×64 chunks.
//! Chunks are updated in 4 checkerboard phases; chunks within a phase are ≥64 cells apart,
//! and no cell moves more than 6·S cells per tick, so a phase can run in parallel (rayon) safely
//! and deterministically (RNG is a hash of position+tick, not a shared stream).
use crate::mat::{Kind, Mat, PROPS, S};
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

pub const CHUNK: usize = 64;
pub const F_LOOSE: u8 = 1;    // static soil knocked loose -> behaves as powder
pub const F_SEEN: u8 = 2;     // revealed (fog of war)
pub const F_WET: u8 = 4;      // aquifer-saturated soil (seeps water when dug)

pub struct Grid {
    pub w: usize, pub h: usize, pub cx: usize, pub cy: usize,
    pub mat: Vec<u8>, pub flags: Vec<u8>, pub shade: Vec<u8>, pub life: Vec<u8>, pub moved: Vec<u8>,
    pub awake: Vec<AtomicU8>, pub awake_next: Vec<AtomicU8>,
    pub tick: u64,
    /// bounding box of structural changes since last stress solve (x0,y0,x1,y1)
    pub struct_dirty: Option<(usize, usize, usize, usize)>,
    pub reactions: AtomicU8,
    /// set when a moving cell leaves from under static rock (its support may be gone: re-solve stress)
    pub undermined: AtomicU8,
    /// per-chunk flags for the stress re-solve: bit 0 a structural change, bit 1 a reaction (fire,
    /// quenching). Set from the main thread and from CA threads alike.
    pub sdirty: Vec<AtomicU8>,
}

#[inline] pub fn hash3(x: u32, y: u32, t: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E3779B1) ^ y.wrapping_mul(0x85EBCA77) ^ t.wrapping_mul(0xC2B2AE3D);
    h ^= h >> 15; h = h.wrapping_mul(0x2C1B3C6D); h ^= h >> 12; h = h.wrapping_mul(0x297A2D39); h ^= h >> 15; h
}

#[derive(Clone, Copy)]
struct Ptr(*mut Grid);
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

impl Grid {
    pub fn new(w: usize, h: usize) -> Grid {
        assert!(w % CHUNK == 0 && h % CHUNK == 0);
        let n = w * h; let (cx, cy) = (w / CHUNK, h / CHUNK);
        Grid { w, h, cx, cy, mat: vec![0; n], flags: vec![0; n], shade: (0..n).map(|i| (hash3(i as u32, 7, 1) & 255) as u8).collect(),
            life: vec![0; n], moved: vec![0; n], awake: (0..cx * cy).map(|_| AtomicU8::new(1)).collect(), awake_next: (0..cx * cy).map(|_| AtomicU8::new(1)).collect(),
            tick: 0, struct_dirty: None, reactions: AtomicU8::new(0), undermined: AtomicU8::new(0), sdirty: (0..cx * cy).map(|_| AtomicU8::new(0)).collect() }
    }
    /// Flag the chunk holding cell i for a stress re-solve (`bit` 1 structural, 2 reaction); thread-safe.
    #[inline] pub fn note_struct(&self, i: usize, bit: u8) { let (x, y) = (i % self.w, i / self.w); self.sdirty[(y / CHUNK) * self.cx + x / CHUNK].fetch_or(bit, Relaxed); }
    /// Take the chunks flagged with any of `bits` (clearing those bits), as cell boxes (x0, y0, x1, y1).
    pub fn take_struct(&self, bits: u8) -> Vec<(usize, usize, usize, usize)> {
        let mut v = vec![];
        for (c, f) in self.sdirty.iter().enumerate() { if f.load(Relaxed) & bits != 0 { f.fetch_and(!bits, Relaxed);
            let (x0, y0) = ((c % self.cx) * CHUNK, (c / self.cx) * CHUNK); v.push((x0, y0, x0 + CHUNK - 1, y0 + CHUNK - 1)); } }
        v
    }
    /// Forget pending structural changes (after a full solve).
    pub fn clear_struct(&mut self) { self.struct_dirty = None; for f in &self.sdirty { f.store(0, Relaxed); } }
    #[inline] pub fn idx(&self, x: usize, y: usize) -> usize { y * self.w + x }
    #[inline] pub fn get(&self, x: usize, y: usize) -> Mat { Mat::from_u8(self.mat[y * self.w + x]) }
    #[inline] pub fn m(&self, i: usize) -> Mat { Mat::from_u8(self.mat[i]) }
    #[inline] pub fn loose(&self, i: usize) -> bool { self.flags[i] & F_LOOSE != 0 }
    #[inline] pub fn is_static(&self, i: usize) -> bool { self.m(i).kind() == Kind::Solid && !self.loose(i) }
    #[inline] pub fn is_solidish(&self, i: usize) -> bool { matches!(self.m(i).kind(), Kind::Solid | Kind::Powder) }
    pub fn in_bounds(&self, x: i32, y: i32) -> bool { x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h }

    pub fn wake(&self, x: usize, y: usize) {
        let (cx, cy) = (x / CHUNK, y / CHUNK);
        self.awake_next[cy * self.cx + cx].store(1, Relaxed);
        let (lx, ly) = (x % CHUNK, y % CHUNK);
        if lx < 2 && cx > 0 { self.awake_next[cy * self.cx + cx - 1].store(1, Relaxed); }
        if lx >= CHUNK - 2 && cx + 1 < self.cx { self.awake_next[cy * self.cx + cx + 1].store(1, Relaxed); }
        if ly < 2 && cy > 0 { self.awake_next[(cy - 1) * self.cx + cx].store(1, Relaxed); }
        if ly >= CHUNK - 2 && cy + 1 < self.cy { self.awake_next[(cy + 1) * self.cx + cx].store(1, Relaxed); }
    }
    pub fn wake_rect(&self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let cl = |v: i32, m: usize| v.clamp(0, m as i32 - 1) as usize;
        for cy in cl(y0, self.h) / CHUNK..=cl(y1, self.h) / CHUNK { for cx in cl(x0, self.w) / CHUNK..=cl(x1, self.w) / CHUNK { self.awake_next[cy * self.cx + cx].store(1, Relaxed); } }
    }
    pub fn wake_all(&self) { for a in &self.awake_next { a.store(1, Relaxed); } }
    pub fn mark_struct(&mut self, x: usize, y: usize) {
        self.note_struct(y * self.w + x, 1);
        self.struct_dirty = Some(match self.struct_dirty { None => (x, y, x, y), Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)) });
    }
    /// Set a cell (main thread). Marks structure + wakes.
    pub fn set(&mut self, i: usize, m: Mat) {
        let was = self.m(i);
        self.mat[i] = m as u8; self.flags[i] &= !F_LOOSE;
        self.life[i] = m.p().life;
        let (x, y) = (i % self.w, i / self.w);
        // structural change, or something that held up static rock above just went away
        let undermines = y > 0 && matches!(was.kind(), Kind::Solid | Kind::Powder) && !matches!(m.kind(), Kind::Solid | Kind::Powder) && self.is_static(i - self.w);
        if was.is_structural() || m.is_structural() || undermines { self.mark_struct(x, y); }
        self.wake(x, y);
    }
    pub fn set_loose(&mut self, i: usize) { self.flags[i] |= F_LOOSE; let (x, y) = (i % self.w, i / self.w); self.mark_struct(x, y); self.wake(x, y); }
    pub fn fill_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, m: Mat) {
        for y in y0..=y1.min(self.h - 1) { for x in x0..=x1.min(self.w - 1) { let i = self.idx(x, y); self.set(i, m); } }
    }
    pub fn count(&self, m: Mat) -> usize { self.mat.iter().filter(|&&v| v == m as u8).count() }
    pub fn active_chunks(&self) -> usize { self.awake.iter().filter(|a| a.load(Relaxed) != 0).count() }

    /// One CA tick.
    pub fn step_ca(&mut self) {
        self.tick += 1;
        for c in 0..self.awake.len() { self.awake[c].store(self.awake_next[c].swap(0, Relaxed), Relaxed); }
        let stamp = ((self.tick % 255) + 1) as u8;
        // stamps repeat every 255 ticks: clear old ones so a cell that last moved exactly
        // 255·k ticks ago isn't mistaken for one that already moved this tick
        if stamp == 1 { self.moved.fill(0); }
        let ptr = Ptr(self as *mut Grid);
        for phase in 0..4usize {
            let list: Vec<usize> = (0..self.cx * self.cy).filter(|&c| {
                let (cx, cy) = (c % self.cx, c / self.cx);
                (cx & 1) + 2 * (cy & 1) == phase && self.awake[c].load(Relaxed) != 0
            }).collect();
            let run = move |c: usize| { let g = unsafe { &mut *{ ptr }.0 }; g.update_chunk(c, stamp); };
            #[cfg(feature = "parallel")]
            { use rayon::prelude::*; list.par_iter().for_each(|&c| run(c)); }
            #[cfg(not(feature = "parallel"))]
            { for c in list { run(c); } }
        }
    }

    fn update_chunk(&mut self, c: usize, stamp: u8) {
        let (cx, cy) = (c % self.cx, c / self.cx);
        let (x0, y0) = (cx * CHUNK, cy * CHUNK);
        let rev = self.tick & 1 == 1;
        for ly in (0..CHUNK).rev() {
            let y = y0 + ly;
            for k in 0..CHUNK { let lx = if rev { CHUNK - 1 - k } else { k }; self.update_cell(x0 + lx, y, stamp); }
        }
    }

    #[inline] fn swap(&mut self, i: usize, j: usize, stamp: u8) {
        self.mat.swap(i, j); self.flags.swap(i, j); self.shade.swap(i, j); self.life.swap(i, j);
        self.moved[i] = stamp; self.moved[j] = stamp;
        // whatever just moved out from under static rock may have been holding it up
        // (only grains carry load: water slipping along under a roof changes nothing)
        let w = self.w; for (a, b) in [(i, j), (j, i)] { if a >= w && self.is_solidish(b) && !self.is_solidish(a) && self.is_static(a - w) { self.undermined.store(1, Relaxed); self.note_struct(a - w, 1); } }
        let w = self.w; self.wake(i % w, i / w); self.wake(j % w, j / w);
    }
    #[inline] fn rnd(&self, x: usize, y: usize, salt: u32) -> u32 { hash3(x as u32, y as u32 ^ (salt << 20), self.tick as u32) }
    #[inline] fn kind_at(&self, i: usize) -> Kind { PROPS[self.mat[i] as usize].kind }

    fn update_cell(&mut self, x: usize, y: usize, stamp: u8) {
        let w = self.w; let i = y * w + x;
        if self.moved[i] == stamp { return; }
        let m = self.m(i); if m == Mat::Empty { return; }
        let kind = m.kind();
        let powder = kind == Kind::Powder || (kind == Kind::Solid && self.loose(i));
        if powder {
            if y + 1 >= self.h { return; }
            let d = i + w; let kd = self.kind_at(d);
            // grains sink through a liquid lighter than themselves (sand, gravel, scoria in water)
            // and float on a denser one (sand on lava); drag makes sinking slower than falling
            let rho = m.p().density;
            let sinks = |j: usize| PROPS[self.mat[j] as usize].kind == Kind::Liquid && PROPS[self.mat[j] as usize].density < rho;
            if kd == Kind::Empty || kd == Kind::Gas { let j = self.fall_to(x, y); self.swap(i, j, stamp); return; }
            // ...and bob up through a denser one (wood debris broken loose underwater)
            if y > 0 { let a = i - w; if PROPS[self.mat[a] as usize].kind == Kind::Liquid && PROPS[self.mat[a] as usize].density > rho {
                if self.rnd(x, y, 11) & 1 == 0 { self.swap(i, a, stamp); } else { self.wake(x, y); } return; } }
            if sinks(d) { if self.rnd(x, y, 1) & 1 == 0 { self.swap(i, d, stamp); } else { self.wake(x, y); } return; }
            // a grain that can't slide anywhere rests without keeping its chunk awake
            let pass = |j: usize| self.mat[j] == 0 || sinks(j);
            let open = |s: i32| { let nx = x as i32 + s; nx >= 0 && (nx as usize) < w && pass((i as i32 + s) as usize) && pass((d as i32 + s) as usize) };
            let rep = m.p().repose;
            if rep >= 1. || (!open(-1) && !open(1)) { return; } // never slides / nowhere to slide
            if rep > 0. && (self.rnd(x, y, 2) & 1023) as f32 / 1024. < rep { self.wake(x, y); return; }
            let dir: i32 = if self.rnd(x, y, 3) & 1 == 0 { -1 } else { 1 };
            for s in [dir, -dir] { if open(s) { self.swap(i, (d as i32 + s) as usize, stamp); return; } }
            return;
        }
        match kind {
            Kind::Liquid => {
                let p = m.p();
                if p.heat > 0 {
                    for n in self.n4(x, y) {
                        let nm = self.m(n);
                        // quenched lava shatters into loose scoria (F_LOOSE) instead of a free-floating crust
                        if nm == Mat::Water { self.mat[i] = p.cools_to as u8; self.flags[i] = F_LOOSE; self.mat[n] = Mat::Steam as u8; self.life[n] = Mat::Steam.p().life; self.moved[i] = stamp; self.moved[n] = stamp; self.wake(x, y); self.reactions.store(1, Relaxed); self.note_struct(i, 2); return; }
                        if nm.lit_by(p.heat) && self.rnd(x, y, 4) % 40 == 0 { self.mat[n] = Mat::Fire as u8; self.life[n] = Mat::Fire.p().life; self.reactions.store(1, Relaxed); self.note_struct(n, 2); }
                    }
                }
                if let Some(j) = self.liquid_target(x, y, m) {
                    // a viscous liquid (lava) only gets going on some ticks (but stays awake meanwhile)
                    if self.rnd(x, y, 5) % 10 >= p.flow_in_10 as u32 { self.wake(x, y); return; }
                    self.swap(i, j, stamp);
                }
            }
            Kind::Gas => {
                if m == Mat::Steam {
                    self.life[i] = self.life[i].saturating_sub(1);
                    if self.life[i] == 0 { self.mat[i] = if self.rnd(x, y, 7) % 7 == 0 { Mat::Water as u8 } else { 0 }; self.wake(x, y); return; }
                    self.wake(x, y);
                    let dx = (self.rnd(x, y, 8) % 3) as i32 - 1;
                    let nx = x as i32 + dx;
                    if y > 0 && nx >= 0 && (nx as usize) < w { let j = (i - w) as i32 + dx; let j = j as usize; if self.mat[j] == 0 { self.swap(i, j, stamp); return; } }
                    if nx >= 0 && (nx as usize) < w { let j = (i as i32 + dx) as usize; if self.mat[j] == 0 { self.swap(i, j, stamp); } }
                } else { // fire
                    self.wake(x, y);
                    for n in self.n4(x, y) {
                        let nm = self.m(n);
                        if nm == Mat::Water { self.mat[i] = m.p().cools_to as u8; self.life[i] = 60; return; } // a short puff
                        if nm.lit_by(m.p().heat) && self.rnd(x, y, 9) % 30 == 0 { self.mat[n] = Mat::Fire as u8; self.life[n] = Mat::Fire.p().life; self.reactions.store(1, Relaxed); self.note_struct(n, 2); }
                    }
                    self.life[i] = self.life[i].saturating_sub(1);
                    if self.life[i] == 0 { self.mat[i] = 0; self.reactions.store(1, Relaxed); self.note_struct(i, 2); }
                    else if y > 0 && self.mat[i - w] == 0 && self.rnd(x, y, 10) % 4 == 0 { self.swap(i, i - w, stamp); }
                }
            }
            _ => {}
        }
    }
    /// Free fall from (x, y) into the open cell below: up to S cells per tick, so things fall as fast
    /// in metres at any resolution (the cell below is known to be empty or gas).
    #[inline] fn fall_to(&self, x: usize, y: usize) -> usize {
        let w = self.w; let mut j = (y + 1) * w + x;
        for _ in 1..S { if j + w < self.mat.len() && matches!(self.kind_at(j + w), Kind::Empty | Kind::Gas) { j += w; } else { break; } }
        j
    }
    /// Where a liquid cell moves this tick, if anywhere.
    fn liquid_target(&self, x: usize, y: usize, m: Mat) -> Option<usize> {
        let (w, i) = (self.w, y * self.w + x);
        if y + 1 < self.h {
            let d = i + w;
            if matches!(self.kind_at(d), Kind::Empty | Kind::Gas) { return Some(self.fall_to(x, y)); }
            let dir: i32 = if self.rnd(x, y, 6) & 1 == 0 { -1 } else { 1 };
            for s in [dir, -dir] { let nx = x as i32 + s; if nx >= 0 && (nx as usize) < w { let j = (d as i32 + s) as usize; if self.mat[j] == 0 { return Some(j); } } }
        }
        // Sideways flow. A cell spreads only when it has somewhere lower to go within reach
        // or liquid is pressing on it from above (a heap levelling out). A resting surface
        // with a part-filled top row then stays put instead of sloshing forever, so settled
        // pools let their chunks sleep.
        let disp = m.p().spread as i32 * S as i32;
        let pressed = y > 0 && self.kind_at(i - w) == Kind::Liquid;
        let dir: i32 = if (self.tick as usize + x) & 1 == 1 { 1 } else { -1 };
        // how far a surface cell looks along its row for a lower spot. Reads stay < 64+48 cells from
        // the chunk origin, clear of anything a same-phase chunk (128 away, moving ≤ 6·S) can write.
        const LOOK: i32 = 48;
        const _: () = assert!(64 + LOOK as usize <= 128 - 6 * S, "liquid look-ahead would race a same-phase chunk");
        let (mut near, mut nd, mut level, mut lvl_dir) = (0i32, i32::MAX, None, 0i32);
        for s in [dir, -dir] {
            for k in 1..=LOOK { let nx = x as i32 + s * k; if nx < 0 || nx as usize >= w { break; } let j = (i as i32 + s * k) as usize; if self.mat[j] != 0 { break; }
                if k <= disp && k <= S as i32 && (level.is_none() || s == lvl_dir) { level = Some(j); lvl_dir = s; } // pressed: up to S cells (one tile) a tick
                if y + 1 < self.h && self.mat[j + w] == 0 { if k < nd { nd = k; near = s; } break; } }
        }
        if near != 0 { return Some((i as i32 + near * nd.min(disp)) as usize); }
        if pressed { level } else { None }
    }
    #[inline] pub fn n4(&self, x: usize, y: usize) -> impl Iterator<Item = usize> {
        let (w, h) = (self.w, self.h); let i = y * w + x;
        [(y > 0).then(|| i - w), (y + 1 < h).then(|| i + w), (x > 0).then(|| i - 1), (x + 1 < w).then(|| i + 1)].into_iter().flatten()
    }
}
