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
#[test] fn player_walks_jumps_and_mines() {
    let mut w = World::new(512, 256, 11); w.disasters.enabled = false;
    assert_eq!(w.moles.len(), 1, "exactly one mole: the player");
    for _ in 0..30 { w.step(); }
    let x0 = w.moles[0].x;
    assert!(w.pl.grounded, "starts on the burrow floor");
    // walk right for half a second
    w.input.right = true; for _ in 0..30 { w.step(); } w.input.right = false;
    assert!(w.moles[0].x > x0 + 2, "walked right: {} -> {}", x0, w.moles[0].x);
    // jump: rises, then lands back on the floor
    for _ in 0..20 { w.step(); } let yb = w.moles[0].y;
    w.input.jump = true; let mut top = yb; for _ in 0..40 { w.step(); top = top.min(w.moles[0].y); } w.input.jump = false;
    assert!(yb - top >= 3, "jumped at least 3 cells (from {yb} up to {top})");
    for _ in 0..90 { w.step(); } assert!(w.pl.grounded && w.moles[0].y == yb, "landed");
    // mine the floor under its feet: hold the mouse there until it opens, then it falls in
    let (x, y) = (w.moles[0].x, w.moles[0].y);
    let i = w.g.idx(x, y + 1); assert!(w.mine_time(i).is_some(), "floor is mineable");
    w.input.mine = Some((x, y + 1)); let mut t = 0; while w.g.mat[i] != 0 && t < 2000 { w.step(); t += 1; }
    assert_eq!(w.g.mat[i], 0, "mined the floor cell"); assert!(t > 1, "mining takes time");
    w.input.mine = None; for _ in 0..30 { w.step(); }
    assert!(w.moles[0].y > y, "dropped into the hole");
    // out of reach does nothing
    let far = w.g.idx(x, (y + 40).min(250)); w.input.mine = Some((x, (y + 40).min(250))); let m0 = w.g.mat[far]; for _ in 0..600 { w.step(); }
    assert_eq!(w.g.mat[far], m0, "can't mine beyond reach");
}
#[test] fn player_places_pillar_and_respawns() {
    let mut w = World::new(512, 256, 11); w.disasters.enabled = false;
    for _ in 0..30 { w.step(); }
    let (x, y) = (w.moles[0].x, w.moles[0].y); let n0 = w.inv[Item::WoodPillar as usize];
    assert!(w.place_block(x + 3, y, Item::WoodPillar).is_ok());
    assert_eq!(w.g.get(x + 3, y), Mat::WoodBeam);
    assert!(w.place_block(x, y, Item::WoodPillar).is_err(), "not inside yourself");
    assert!(w.place_block(x + 60, y, Item::WoodPillar).is_err(), "not out of reach");
    assert!(w.inv[Item::WoodPillar as usize] <= n0);
    w.moles[0].hp = -1.; for _ in 0..400 { w.step(); }
    assert!(w.moles[0].alive && w.pl.deaths == 1, "respawned at the burrow");
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
    for h in hs { let (s, n) = h.join().unwrap(); assert_eq!(n, 1, "seed {s}: the mole died in the start burrow"); }
}

