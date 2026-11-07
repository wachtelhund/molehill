//! Lists the grottos in a fresh holt's window and draws the first one (one char per 0.25 m tile).
//! Usage: cargo run --release -p molehill-sim --example grottos [seed]
use molehill_sim::{mat::{Mat, S}, world::World};
fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4242);
    let t = std::time::Instant::now(); let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
    println!("World::new {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
    let gs = w.gen.grottos_in_rows(w.oy, w.oy + w.g.h as i64);
    for g in &gs { println!("grotto {:08x} at x {} depth {:.1} m, {}x{} m, {} trees", g.id, (g.cx as usize), g.cy as f32 / molehill_sim::mat::CPM as f32, 2 * g.rx / molehill_sim::mat::CPM, g.ry / molehill_sim::mat::CPM, g.ntrees); }
    let c0 = w.crumbled; for _ in 0..1200 { w.step(); } println!("crumbled in 20 s: {}", w.crumbled - c0);
    if let Some(g) = gs.iter().find(|g| g.cy - w.oy > 0 && ((g.cy - w.oy) as usize) < w.g.h) {
        let cy = (g.cy - w.oy) as usize; let (rx, ry) = (g.rx + 2 * S * 4, g.ry + 2 * S * 4);
        for y in (cy.saturating_sub(ry)..(cy + ry).min(w.g.h)).step_by(S) { let row: String = ((g.cx as usize).saturating_sub(rx)..((g.cx as usize) + rx).min(w.g.w)).step_by(S).map(|x| match w.g.get(x, y) {
            Mat::Empty => '.', Mat::Water => 'w', Mat::RootTree => 'T', Mat::TreeRoot => 'R', Mat::Moss => 'm', Mat::Mycelium => 'y', Mat::Limestone => 'L', Mat::Dirt => 'd', Mat::Clay => 'c', Mat::Granite => '#', Mat::Rock => 'r', _ => '?' }).collect(); println!("{row}"); }
    }
}
