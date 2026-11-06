//! Streaming fresh ground (`stream.rs`): a band nobody has visited is a pure function of the seed and its
//! row, however and whenever it is built, and building ahead never changes the world; a slide onto
//! ground built ahead stays cheap.
use molehill_sim::{band::BH, mat::S, stream::{band_cells, BandCache}, world::{World, SLIDE}};

/// CPU time of this thread (ms): other load on the machine doesn't count.
fn cpu_ms() -> f64 { let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 }; unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t); } t.tv_sec as f64 * 1e3 + t.tv_nsec as f64 / 1e6 }

#[test]
fn a_band_is_the_same_however_it_is_built() {
    let w = World::blank(512 * S, 256 * S, 31); let gw = w.g.w; let below = w.oy + w.g.h as i64;
    let (a, b) = (below, below + 5 * BH as i64);
    let once = band_cells(&w.gen, gw, a);
    // after another band, in slices, and on the worker thread
    let _ = band_cells(&w.gen, gw, b); let again = band_cells(&w.gen, gw, a);
    let mut sliced = BandCache::default(); sliced.sliced = true; while sliced.step(&w.gen, gw, &[b, a]) {}
    let mut threaded = BandCache::default(); while threaded.step(&w.gen, gw, &[a, b]) { std::thread::sleep(std::time::Duration::from_millis(2)); }
    for (name, band) in [("again", again), ("sliced", sliced.take(&w.gen, gw, a)), ("threaded", threaded.take(&w.gen, gw, a))] {
        assert!(band.mat == once.mat && band.flags == once.flags, "{name}: the band differs");
    }
}

#[test]
fn building_ahead_never_changes_the_world() {
    let mk = || { let mut w = World::new(512 * S, 256 * S, 5); w.disasters.enabled = false; w };
    let (mut ahead, mut plain) = (mk(), mk());
    ahead.stream.sliced = true; while ahead.prefetch_step() {}
    assert_eq!(ahead.stream_progress(), 1.);
    for rows in [SLIDE as i64, SLIDE as i64, -(SLIDE as i64)] {
        ahead.shift(rows); plain.shift(rows); ahead.finish_seam(); plain.finish_seam();
        assert!(ahead.g.mat == plain.g.mat && ahead.g.flags == plain.g.flags, "slide {rows}: the window differs");
    }
}

/// The perf gate: on the game-sized window, a slide onto ground built ahead costs a few tens of ms of CPU
/// (it was 0.4-1.3 s when the fresh rows were generated and shored in the slide). Generous: other work on
/// the machine still slows a thread's CPU time down somewhat.
#[test]
fn a_slide_onto_ground_built_ahead_is_cheap() {
    let mut w = World::new(1024 * S, 512 * S, 4242); w.disasters.enabled = false;
    while w.prefetch_step() { std::thread::sleep(std::time::Duration::from_millis(2)); }
    let mut worst = 0f64;
    for rows in [SLIDE as i64, SLIDE as i64, -(SLIDE as i64)] {
        let t = cpu_ms(); w.shift(rows); worst = worst.max(cpu_ms() - t);
        while w.seam_busy() { let t = cpu_ms(); w.step(); worst = worst.max(cpu_ms() - t); }
        while w.prefetch_step() { std::thread::sleep(std::time::Duration::from_millis(2)); }
    }
    assert!(worst < 150., "a slide (or a tick of its seam) took {worst:.0} ms of CPU");
}
