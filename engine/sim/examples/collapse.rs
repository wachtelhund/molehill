use molehill_sim::world::World;
use std::time::Instant;
fn main() {
    let mut w = World::new(1024 * molehill_sim::mat::S, 512 * molehill_sim::mat::S, 9630);
    let mut hist = [0u32; 12];
    let t = Instant::now();
    for k in 0..600 { let before = w.crumbled; w.step(); if w.crumbled > before { let _ = k; } }
    println!("600 ticks {:?} crumbled {} cracking {}", t.elapsed(), w.crumbled, w.st.cracking.len());
    for &i in &w.st.cracking { let y = i as usize / 1024; hist[(y / 48).min(11)] += 1; }
    println!("cracking by y/48: {:?}", hist);
    let t = Instant::now(); for _ in 0..10 { w.compute_light((300, 0, 700, 250)); } println!("light 400x250 x10 {:?}", t.elapsed());
    let t = Instant::now(); for _ in 0..10 { w.st.solve(&w.g); } println!("solve x10 {:?}", t.elapsed());
}
