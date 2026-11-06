//! Survival pacing: what the ground offers around the burrow (counts in items, nearest in metres), per seed.
//! `cargo run --release -p molehill-sim --example survey -- SEEDS`
//! `grotto m` is the nearest grotto with root-trees (the near grotto, `gen::NEAR_GROTTO`, when there is one),
//! `mine m` the nearest abandoned mine (its middle), both from the mole.
use molehill_sim::{world::World, mat::{Mat, CPM, S}, craft::{drop_of, Item, CELLS_PER_ITEM}};
fn main() {
    let n: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let want = [Item::Wood, Item::Stone, Item::Clay, Item::Coal, Item::CopperOre, Item::TinOre, Item::IronOre, Item::Quartz, Item::Crystal, Item::Fungus, Item::Sand];
    println!("{:>7} {:>5} {:>8} {:>9} {:>7} {}", "roots m", "seed", "lava m", "grotto m", "mine m", want.iter().map(|i| format!("{:>12}", format!("{:?}", i))).collect::<String>());
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
        let c = CELLS_PER_ITEM as u32; let cm = CPM as f32;
        let dist = |x: f32, wy: f32| ((x - mx as f32).powi(2) + (wy - (w.oy + my) as f32).powi(2)).sqrt() / cm;
        // the first-hour finds that aren't items: root tangles (food), the nearest grotto with root-trees, an abandoned mine
        let mut roots = f32::MAX; for y in 0..w.g.h { for x in 0..w.g.w { if w.g.get(x, y) == Mat::Roots { roots = roots.min(dist(x as f32, (w.oy + y as i64) as f32)); } } }
        let grotto = w.gen.grottos_in_rows(w.oy, w.oy + w.g.h as i64).iter().filter(|g| g.ntrees > 0).map(|g| dist(g.cx as f32, g.cy as f32)).fold(f32::MAX, f32::min);
        let mine = w.gen.sites_in_rows(w.oy, w.oy + w.g.h as i64).iter().filter(|s| s.kind == molehill_sim::sites::Kind::Mine).map(|s| { let (x, y) = s.centre(); dist(x as f32, y as f32) }).fold(f32::MAX, f32::min);
        println!("{:>7.0} {seed:>5} {lava:>8.0} {:>9.0} {:>7.0} {}", roots.min(999.), grotto.min(999.), mine.min(999.), (0..want.len()).map(|k| format!("{:>12}", format!("{}/{}@{:.0}", cnt[k][0] / c, cnt[k][1] / c, near[k].min(999.)))).collect::<String>());
    }
    println!("(items within 20 m / 40 m @ nearest metres)");
}
