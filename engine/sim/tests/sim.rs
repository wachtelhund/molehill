use molehill_sim::{grid::Grid, mat::Mat, stress::Stress, world::*, craft::Item};

fn boxed(w: usize, h: usize) -> Grid { let mut g = Grid::new(w, h); g.fill_rect(0, h - 2, w - 1, h - 1, Mat::Bedrock); g.fill_rect(0, 0, 1, h - 1, Mat::Bedrock); g.fill_rect(w - 2, 0, w - 1, h - 1, Mat::Bedrock); g }
fn run(g: &mut Grid, n: usize) { for _ in 0..n { g.step_ca(); } }

#[test] fn sand_piles_with_slope() {
    let mut g = boxed(128, 128); for y in 2..12 { for x in 60..68 { let i = g.idx(x, y); g.set(i, Mat::Sand); } }
    let n = g.count(Mat::Sand); run(&mut g, 600); assert_eq!(g.count(Mat::Sand), n);
    let col = |x: usize| (0..126).filter(|&y| g.get(x, y) == Mat::Sand).count();
    assert!(col(64) > col(48) && col(64) >= 3, "pile peak {} edge {}", col(64), col(50));
}
#[test] fn water_conserved_and_levels() {
    let mut g = boxed(128, 64); for y in 5..25 { for x in 10..20 { let i = g.idx(x, y); g.set(i, Mat::Water); } }
    let n = g.count(Mat::Water); run(&mut g, 1500); assert_eq!(g.count(Mat::Water), n);
    let top = |x: usize| (0..62).find(|&y| g.get(x, y) == Mat::Water).unwrap_or(62);
    assert!((top(10) as i32 - top(110) as i32).abs() <= 2);
}
#[test] fn lava_meets_water_makes_basalt() {
    let mut g = boxed(64, 64); for y in 30..46 { for x in 2..20 { let i = g.idx(x, y); g.set(i, Mat::Lava); } for x in 30..62 { let i = g.idx(x, y); g.set(i, Mat::Water); } }
    run(&mut g, 800); assert!(g.count(Mat::Basalt) > 0);
}
fn slab(m: Mat, span: usize, thick: usize) -> (Grid, Vec<u32>) {
    // solid block, carve a void under a roof of `thick` cells, span `span` cells
    let mut g = boxed(128, 64); for y in 10..62 { for x in 2..126 { let i = g.idx(x, y); g.set(i, m); } }
    let (x0, y0) = (64 - span / 2, 10 + thick); for y in y0..y0 + 8 { for x in x0..x0 + span { let i = g.idx(x, y); g.set(i, Mat::Empty); } }
    let mut st = Stress::new(128 * 64); let f = st.solve(&g); (g, f)
}
#[test] fn dirt_roof_short_holds_long_fails() {
    let (_, f) = slab(Mat::Dirt, 4, 4); assert!(f.is_empty(), "1 m dirt span should hold");
    let (_, f) = slab(Mat::Dirt, 24, 4); assert!(!f.is_empty(), "6 m dirt span should fail");
}
#[test] fn granite_spans_further_than_dirt() {
    let (_, f) = slab(Mat::Granite, 24, 6); assert!(f.is_empty(), "6 m granite span holds");
}
#[test] fn floating_island_falls() {
    let mut g = boxed(64, 64); for y in 10..14 { for x in 20..30 { let i = g.idx(x, y); g.set(i, Mat::Limestone); } }
    let mut st = Stress::new(64 * 64); let f = st.solve(&g); assert!(f.len() >= 30);
}
#[test] fn wood_pillar_saves_wide_dirt_room() {
    let (mut g, f) = slab(Mat::Dirt, 24, 4); assert!(!f.is_empty());
    for k in [56usize, 60, 64, 68, 72] { for y in 14..22 { let i = g.idx(k, y); g.set(i, Mat::WoodBeam); } }
    let mut st = Stress::new(128 * 64); let f = st.solve(&g); assert!(f.is_empty(), "pillars at 1 m spacing should hold, {} failing", f.len());
}
#[test] fn deterministic_worlds() {
    let a = World::new(256, 256, 7); let b = World::new(256, 256, 7); let c = World::new(256, 256, 8);
    assert_eq!(a.g.mat, b.g.mat); assert_ne!(a.g.mat, c.g.mat);
}
#[test] fn generated_world_is_mostly_stable() {
    let mut w = World::new(512, 256, 3);
    for _ in 0..200 { w.step(); }
    assert!(w.crumbled < 400, "crumbled {}", w.crumbled);
    assert!(w.g.count(Mat::Water) > 0 && w.g.count(Mat::Coal) > 0);
}
#[test] fn moles_dig_and_haul() {
    let mut w = World::new(512, 256, 11);
    let m = w.moles[0].clone(); w.designate(m.x, m.y + 4, 4, D_DIG);
    let before = w.g.count(Mat::Empty);
    for _ in 0..3000 { w.step(); }
    assert!(w.g.count(Mat::Empty) > before + 10, "moles should dig");
    let stocked: u32 = [Item::Dirt, Item::Clay, Item::Stone, Item::Sand, Item::Gravel].iter().map(|&i| w.inv[i as usize]).sum();
    assert!(stocked > 0 || !w.drops.is_empty(), "digging yields items");
}

