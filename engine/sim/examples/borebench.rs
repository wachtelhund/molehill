//! Frame cost and cave-ins while a ridden steam borer cuts through real ground on the game-sized window
//! (256 × 128 m): a fuelled worm borer starts beside the player mole and is steered along a route for a while.
//! * `east`, `west`, `down`, `up`, `diag` (east and down): the stick held that way
//! * `wave`: east, swinging down and up every 3 s
//! * `idle`: nobody steers (the borer and the mole just sit there): the cost of the world ticking by itself
//! Prints the tick times, how far it got, and what the ground did around it: cells that crumbled, split into
//! cut free (no load path left) and overloaded, by material, and the ticks the rider was hurt.
//! Usage: cargo run --release -p molehill-sim --no-default-features --example borebench -- [seed] [route] [seconds]
//! Times are CPU ms of the stepping thread (single-threaded, as on the web); disasters are off.
use molehill_sim::{borer::Borer, craft::Item, mat::{Mat, CPM, S}, world::World};
fn cpu_ms() -> f64 { let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 }; unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t); } t.tv_sec as f64 * 1e3 + t.tv_nsec as f64 / 1e6 }
fn q(v: &[f64], p: f64) -> f64 { let mut s = v.to_vec(); s.sort_by(|a, b| a.partial_cmp(b).unwrap()); if s.is_empty() { 0. } else { s[((s.len() as f64 - 1.) * p) as usize] } }
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: u32 = a.first().and_then(|s| s.parse().ok()).unwrap_or(4242);
    let route = a.get(1).map(|s| s.as_str()).unwrap_or("east");
    let secs: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
    while w.prefetch_step() { std::thread::sleep(std::time::Duration::from_millis(1)); }
    let (mx, my) = (w.moles[0].x as f32, w.moles[0].y as f32);
    let mut b = Borer::new(mx, my - 6., 0.); b.fuel = vec![(Item::Coal, 500)]; b.steam = 300.;
    w.tr.borers.push(b); w.tr.borer_ride = Some(0);
    let (sx, sy) = (w.ox as f32 + mx, w.oy as f32 + my);
    let (mut ts, mut worst, c0, mut hurt) = (vec![], (0., 0usize), w.crumbled, 0);
    let (mut by_mat, mut hang, mut over) = (std::collections::BTreeMap::<String, u32>::new(), 0, 0);
    for t in 0..secs * 60 {
        w.input.right = false; w.input.left = false; w.input.down = false; w.input.up = false;
        match route { "east" => w.input.right = true, "west" => w.input.left = true, "down" => w.input.down = true, "up" => w.input.up = true,
            "diag" => { w.input.right = true; w.input.down = true; }
            "wave" => { w.input.right = true; if (t / 180) % 2 == 0 { w.input.down = true; } else { w.input.up = true; } }
            "idle" => {} _ => { eprintln!("routes: east west down up diag wave idle"); std::process::exit(2); } }
        let hp = w.moles[0].hp;
        // cells on the last tick of their countdown: the ones that are something else after the step gave way
        let due: Vec<(usize, Mat)> = w.st.cracking.iter().map(|&i| i as usize).filter(|&i| w.st.pending[i] == 1).map(|i| (i, w.g.m(i))).collect();
        let t0 = cpu_ms(); w.step(); w.prefetch_step(); let ms = cpu_ms() - t0; ts.push(ms);
        if ms > worst.0 { worst = (ms, t); }
        for (i, m) in due { if w.g.m(i) != m || w.g.loose(i) { *by_mat.entry(format!("{m:?}")).or_default() += 1; if w.st.reach.get(i) == Some(&false) { hang += 1; } else { over += 1; } } }
        if w.moles[0].hp < hp { hurt += 1; } w.moles[0].hp = 100.; w.moles[0].breath = 100.;
        if !w.moles[0].alive || w.tr.borer_ride.is_none() { println!("  ride ended at tick {t} (alive {})", w.moles[0].alive); break; }
    }
    let b = &w.tr.borers[0];
    let (ex, ey) = (w.ox as f32 + b.x, w.oy as f32 + b.y);
    println!("seed {seed} {route}: {} ticks, went {:.0} m east {:.0} m down; mean {:.2} p99 {:.2} worst {:.1} ms (tick {}); over 16.7 ms: {}, over 8 ms: {}",
        ts.len(), (ex - sx) / CPM as f32, (ey - sy) / CPM as f32, ts.iter().sum::<f64>() / ts.len() as f64, q(&ts, 0.99), worst.0, worst.1, ts.iter().filter(|&&t| t > 16.7).count(), ts.iter().filter(|&&t| t > 8.).count());
    println!("   crumbled {} cells ({:.1} m²): cut free {hang}, overloaded {over}, by material {by_mat:?}; rider hurt on {hurt} ticks", w.crumbled - c0, (w.crumbled - c0) as f32 / (CPM * CPM) as f32);
}
