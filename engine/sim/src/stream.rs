//! Streaming fresh ground into the window without a hitch. A tile (`band::TW` columns by `band::BH`
//! rows) nobody has visited is a pure function of the seed and its first column and row: its cells are
//! generated with `CTX_UP` rows of context above, `CTX_DN` below and `TILE_CTX` columns on each side,
//! the whole is shored up (`gen::Stab`), and the context is thrown away. So a tile comes out the same
//! whatever route reaches it (down, sideways or back) and whenever it is built: ahead of time, a small
//! step at a time (`BandJob::step`, a few ms), kept in `BandCache` until the window slides onto it, or
//! at once if the window gets there first.
use crate::band::BH;
use crate::gen::{self, Gen, Stab};
use crate::grid::Grid;
use crate::mat::Mat;
use crate::stress::Stress;
use std::collections::BTreeMap;

/// A tile's key: its first absolute column and row.
pub type Key = (i64, i64);
/// Context generated above a tile while it is shored up (16 m: the stress solver's arch height).
pub const CTX_UP: usize = 2 * BH;
/// Context generated below it (8 m: what the solver looks at under a region).
pub const CTX_DN: usize = BH;
/// Rows generated per step.
const FILL_PER: usize = 16;
/// Width of a shoring tile (32 m, `band::TW`) and the columns of context it sees on each side (16 m).
pub const TILE: usize = crate::band::TW;
pub const TILE_CTX: usize = 128;

/// One tile being built, step by step: `step` until it returns true, then `take` the tile's cells.
pub struct BandJob { pub key: Key, tw: usize, g: Grid, row: usize, shore: Option<(Stress, Stab)>, done: bool }
impl BandJob {
    pub fn new(tw: usize, key: Key) -> BandJob { BandJob { key, tw, g: Grid::new(tw + 2 * TILE_CTX, CTX_UP + BH + CTX_DN), row: 0, shore: None, done: false } }
    /// Absolute column and row of the scratch grid's cell (0, 0).
    fn origin(&self) -> (i64, i64) { (self.key.0 - TILE_CTX as i64, self.key.1 - CTX_UP as i64) }
    /// One slice of work (a few ms); true when the tile is built.
    pub fn step(&mut self, gen: &Gen) -> bool {
        if self.done { return true; }
        let (h, (ox, oy)) = (self.g.h, self.origin());
        if self.row < h { // generate
            let y1 = (self.row + FILL_PER).min(h); gen.fill_cells(&mut self.g, ox, oy, self.row, y1); self.row = y1;
            if y1 == h { gen.fill_post(&mut self.g, ox, oy, 0, h); }
            return false;
        }
        let (st, stab) = self.shore.get_or_insert_with(|| (Stress::new(self.g.w * h), Stab::new(CTX_UP, CTX_UP + BH)));
        let kept = |x: usize, y: usize| gen.in_grotto_void(ox + x as i64, oy + y as i64);
        let soft = |x: usize, y: usize| gen::in_start_zone(gen.w0, ox + x as i64, oy + y as i64);
        self.done = stab.round(&mut self.g, st, &kept, &soft);
        self.done
    }
    /// The tile's cells (materials, flags), `BH` rows of `tw` columns.
    pub fn take(self) -> Band {
        let (sw, tw) = (self.g.w, self.tw); let (mut mat, mut flags) = (Vec::with_capacity(tw * BH), Vec::with_capacity(tw * BH));
        for y in CTX_UP..CTX_UP + BH { let a = y * sw + TILE_CTX; mat.extend_from_slice(&self.g.mat[a..a + tw]); flags.extend_from_slice(&self.g.flags[a..a + tw]); }
        Band { mat, flags }
    }
}
/// A built tile's cells.
pub struct Band { pub mat: Vec<u8>, pub flags: Vec<u8> }
/// The tile `tw` columns wide at `key`, built at once.
pub fn band_cells(gen: &Gen, tw: usize, key: Key) -> Band { let mut j = BandJob::new(tw, key); while !j.step(gen) {} j.take() }

