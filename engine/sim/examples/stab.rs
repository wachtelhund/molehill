use molehill_sim::world::World;
fn main() {
    let (w, h): (usize, usize) = (std::env::args().nth(1).map(|v| v.parse().unwrap()).unwrap_or(1024), std::env::args().nth(2).map(|v| v.parse().unwrap()).unwrap_or(512));
    let ticks: usize = std::env::args().nth(3).map(|v| v.parse().unwrap()).unwrap_or(18000);
    let seeds: Vec<u32> = std::env::args().skip(4).map(|v| v.parse().unwrap()).collect();
    let hs: Vec<_> = seeds.into_iter().map(|s| std::thread::spawn(move || {
        let t0 = std::time::Instant::now();
        let mut wd = World::new(w, h, s); let tg = t0.elapsed();
        wd.disasters.enabled = false;
        let f0 = wd.st.solve(&wd.g).len();
        let solid = (0..wd.g.mat.len()).filter(|&i| wd.g.is_static(i)).count();
        let mut marks = vec![];
        for k in 0..ticks { wd.step(); if k % 1800 == 1799 { marks.push(wd.crumbled); } }
        format!("seed {s}: gen {:.2?} fail0 {f0} crumbled {} / {} = {:.3}% marks {:?}", tg, wd.crumbled, solid, 100. * wd.crumbled as f64 / solid as f64, marks)
    })).collect();
    for h in hs { println!("{}", h.join().unwrap()); }
}
