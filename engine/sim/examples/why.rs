use molehill_sim::{world::World, mat::Mat};
fn main() {
    let s: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut w = World::new(1024 * molehill_sim::mat::S, 512 * molehill_sim::mat::S, s); w.disasters.enabled = false;
    let (x0, y) = (w.moles[3].x + 6, w.moles[3].y - 1);
    let row = |w: &World| (x0..x0 + 20).flat_map(|x| [x]).collect::<Vec<_>>().into_iter().map(|x| { let m = Mat::from_u8(w.g.mat[y * 1024 + x]); if m == Mat::Empty { '.' } else { m.p().name.chars().next().unwrap() } }).collect::<String>();
    for yy in y-4..y+1 { println!("{yy} {}", (x0-4..x0 + 20).map(|x| { let m = Mat::from_u8(w.g.mat[yy * 1024 + x]); if m == Mat::Empty { '.' } else { m.p().name.chars().next().unwrap() } }).collect::<String>()); }
    for x in x0..x0 + 20 { w.designate(x, y - 2, 2, molehill_sim::world::D_DIG); }
    for k in 0..5400 { w.step(); if k == 5399 { for yy in y-4..y+1 { println!("{yy} {}", (x0-4..x0 + 20).map(|x| { let m = Mat::from_u8(w.g.mat[yy * 1024 + x]); let d = w.desig[yy*1024+x]; if m == Mat::Empty { '.' } else if d != 0 { '*' } else { m.p().name.chars().next().unwrap() } }).collect::<String>()); } } if k % 1800 == 0 { println!("{k} {} {:?}", row(&w), w.moles.iter().map(|m| (m.x, m.y, format!("{:?}", m.job))).collect::<Vec<_>>()); } }
}
