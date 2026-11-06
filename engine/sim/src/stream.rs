//! Streaming fresh ground into the window without a hitch. A band (`band::BH` rows) nobody has visited is
//! a pure function of the seed and its first row: its cells are generated with `CTX_UP` rows of context
//! above and `CTX_DN` below, the band is shored up (`gen::Stab`) against that context in `TILE`-wide
//! tiles (each sees `TILE_CTX` columns either side, and the tiles before it already shored), and the
//! context is thrown away. So a band comes out the same whatever route reaches it and whenever it is
//! built: ahead of time, a small step at a time (`BandJob::step`, a few ms), kept in `BandCache` until the
//! window slides onto it, or at once if the window gets there first.
use crate::band::BH;
use crate::gen::{self, Gen, Stab};
use crate::grid::Grid;
use crate::mat::Mat;
use crate::stress::Stress;
use std::collections::BTreeMap;

/// Context generated above a band while it is shored up (16 m: the stress solver's arch height).
pub const CTX_UP: usize = 2 * BH;
/// Context generated below it (8 m: what the solver looks at under a region).
pub const CTX_DN: usize = BH;
/// Rows generated per step.
const FILL_PER: usize = 16;
/// Width of a shoring tile (32 m) and the columns of context it sees on each side (16 m).
pub const TILE: usize = 256;
pub const TILE_CTX: usize = 128;

/// The tile being shored: its scratch grid (`TILE + 2·TILE_CTX` columns), solver, rounds, and where it
/// sits in the band's grid (`sx` = scratch column 0, `tx..tx + tw` = the columns it owns).
struct Tile { g: Grid, st: Stress, stab: Stab, sx: usize, tx: usize, tw: usize }

/// One band being built, step by step: `step` until it returns true, then `take` the band's cells.
pub struct BandJob { pub wy0: i64, g: Grid, row: usize, next_tx: usize, tile: Option<Tile>, done: bool }
impl BandJob {
    pub fn new(w: usize, wy0: i64) -> BandJob { BandJob { wy0, g: Grid::new(w, CTX_UP + BH + CTX_DN), row: 0, next_tx: 0, tile: None, done: false } }
    /// Absolute row of the band grid's row 0.
    fn oy(&self) -> i64 { self.wy0 - CTX_UP as i64 }
    /// One slice of work (a few ms on the game-sized window); true when the band is built.
    pub fn step(&mut self, gen: &Gen) -> bool {
        if self.done { return true; }
        let (w, h, oy) = (self.g.w, self.g.h, self.oy());
        if self.row < h { // generate
            let y1 = (self.row + FILL_PER).min(h); gen.fill_cells(&mut self.g, oy, self.row, y1); self.row = y1;
            if y1 == h { gen.fill_post(&mut self.g, oy, 0, h); }
            return false;
        }
        if self.tile.is_none() { // set up the next tile: copy its columns (with context) out of the band grid
            let sw = (TILE + 2 * TILE_CTX).min(w); let tx = self.next_tx; let tw = TILE.min(w - tx);
            let sx = (tx as i64 - TILE_CTX as i64).clamp(0, (w - sw) as i64) as usize;
            let mut g = Grid::new(sw, h);
            for y in 0..h { let (a, b) = (y * w + sx, y * sw); g.mat[b..b + sw].copy_from_slice(&self.g.mat[a..a + sw]); g.flags[b..b + sw].copy_from_slice(&self.g.flags[a..a + sw]); }
            self.tile = Some(Tile { g, st: Stress::new(sw * h), stab: Stab::new(CTX_UP, CTX_UP + BH), sx, tx, tw });
            return false;
        }
        let t = self.tile.as_mut().unwrap(); let sx = t.sx;
        let kept = |x: usize, y: usize| gen.in_grotto_void(x + sx, oy + y as i64);
        let soft = |x: usize, y: usize| gen::in_start_zone(w, x + sx, oy + y as i64);
        if !t.stab.round(&mut t.g, &mut t.st, &kept, &soft) { return false; }
        // shored: the tile's own columns go back into the band grid (the next tile sees them as context)
        let t = self.tile.take().unwrap(); let sw = t.g.w;
        for y in 0..h { let (a, b) = (y * w + t.tx, y * sw + (t.tx - t.sx)); self.g.mat[a..a + t.tw].copy_from_slice(&t.g.mat[b..b + t.tw]); }
        self.next_tx = t.tx + t.tw; self.done = self.next_tx >= w;
        self.done
    }
    /// The band's cells (materials, flags), `BH` rows of the band grid's width.
    pub fn take(self) -> Band { let (a, b) = (CTX_UP * self.g.w, (CTX_UP + BH) * self.g.w); Band { mat: self.g.mat[a..b].to_vec(), flags: self.g.flags[a..b].to_vec() } }
}
/// A built band's cells.
pub struct Band { pub mat: Vec<u8>, pub flags: Vec<u8> }
/// The band starting at absolute row `wy0`, built at once.
pub fn band_cells(gen: &Gen, w: usize, wy0: i64) -> Band { let mut j = BandJob::new(w, wy0); while !j.step(gen) {} j.take() }

