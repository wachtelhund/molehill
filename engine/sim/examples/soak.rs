//! Crash hunt: random play (move, dig, place, devices, wires, TNT, save/load) on a game-sized world.
//! Run in debug for overflow checks: `cargo run -p molehill-sim --no-default-features --example soak -- SEEDS TICKS`.
use molehill_sim::{world::{World, Brush}, craft::Item, save};
struct Rng(u64);
impl Rng { fn n(&mut self, k: u64) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 % k.max(1) } }
fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let (seeds, ticks) = (a.first().copied().unwrap_or(4), a.get(1).copied().unwrap_or(3000));
    for seed in 1..=seeds {
        let mut w = World::new(1024, 512, seed as u32 * 7919); w.disasters.enabled = seed % 2 == 0;
        for v in w.inv.iter_mut() { *v = (*v).max(999); }
        let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let devs = molehill_sim::logic::DEVS;
        for t in 0..ticks {
            if t % 20 == 0 {
                w.input.left = r.n(3) == 0; w.input.right = !w.input.left && r.n(2) == 0;
                w.input.jump = r.n(4) == 0; w.input.down = r.n(3) == 0; w.input.up = r.n(5) == 0;
                w.input.tool = r.n(4) as u8; w.input.brush = r.n(12) as f32 * 0.5;
            }
            let (mx, my) = (w.moles[0].x as i64, w.moles[0].y as i64);
            let tx = (mx + r.n(41) as i64 - 20).clamp(0, 1023) as usize; let ty = (my + r.n(41) as i64 - 20).clamp(0, 511) as usize;
            w.input.mine = if r.n(3) == 0 { Some((tx, ty)) } else { None };
            match r.n(40) {
                0 => { let it = [Item::Dirt, Item::Stone, Item::WoodPillar, Item::IronBeam, Item::Brick, Item::Sand, Item::Glass][r.n(7) as usize]; let _ = w.place_brush(tx, ty, it, r.n(8) as f32 * 0.5); }
                1 => { let _ = w.place_device(devs[r.n(devs.len() as u64) as usize], tx, ty); }
                2 => { w.use_device(tx, ty); }
                3 => { w.lay_wire(tx, ty); }
                4 => { w.pick_up(tx, ty); }
                5 if r.n(10) == 0 => { w.blast(tx, ty, 0.05 + r.n(10) as f32 * 0.05); }
                6 => { w.place_torch(tx, ty); }
                7 if r.n(20) == 0 => { let b = save::save(&w); w = save::load(&b).expect("reload"); }
                8 => { let _ = w.plant(tx, ty); }
                _ => {}
            }
            let _ = Brush::Dig(0);
            w.step();
            if t % 2 == 0 { let (x0, y0) = (mx.saturating_sub(60).max(0) as usize, (my - 40).max(0) as usize); w.compute_light((x0, y0, (mx + 60).min(1023) as usize, (my + 40).min(511) as usize)); }
        }
        println!("seed {seed}: ok, mole at {:?} oy {} hp {:.0}", (w.moles[0].x, w.moles[0].y), w.oy, w.moles[0].hp);
    }
}