/// The start burrow is not a cage: the player can mine out through its walls.
#[test]
fn player_can_dig_out_of_the_burrow() {
    for s in [3636u32, 777] {
        let mut w = molehill_sim::world::World::new(1024, 512, s); w.disasters.enabled = false;
        let y = w.moles[0].y; let x0 = w.moles[0].x;
        // walk right, mining whatever blocks the way at head and chest height
        w.input.right = true; let mut best = x0;
        for _ in 0..60 * 40 { let (x, yy) = ((w.pl.px + 1.2) as usize, w.moles[0].y); // the column the body's leading edge pushes into
            let t = (0..3).map(|j| (x, yy - j)).find(|&(a, b)| w.g.is_solidish(w.g.idx(a, b))); w.input.mine = t; w.step(); best = best.max(w.moles[0].x); }
        assert!(best > x0 + 30, "seed {s}: tunnelled out ({x0} -> {best}, y {y})");
    }
}
#[test] fn dig_tools_bite_fast_and_scale() {
    let mut w = World::new(512, 256, 11); w.disasters.enabled = false;
    for _ in 0..30 { w.step(); }
    let (x, y) = (w.moles[0].x, w.moles[0].y);
    // a pickaxe bite into dirt is near-instant and clears a mole-sized disc
    let (tx, ty) = (x + 8, y - 1);
    for c in w.bite_cells(tx, ty) { w.g.set(c, Mat::Dirt); }
    w.input.tool = 1; let n_bite = w.bite_cells(tx, ty).len(); assert!(n_bite >= 15, "pickaxe bite is mole-sized ({n_bite} cells)");
    w.input.mine = Some((tx, ty)); let mut t = 0; while w.g.get(tx, ty) != Mat::Empty && t < 200 { w.step(); t += 1; }
    assert!(t <= 9, "dirt bite took {t} ticks (want ~0.1 s)");
    let cleared = w.bite_cells(tx, ty).iter().filter(|&&c| w.g.m(c) == Mat::Empty || w.g.m(c) == Mat::Water).count(); assert!(cleared * 10 >= n_bite * 8, "bite cleared {cleared}/{n_bite}");
    w.input.mine = None;
    // stone ~0.3-0.5 s with the pickaxe; granite needs the drill; the bore is bigger and faster
    let lime = w.g.idx(tx, ty); w.g.set(lime, Mat::Limestone);
    let tp = w.mine_time(lime).unwrap(); assert!((15..=32).contains(&tp), "limestone with pickaxe: {tp} ticks");
    w.g.set(lime, Mat::Granite); assert!(w.mine_time(lime).is_none(), "pickaxe can't cut granite");
    w.input.tool = 2; assert!(w.mine_time(lime).is_some(), "drill cuts granite");
    w.input.tool = 3; let tb = w.mine_time(lime).unwrap(); w.input.tool = 2; let td = w.mine_time(lime).unwrap(); assert!(tb < td, "bore faster than drill");
    w.input.tool = 3; assert!(w.bite_cells(tx, ty).len() > n_bite, "bore bite is bigger");
}
/// A clean arena around the spawn: open box, granite floor and walls.
fn arena() -> (World, usize, usize) {
    let mut w = World::new(512, 256, 11); w.disasters.enabled = false;
    let (x0, y0) = (w.moles[0].x, w.moles[0].y);
    for y in y0 - 30..=y0 + 3 { for x in x0 - 40..=x0 + 40 { let i = w.g.idx(x, y); w.g.set(i, if y > y0 || x < x0 - 30 || x > x0 + 30 { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = 0; } }
    w.drops.clear(); w.torches.clear(); w.farms.clear();
    for _ in 0..10 { w.step(); }
    (w, x0, y0)
}
fn press(w: &mut World, n: usize, f: impl Fn(&mut Input)) -> (f32, f32) { let (mut top, mut _b) = (f32::MAX, 0f32); for _ in 0..n { f(&mut w.input); w.step(); top = top.min(w.pl.py); _b = _b.max(w.pl.py); } (top, _b) }
#[test] fn jump_height_is_snappy_and_variable() {
    let (mut w, _, _) = arena(); assert!(w.pl.grounded);
    let y0 = w.pl.py;
    let (top, _) = press(&mut w, 40, |i| i.jump = true); let full = y0 - top;
    press(&mut w, 60, |i| i.jump = false); assert!(w.pl.grounded, "lands again");
    assert!((4.5..9.).contains(&full), "full jump {full} cells");
    let (top2, _) = press(&mut w, 3, |i| i.jump = true); let (top3, _) = press(&mut w, 40, |i| i.jump = false); let hop = y0 - top2.min(top3);
    assert!(hop < full * 0.6, "short hop {hop} vs full {full}");
    // ground speed: max in ~3 frames, stops almost instantly
    press(&mut w, 60, |_| {}); press(&mut w, 3, |i| { i.jump = false; i.right = true; }); assert!(w.pl.vx > 0.8 * TUNE.run, "accel: vx {}", w.pl.vx);
    press(&mut w, 3, |i| i.right = false); assert!(w.pl.vx.abs() < 0.5, "decel: vx {}", w.pl.vx);
}
#[test] fn coyote_time_and_wall_climb() {
    let (mut w, x0, y0) = arena();
    // a ledge: raise a platform under the mole and run off its right edge
    for x in x0 - 30..=x0 + 2 { for y in y0 - 14..=y0 { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); } }
    w.moles[0].y = y0 - 15; w.pl.sync = None; press(&mut w, 30, |_| {}); assert!(w.pl.grounded, "on the ledge");
    let ylev = w.pl.py;
    let mut t = 0; while w.pl.grounded && t < 120 { w.input.right = true; w.step(); t += 1; }
    press(&mut w, 3, |i| i.right = true); // 3 frames after leaving the edge
    let (top, _) = press(&mut w, 20, |i| { i.right = false; i.jump = true; }); assert!(top < ylev - 2., "coyote jump worked: top {top} ledge {ylev}");
    press(&mut w, 120, |i| i.jump = false); assert!(w.pl.grounded);
    // too late: 15 frames after the edge, no jump
    let (mut w, x0, y0) = arena();
    for x in x0 - 30..=x0 + 2 { for y in y0 - 14..=y0 { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); } }
    w.moles[0].y = y0 - 15; w.pl.sync = None; press(&mut w, 30, |_| {});
    let mut t = 0; while w.pl.grounded && t < 120 { w.input.right = true; w.step(); t += 1; }
    let ya = w.pl.py; press(&mut w, 15, |i| i.right = false); let (top, _) = press(&mut w, 20, |i| i.jump = true); assert!(top >= ya - 0.1, "no late jump: {top} vs {ya}");
    // wall climb: run right into the arena wall, jump, cling, climb up, then wall-jump away
    let (mut w, _, _) = arena(); press(&mut w, 1, |i| i.jump = false);
    press(&mut w, 120, |i| i.right = true);
    press(&mut w, 10, |i| { i.right = true; i.jump = true; }); press(&mut w, 4, |i| { i.right = true; i.jump = false; });
    assert_eq!(w.pl.cling, 1, "clinging to the right wall");
    let yc = w.pl.py; press(&mut w, 40, |i| { i.right = true; i.up = true; }); assert!(w.pl.py < yc - 3., "climbed: {} -> {}", yc, w.pl.py);
    let xw = w.pl.px; press(&mut w, 2, |i| { i.up = false; i.jump = true; }); press(&mut w, 10, |i| i.jump = true);
    assert!(w.pl.px < xw - 1.5 && w.pl.cling == 0, "wall-jumped away: {} -> {}", xw, w.pl.px);
}