/// Fresh bands built ahead of the window. Never saved: each is a pure function of the seed, rebuilt
/// after a load. Bands already visited live in `World::bands`, and are never built here. Natively a
/// worker thread builds them; on the web (or with `sliced` set) `step` builds a slice at a time. Either way a band's cells are the same, so where and when it was built never shows.
#[derive(Default)]
pub struct BandCache { pub ready: BTreeMap<i64, Band>, job: Option<BandJob>, seam: Option<Seam>,
    /// build in slices on this thread even where threads exist (what the web does; for benches and tests)
    pub sliced: bool,
    #[cfg(not(target_arch = "wasm32"))] worker: Option<worker::Worker> }
impl BandCache {
    /// Take the built band at `wy0`, or build it now (when the window got there before the prefetch did).
    /// A band half-way built (by the job or the worker) is finished rather than restarted.
    pub fn take(&mut self, gen: &Gen, w: usize, wy0: i64) -> Band {
        #[cfg(not(target_arch = "wasm32"))] if let Some(wk) = self.worker.as_ref().filter(|_| !self.sliced) { if let Some(b) = wk.wait_for(wy0, &mut self.ready) { return b; } }
        if let Some(b) = self.ready.remove(&wy0) { return b; }
        if self.job.as_ref().is_some_and(|j| j.wy0 == wy0) { let mut j = self.job.take().unwrap(); while !j.step(gen) {} return j.take(); }
        band_cells(gen, w, wy0)
    }
    /// Work toward the bands of `want` (in order of need) not built yet: on the web one step of the first
    /// one; with threads, hand the list to the worker and collect what it built. False when every one is
    /// built. Bands no longer wanted are dropped, and so is a job for one.
    pub fn step(&mut self, gen: &Gen, w: usize, want: &[i64]) -> bool {
        self.ready.retain(|k, _| want.contains(k));
        #[cfg(not(target_arch = "wasm32"))] if !self.sliced {
            let wk = self.worker.get_or_insert_with(|| worker::Worker::spawn(gen.seed, w));
            wk.collect(&mut self.ready); self.ready.retain(|k, _| want.contains(k)); wk.want(want);
            return want.iter().any(|k| !self.ready.contains_key(k));
        }
        {
            if self.job.as_ref().is_some_and(|j| !want.contains(&j.wy0)) { self.job = None; }
            if self.job.is_none() {
                let Some(&wy0) = want.iter().find(|k| !self.ready.contains_key(k)) else { return false };
                self.job = Some(BandJob::new(w, wy0));
            }
            let j = self.job.as_mut().unwrap();
            if j.step(gen) { let j = self.job.take().unwrap(); self.ready.insert(j.wy0, j.take()); }
            true
        }
    }
    /// How many of `want` are built (for a loading line).
    pub fn built(&self, want: &[i64]) -> usize { want.iter().filter(|k| self.ready.contains_key(k)).count() }
}

