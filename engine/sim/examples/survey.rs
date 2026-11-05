//! Survival pacing: what the ground offers around the burrow (counts in items, nearest in metres), per seed.
//! `cargo run --release -p molehill-sim --example survey -- SEEDS`
use molehill_sim::{world::World, mat::{Mat, CPM, S}, craft::{drop_of, Item, CELLS_PER_ITEM}};
fn main() {
    let n: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let want = [Item::Wood, Item::Stone, Item::Clay, Item::Coal, Item::CopperOre, Item::TinOre, Item::IronOre, Item::Quartz, Item::Crystal, Item::Fungus, Item::Sand];
    println!("{:>5} {:>8} {}", "seed", "lava m", want.iter().map(|i| format!("{:>12}", format!("{:?}", i))).collect::<String>());
    for seed in 1..=n {
        let w = World::new(1024 * S, 512 * S, seed * 7919);
        let (mx, my) = (w.moles[0].x as i64, w.moles[0].y as i64);
        let mut cnt = vec![[0u32; 2]; want.len()]; let mut near = vec![f32::MAX; want.len()]; let mut lava = f32::MAX;
        for y in 0..w.g.h { for x in 0..w.g.w {
            let d = (((x as i64 - mx).pow(2) + (y as i64 - my).pow(2)) as f32).sqrt() / CPM as f32;
            let m = w.g.get(x, y); if m == Mat::Lava { lava = lava.min(d); }
            if let Some(it) = drop_of(m) { if let Some(k) = want.iter().position(|&i| i == it) {
                near[k] = near[k].min(d); if d <= 20. { cnt[k][0] += 1; } if d <= 40. { cnt[k][1] += 1; } } }
        } }
        let c = CELLS_PER_ITEM as u32;
        println!("{seed:>5} {lava:>8.0} {}", (0..want.len()).map(|k| format!("{:>12}", format!("{}/{}@{:.0}", cnt[k][0] / c, cnt[k][1] / c, near[k].min(999.)))).collect::<String>());
    }
    println!("(items within 20 m / 40 m @ nearest metres)");
}
