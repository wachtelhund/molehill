use molehill_sim::world::World;
use molehill_sim::mat::Mat;
fn main() {
    let seeds: Vec<u32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let seeds = if seeds.is_empty() { vec![9630, 49934, 1, 777, 31337] } else { seeds };
    for s in seeds {
        let t = std::time::Instant::now();
        let mut w = World::new(1024 * molehill_sim::mat::S, 512 * molehill_sim::mat::S, s);
        let solid = (0..w.g.mat.len()).filter(|&i| w.g.is_static(i)).count();
        let mut hist = std::collections::BTreeMap::new();
        for k in 0..7200 { let before = w.g.mat.clone(); let c0 = w.crumbled; w.step();
            if w.crumbled > c0 { for i in 0..before.len() { if before[i] != w.g.mat[i] && w.g.is_static(i) == false && before[i] != 0 { } } let _=k; }
            if k % 1200 == 0 { for &i in &w.st.cracking { *hist.entry((format!("{:?}", Mat::from_u8(w.g.mat[i as usize])), i as usize / 1024 / 64)).or_insert(0) += 1; } }
        }
        println!("seed {s}: crumbled {} / {solid} = {:.3}%  ({:?}) cracking-by(mat,y/64): {:?}", w.crumbled, 100. * w.crumbled as f64 / solid as f64, t.elapsed(), hist);
    }
}
