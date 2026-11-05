//! Ruins of the Ancients: how many per seed, and a look at one in a generated world.
//! Usage: cargo run --release -p molehill-sim --example ruinsurvey -- [SEEDS] [--show SEED]
use molehill_sim::gen::Gen;
use molehill_sim::mat::{Mat, CPM, S};
use molehill_sim::world::World;
const W: usize = 1024 * S; const H: usize = 512 * S;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    if let Some(i) = a.iter().position(|x| x == "--show") { show(a[i + 1].parse().unwrap()); return; }
    let n: u32 = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(200);
    let (mut tot, mut kinds, mut first) = (0, [0; 3], vec![]);
    for seed in 0..n { let g = Gen::new(seed, W);
        let rs = g.ruins_in_rows(0, 1000 * CPM as i64); tot += rs.len();
        for r in &rs { kinds[r.kind as usize] += 1; if r.y0 + (r.h as i64) < 60 * CPM as i64 && first.len() < 8 { first.push((seed, r.kind, r.y0 / CPM as i64, r.x0 as i64 / CPM as i64 - 128)); } } }
    println!("{n} seeds, 0-1000 m: {tot} ruins ({:.1} per seed per km); halls {} vaults {} tube stations {}", tot as f32 / n as f32, kinds[0], kinds[1], kinds[2]);
    println!("in the starting window (above 60 m): {first:?}  (seed, kind, depth m, x m from the burrow)");
}
fn show(seed: u32) {
    let w = World::new(W, H, seed);
    let Some(r) = w.ruins.iter().find(|r| r.y0 - w.oy >= 0 && ((r.y0 - w.oy) as usize + r.h) < H) else { println!("no ruin in the window"); return };
    println!("seed {seed}: a {:?} at x {} depth {:.1} m ({}x{} m)", r.kind, r.x0, r.y0 as f32 / CPM as f32, r.w / CPM, r.h / CPM);
    let k = CPM / 4; let (x0, y0) = (r.x0.saturating_sub(4 * CPM), (r.y0 - w.oy) as usize - 3 * CPM);
    for y in (y0..y0 + r.h + 6 * CPM).step_by(k) { let mut l = String::new(); for x in (x0..(x0 + r.w + 8 * CPM).min(W)).step_by(k) {
        use Mat::*; l.push(match w.g.get(x, y) { Empty => '.', Water => 'w', StoneBrick => 'k', IronBeam => 'I', Door => 'D', Gravel => 'g', Granite => '#', Limestone => 'L', Rock => 'r', Lava => 'l', Basalt => 'a', _ => '+' }); }
        println!("{l}"); }
    println!("(one char = 0.25 m: k brick, I iron, D the waygate's sealed gate, g rubble, w water)");
}
