use molehill_sim::{world::World, mat::Mat};
fn main() {
    let s: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut w = World::new(1024, 512, s);
    let f = w.st.solve(&w.g);
    println!("oy {} fail {}", w.oy, f.len());
    for &i in f.iter().step_by((f.len()/12).max(1)) { let i = i as usize; println!("({},{}) {} u{} reach {}", i%1024, i/1024, Mat::from_u8(w.g.mat[i]).p().name, w.st.util[i], w.st.reach[i]); }
}
