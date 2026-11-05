//! Cost of one TNT blast in a fresh holt's burrow (game-sized window). Usage: blastbench [seed]
use molehill_sim::{blast::TNT_KG, mat::S, world::World};
fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4242);
    let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
    let (x, y) = (w.moles[0].x + 12 * S, w.moles[0].y);
    let t = std::time::Instant::now(); let r = w.blast(x, y, TNT_KG); let dt = t.elapsed().as_secs_f64() * 1e3;
    println!("blast {dt:.1} ms: {} cells reached ({:.1} m), {} shattered, {} cracked, {} thrown; mole hp {:.0}", r.cells, r.reach_m, r.shattered, r.cracked, r.moved, w.moles[0].hp);
    let t = std::time::Instant::now(); for _ in 0..30 { w.step(); } println!("30 ticks after: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
}