/// The band builder thread (native): it gets the wanted list, builds what it hasn't sent yet in that
/// order, and sends each band back. Dropping the `Worker` (the world) ends it.
#[cfg(not(target_arch = "wasm32"))]
mod worker {
    use super::{band_cells, Band};
    use crate::gen::Gen;
    use std::collections::{BTreeMap, HashSet};
    use std::sync::{atomic::{AtomicI64, Ordering::SeqCst}, mpsc, Arc, Mutex};
    const IDLE: i64 = i64::MIN;
    /// (`rx` sits in a mutex only so the world can be shared between threads, as a Bevy resource is.)
    pub struct Worker { tx: mpsc::Sender<Vec<i64>>, rx: Mutex<mpsc::Receiver<(i64, Band)>>, building: Arc<AtomicI64>, last: Vec<i64> }
    impl Worker {
        pub fn spawn(seed: u32, w: usize) -> Worker {
            let (tx, wants) = mpsc::channel::<Vec<i64>>(); let (out, rx) = mpsc::channel(); let building = Arc::new(AtomicI64::new(IDLE)); let b = building.clone();
            std::thread::Builder::new().name("molehill-stream".into()).spawn(move || {
                let gen = Gen::new(seed, w); let (mut want, mut sent): (Vec<i64>, HashSet<i64>) = (vec![], HashSet::new());
                loop {
                    if !want.iter().any(|k| !sent.contains(k)) { match wants.recv() { Ok(v) => want = v, Err(_) => return } }
                    while let Ok(v) = wants.try_recv() { want = v; }
                    sent.retain(|k| want.contains(k));
                    let Some(k) = want.iter().copied().find(|k| !sent.contains(k)) else { continue };
                    b.store(k, SeqCst); let band = band_cells(&gen, w, k); sent.insert(k);
                    let gone = out.send((k, band)).is_err(); b.store(IDLE, SeqCst); if gone { return; }
                }
            }).expect("spawn the band builder thread");
            Worker { tx, rx: Mutex::new(rx), building, last: vec![] }
        }
        /// Tell the worker what is wanted now (only when it changed).
        pub fn want(&mut self, want: &[i64]) { if self.last != want { self.last = want.to_vec(); let _ = self.tx.send(self.last.clone()); } }
        /// Move what the worker has built into `ready`.
        pub fn collect(&self, ready: &mut BTreeMap<i64, Band>) { let rx = self.rx.lock().unwrap(); while let Ok((k, b)) = rx.try_recv() { ready.insert(k, b); } }
        /// The band at `wy0` if built, waiting for the worker if it is building that one right now.
        pub fn wait_for(&self, wy0: i64, ready: &mut BTreeMap<i64, Band>) -> Option<Band> {
            self.collect(ready);
            while !ready.contains_key(&wy0) && self.building.load(SeqCst) == wy0 {
                match self.rx.lock().unwrap().recv() { Ok((k, b)) => { ready.insert(k, b); } Err(_) => break }
            }
            ready.remove(&wy0)
        }
    }
}

/// After the window slid by `rows` (cells already moved): move the cells' `life` and the chunks' sleep
/// with the rock, and wake only the entering rows `en..en1` (and the chunk row they meet), instead of
/// the whole window. A slide is a whole number of chunks (`BH` = `CHUNK`).
pub fn shift_wake(g: &mut Grid, rows: i64, en: usize, en1: usize) {
    use crate::grid::CHUNK; use std::sync::atomic::Ordering::Relaxed;
    let (w, n) = (g.w, g.mat.len()); let d = rows.unsigned_abs() as usize * w;
    if rows > 0 { g.life.copy_within(d.., 0); } else { g.life.copy_within(..n - d, d); }
    g.life[en * w..en1 * w].fill(0);
    let (cx, cy) = (g.cx, g.cy); let cs = rows.unsigned_abs() as usize / CHUNK;
    for v in [&g.awake, &g.awake_next] {
        let old: Vec<u8> = v.iter().map(|a| a.load(Relaxed)).collect();
        for r in 0..cy { let src = if rows > 0 { r + cs } else { r.wrapping_sub(cs) };
            for c in 0..cx { v[r * cx + c].store(if src < cy { old[src * cx + c] } else { 0 }, Relaxed); } }
    }
    let (r0, r1) = ((en / CHUNK).saturating_sub(1), ((en1 + CHUNK - 1) / CHUNK + 1).min(cy));
    for r in r0..r1 { for c in 0..cx { g.awake_next[r * cx + c].store(1, Relaxed); } }
}

