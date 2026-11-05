use molehill_sim::{world::World, mat::Mat, mat::Kind};
fn main() {
    let s: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let w = World::new(1024 * molehill_sim::mat::S, 512 * molehill_sim::mat::S, s);
    println!("mole0 {} {}", w.moles[0].x, w.moles[0].y);
    for y in (200..320).step_by(3) { let mut a = String::new(); for x in (400..624).step_by(2) { let m = Mat::from_u8(w.g.mat[y*1024+x]);
        a.push(match m { Mat::Water => '~', Mat::Lava => 'L', Mat::Sand | Mat::Gravel => 's', _ => match m.kind() { Kind::Empty | Kind::Gas => '.', _ => '#' } }); } println!("{y:3} {a}"); }
}
