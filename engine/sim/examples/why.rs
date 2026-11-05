use molehill_sim::{world::World, mat::Mat};
fn main() {
    let s: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut w = World::new(1024, 512, s); w.disasters.enabled = false;
    let f = w.st.solve(&w.g); println!("fail0 {}", f.len());
    for &i in f.iter().take(8) { let i = i as usize; println!("  ({},{}) {} u{} r{}", i%1024, i/1024, Mat::from_u8(w.g.mat[i]).p().name, w.st.util[i], w.st.reach[i]); }
    for k in 0..240 { w.step(); if let Some(&i) = w.st.cracking.iter().find(|&&i| { let (x, y) = (i as usize % 1024, i as usize / 1024); x > 480 && x < 545 && y > 230 && y < 262 }) {
        let i = i as usize; println!("t{k} ({},{}) {} u{} r{}", i%1024, i/1024, Mat::from_u8(w.g.mat[i]).p().name, w.st.util[i], w.st.reach[i]); break; } }
}
