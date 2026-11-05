//! Diagnostic: what one TNT charge sets off, and for how long. A charge goes off in a generated world
//! (as in the game: disasters on unless `calm` is given), then every event the sim emits is logged per
//! second for the next N seconds, with the blasts actually run (`World::blasts_run`).
//! Usage: cargo run --release -p molehill-sim --example blastevents [seed] [seconds] [dx_m] [dy_m] [calm]
use molehill_sim::world::{Event, World};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(11);
    let secs: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let dx: f32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(3.);
    let dy: f32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.);
    let calm = a.iter().any(|s| s == "calm");
    let c = molehill_sim::mat::CPM as f32;
    let mut w = World::new(1024 * molehill_sim::mat::S, 512 * molehill_sim::mat::S, seed); w.disasters.enabled = !calm;
    for _ in 0..120 { w.step(); }
    let (mx, my) = (w.moles[0].x as f32, w.moles[0].y as f32);
    let (x, y) = ((mx + dx * c) as usize, (my + dy * c) as usize);
    println!("seed {seed}: charge at {:.1} m, {:.1} m from the mole ({:?} there)", dx, dy, w.g.get(x, y));
    let b0 = w.blasts_run; w.blast_tnt(x, y);
    let names = ["Dig", "Crumble", "Crack", "Place", "Chest", "Quest", "Explode", "Discover", "Hurt", "Craft", "Eat", "Death", "Warning", "Quake", "Click", "Harvest", "Gasp", "Lore"];
    let mut first = true;
    for s in 0..secs {
        let mut n = [0u32; 18]; let mut crumbled = 0;
        for t in 0..60 { if !(first && t == 0) { w.events.clear(); } let before = w.crumbled; w.step(); crumbled += w.crumbled - before; for e in &w.events { n[*e as usize] += 1; } first = false; }
        let list: Vec<String> = names.iter().enumerate().filter(|(k, _)| n[*k] > 0).map(|(k, nm)| format!("{nm}×{}", n[k])).collect();
        println!("  s{:>2}: blasts {:>2}  crumbled cells {:>5}  debris {:>4}  {}", s + 1, w.blasts_run - b0, crumbled, w.debris.len(), list.join(" "));
    }
    let _ = Event::Explode;
}
