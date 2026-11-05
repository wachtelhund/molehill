use molehill_sim::{world::World, mat::Mat};
fn main() {
    let mut w = World::new(1024 * molehill_sim::mat::S, 512 * molehill_sim::mat::S, 9630);
    println!("initial cracking {}", w.st.cracking.len());
    let mut first = None;
    for k in 0..300 { let b = w.crumbled; let snap = w.g.mat.clone(); w.step(); if w.crumbled > b && first.is_none() { first = Some(k);
        for i in 0..snap.len() { if snap[i] != w.g.mat[i] && Mat::from_u8(snap[i]).kind() == molehill_sim::mat::Kind::Solid && w.g.loose(i) { let (x, y) = (i % 1024, i / 1024);
            println!("tick {} crumble at {},{} {:?} oy {}", k, x, y, Mat::from_u8(snap[i]), w.oy);
            for yy in y.saturating_sub(6)..y + 6 { let mut s = String::new(); for xx in x.saturating_sub(20)..x + 20 { let j = yy * 1024 + xx; s.push(if j == i { '@' } else { match w.g.m(j) { Mat::Empty => ' ', Mat::Water => '~', Mat::Dirt => '.', Mat::Clay => ',', Mat::Sand => ':', Mat::Gravel => ';', Mat::Limestone => '=', Mat::Granite => '#', Mat::Roots => 'r', Mat::Lava => '^', Mat::Basalt => 'B', Mat::Steam => 's', Mat::Fire => 'f', _ => '*' } }); } println!("{}", s); }
            break; } } }
        if k % 50 == 0 { println!("k {} crumbled {} cracking {}", k, w.crumbled, w.st.cracking.len()); }
    }
}
