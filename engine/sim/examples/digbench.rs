//! Frame cost of moving through the world on the game-sized window (256 × 128 m): every tick is timed
//! along several routes, and ticks where the window slid (`World::shift` / `shift_x`: tiles packed,
//! unpacked or generated) are counted apart. Each route starts from a fresh world with the same seed.
//! * `down`: a shaft is cleared under the player mole so it falls as fast as it can (digging straight down;
//!   about 15 m/s, `TUNE.terminal`: the longest real fall)
//! * `up`, `diag`: the mole is carried along at running speed (6 m/s) through a cleared pocket
//! * `side`, `west`: the same, 300 m east or west (the window slides sideways a tile at a time)
//! * `fastdown`, `fastup`: the same at 40 m/s, faster than the mole can fall (a stress test: the 40 m/s climb
//!   outruns the ground built ahead)
//! * `respawn`: dies 200 m down and comes back to the burrow (the window slides all the way back at once)
//! * `load`: saved 200 m down and loaded, then carried 100 m deeper into ground nobody has seen
//! Like the game, each frame is a tick plus building fresh ground ahead of the window (`World::prefetch_step`:
//! one call natively, slices for up to 6 ms with `--sliced`, as the web build's `PREFETCH_MS`), and a world is
//! first built `AHEAD_SLIDES` deep behind the loading line (timed apart, `prep`).
//! `--no-prefetch` skips both: every fresh band is then built at the slide that needs it. Natively a worker
//! thread builds the bands (off the timed thread); `--sliced` builds them in slices on the stepping thread,
//! as the web build does.
//! Usage: cargo run --release -p molehill-sim --no-default-features --example digbench -- [seed] [route ...] [--sliced] [--no-prefetch] [--realtime] [--max-ms N]
//! Times are CPU ms of the stepping thread (single-threaded, as on the web), so other load on the machine doesn't skew them.
//! `--max-ms N` exits with an error if any route's worst tick is over N ms (a coarse regression gate).
//! `--realtime` paces the frames at 60 Hz of wall time, as the game runs them, so the worker thread has
//! the time it has in play (without it the ticks run back to back, several times faster than the game).
use molehill_sim::{mat::{Mat, CPM, S}, save, world::World};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
static PREFETCH: AtomicBool = AtomicBool::new(true);
static SLICED: AtomicBool = AtomicBool::new(false);
static REALTIME: AtomicBool = AtomicBool::new(false);