#[test]
fn fresh_world_does_not_collapse() {
    // 2 minutes at 60 tps with no player action: < 1% of static cells may crumble.
    let mut w = molehill_sim::world::World::new(1024, 512, 777);
    let solid = (0..w.g.mat.len()).filter(|&i| w.g.is_static(i)).count();
    for _ in 0..7200 { w.step(); }
    assert!((w.crumbled as f64) < 0.01 * solid as f64, "crumbled {} of {}", w.crumbled, solid);
}

mod v3b {
    use molehill_sim::world::World;
    use molehill_sim::logic::{Dev, Logic};
    use molehill_sim::grid::Grid;
    use molehill_sim::mat::Mat;
    fn open_grid() -> Grid { let mut g = Grid::new(128, 64); for x in 0..128 { for y in 60..64 { let i = g.idx(x, y); g.mat[i] = Mat::Bedrock as u8; } } g }
    #[test]
    fn lever_wire_lamp_and_not_gate() {
        let mut g = open_grid(); let mut l = Logic::new(128, 64);
        let b = |x: usize, y: usize| y * 32 + x;
        assert!(l.place(&mut g, Dev::Lever, b(2, 5)));
        for x in 3..8 { l.place_wire(b(x, 5)); }
        assert!(l.place(&mut g, Dev::Lamp, b(8, 5)));
        l.step(&mut g, &[], 1); l.step(&mut g, &[], 2); assert_eq!(l.dev_s[b(8, 5)], 0);
        l.interact(b(2, 5)); l.step(&mut g, &[], 3); assert_eq!(l.dev_s[b(8, 5)], 1, "lamp lit by lever");
        // NOT gate: input west wire (net of lever) -> output east wire -> door
        for y in 6..9 { l.place_wire(b(3, y)); } l.place(&mut g, Dev::Not, b(4, 8)); l.place_wire(b(5, 8)); l.place(&mut g, Dev::Door, b(6, 8));
        for t in 4..8 { l.step(&mut g, &[], t); }
        let di = g.idx(6 * 4 + 1, 8 * 4 + 1); assert_eq!(g.m(di), Mat::Door, "NOT(on)=off -> door closed");
        l.interact(b(2, 5)); for t in 8..12 { l.step(&mut g, &[], t); } assert_eq!(g.m(di), Mat::Empty, "NOT(off)=on -> door open");
    }
    #[test]
    fn thrumming_carries_through_stone_not_air() {
        // solid granite, two pockets 30 m apart; the knock travels through the rock
        let mut g = open_grid(); let mut l = Logic::new(128, 64);
        let b = |x: usize, y: usize| y * 32 + x;
        let set = |g: &mut molehill_sim::grid::Grid, m: Mat, x0: usize, x1: usize| for y in 0..64 { for x in x0..x1 { let i = g.idx(x, y); g.mat[i] = m as u8; g.flags[i] = 0; } };
        set(&mut g, Mat::Granite, 0, 128);
        for &(bx, by) in &[(1usize, 3usize), (2, 3), (3, 3), (28, 3), (29, 3), (30, 3)] { for y in by * 4..by * 4 + 4 { for x in bx * 4..bx * 4 + 4 { let i = g.idx(x, y); g.mat[i] = 0; } } }
        l.place(&mut g, Dev::Lever, b(1, 3)); l.place_wire(b(2, 3)); l.place(&mut g, Dev::Tx, b(3, 3));
        l.place(&mut g, Dev::Rx, b(28, 3)); l.place_wire(b(29, 3)); l.place(&mut g, Dev::Lamp, b(30, 3));
        l.interact(b(1, 3)); for t in 0..20 { l.step(&mut g, &[], t); }
        assert_eq!(l.dev_s[b(30, 3)], 1, "granite carries the thrum");
        let stone = l.link_budget(&g, b(3, 3), b(28, 3));
        // an open tunnel across the path reflects the wave (two rock/air boundaries)
        set(&mut g, Mat::Empty, 60, 64);
        l.invalidate(); for t in 20..40 { l.step(&mut g, &[], t); }
        assert_eq!(l.dev_s[b(30, 3)], 0, "air gap blocks");
        // the same gap packed with loose dirt passes, but weaker than solid stone
        set(&mut g, Mat::Dirt, 60, 64); for y in 0..64 { for x in 60..64 { let i = g.idx(x, y); g.flags[i] = molehill_sim::grid::F_LOOSE; } }
        let dirt = l.link_budget(&g, b(3, 3), b(28, 3));
        assert!(dirt < stone, "dirt muffles: {dirt} vs {stone}");
        // a 30 m run of loose sand-like dirt muffles it entirely
        set(&mut g, Mat::Dirt, 16, 112); for y in 0..64 { for x in 16..112 { let i = g.idx(x, y); g.flags[i] = molehill_sim::grid::F_LOOSE; } }
        for &(bx, by) in &[(28usize, 3usize)] { for y in by * 4..by * 4 + 4 { for x in bx * 4..bx * 4 + 4 { let i = g.idx(x, y); g.mat[i] = 0; } } }
        assert!(l.link_budget(&g, b(3, 3), b(28, 3)) < 0., "loose dirt damps out");
    }
    #[test]
    fn save_load_roundtrip() {
        let mut w = World::new(512, 256, 4242);
        for _ in 0..120 { w.step(); }
        let bytes = molehill_sim::save::save(&w);
        let s = molehill_sim::save::to_b64(&bytes); let back = molehill_sim::save::from_b64(&s).unwrap(); assert_eq!(bytes, back);
        let w2 = molehill_sim::save::load(&back).expect("load");
        assert_eq!(w.g.mat, w2.g.mat); assert_eq!(w.inv, w2.inv); assert_eq!(w.moles.len(), w2.moles.len()); assert_eq!(w.tick, w2.tick);
        assert!(molehill_sim::save::load(&bytes[..bytes.len() / 2]).is_err());
        assert!(molehill_sim::save::load(b"garbage").is_err());
    }
    #[test]
    fn moles_get_hungry_and_eat() {
        let mut w = World::new(512, 256, 99);
        for m in w.moles.iter_mut() { m.hunger = 30.; }
        let food0 = w.inv[molehill_sim::craft::Item::Grub as usize];
        for _ in 0..1200 { w.step(); }
        assert!(w.inv[molehill_sim::craft::Item::Grub as usize] < food0, "ate grubs");
        for m in &w.moles { eprintln!("{} h{} s{} {:?} hp{}", m.name, m.hunger, m.sleep, m.job, m.hp); }
        assert!(w.moles.iter().all(|m| m.hunger > 30.));
    }
    #[test]
    fn quake_loosens_overhangs() {
        let mut w = World::new(512, 256, 5);
        use molehill_sim::disasters::Kind2;
        let mut d = std::mem::replace(&mut w.disasters, molehill_sim::disasters::Disasters::new(1));
        assert!(d.schedule(&mut w, Kind2::Quake)); w.disasters = d;
        let c0 = (0..w.g.mat.len()).filter(|&i| w.g.loose(i)).count();
        for _ in 0..(16 * 60) { w.step(); }
        assert!(w.crumbled > 0 || (0..w.g.mat.len()).filter(|&i| w.g.loose(i)).count() > c0);
    }
}
#[test] fn endless_world_shifts_deterministically() {
    let mut w = World::new(256, 256, 5); let oy0 = w.oy;
    assert!(w.depth_m(0, (-oy0) as usize).abs() < 0.01, "start layer is depth 0");
    let top: Vec<u8> = w.g.mat[..256 * 8].to_vec();
    w.shift(128); assert_eq!(w.oy, oy0 + 128);
    w.shift(256 - 128 - 128 + 128); // deeper still
    w.shift(-128); w.shift(-128); assert_eq!(w.oy, oy0);
    assert_eq!(&w.g.mat[..256 * 8], &top[..], "rows restored from cache");
    // fresh deep rows are generated identically per seed
    let mut a = World::new(256, 256, 5); let mut b = World::new(256, 256, 5);
    for _ in 0..4 { a.shift(128); b.shift(128); } assert_eq!(a.g.mat, b.g.mat);
    assert_eq!(a.g.count(Mat::Bedrock) , b.g.count(Mat::Bedrock));
    // no surface: no grass or sky anywhere, more lava than water deep down
    assert_eq!(a.g.count(Mat::Grass), 0);
}
#[test] fn mushrooms_grow_in_dark_damp_and_air_goes_stale() {
    let mut w = World::new(256, 256, 21);
    w.torches.clear();
    let (x, y) = (w.moles[0].x + 3, w.moles[0].y);
    w.plant(x, y).expect("plant");
    let (bright, _moist) = w.farm_conditions(x, y); assert!(!bright, "no torch nearby");
    for _ in 0..60 * 60 * 3 { w.step(); }
    assert!(w.harvested > 0 || w.inv[Item::Mushroom as usize] > 0 || w.farms.iter().any(|f| f.growth > 0.3), "mushrooms grow");
    // a sealed pocket fills with CO2 from breathing; a vent clears it
    let mut a = molehill_sim::air::Air::new(64, 64);
    let mut g = Grid::new(64, 64); for i in 0..64 * 64 { g.mat[i] = Mat::Granite as u8; } for y in 20..40 { for x in 20..40 { let i = g.idx(x, y); g.mat[i] = 0; } }
    for _ in 0..1000 { a.add(30, 30, 4. * 0.08); a.update(&g, &[]); } // four moles, ~4 min
    let stale = a.at(30, 30); assert!(stale > 0.45, "sealed tunnel goes stale: {stale}");
    for _ in 0..100 { a.update(&g, &[(30, 30)]); } assert!(a.at(30, 30) < stale * 0.5, "vent clears");
}

