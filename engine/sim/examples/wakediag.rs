//! Diagnostic: after a world settles, how much of the CA is still awake and what is still changing?
//! Usage: cargo run --release -p molehill-sim --example wakediag [seed]
use molehill_sim::mat::{Mat, NMAT};
use molehill_sim::world::World;
fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(11);
    let mut w = World::new(512, 256, seed); w.disasters.enabled = false;
    let t = std::time::Instant::now();
    for _ in 0..1200 { w.step(); }
    let settle = t.elapsed();
    let mut changed = vec![0u32; NMAT]; let mut awake = 0;
    let t = std::time::Instant::now();
    for _ in 0..60 {
        let before = w.g.mat.clone(); w.step(); awake += w.g.active_chunks();
        for i in 0..before.len() { if before[i] != w.g.mat[i] { changed[w.g.mat[i] as usize] += 1; } }
    }
    println!("seed {seed}: 1200 ticks in {:.0} ms; then avg awake chunks {:.1}/{} over 60 ticks ({:.2} ms/tick incl. diff)", settle.as_secs_f64() * 1e3, awake as f64 / 60., w.g.awake.len(), t.elapsed().as_secs_f64() * 1e3 / 60.);
    for k in 0..NMAT { if changed[k] > 0 { println!("  {:?}: {} cell changes", Mat::from_u8(k as u8), changed[k]); } }
}