/// How many slides of fresh ground beyond each edge of the window are kept built ahead: 96 m above and
/// below, so a fall or climb of that far never waits for the generator.
pub const AHEAD_SLIDES: usize = 3;
impl crate::world::World {
    /// Put the fresh band for window rows `y0..y0 + BH` in place (from the stream, or built now).
    pub(crate) fn put_fresh_band(&mut self, y0: usize) {
        let w = self.g.w; let b = self.stream.take(&self.gen, w, self.oy + y0 as i64);
        self.g.mat[y0 * w..(y0 + BH) * w].copy_from_slice(&b.mat); self.g.flags[y0 * w..(y0 + BH) * w].copy_from_slice(&b.flags);
    }
    /// The fresh bands (first absolute rows) to keep built: `slides` slides beyond the window's top and
    /// bottom, nearest the player first; bands already visited (stored) are never rebuilt.
    pub fn stream_want(&self, slides: usize) -> Vec<i64> {
        let (h, n) = (self.g.h as i64, (slides * crate::world::SLIDE / BH) as i64);
        let py = self.oy + self.moles.first().map_or(h / 2, |m| m.y as i64);
        let mut v: Vec<i64> = (0..n).flat_map(|k| [self.oy + h + k * BH as i64, self.oy - (k + 1) * BH as i64]).filter(|k| !self.bands.contains_key(k)).collect();
        v.sort_by_key(|&k| ((k + BH as i64 / 2) - py).abs()); v
    }
    /// One step (a few ms) of building fresh ground ahead of the window; false when it is all built.
    /// The game calls it each frame it has time to spare, and behind the loading line until
    /// `stream_progress` is 1. Never changes the world: a band is the same built now or at the slide.
    pub fn prefetch_step(&mut self) -> bool { let want = self.stream_want(AHEAD_SLIDES); let w = self.g.w; self.stream.step(&self.gen, w, &want) }
    /// Share of the fresh ground ahead that is built, 0..1.
    pub fn stream_progress(&self) -> f32 { let want = self.stream_want(AHEAD_SLIDES); if want.is_empty() { 1. } else { self.stream.built(&want) as f32 / want.len() as f32 } }
}

