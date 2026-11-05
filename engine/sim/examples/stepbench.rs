//! Per-tick cost of `World::step` on the game-sized window (256 × 128 m), from a fresh holt.
//! Usage: cargo run --release -p molehill-sim --example stepbench [seed] [ticks]
use molehill_sim::{mat::S, world::World};
fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4242);
    let n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(900);
    let t = std::time::Instant::now(); let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
    println!("new world {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
    let mut ts = vec![]; let mut solves = 0;
    for k in 0..n { if k > n / 3 { w.input.right = (k / 120) % 2 == 0; }
        let dirty = w.g.struct_dirty.is_some(); let t = std::time::Instant::now(); w.step(); ts.push(t.elapsed().as_secs_f64() * 1e3); if dirty && w.g.struct_dirty.is_none() { solves += 1; } }
    let mut s = ts.clone(); s.sort_by(|a, b| a.partial_cmp(b).unwrap()); let q = |p: f64| s[((s.len() as f64 - 1.) * p) as usize];
    println!("{n} ticks: mean {:.2} ms  p50 {:.2}  p95 {:.2}  p99 {:.2}  max {:.2}  ({solves} stress solves)", ts.iter().sum::<f64>() / n as f64, q(0.5), q(0.95), q(0.99), q(1.));
    // which periodic sweeps cost what: average tick time by tick class over the last third
    let start = w.tick - n as u64; let mut cls = std::collections::BTreeMap::<&str, (f64, u32)>::new();
    for (k, &t) in ts.iter().enumerate().skip(2 * n / 3) { let tk = start + 1 + k as u64;
        let c = if tk % 30 == 0 { "t%30 (cool lava, air, farms, chests)" } else if tk % 15 == 0 { "t%15 (air)" } else if tk % 6 == 0 { "t%6 (stress)" } else { "plain" };
        let e = cls.entry(c).or_insert((0., 0)); e.0 += t; e.1 += 1; }
    for (c, (t, k)) in cls { println!("  {c}: {:.2} ms avg over {k}", t / k as f64); }
    let t = std::time::Instant::now(); for _ in 0..10 { let _ = w.st.solve(&w.g); } println!("stress solve {:.2} ms", t.elapsed().as_secs_f64() * 100.);
    let t = std::time::Instant::now(); for _ in 0..10 { let r = (900 * S, 400 * S, 1150 * S, 560 * S); w.compute_light(r); } println!("light (view rect) {:.2} ms", t.elapsed().as_secs_f64() * 100.);
    // the same with the headlamp on and 20 torches in view
    w.gear.lamp = true; for k in 0..20 { w.torches.push((900 * S + k * 12 * S, 480 * S)); }
    let t = std::time::Instant::now(); for _ in 0..10 { let r = (900 * S, 400 * S, 1150 * S, 560 * S); w.compute_light(r); } println!("light (view rect, lamp + 20 torches) {:.2} ms", t.elapsed().as_secs_f64() * 100.);
}
