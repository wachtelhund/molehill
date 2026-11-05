use molehill_sim::{world::World, mat::Mat};
fn main() {
    let hs: Vec<_> = (1..=48u32).map(|s| std::thread::spawn(move || {
        let w = World::new(1024, 512, s * 101);
        let (sx, sy) = (512usize, 256usize); let (mut wa, mut la, mut sa) = (0, 0, 0);
        for y in sy - 50..sy + 60 { for x in sx - 110..sx + 110 { match Mat::from_u8(w.g.mat[y * 1024 + x]) { Mat::Water => wa += 1, Mat::Lava => la += 1, Mat::Sand | Mat::Gravel => sa += 1, _ => {} } } }
        (s * 101, wa, la, sa)
    })).collect();
    for h in hs { let r = h.join().unwrap(); if r.1 > 50 && r.2 > 50 { println!("{:?}", r); } }
}