/// Loading a save where the mole stands far below the burrow must not count as a fall (it used to kill).
#[test] fn load_deep_does_not_fall_damage() {
    let (mut w, x0, y0) = arena();
    // a pocket 60 cells below the arena floor, and move the mole there as if it had dug down
    let y1 = y0 + 60; for y in y1 - 4..=y1 { for x in x0 - 3..=x0 + 3 { let i = w.g.idx(x, y); w.g.set(i, Mat::Empty); } }
    for x in x0 - 4..=x0 + 4 { let i = w.g.idx(x, y1 + 1); w.g.set(i, Mat::Granite); }
    w.moles[0].y = y1; w.pl.sync = None; for _ in 0..5 { w.step(); }
    assert_eq!(w.moles[0].hp, 100.);
    let b = molehill_sim::save::save(&w); let mut w2 = molehill_sim::save::load(&b).expect("load");
    for _ in 0..30 { w2.step(); }
    assert!(w2.moles[0].hp > 99., "hp after load {}", w2.moles[0].hp);
}
/// Copperline and knots: dragging wire over a knot costs nothing, picking up refunds the wire and the device,
/// and a knot placed on copperline gives that copperline back.
#[test] fn wire_and_device_pack_accounting() {
    use molehill_sim::logic::Dev;
    let (mut w, x0, y0) = arena(); let (x, y) = (x0 + 8, y0 - 2);
    let cu = |w: &World| w.inv[Item::CopperWire as usize]; let kn = |w: &World| w.inv[Item::Gate as usize];
    let (c0, k0) = (cu(&w), kn(&w));
    assert!(w.place_device(Dev::And, x, y).is_ok()); assert_eq!(kn(&w), k0 - 1);
    for _ in 0..10 { assert!(!w.lay_wire(x, y)); } assert_eq!(cu(&w), c0, "no copperline lost on a knot");
    assert!(w.lay_wire(x + 4, y)); assert_eq!(cu(&w), c0 - 1); assert!(!w.lay_wire(x + 4, y), "already wired");
    assert!(w.place_device(Dev::Or, x + 4, y).is_ok()); assert_eq!(cu(&w), c0, "knot on copperline refunds it");
    w.inv[Item::Lamp as usize] = 1; assert!(w.lay_wire(x + 8, y)); assert!(w.place_device(Dev::Lamp, x + 8, y).is_ok());
    let c1 = cu(&w); assert_eq!(w.pick_up(x + 8, y), Some(Dev::Lamp)); assert_eq!(cu(&w), c1 + 1, "wire under a device refunded");
    assert_eq!(w.pick_up(x, y), Some(Dev::And)); assert_eq!(w.pick_up(x + 4, y), Some(Dev::Or)); assert_eq!(kn(&w), k0);
    assert!(w.place_device(Dev::Lever, x + 60, y).is_err(), "out of reach");
}
/// A fuse cord picked up while burning, then put down again, is a fresh unlit cord.
#[test] fn replaced_fuse_is_not_burning() {
    use molehill_sim::logic::Dev;
    let (mut w, x0, y0) = arena(); let (x, y) = (x0 + 8, y0 - 2); let b = w.logic.block_of(x, y);
    assert!(w.place_device(Dev::Fuse, x, y).is_ok()); assert!(w.use_device(x, y)); w.step(); assert!(w.logic.lit(b));
    w.pick_up(x, y); assert!(w.place_device(Dev::Fuse, x, y).is_ok()); w.step(); assert!(!w.logic.lit(b));
}