/// Width of a seam box (32 m): after a slide, the region solve over the entering rows (and 16 m of the
/// old rows beside them) and over the edge the window left runs one box per tick, not all in the slide.
pub const SEAM_COLS: usize = 256;
/// The boxes still to solve after a slide; `true` = the seam (repaired), `false` = the far edge (cracked).
/// A repaired box is solved again (up to `SEAM_TRIES` times in all) until nothing in it fails.
struct Seam { boxes: Vec<((usize, usize, usize, usize), bool)>, k: usize, tries: Vec<u8>, reach: bool }
const SEAM_TRIES: u8 = 8;
impl crate::world::World {
    /// Queue the solves a slide by `rows` needs: `ent` (the entering rows and the old rows they meet), `far`
    /// (the edge the window left). What an earlier slide still had queued moves with the rock (boxes that
    /// left the window are dropped), so several slides in one tick (a respawn) don't each solve at once.
    /// A queue taller than the window is cut to one pass over the whole window.
    pub(crate) fn start_seam(&mut self, rows: i64, ent: (usize, usize, usize, usize), far: (usize, usize, usize, usize)) {
        let (w, h) = (self.g.w as i64, self.g.h as i64);
        let split = |b: (usize, usize, usize, usize)| (b.0..=b.2).step_by(SEAM_COLS).map(move |x0| (x0, b.1, (x0 + SEAM_COLS - 1).min(b.2), b.3));
        let mut boxes: Vec<((usize, usize, usize, usize), bool)> = vec![];
        if let Some(sm) = self.stream.seam.take() {
            for &((x0, y0, x1, y1), r) in &sm.boxes[sm.k..] {
                let (a, b) = ((y0 as i64 - rows).max(0), (y1 as i64 - rows).min(h - 1));
                if a <= b { boxes.push(((x0, a as usize, x1, b as usize), r)); }
            }
        }
        boxes.extend(split(ent).map(|b| (b, true)).chain(split(far).map(|b| (b, false))));
        let rows_queued: i64 = boxes.iter().map(|((x0, y0, x1, y1), _)| ((x1 - x0 + 1) * (y1 - y0 + 1)) as i64).sum::<i64>() / w;
        if rows_queued > h { boxes = split((0, 0, w as usize - 1, h as usize - 1)).map(|b| (b, true)).collect(); }
        let tries = vec![1; boxes.len()]; self.stream.seam = Some(Seam { boxes, k: 0, tries, reach: false });
    }
    /// One box of the seam; true while more remain (the regular stress pass waits for them). At the seam,
    /// fresh bands were shored against generated rock and the old rows against the window's edge, so what
    /// fails there is put right as the generator's shoring does: rock left with no load path is cleared,
    /// an overloaded cell becomes limestone (weak ground, the start zone) or granite. The far edge cracks.
    pub(crate) fn seam_step(&mut self) -> bool {
        let Some(sm) = self.stream.seam.as_mut() else { return false };
        // first, which rock has a load path over the whole window: rows the old edge held up may hang free
        // now, and the boxes anchor their sides on this (as one full-width solve would see it)
        if !sm.reach { sm.reach = true; self.st.reach = reach_all(&self.g); return true; }
        let (b, repair) = sm.boxes[sm.k]; let tries = sm.tries[sm.k];
        let f = self.st.solve_rect(&self.g, Some(b));
        let sm = self.stream.seam.as_mut().unwrap(); sm.k += 1;
        if repair && !f.is_empty() && tries < SEAM_TRIES { sm.boxes.push((b, true)); sm.tries.push(tries + 1); } // check the repair next time round
        let more = sm.k < sm.boxes.len(); if !more { self.stream.seam = None; }
        if !repair || tries == SEAM_TRIES { self.st.crack(&self.g, &f); return more; }
        // the generator's shoring rules (`gen::Stab::round`), a try standing for its rounds: first weak ground
        // turns to limestone and the rest to granite; a cell still failing after that has the voids round it
        // filled, further out each try (a long roof gets thicker, a thin pillar wider). Grotto hollows are kept.
        let (w, h, oy) = (self.g.w, self.g.h, self.oy); let it = if tries == 1 { 0 } else { 4 + 3 * (tries as i32 - 2) };
        let soft = |x: usize, y: usize| gen::in_start_zone(w, x, oy + y as i64);
        for &i in &f { let i = i as usize; let (x, y) = (i % w, i / w);
            if !self.st.reach[i] { self.g.set(i, Mat::Empty); continue; } // rock left hanging with no load path
            if it >= 4 { let r = (1 + (it - 4) / 3).min(4 * crate::mat::S as i32);
                for dx in -r..=r { for dy in -r..=r + 1 { let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx <= 1 || ny < 0 || nx as usize >= w - 2 || ny as usize >= h { continue; } let (nx, ny) = (nx as usize, ny as usize); let j = ny * w + nx;
                    if self.gen.in_grotto_void(nx, oy + ny as i64) { continue; }
                    if matches!(Mat::from_u8(self.g.mat[j]), Mat::Empty | Mat::Water | Mat::Lava) { self.g.set(j, if soft(nx, ny) { Mat::Limestone } else { Mat::Granite }); } } }
            }
            let weak = Mat::from_u8(self.g.mat[i]).p().tensile < Mat::Limestone.p().tensile;
            self.g.set(i, if (it < 3 && weak) || soft(x, y) { Mat::Limestone } else { Mat::Granite });
        }
        more
    }
    /// Is a slide's seam still being solved (a box a tick)?
    pub fn seam_busy(&self) -> bool { self.stream.seam.is_some() }
    /// Run what is left of the seam now.
    pub fn finish_seam(&mut self) { while self.seam_step() {} }
}

/// Which cells have a load path, over the whole window, by the stress solver's rules (`Stress::solve_rect`,
/// step 2): from bedrock and the static cells of the window's top and bottom rows, static rock carries
/// static rock beside it, and anything solid or loose carries what rests on top of it. One pass, no
/// strength sums: a few ms where a full solve takes tens.
pub fn reach_all(g: &Grid) -> Vec<bool> {
    use crate::mat::Kind;
    let (w, n) = (g.w, g.mat.len()); let mut reach = vec![false; n]; let mut q: Vec<u32> = Vec::with_capacity(n / 2);
    let supports = |i: usize| matches!(g.m(i).kind(), Kind::Powder | Kind::Solid);
    for i in 0..n { if g.mat[i] == Mat::Bedrock as u8 || ((i < w || i >= n - w) && g.is_static(i)) { reach[i] = true; q.push(i as u32); } }
    let mut head = 0;
    while head < q.len() {
        let i = q[head] as usize; head += 1; let (x, y) = (i % w, i / w);
        if g.is_static(i) { for j in g.n4(x, y) { if !reach[j] && g.is_static(j) { reach[j] = true; q.push(j as u32); } } }
        if y > 0 { let j = i - w; if !reach[j] && supports(j) { reach[j] = true; q.push(j as u32); } }
    }
    reach
}