/// CPU time of this thread (ms): machine load from other processes doesn't count. Run single-threaded
/// (`--no-default-features`) so a tick's whole cost is on this thread, as on the web.
fn cpu_ms() -> f64 { let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 }; unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t); } t.tv_sec as f64 * 1e3 + t.tv_nsec as f64 / 1e6 }
struct Run { name: &'static str, ts: Vec<f64>, slide: Vec<f64>, seam: Vec<f64>, other: Vec<f64>, metres: f32, extra: String }
fn q(v: &[f64], p: f64) -> f64 { let mut s = v.to_vec(); s.sort_by(|a, b| a.partial_cmp(b).unwrap()); if s.is_empty() { 0. } else { s[((s.len() as f64 - 1.) * p) as usize] } }
fn fresh(seed: u32) -> (World, f64) { let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false; let p = prep(&mut w); (w, p) }
/// What the loading line does after a new holt or a load: build the fresh ground ahead (wall ms: with a
/// worker thread the time is spent there, polled once a millisecond as a loading screen polls per frame).
fn prep(w: &mut World) -> f64 {
    w.stream.sliced = SLICED.load(Relaxed); let t = std::time::Instant::now();
    if PREFETCH.load(Relaxed) { while w.prefetch_step() { if !w.stream.sliced { std::thread::sleep(std::time::Duration::from_millis(1)); } } }
    t.elapsed().as_secs_f64() * 1e3
}
/// Clear a pocket the mole's body fits in around (x, y) (its feet row), bedrock kept.
fn pocket(w: &mut World, x: usize, y: usize, below: usize) {
    let gw = w.g.w;
    for yy in y.saturating_sub(3 * CPM)..(y + below).min(w.g.h - 1) { for xx in x.saturating_sub(CPM)..(x + CPM).min(gw - 1) {
        let i = yy * gw + xx; if w.g.mat[i] != Mat::Empty as u8 && w.g.mat[i] != Mat::Bedrock as u8 { w.g.set(i, Mat::Empty); } } }
}
/// One frame: a tick, then (as the game does) one step of building ground ahead.
fn timed(w: &mut World, r: &mut Run) {
    // (sliced, as the web build: slices for up to 6 ms of the frame, as the game's PREFETCH_MS)
    let (oy, ox, seam) = (w.oy, w.ox, w.seam_busy()); let t = cpu_ms(); w.step();
    if PREFETCH.load(Relaxed) { if w.stream.sliced { let t1 = cpu_ms(); while w.prefetch_step() && cpu_ms() - t1 < 6. {} } else { w.prefetch_step(); } }
    let ms = cpu_ms() - t; r.ts.push(ms);
    if REALTIME.load(Relaxed) { let left = 1000. / 60. - ms; if left > 0. { std::thread::sleep(std::time::Duration::from_secs_f64(left / 1e3)); } } if w.oy != oy || w.ox != ox { r.slide.push(ms); } else if seam { r.seam.push(ms); } else { r.other.push(ms); }
}
fn wy(w: &World) -> i64 { w.oy + w.moles[0].y as i64 }
fn wx(w: &World) -> i64 { w.ox + w.moles[0].x as i64 }
/// Carry the mole by (dx, dy) cells per tick (fractional steps accumulate) until it has gone `metres`.
/// Positions are world cells, so a sideways slide carries on where it was.
fn carry(w: &mut World, r: &mut Run, vx: f32, vy: f32, metres: f32) {
    let (x0, y0) = (wx(w) as f32, wy(w) as f32); let (mut fx, mut fy) = (x0, y0);
    let (mut vx, mut gone, mut k) = (vx, 0f32, 0);
    while gone / (CPM as f32) < metres && k < 100_000 {
        if !w.roams_x() { let (gw, wx0) = (w.g.w as f32, w.ox as f32); // a window that doesn't slide sideways: turn at its sides
            if fx + vx < wx0 + 12. * CPM as f32 { vx = vx.abs(); } else if fx + vx > wx0 + gw - 12. * CPM as f32 { vx = -vx.abs(); } }
        fx += vx; fy += vy; gone += vx.abs() + vy.abs();
        let y = (fy as i64 - w.oy).clamp(3 * CPM as i64, w.g.h as i64 - 2) as usize; let x = (fx as i64 - w.ox).clamp(2, w.g.w as i64 - 3) as usize;
        pocket(w, x, y, CPM); let m = &mut w.moles[0]; m.x = x; m.y = y; m.fall = 0; m.hp = 100.; m.breath = 100.;
        timed(w, r); k += 1;
    }
    r.metres = gone / CPM as f32; let _ = (x0, y0);
}
fn fall(w: &mut World, r: &mut Run, metres: f32) {
    let start = wy(w); let mut k = 0;
    while ((wy(w) - start) as f32 / CPM as f32) < metres && k < 200_000 {
        let (x, y) = (w.moles[0].x, w.moles[0].y); pocket(w, x, y, 2 * CPM); w.moles[0].hp = 100.; timed(w, r); k += 1;
    }
    r.metres = (wy(w) - start) as f32 / CPM as f32;
}
fn route(name: &'static str, seed: u32) -> Run {
    let (mut w, p) = fresh(seed); let mut r = Run { name, ts: vec![], slide: vec![], metres: 0., seam: vec![], other: vec![], extra: format!("prep {p:.0} ms") };
    let run = 6. * CPM as f32 / 60.; let fast = 40. * CPM as f32 / 60.;
    match name {
        "down" => fall(&mut w, &mut r, 200.),
        "up" => carry(&mut w, &mut r, 0., -run, 200.),
        "side" => carry(&mut w, &mut r, run, 0., 300.),
        "west" => carry(&mut w, &mut r, -run, 0., 300.),
        "diag" => carry(&mut w, &mut r, run * 0.7, run * 0.7, 200.),
        "fastdown" => carry(&mut w, &mut r, 0., fast, 400.),
        "fastup" => carry(&mut w, &mut r, 0., -fast, 400.),
        "respawn" => {
            carry(&mut w, &mut r, 0., fast, 200.); r.ts.clear(); r.slide.clear(); r.seam.clear(); r.other.clear();
            w.moles[0].alive = false; for _ in 0..200 { timed(&mut w, &mut r); }
            r.metres = 200.; r.extra = format!("respawn tick {:.1} ms", q(&r.ts, 1.));
        }
        "load" => {
            carry(&mut w, &mut r, 0., fast, 200.); r.ts.clear(); r.slide.clear(); r.seam.clear(); r.other.clear();
            let b = save::save(&w); let t = cpu_ms(); let mut l = save::load(&b).unwrap(); let ld = cpu_ms() - t; l.disasters.enabled = false; let p = prep(&mut l);
            carry(&mut l, &mut r, 0., run, 100.); r.extra = format!("load {ld:.0} ms + prep {p:.0} ms");
        }
        _ => panic!("unknown route {name}"),
    }
    r
}
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let max_ms: Option<f64> = a.iter().position(|s| s == "--max-ms").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok());
    if a.iter().any(|s| s == "--no-prefetch") { PREFETCH.store(false, Relaxed); }
    if a.iter().any(|s| s == "--sliced") { SLICED.store(true, Relaxed); }
    if a.iter().any(|s| s == "--realtime") { REALTIME.store(true, Relaxed); }
    let pos: Vec<&String> = a.iter().take_while(|s| *s != "--max-ms").filter(|s| !s.starts_with("--")).collect();
    let seed: u32 = pos.first().and_then(|s| s.parse().ok()).unwrap_or(4242);
    let all = ["down", "up", "side", "west", "diag", "fastdown", "fastup", "respawn", "load"];
    let names: Vec<&'static str> = { let v: Vec<&'static str> = all.iter().copied().filter(|n| pos.iter().skip(1).any(|p| p.as_str() == *n)).collect(); if v.is_empty() { all.to_vec() } else { v } };
    println!("seed {seed}: CPU ms per frame, single thread (60 Hz budget 16.7 ms){}{}", if !PREFETCH.load(Relaxed) { ", no prefetch" } else if SLICED.load(Relaxed) { ", tiles built in slices (web)" } else { ", tiles built on a worker thread" }, if REALTIME.load(Relaxed) { ", paced at 60 Hz" } else { "" });
    println!("{:<9} {:>6} {:>6} {:>7} {:>7} {:>8} {:>7} {:>9} {:>9} {:>9}  ", "route", "metres", "ticks", "mean", "p99", "worst", "slides", "slide max", "seam max", "rest max");
    let mut worst_all = 0f64;
    for n in names {
        let r = route(n, seed); let worst = q(&r.ts, 1.); worst_all = worst_all.max(worst);
        println!("{:<9} {:>6.0} {:>6} {:>7.2} {:>7.2} {:>8.1} {:>7} {:>9.1} {:>9.1} {:>9.1}  {}", r.name, r.metres, r.ts.len(), r.ts.iter().sum::<f64>() / r.ts.len().max(1) as f64, q(&r.ts, 0.99), worst, r.slide.len(), q(&r.slide, 1.), q(&r.seam, 1.), q(&r.other, 1.), r.extra);
    }
    if let Some(m) = max_ms { if worst_all > m { eprintln!("worst tick {worst_all:.1} ms is over the {m} ms gate"); std::process::exit(1); } }
}