/// Fresh worlds must not shed their caves: < 0.1% of static cells lost in 5 minutes (18000 ticks), many seeds.
#[test]
fn fresh_worlds_are_stable_multi_seed() {
    let seeds = [1u32, 4, 777, 9630, 1295, 12822, 11, 14];
    let hs: Vec<_> = seeds.iter().map(|&s| std::thread::spawn(move || {
        let mut w = molehill_sim::world::World::new(1024, 512, s); w.disasters.enabled = false;
        let solid = (0..w.g.mat.len()).filter(|&i| w.g.is_static(i)).count();
        for _ in 0..18000 { w.step(); }
        (s, w.crumbled as f64 / solid as f64)
    })).collect();
    for h in hs { let (s, f) = h.join().unwrap(); assert!(f < 0.001, "seed {s} lost {:.3}%", f * 100.); }
}

/// Old (surface-era, version < 3), truncated or garbage saves are refused with a message, never a panic.
#[test]
fn old_and_broken_saves_fail_cleanly() {
    let w = molehill_sim::world::World::new(256, 256, 3);
    let good = molehill_sim::save::save(&w);
    let mut old = good.clone(); old[8..12].copy_from_slice(&2u32.to_le_bytes());
    let e = molehill_sim::save::load(&old).err().expect("old save refused"); assert!(e.contains("new holt"), "{e}");
    for cut in [0, 5, 12, 40, good.len() / 3, good.len() - 1] { assert!(molehill_sim::save::load(&good[..cut]).is_err()); }
    assert!(molehill_sim::save::load(b"definitely not a save file at all").is_err());
    assert!(molehill_sim::save::load(&good).is_ok());
}

