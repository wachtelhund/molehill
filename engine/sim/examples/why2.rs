use molehill_sim::{world::World, mat::Mat};
fn main() {
    let s: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut w = World::new(1024, 512, s); w.disasters.enabled = false;
    for k in 0..1800 { w.step(); if k % 150 == 0 { for m in &w.moles { let (x, y) = (m.x, m.y);
        print!("{}({},{}) hp{:.0} br{:.0} f{:.0} {:?} | ", m.name, x, y, m.hp, m.breath, m.hunger, (0..3).map(|j| Mat::from_u8(w.g.mat[(y - j) * 1024 + x]).p().name).collect::<Vec<_>>()); } println!(); } }
    println!("{:?}", &w.log[w.log.len().saturating_sub(6)..]);
}