/// Fresh tiles built ahead of the window. Never saved: each is a pure function of the seed, rebuilt
/// after a load. Tiles already visited live in `World::bands`, and are never built here. Natively a
/// worker thread builds them; on the web (or with `sliced` set) `step` builds a slice at a time. Either
/// way a band's cells are the same, so where and when it was built never shows.
#[derive(Default)]
pub struct BandCache { pub ready: BTreeMap<Key, Band>, job: Option<BandJob>, seam: Option<Seam>,
    /// build in slices on this thread even where threads exist (what the web does; for benches and tests)
    pub sliced: bool,
    #[cfg(not(target_arch = "wasm32"))] worker: Option<worker::Worker> }
impl BandCache {
    /// Take the built band at `wy0`, or build it now (when the window got there before the prefetch did).
    /// A band half-way built (by the job or the worker) is finished rather than restarted.
    pub fn take(&mut self, gen: &Gen, tw: usize, key: Key) -> Band {
        #[cfg(not(target_arch = "wasm32"))] if let Some(wk) = self.worker.as_ref().filter(|_| !self.sliced) { if let Some(b) = wk.wait_for(key, &mut self.ready) { return b; } }
        if let Some(b) = self.ready.remove(&key) { return b; }
        if self.job.as_ref().is_some_and(|j| j.key == key) { let mut j = self.job.take().unwrap(); while !j.step(gen) {} return j.take(); }
        band_cells(gen, tw, key)
    }
    /// Work toward the bands of `want` (in order of need) not built yet: on the web one step of the first
    /// one; with threads, hand the list to the worker and collect what it built. False when every one is
    /// built. Bands no longer wanted are dropped, and so is a job for one.
    pub fn step(&mut self, gen: &Gen, tw: usize, want: &[Key]) -> bool {
        self.ready.retain(|k, _| want.contains(k));
        #[cfg(not(target_arch = "wasm32"))] if !self.sliced {
            let wk = self.worker.get_or_insert_with(|| worker::Worker::spawn(gen.seed, gen.w0, tw));
            wk.collect(&mut self.ready); self.ready.retain(|k, _| want.contains(k)); wk.want(want);
            return want.iter().any(|k| !self.ready.contains_key(k));
        }
        {
            if self.job.as_ref().is_some_and(|j| !want.contains(&j.key)) { self.job = None; }
            if self.job.is_none() {
                let Some(&key) = want.iter().find(|k| !self.ready.contains_key(k)) else { return false };
                self.job = Some(BandJob::new(tw, key));
            }
            let j = self.job.as_mut().unwrap();
            if j.step(gen) { let j = self.job.take().unwrap(); self.ready.insert(j.key, j.take()); }
            true
        }
    }
    /// How many of `want` are built (for a loading line).
    pub fn built(&self, want: &[Key]) -> usize { want.iter().filter(|k| self.ready.contains_key(k)).count() }
}