/// Nobody drowns or burns in the starting burrow: all moles alive after 30 s for many seeds.
#[test]
fn start_burrow_is_safe() {
    let hs: Vec<_> = [4242u32, 1, 2, 3, 11, 777, 9630, 31337].into_iter().map(|s| std::thread::spawn(move || {
        let mut w = molehill_sim::world::World::new(1024, 512, s); w.disasters.enabled = false;
        for _ in 0..1800 { w.step(); }
        (s, w.moles.iter().filter(|m| m.alive).count())
    })).collect();
    for h in hs { let (s, n) = h.join().unwrap(); assert_eq!(n, 4, "seed {s}: a mole died in the start burrow"); }
}

/// The start burrow is not a cage: claws can dig out through its walls.
#[test]
fn colony_can_dig_out_of_the_burrow() {
    for s in [3636u32, 777] {
        let mut w = molehill_sim::world::World::new(1024, 512, s); w.disasters.enabled = false;
        let (x0, y) = (w.moles[3].x + 6, w.moles[3].y - 1);
        let before: Vec<u8> = (x0..x0 + 12).map(|x| w.g.mat[y * 1024 + x]).collect();
        for x in x0..x0 + 12 { w.designate(x, y - 1, 1, molehill_sim::world::D_DIG); }
        for _ in 0..60 * 90 { w.step(); }
        // open, or dug and then caved in to rubble: either way the wall rock is breached
        let dug = (x0..x0 + 12).filter(|&x| { let m = w.g.mat[y * 1024 + x]; m == 0 || m != before[x - x0] }).count();
        assert!(dug >= 9, "seed {s}: only {dug}/12 tunnel cells dug");
    }
}