#[test] fn zoom_is_clamped_both_ways() {
    use molehill_sim::view::*;
    let mut z = ZOOM_DEFAULT;
    for _ in 0..3 { z = zoom_step(z, 1.); } assert!(z < ZOOM_DEFAULT, "zooming in makes the scale smaller");
    for _ in 0..100 { z = zoom_step(z, 1.); } assert_eq!(z, ZOOM_MIN, "zoom in stops at the minimum scale");
    for _ in 0..3 { z = zoom_step(z, -1.); } assert!(z > ZOOM_MIN, "zoom out works again from the closest zoom");
    for _ in 0..100 { z = zoom_step(z, -1.); } assert_eq!(z, ZOOM_MAX, "zoom out stops at the maximum scale");
    assert!(zoom_step(z, 1.) < ZOOM_MAX, "and zooms back in");
    assert_eq!(clamp_zoom(f32::NAN), ZOOM_DEFAULT); assert_eq!(clamp_zoom(-1.), ZOOM_DEFAULT); assert_eq!(clamp_zoom(1.5), ZOOM_MAX);
    assert!((zoom_level(ZOOM_DEFAULT) - 1.).abs() < 1e-6 && zoom_level(ZOOM_MIN) > 1. && zoom_level(ZOOM_MAX) < 1.);
}

#[test] fn brush_sizes_clamp_per_item() {
    use molehill_sim::world::{brush_radius, brush_range, disc, Brush, BITES};
    use molehill_sim::craft::Item;
    for t in 0..4u8 { assert_eq!(brush_radius(Brush::Dig(t), 0.), BITES[t as usize], "default bite unchanged"); }
    let max = |b| brush_range(b).unwrap().1;
    assert!(max(Brush::Dig(1)) < max(Brush::Dig(2)) && max(Brush::Dig(2)) < max(Brush::Dig(3)), "bigger tools allow bigger bites");
    assert_eq!(brush_radius(Brush::Dig(1), 99.), max(Brush::Dig(1)));
    assert_eq!(brush_radius(Brush::Place(Item::IronBeam), 99.), 2.0);
    assert_eq!(brush_radius(Brush::Place(Item::WoodPillar), 0.01), 0.5);
    assert!(brush_range(Brush::Place(Item::Torch)).is_none(), "torches are not a brush");
    assert_eq!(disc(0.5).len(), 1); assert_eq!(disc(1.0).len(), 5); assert_eq!(disc(2.0).len(), 13); assert_eq!(disc(3.0).len(), 29);
    assert_eq!(disc(2.0)[0], (0, 0), "centre first");
}

#[test] fn every_thing_has_a_lore_safe_tooltip() {
    use molehill_sim::{craft::{ITEMS, STATIONS}, logic::DEVS, mat::{Mat, NMAT}};
    let mut all: Vec<(String, &str)> = ITEMS.iter().map(|i| (i.name().to_string(), i.desc())).collect();
    all.extend(DEVS.iter().map(|d| (d.name().to_string(), d.desc())));
    all.extend(STATIONS.iter().map(|s| (s.name().to_string(), s.desc())));
    all.extend((0..NMAT as u8).map(|m| (format!("{:?}", Mat::from_u8(m)), Mat::from_u8(m).desc())));
    // LORE.md: handmade tech, no modern or magic words
    let banned = ["radio", "wifi", "wi-fi", "electric", "battery", "signal", "wireless", "antenna", "magic", "mana", "spell", "sky", "sun", "surface", "logic gate", "sensor", "laser"];
    for (n, d) in &all {
        assert!(!d.trim().is_empty() && d.len() <= 110, "{n}: tooltip must be one short line ({d:?})");
        let l = d.to_lowercase(); for b in banned { assert!(!l.contains(b), "{n}: tooltip uses '{b}': {d}"); }
    }
}