/// The band builder thread (native): it gets the wanted list, builds what it hasn't sent yet in that
/// order, and sends each band back. Dropping the `Worker` (the world) ends it.
#[cfg(not(target_arch = "wasm32"))]
mod worker {
    use super::{band_cells, Band, Key};
    use crate::gen::Gen;
    use std::collections::{BTreeMap, HashSet};
    use std::sync::{mpsc, Arc, Mutex};
    /// (`rx` sits in a mutex only so the world can be shared between threads, as a Bevy resource is.)
    pub struct Worker { tx: mpsc::Sender<Vec<Key>>, rx: Mutex<mpsc::Receiver<(Key, Band)>>, building: Arc<Mutex<Option<Key>>>, last: Vec<Key> }
    impl Worker {
        pub fn spawn(seed: u32, w0: usize, tw: usize) -> Worker {
            let (tx, wants) = mpsc::channel::<Vec<Key>>(); let (out, rx) = mpsc::channel(); let building = Arc::new(Mutex::new(None)); let b = building.clone();
            std::thread::Builder::new().name("molehill-stream".into()).spawn(move || {
                let gen = Gen::new(seed, w0); let (mut want, mut sent): (Vec<Key>, HashSet<Key>) = (vec![], HashSet::new());
                loop {
                    if !want.iter().any(|k| !sent.contains(k)) { match wants.recv() { Ok(v) => want = v, Err(_) => return } }
                    while let Ok(v) = wants.try_recv() { want = v; }
                    sent.retain(|k| want.contains(k));
                    let Some(k) = want.iter().copied().find(|k| !sent.contains(k)) else { continue };
                    *b.lock().unwrap() = Some(k); let band = band_cells(&gen, tw, k); sent.insert(k);
                    // (sent before `building` clears: a `wait_for` seeing None has the tile in the channel already)
                    let gone = out.send((k, band)).is_err(); *b.lock().unwrap() = None; if gone { return; }
                }
            }).expect("spawn the band builder thread");
            Worker { tx, rx: Mutex::new(rx), building, last: vec![] }
        }
        /// Tell the worker what is wanted now (only when it changed).
        pub fn want(&mut self, want: &[Key]) { if self.last != want { self.last = want.to_vec(); let _ = self.tx.send(self.last.clone()); } }
        /// Move what the worker has built into `ready`.
        pub fn collect(&self, ready: &mut BTreeMap<Key, Band>) { let rx = self.rx.lock().unwrap(); while let Ok((k, b)) = rx.try_recv() { ready.insert(k, b); } }
        /// The tile at `key` if built, waiting for the worker if it is building that one right now.
        pub fn wait_for(&self, key: Key, ready: &mut BTreeMap<Key, Band>) -> Option<Band> {
            self.collect(ready);
            while !ready.contains_key(&key) && *self.building.lock().unwrap() == Some(key) {
                match self.rx.lock().unwrap().recv() { Ok((k, b)) => { ready.insert(k, b); } Err(_) => break }
            }
            ready.remove(&key)
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

/// After the window slid sideways by `cols` (cells and `life` already moved): move the chunks' sleep
/// with the rock and wake only the entering columns `en..en1` (and the chunk column they meet).
pub fn shift_wake_x(g: &mut Grid, cols: i64, en: usize, en1: usize) {
    use crate::grid::CHUNK; use std::sync::atomic::Ordering::Relaxed;
    let (cx, cy) = (g.cx, g.cy); let cs = cols.unsigned_abs() as usize / CHUNK;
    for v in [&g.awake, &g.awake_next] {
        let old: Vec<u8> = v.iter().map(|a| a.load(Relaxed)).collect();
        for r in 0..cy { for c in 0..cx { let src = if cols > 0 { c + cs } else { c.wrapping_sub(cs) };
            v[r * cx + c].store(if src < cx { old[r * cx + src] } else { 0 }, Relaxed); } }
    }
    let (c0, c1) = ((en / CHUNK).saturating_sub(1), ((en1 + CHUNK - 1) / CHUNK + 1).min(cx));
    for r in 0..cy { for c in c0..c1 { g.awake_next[r * cx + c].store(1, Relaxed); } }
}

/// How many slides of fresh ground beyond each edge of the window are kept built ahead: 96 m above and
/// below, so a fall or climb of that far never waits for the generator.
pub const AHEAD_SLIDES: usize = 3;
/// ...and tile columns beyond each side of a window that slides sideways: 64 m.
pub const AHEAD_X: usize = 2;
impl crate::world::World {
    /// Put the fresh tile for window cells `x0..x0 + tw`, rows `y0..y0 + BH` in place (from the stream, or built now).
    pub(crate) fn put_fresh_tile(&mut self, x0: usize, y0: usize) {
        let (w, tw) = (self.g.w, crate::band::tile_w(self.g.w)); let b = self.stream.take(&self.gen, tw, (self.ox + x0 as i64, self.oy + y0 as i64));
        for y in 0..BH { let (a, k) = ((y0 + y) * w + x0, y * tw); self.g.mat[a..a + tw].copy_from_slice(&b.mat[k..k + tw]); self.g.flags[a..a + tw].copy_from_slice(&b.flags[k..k + tw]); }
    }
    /// The fresh tiles (keys) to keep built: `slides` slides beyond the window's top and bottom, and (in a
    /// window that slides sideways) `AHEAD_X` tile columns beyond each side, the ones a slide would need
    /// soonest first (by how far the player has to go before the window slides onto them, down or sideways);
    /// tiles already visited (stored) are never rebuilt.
    pub fn stream_want(&self, slides: usize) -> Vec<Key> {
        let (w, h, tw) = (self.g.w as i64, self.g.h as i64, crate::band::tile_w(self.g.w) as i64);
        let n = (slides * crate::world::SLIDE / BH) as i64; let bh = BH as i64;
        let (px, py) = self.moles.first().map_or((w / 2, h / 2), |m| (m.x as i64, m.y as i64));
        let cols: Vec<i64> = (0..w / tw).map(|k| self.ox + k * tw).collect();
        let mut v: Vec<Key> = (0..n).flat_map(|k| [self.oy + h + k * bh, self.oy - (k + 1) * bh]).flat_map(|y| cols.iter().map(move |&x| (x, y))).collect();
        if self.roams_x() { let rows: Vec<i64> = (0..h / bh).map(|k| self.oy + k * bh).collect();
            v.extend((0..AHEAD_X as i64).flat_map(|k| [self.ox + w + k * tw, self.ox - (k + 1) * tw]).flat_map(|x| rows.iter().map(move |&y| (x, y)))); }
        v.retain(|k| !self.bands.contains_key(k));
        // how far the player goes before a slide brings tile k in: to the slide's trigger margin, then a slide's
        // length per slide further out (the margins as in `follow_colony`)
        let (sl, mv, mx) = (crate::world::SLIDE as i64, ((100 * crate::mat::S) as i64).min((h - crate::world::SLIDE as i64) / 2), (crate::world::SIDE_MARGIN as i64).min((w - tw) / 2));
        let need = |k: &Key| { let (tx, ty) = (k.0 - self.ox, k.1 - self.oy);
            let dx = if tx >= w { (px - (w - mx)).min(0).abs() + (tx - w) / tw * tw } else if tx < 0 { (px - mx).max(0) + (-tx - tw) / tw * tw } else { 0 };
            let dy = if ty >= h { (py - (h - mv)).min(0).abs() + (ty - h) / sl * sl } else if ty < 0 { (py - mv).max(0) + (-ty - bh) / sl * sl } else { 0 };
            dx + dy };
        v.sort_by_key(need); v
    }
    /// One step (a few ms) of building fresh ground ahead of the window; false when it is all built.
    /// The game calls it each frame it has time to spare, and behind the loading line until
    /// `stream_progress` is 1. Never changes the world: a tile is the same built now or at the slide.
    pub fn prefetch_step(&mut self) -> bool { let want = self.stream_want(AHEAD_SLIDES); let tw = crate::band::tile_w(self.g.w); self.stream.step(&self.gen, tw, &want) }
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
    /// Queue the solves a slide by `cols`, `rows` needs: `ent` (the entering cells and the old ones they meet),
    /// `far` (the edge the window left). What an earlier slide still had queued moves with the rock (boxes that
    /// left the window are dropped), so several slides in one tick (a respawn) don't each solve at once.
    /// A queue bigger than the window is cut to one pass over the whole window.
    pub(crate) fn start_seam(&mut self, cols: i64, rows: i64, ent: (usize, usize, usize, usize), far: (usize, usize, usize, usize)) {
        let (w, h) = (self.g.w as i64, self.g.h as i64);
        let split = |b: (usize, usize, usize, usize)| (b.0..=b.2).step_by(SEAM_COLS).map(move |x0| (x0, b.1, (x0 + SEAM_COLS - 1).min(b.2), b.3));
        let mut boxes: Vec<((usize, usize, usize, usize), bool)> = vec![];
        if let Some(sm) = self.stream.seam.take() {
            for &((x0, y0, x1, y1), r) in &sm.boxes[sm.k..] {
                let (a, b) = ((y0 as i64 - rows).max(0), (y1 as i64 - rows).min(h - 1));
                let (c, d) = ((x0 as i64 - cols).max(0), (x1 as i64 - cols).min(w - 1));
                if a <= b && c <= d { boxes.push(((c as usize, a as usize, d as usize, b as usize), r)); }
            }
        }
        boxes.extend(split(ent).map(|b| (b, true)).chain(split(far).map(|b| (b, false))));
        let queued: i64 = boxes.iter().map(|((x0, y0, x1, y1), _)| ((x1 - x0 + 1) * (y1 - y0 + 1)) as i64).sum::<i64>();
        if queued > w * h { boxes = split((0, 0, w as usize - 1, h as usize - 1)).map(|b| (b, true)).collect(); }
        let tries = vec![1; boxes.len()]; self.stream.seam = Some(Seam { boxes, k: 0, tries, reach: false });
    }
    /// One step of the seam (the reach pass, then a box a tick); true while more remain (the regular stress
    /// pass waits for them). At the seam, fresh bands were shored against generated rock and the old rows
    /// against the window's edge, so what fails there is put right as the generator's shoring does: rock
    /// left with no load path is cleared, an overloaded cell becomes limestone (weak ground, the start zone)
    /// or granite, and a box still failing is solved again with the voids round its failures filled. The
    /// far edge cracks, as does a seam box still failing after `SEAM_TRIES`.
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
        let (w, h, ox, oy, w0) = (self.g.w, self.g.h, self.ox, self.oy, self.gen.w0); let it = if tries == 1 { 0 } else { 4 + 3 * (tries as i32 - 2) };
        let soft = |x: usize, y: usize| gen::in_start_zone(w0, ox + x as i64, oy + y as i64);
        for &i in &f { let i = i as usize; let (x, y) = (i % w, i / w);
            if !self.st.reach[i] { self.g.set(i, Mat::Empty); continue; } // rock left hanging with no load path
            if it >= 4 { let r = (1 + (it - 4) / 3).min(4 * crate::mat::S as i32);
                for dx in -r..=r { for dy in -r..=r + 1 { let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx <= 1 || ny < 0 || nx as usize >= w - 2 || ny as usize >= h { continue; } let (nx, ny) = (nx as usize, ny as usize); let j = ny * w + nx;
                    if self.gen.in_grotto_void(ox + nx as i64, oy + ny as i64) { continue; }
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
/// step 2): from bedrock and the static cells of the window's edges (top, bottom and sides), static rock carries
/// static rock beside it, and anything solid or loose carries what rests on top of it. One pass, no
/// strength sums: 10-30 ms on the game window where a full solve takes 60 or more.
pub fn reach_all(g: &Grid) -> Vec<bool> {
    use crate::mat::Kind;
    let (w, n) = (g.w, g.mat.len()); let mut reach = vec![false; n]; let mut q: Vec<u32> = Vec::with_capacity(n / 2);
    let supports = |i: usize| matches!(g.m(i).kind(), Kind::Powder | Kind::Solid);
    for i in 0..n { if g.mat[i] == Mat::Bedrock as u8 || ((i < w || i >= n - w || i % w == 0 || i % w == w - 1) && g.is_static(i)) { reach[i] = true; q.push(i as u32); } }
    let mut head = 0;
    while head < q.len() {
        let i = q[head] as usize; head += 1; let (x, y) = (i % w, i / w);
        if g.is_static(i) { for j in g.n4(x, y) { if !reach[j] && g.is_static(j) { reach[j] = true; q.push(j as u32); } } }
        if y > 0 { let j = i - w; if !reach[j] && supports(j) { reach[j] = true; q.push(j as u32); } }
    }
    reach
}
