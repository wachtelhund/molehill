use molehill_sim::{grid::Grid, mat::Mat, stress::Stress, world::*, craft::Item};
// Lengths in these tests are in legacy 0.25 m tiles times S (cells per tile), so they mean the same in metres.
use molehill_sim::mat::{S, SF};

/// A grid walled with bedrock 2 tiles thick (floor and sides).
fn boxed(w: usize, h: usize) -> Grid { let t = 2 * S; let mut g = Grid::new(w, h); g.fill_rect(0, h - t, w - 1, h - 1, Mat::Bedrock); g.fill_rect(0, 0, t - 1, h - 1, Mat::Bedrock); g.fill_rect(w - t, 0, w - 1, h - 1, Mat::Bedrock); g }
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
    // solid block, carve a void under a roof of `thick` tiles, span `span` tiles (tiles = 0.25 m)
    let (span, thick) = (span * S, thick * S);
    let mut g = boxed(128 * S, 64 * S); for y in 10 * S..62 * S { for x in 2 * S..126 * S { let i = g.idx(x, y); g.set(i, m); } }
    let (x0, y0) = (64 * S - span / 2, 10 * S + thick); for y in y0..y0 + 8 * S { for x in x0..x0 + span { let i = g.idx(x, y); g.set(i, Mat::Empty); } }
    let mut st = Stress::new(128 * 64 * S * S); let f = st.solve(&g); (g, f)
}
#[test] fn dirt_roof_short_holds_long_fails() {
    let (_, f) = slab(Mat::Dirt, 4, 4); assert!(f.is_empty(), "1 m dirt span should hold");
    let (_, f) = slab(Mat::Dirt, 24, 4); assert!(!f.is_empty(), "6 m dirt span should fail");
}
#[test] fn granite_spans_further_than_dirt() {
    let (_, f) = slab(Mat::Granite, 24, 6); assert!(f.is_empty(), "6 m granite span holds");
}
#[test] fn floating_island_falls() {
    let mut g = boxed(64 * S, 64 * S); for y in 10 * S..14 * S { for x in 20 * S..30 * S { let i = g.idx(x, y); g.set(i, Mat::Limestone); } }
    let mut st = Stress::new(64 * 64 * S * S); let f = st.solve(&g); assert!(f.len() >= 30 * S * S);
}
#[test] fn wood_pillar_saves_wide_dirt_room() {
    let (mut g, f) = slab(Mat::Dirt, 24, 4); assert!(!f.is_empty());
    for k in [56usize, 60, 64, 68, 72] { for y in 14 * S..22 * S { for dx in 0..S { let i = g.idx(k * S + dx, y); g.set(i, Mat::WoodBeam); } } }
    let mut st = Stress::new(128 * 64 * S * S); let f = st.solve(&g); assert!(f.is_empty(), "pillars at 1 m spacing should hold, {} failing", f.len());
}
#[test] fn deterministic_worlds() {
    let a = World::new(256 * S, 256 * S, 7); let b = World::new(256 * S, 256 * S, 7); let c = World::new(256 * S, 256 * S, 8);
    assert_eq!(a.g.mat, b.g.mat); assert_ne!(a.g.mat, c.g.mat);
}
#[test] fn generated_world_is_mostly_stable() {
    let mut w = World::new(512 * S, 256 * S, 3);
    for _ in 0..200 { w.step(); }
    assert!(w.crumbled < 400 * (S * S) as u64, "crumbled {}", w.crumbled);
    assert!(w.g.count(Mat::Water) > 0 && w.g.count(Mat::Coal) > 0);
}
#[test] fn player_walks_jumps_and_mines() {
    let mut w = World::new(512 * S, 256 * S, 11); w.disasters.enabled = false;
    assert_eq!(w.moles.len(), 1, "exactly one mole: the player");
    for _ in 0..30 { w.step(); }
    let x0 = w.moles[0].x;
    assert!(w.pl.grounded, "starts on the burrow floor");
    // walk right for half a second
    w.input.right = true; for _ in 0..30 { w.step(); } w.input.right = false;
    assert!(w.moles[0].x > x0 + 2 * S, "walked right: {} -> {}", x0, w.moles[0].x);
    // jump: rises, then lands back on the floor
    for _ in 0..20 { w.step(); } let yb = w.moles[0].y;
    w.input.jump = true; let mut top = yb; for _ in 0..40 { w.step(); top = top.min(w.moles[0].y); } w.input.jump = false;
    assert!(yb - top >= 3 * S, "jumped at least 3 cells (from {yb} up to {top})");
    for _ in 0..90 { w.step(); } assert!(w.pl.grounded && w.moles[0].y == yb, "landed");
    // mine the floor under its feet: hold the mouse there until it opens, then it falls in
    let (x, y) = (w.moles[0].x, w.moles[0].y);
    let i = w.g.idx(x, y + 1); assert!(w.mine_time(i).is_some(), "floor is mineable");
    w.input.mine = Some((x, y + 1)); let mut t = 0; while w.g.mat[i] != 0 && t < 2000 { w.step(); t += 1; }
    assert_eq!(w.g.mat[i], 0, "mined the floor cell"); assert!(t > 1, "mining takes time");
    w.input.mine = None; for _ in 0..30 { w.step(); }
    assert!(w.moles[0].y > y, "dropped into the hole");
    // out of reach does nothing
    let fy = (y + 40 * S).min(w.g.h - 6); let far = w.g.idx(x, fy); w.input.mine = Some((x, fy)); let m0 = w.g.mat[far]; for _ in 0..600 { w.step(); }
    assert_eq!(w.g.mat[far], m0, "can't mine beyond reach");
}
#[test] fn player_places_pillar_and_respawns() {
    let mut w = World::new(512 * S, 256 * S, 11); w.disasters.enabled = false;
    for _ in 0..30 { w.step(); }
    let (x, y) = (w.moles[0].x, w.moles[0].y); let n0 = w.inv[Item::WoodPillar as usize];
    assert!(w.place_block(x + 3 * S, y, Item::WoodPillar).is_ok());
    assert_eq!(w.g.get(x + 3 * S, y), Mat::WoodBeam);
    assert!(w.place_block(x, y, Item::WoodPillar).is_err(), "not inside yourself");
    assert!(w.place_block(x + 60 * S, y, Item::WoodPillar).is_err(), "not out of reach");
    assert!(w.inv[Item::WoodPillar as usize] <= n0);
    w.moles[0].hp = -1.; for _ in 0..400 { w.step(); }
    assert!(w.moles[0].alive && w.pl.deaths == 1, "respawned at the burrow");
}

#[test]
fn fresh_world_does_not_collapse() {
    // 2 minutes at 60 tps with no player action: < 1% of static cells may crumble.
    let mut w = molehill_sim::world::World::new(1024 * S, 512 * S, 777);
    let solid = (0..w.g.mat.len()).filter(|&i| w.g.is_static(i)).count();
    for _ in 0..7200 { w.step(); }
    assert!((w.crumbled as f64) < 0.01 * solid as f64, "crumbled {} of {}", w.crumbled, solid);
}

mod v3b {
    use molehill_sim::world::World;
    use molehill_sim::logic::{Dev, Logic};
    use molehill_sim::grid::Grid;
    use molehill_sim::mat::Mat;
    use molehill_sim::mat::S; use molehill_sim::logic::B;
    fn open_grid() -> Grid { let mut g = Grid::new(128 * S, 64 * S); for x in 0..128 * S { for y in 60 * S..64 * S { let i = g.idx(x, y); g.mat[i] = Mat::Bedrock as u8; } } g }
    #[test]
    fn lever_wire_lamp_and_not_gate() {
        let mut g = open_grid(); let mut l = Logic::new(128 * S, 64 * S);
        let b = |x: usize, y: usize| y * 32 + x;
        assert!(l.place(&mut g, Dev::Lever, b(2, 5)));
        for x in 3..8 { l.place_wire(b(x, 5)); }
        assert!(l.place(&mut g, Dev::Lamp, b(8, 5)));
        l.step(&mut g, &[], 1); l.step(&mut g, &[], 2); assert_eq!(l.dev_s[b(8, 5)], 0);
        l.interact(b(2, 5)); l.step(&mut g, &[], 3); assert_eq!(l.dev_s[b(8, 5)], 1, "lamp lit by lever");
        // NOT gate: input west wire (net of lever) -> output east wire -> door
        for y in 6..9 { l.place_wire(b(3, y)); } l.place(&mut g, Dev::Not, b(4, 8)); l.place_wire(b(5, 8)); l.place(&mut g, Dev::Door, b(6, 8));
        for t in 4..8 { l.step(&mut g, &[], t); }
        let di = g.idx(6 * B + 1, 8 * B + 1); assert_eq!(g.m(di), Mat::Door, "NOT(on)=off -> door closed");
        l.interact(b(2, 5)); for t in 8..12 { l.step(&mut g, &[], t); } assert_eq!(g.m(di), Mat::Empty, "NOT(off)=on -> door open");
    }
    #[test]
    fn thrumming_carries_through_stone_not_air() {
        // solid granite, two pockets 30 m apart; the knock travels through the rock
        let mut g = open_grid(); let mut l = Logic::new(128 * S, 64 * S);
        let b = |x: usize, y: usize| y * 32 + x;
        let set = |g: &mut molehill_sim::grid::Grid, m: Mat, x0: usize, x1: usize| for y in 0..64 * S { for x in x0 * S..x1 * S { let i = g.idx(x, y); g.mat[i] = m as u8; g.flags[i] = 0; } };
        set(&mut g, Mat::Granite, 0, 128);
        for &(bx, by) in &[(1usize, 3usize), (2, 3), (3, 3), (28, 3), (29, 3), (30, 3)] { for y in by * B..by * B + B { for x in bx * B..bx * B + B { let i = g.idx(x, y); g.mat[i] = 0; } } }
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
        set(&mut g, Mat::Dirt, 60, 64); for y in 0..64 * S { for x in 60 * S..64 * S { let i = g.idx(x, y); g.flags[i] = molehill_sim::grid::F_LOOSE; } }
        let dirt = l.link_budget(&g, b(3, 3), b(28, 3));
        assert!(dirt < stone, "dirt muffles: {dirt} vs {stone}");
        // a 30 m run of loose sand-like dirt muffles it entirely
        set(&mut g, Mat::Dirt, 16, 112); for y in 0..64 * S { for x in 16 * S..112 * S { let i = g.idx(x, y); g.flags[i] = molehill_sim::grid::F_LOOSE; } }
        for &(bx, by) in &[(28usize, 3usize)] { for y in by * B..by * B + B { for x in bx * B..bx * B + B { let i = g.idx(x, y); g.mat[i] = 0; } } }
        assert!(l.link_budget(&g, b(3, 3), b(28, 3)) < 0., "loose dirt damps out");
    }
    #[test]
    fn save_load_roundtrip() {
        let mut w = World::new(512 * S, 256 * S, 4242);
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
        let mut w = World::new(512 * S, 256 * S, 99);
        for m in w.moles.iter_mut() { m.hunger = 30.; }
        let food0 = w.inv[molehill_sim::craft::Item::Grub as usize];
        for _ in 0..1200 { w.step(); }
        assert!(w.inv[molehill_sim::craft::Item::Grub as usize] < food0, "ate grubs");
        for m in &w.moles { eprintln!("{} h{} s{} {:?} hp{}", m.name, m.hunger, m.sleep, m.job, m.hp); }
        assert!(w.moles.iter().all(|m| m.hunger > 30.));
    }
    #[test]
    fn quake_loosens_overhangs() {
        let mut w = World::new(512 * S, 256 * S, 5);
        use molehill_sim::disasters::Kind2;
        let mut d = std::mem::replace(&mut w.disasters, molehill_sim::disasters::Disasters::new(1));
        assert!(d.schedule(&mut w, Kind2::Quake)); w.disasters = d;
        let c0 = (0..w.g.mat.len()).filter(|&i| w.g.loose(i)).count();
        for _ in 0..(16 * 60) { w.step(); }
        assert!(w.crumbled > 0 || (0..w.g.mat.len()).filter(|&i| w.g.loose(i)).count() > c0);
    }
}
#[test] fn endless_world_shifts_deterministically() {
    let mut w = World::new(256 * S, 256 * S, 5); let oy0 = w.oy;
    assert!(w.depth_m(0, (-oy0) as usize).abs() < 0.01, "start layer is depth 0");
    let (n, sl) = (w.g.w * 8, 128 * S as i64);
    let top: Vec<u8> = w.g.mat[..n].to_vec();
    w.shift(sl); assert_eq!(w.oy, oy0 + sl);
    w.shift(sl); // deeper still
    w.shift(-sl); w.shift(-sl); assert_eq!(w.oy, oy0);
    assert_eq!(&w.g.mat[..n], &top[..], "rows restored from cache");
    // fresh deep rows are generated identically per seed
    let mut a = World::new(256 * S, 256 * S, 5); let mut b = World::new(256 * S, 256 * S, 5);
    for _ in 0..4 { a.shift(128 * S as i64); b.shift(128 * S as i64); } assert_eq!(a.g.mat, b.g.mat);
    assert_eq!(a.g.count(Mat::Bedrock) , b.g.count(Mat::Bedrock));
    // no surface: no grass or sky anywhere, more lava than water deep down
    assert_eq!(a.g.count(Mat::Grass), 0);
}
#[test] fn mushrooms_grow_in_dark_damp_and_air_goes_stale() {
    let mut w = World::new(256 * S, 256 * S, 21);
    w.torches.clear();
    let (x, y) = (w.moles[0].x + 3 * S, w.moles[0].y);
    w.plant(x, y).expect("plant");
    let (bright, _moist) = w.farm_conditions(x, y); assert!(!bright, "no torch nearby");
    for _ in 0..60 * 60 * 3 { w.step(); }
    assert!(w.harvested > 0 || w.inv[Item::Mushroom as usize] > 0 || w.farms.iter().any(|f| f.growth > 0.3), "mushrooms grow");
    // a sealed pocket fills with CO2 from breathing; a vent clears it
    let n = 64 * S; let (c, lo, hi) = (30 * S, 20 * S, 40 * S);
    let mut a = molehill_sim::air::Air::new(n, n);
    let mut g = Grid::new(n, n); for i in 0..n * n { g.mat[i] = Mat::Granite as u8; } for y in lo..hi { for x in lo..hi { let i = g.idx(x, y); g.mat[i] = 0; } }
    for _ in 0..1000 { a.add(c, c, 4. * molehill_sim::air::MOLE_CO2); a.update(&g, &[]); } // four moles, ~4 min
    let stale = a.at(c, c); assert!(stale > 0.45, "sealed tunnel goes stale: {stale}");
    for _ in 0..100 { a.update(&g, &[(c, c)]); } assert!(a.at(c, c) < stale * 0.5, "vent clears");
}

/// Fresh worlds must not shed their caves: < 0.1% of static cells lost in 5 minutes (18000 ticks), many seeds.
#[test]
fn fresh_worlds_are_stable_multi_seed() {
    let seeds = [1u32, 4, 777, 9630, 1295, 12822, 11, 14];
    let hs: Vec<_> = seeds.iter().map(|&s| std::thread::spawn(move || {
        let mut w = molehill_sim::world::World::new(1024 * S, 512 * S, s); w.disasters.enabled = false;
        let solid = (0..w.g.mat.len()).filter(|&i| w.g.is_static(i)).count();
        for _ in 0..18000 { w.step(); }
        (s, w.crumbled as f64 / solid as f64)
    })).collect();
    for h in hs { let (s, f) = h.join().unwrap(); assert!(f < 0.001, "seed {s} lost {:.3}%", f * 100.); }
}

/// Old (surface-era, version < 3), truncated or garbage saves are refused with a message, never a panic.
#[test]
fn old_and_broken_saves_fail_cleanly() {
    let w = molehill_sim::world::World::new(256 * S, 256 * S, 3);
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
        let mut w = molehill_sim::world::World::new(1024 * S, 512 * S, s); w.disasters.enabled = false;
        for _ in 0..1800 { w.step(); }
        (s, w.moles.iter().filter(|m| m.alive).count())
    })).collect();
    for h in hs { let (s, n) = h.join().unwrap(); assert_eq!(n, 1, "seed {s}: the mole died in the start burrow"); }
}

/// The start burrow is not a cage: the player can mine out through its walls.
#[test]
fn player_can_dig_out_of_the_burrow() {
    for s in [3636u32, 777] {
        let mut w = molehill_sim::world::World::new(1024 * S, 512 * S, s); w.disasters.enabled = false;
        let y = w.moles[0].y; let x0 = w.moles[0].x;
        // walk right, mining whatever blocks the way at head and chest height
        w.input.right = true; let mut best = x0;
        for _ in 0..60 * 40 { let (x, yy) = ((w.pl.px + 1.2 * SF) as usize, w.moles[0].y); // the column the body's leading edge pushes into
            let t = (0..3 * S).map(|j| (x, yy - j)).find(|&(a, b)| w.g.is_solidish(w.g.idx(a, b))); w.input.mine = t; w.step(); best = best.max(w.moles[0].x); }
        assert!(best > x0 + 30 * S, "seed {s}: tunnelled out ({x0} -> {best}, y {y})");
    }
}
#[test] fn dig_tools_bite_fast_and_scale() {
    let mut w = World::new(512 * S, 256 * S, 11); w.disasters.enabled = false;
    for _ in 0..30 { w.step(); }
    let (x, y) = (w.moles[0].x, w.moles[0].y);
    // a pickaxe bite into dirt is near-instant and clears a mole-sized disc
    let (tx, ty) = (x + 8 * S, y - S);
    for c in w.bite_cells(tx, ty) { w.g.set(c, Mat::Dirt); }
    w.input.tool = 1; let n_bite = w.bite_cells(tx, ty).len(); assert!(n_bite >= 15 * S * S, "pickaxe bite is mole-sized ({n_bite} cells)");
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
    let mut w = World::new(512 * S, 256 * S, 11); w.disasters.enabled = false;
    let (x0, y0) = (w.moles[0].x, w.moles[0].y);
    for y in y0 - 30 * S..=y0 + 3 * S { for x in x0 - 40 * S..=x0 + 40 * S { let i = w.g.idx(x, y); w.g.set(i, if y > y0 || x < x0 - 30 * S || x > x0 + 30 * S { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = 0; } }
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
    assert!((4.5 * SF..9. * SF).contains(&full), "full jump {full} cells");
    let (top2, _) = press(&mut w, 3, |i| i.jump = true); let (top3, _) = press(&mut w, 40, |i| i.jump = false); let hop = y0 - top2.min(top3);
    assert!(hop < full * 0.6, "short hop {hop} vs full {full}");
    // ground speed: max in ~3 frames, stops almost instantly
    press(&mut w, 60, |_| {}); press(&mut w, 3, |i| { i.jump = false; i.right = true; }); assert!(w.pl.vx > 0.8 * TUNE.run, "accel: vx {}", w.pl.vx);
    press(&mut w, 3, |i| i.right = false); assert!(w.pl.vx.abs() < 0.5 * SF, "decel: vx {}", w.pl.vx);
}
#[test] fn coyote_time_and_wall_climb() {
    let (mut w, x0, y0) = arena();
    // a ledge: raise a platform under the mole and run off its right edge
    for x in x0 - 30 * S..=x0 + 2 * S { for y in y0 - 14 * S..=y0 { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); } }
    w.moles[0].y = y0 - 15 * S; w.pl.sync = None; press(&mut w, 30, |_| {}); assert!(w.pl.grounded, "on the ledge");
    let ylev = w.pl.py;
    let mut t = 0; while w.pl.grounded && t < 120 { w.input.right = true; w.step(); t += 1; }
    press(&mut w, 3, |i| i.right = true); // 3 frames after leaving the edge
    let (top, _) = press(&mut w, 20, |i| { i.right = false; i.jump = true; }); assert!(top < ylev - 2. * SF, "coyote jump worked: top {top} ledge {ylev}");
    press(&mut w, 120, |i| i.jump = false); assert!(w.pl.grounded);
    // too late: 15 frames after the edge, no jump
    let (mut w, x0, y0) = arena();
    for x in x0 - 30 * S..=x0 + 2 * S { for y in y0 - 14 * S..=y0 { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); } }
    w.moles[0].y = y0 - 15 * S; w.pl.sync = None; press(&mut w, 30, |_| {});
    let mut t = 0; while w.pl.grounded && t < 120 { w.input.right = true; w.step(); t += 1; }
    let ya = w.pl.py; press(&mut w, 15, |i| i.right = false); let (top, _) = press(&mut w, 20, |i| i.jump = true); assert!(top >= ya - 0.1 * SF, "no late jump: {top} vs {ya}");
    // no wall climb: run right into the arena wall, jump, and hold into it with W: no cling, no climb, back down
    let (mut w, _, _) = arena(); press(&mut w, 1, |i| i.jump = false); let yf = w.pl.py;
    press(&mut w, 120, |i| i.right = true);
    let (top, _) = press(&mut w, 120, |i| { i.right = true; i.up = true; i.jump = true; });
    assert!(top > yf - 7. * SF && w.pl.cling == 0, "no higher than a jump up a bare wall: {top} vs floor {yf}");
    assert!(w.pl.grounded && (w.pl.py - yf).abs() < 0.5, "back on the floor: {} vs {}", w.pl.py, yf);
}
#[test] fn jump_held_through_a_menu_does_not_fire() {
    // the menu closes on Space; the same press must not make the mole jump when play resumes
    let (mut w, _, _) = arena(); let y0 = w.pl.py;
    w.swallow_held(); let (top, _) = press(&mut w, 30, |i| i.jump = true); assert!(top >= y0 - 0.05, "held Space jumped: {top} vs {y0}");
    press(&mut w, 1, |i| i.jump = false); let (top, _) = press(&mut w, 20, |i| i.jump = true); assert!(top < y0 - 3. * SF, "a fresh press jumps: {top} vs {y0}");
    // a fresh world (or respawn / load) with Space already down doesn't jump either
    let (mut w, _, _) = arena(); w.pl.sync = None; let y0 = w.pl.py; let (top, _) = press(&mut w, 30, |i| i.jump = true); assert!(top >= y0 - 0.05, "held Space jumped after resync");
}
#[test] fn digging_and_placing_give_instant_feedback() {
    let (mut w, x0, y0) = arena();
    for y in y0 - 4 * S..=y0 { for x in x0 + 4 * S..=x0 + 9 * S { let i = w.g.idx(x, y); w.g.set(i, Mat::Rock); } }
    press(&mut w, 1, |_| {});
    // the strike is heard on the very first tick of holding the mouse, long before the rock gives
    w.input.mine = Some((x0 + 5 * S, y0 - S)); w.input.tool = 1; w.step();
    assert!(w.events.contains(&Event::Dig), "no strike on press: {:?}", w.events);
    let mut bite = None; for _ in 0..60 { w.step(); if let Some(m) = w.marks.iter().find(|m| m.0 == 0) { bite = Some(*m); break; } }
    let b = bite.expect("bite cleared within 1 s");
    assert!((b.1 - (x0 + 5 * S) as f32 - 0.5).abs() < 1e-3 && b.3 > SF, "bite mark at the aim: {b:?}");
    assert!(w.shake > 0.2, "a bite jolts the view: {}", w.shake);
    w.input.mine = None; press(&mut w, 120, |_| {});
    // placing a pillar marks the cell the same tick
    let (cx, cy) = (x0 - 3 * S, y0); w.place_block(cx, cy, Item::WoodPillar).unwrap();
    assert!(w.marks.iter().any(|m| m.0 == 1 && m.1 as usize == cx && m.2 as usize == cy), "place mark: {:?}", w.marks);
}

#[test] fn window_does_not_thrash_near_its_edge() {
    // a 256-row window used to slide up and straight back down every 30 ticks with the mole at y≈90
    let mut w = World::new(512 * S, 256 * S, 11); w.disasters.enabled = false;
    let (x0, y0) = (w.moles[0].x, 90 * S);
    for y in y0 - 8 * S..=y0 + 6 * S { for x in x0 - 10 * S..=x0 + 10 * S { let i = w.g.idx(x, y); w.g.set(i, if y > y0 || y < y0 - 4 * S || x < x0 - 6 * S || x > x0 + 6 * S { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = 0; } }
    w.moles[0].y = y0; w.pl = Default::default(); w.pl.fall_from = y0;
    let oy = w.oy; let mut shifts = 0; let mut last = oy;
    for _ in 0..600 { w.step(); if w.oy != last { shifts += 1; last = w.oy; } }
    assert_eq!(shifts, 0, "window slid {shifts} times (mole y {})", w.moles[0].y);
}

/// Loading a save where the mole stands far below the burrow must not count as a fall (it used to kill).
#[test] fn load_deep_does_not_fall_damage() {
    let (mut w, x0, y0) = arena();
    // a pocket 60 cells below the arena floor, and move the mole there as if it had dug down
    let y1 = y0 + 60 * S; for y in y1 - 4 * S..=y1 { for x in x0 - 3 * S..=x0 + 3 * S { let i = w.g.idx(x, y); w.g.set(i, Mat::Empty); } }
    for x in x0 - 4 * S..=x0 + 4 * S { let i = w.g.idx(x, y1 + 1); w.g.set(i, Mat::Granite); }
    w.moles[0].y = y1; w.pl.sync = None; for _ in 0..5 { w.step(); }
    assert_eq!(w.moles[0].hp, 100.);
    let b = molehill_sim::save::save(&w); let mut w2 = molehill_sim::save::load(&b).expect("load");
    for _ in 0..30 { w2.step(); }
    assert!(w2.moles[0].hp > 99., "hp after load {}", w2.moles[0].hp);
}
/// Copperline and knots: dragging wire over a knot costs nothing, picking up (E) takes the device first and
/// then the copperline under it, and a knot placed on copperline gives that copperline back.
#[test] fn wire_and_device_pack_accounting() {
    use molehill_sim::logic::Dev;
    let (mut w, x0, y0) = arena(); let (x, y) = (x0 + 8 * S, y0 - 2 * S); let k = molehill_sim::logic::B;
    let cu = |w: &World| w.inv[Item::CopperWire as usize]; let kn = |w: &World| w.inv[Item::Gate as usize];
    let (c0, k0) = (cu(&w), kn(&w));
    assert!(w.place_device(Dev::And, x, y).is_ok()); assert_eq!(kn(&w), k0 - 1);
    for _ in 0..10 { assert!(!w.lay_wire(x, y)); } assert_eq!(cu(&w), c0, "no copperline lost on a knot");
    assert!(w.lay_wire(x + k, y)); assert_eq!(cu(&w), c0 - 1); assert!(!w.lay_wire(x + k, y), "already wired");
    assert!(w.place_device(Dev::Or, x + k, y).is_ok()); assert_eq!(cu(&w), c0, "knot on copperline refunds it");
    w.inv[Item::Lamp as usize] = 1; assert!(w.lay_wire(x + 2 * k, y)); assert!(w.place_device(Dev::Lamp, x + 2 * k, y).is_ok());
    let c1 = cu(&w); assert_eq!(w.pick_up(x + 2 * k, y), Some(Picked::Dev(Dev::Lamp))); assert_eq!(w.inv[Item::Lamp as usize], 1);
    assert_eq!(cu(&w), c1, "the device comes up first"); assert_eq!(w.pick_up(x + 2 * k, y), Some(Picked::Wire)); assert_eq!(cu(&w), c1 + 1, "then its copperline");
    assert_eq!(w.pick_up(x + 2 * k, y), None, "nothing left there");
    assert_eq!(w.pick_up(x, y), Some(Picked::Dev(Dev::And))); assert_eq!(w.pick_up(x + k, y), Some(Picked::Dev(Dev::Or))); assert_eq!(kn(&w), k0);
    assert!(w.place_device(Dev::Lever, x + 60 * S, y).is_err(), "out of reach");
}
/// A burning fuse cord can't be picked up; one removed while burning and put down again is a fresh unlit cord.
#[test] fn replaced_fuse_is_not_burning() {
    use molehill_sim::logic::Dev;
    let (mut w, x0, y0) = arena(); let (x, y) = (x0 + 8 * S, y0 - 2 * S); let b = w.logic.block_of(x, y);
    assert!(w.place_device(Dev::Fuse, x, y).is_ok()); assert!(w.use_device(x, y)); w.step(); assert!(w.logic.lit(b));
    assert_eq!(w.pick_up(x, y), None, "too late: it's burning"); assert!(w.logic.lit(b));
    w.logic.remove(&mut w.g, b); assert!(w.place_device(Dev::Fuse, x, y).is_ok()); w.step(); assert!(!w.logic.lit(b));
}
/// Every item says, from one table, whether it is set down, thrown, or picked back up, and the hint names the inputs.
#[test] fn item_uses_table_is_consistent() {
    use molehill_sim::craft::ITEMS;
    for &it in ITEMS.iter() {
        let u = it.uses(); let c = it.controls();
        if u.throw { assert!(c.contains("Q: throw"), "{it:?}: throwable, so the hint says Q"); } else { assert!(!c.contains("Q:"), "{it:?}: not throwable"); }
        if u.pick_up { assert!(c.contains("E: "), "{it:?}: picked up with E"); }
        if u.place { assert!(c.contains("mouse"), "{it:?}: placed with the mouse"); }
    }
    for it in [Item::WoodPillar, Item::Brick, Item::IronBeam, Item::CopperWire, Item::Torch, Item::Lamp, Item::Vent, Item::Gate, Item::Thrummer, Item::Bore] {
        assert!(!it.uses().throw, "{it:?} is never thrown");
    }
    assert!(Item::Tnt.uses().place && Item::Tnt.uses().throw, "TNT: set down unlit, or thrown lit");
    let (mut w, _, _) = arena(); w.inv[Item::IronBeam as usize] = 3;
    assert!(w.throw_item(Item::IronBeam, (1., -1.), 1.).is_err() && w.inv[Item::IronBeam as usize] == 3, "a beam isn't thrown");
}
/// Torches come back with E, without digging.
#[test] fn torch_picks_up() {
    let (mut w, x0, y0) = arena(); let (x, y) = (x0 + 8 * S, y0 - S); w.inv[Item::Torch as usize] = 1;
    assert!(w.place_torch(x, y)); assert_eq!(w.inv[Item::Torch as usize], 0);
    assert_eq!(w.pick_up(x, y - S), Some(Picked::Torch), "hovering the flame picks it up");
    assert_eq!(w.inv[Item::Torch as usize], 1); assert!(w.torches.iter().all(|&t| t != (x, y)));
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
    assert_eq!(brush_radius(Brush::Place(Item::IronBeam), 99.), 2.0 * SF);
    assert_eq!(brush_radius(Brush::Place(Item::WoodPillar), 0.01), 0.5 * SF);
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
#[test] fn rock_falls_when_its_support_slumps_away() {
    // a slab resting on a heap of loose rubble held in by two temporary walls. Pulling the walls
    // triggers one stress update while the rubble still props the slab; then the rubble slumps out
    // sideways through CA moves alone. Those moves must tell the solver, or the slab hangs in the air.
    let (mut w, x0, y0) = arena();
    let (sx, sy) = (x0 + 8, y0); // rubble: columns sx..=sx+3, rows sy-3..=sy
    for y in sy - 3..=sy { for x in sx..=sx + 3 { let i = w.g.idx(x, y); w.g.set(i, Mat::Gravel); w.g.set_loose(i); } }
    let walls: Vec<usize> = (sy - 3..=sy).flat_map(|y| [w.g.idx(sx - 1, y), w.g.idx(sx + 4, y)]).collect();
    for &i in &walls { w.g.set(i, Mat::Granite); }
    for x in sx..=sx + 3 { let i = w.g.idx(x, sy - 4); w.g.set(i, Mat::Rock); } // slab, resting only on the rubble
    let rock = |w: &World| (sx..=sx + 3).filter(|&x| { let i = w.g.idx(x, sy - 4); w.g.m(i) == Mat::Rock && !w.g.loose(i) }).count();
    for _ in 0..30 { w.step(); } assert_eq!(rock(&w), 4, "slab rests on the rubble");
    while w.tick % 6 != 5 { w.step(); }
    for &i in &walls { w.g.set(i, Mat::Empty); } // solved on the very next tick, rubble still in place
    for _ in 0..240 { w.step(); }
    assert_eq!(rock(&w), 0, "slab fell once the rubble slumped from under it");
}
/// A damaged or stale save (e.g. an old autosave in localStorage) must fail to load or load into a world
/// that steps safely: never a panic (on the web a panic kills the page).
#[test] fn corrupt_saves_never_panic() {
    let mut w = World::new(256 * S, 128 * S, 77); w.disasters.enabled = false;
    for _ in 0..30 { w.step(); }
    let good = molehill_sim::save::save(&w);
    let mut s = 0x2545F4914F6CDD1Du64; let mut rnd = |k: usize| { s ^= s << 13; s ^= s >> 7; s ^= s << 17; (s % k as u64) as usize };
    for case in 0..120 {
        let mut b = good.clone();
        match case % 3 { 0 => b.truncate(rnd(b.len())), 1 => { for _ in 0..1 + rnd(4) { let i = rnd(b.len()); b[i] = rnd(256) as u8; } }, _ => { let i = rnd(b.len()); b[i] ^= 1 << rnd(8); } }
        let r = std::panic::catch_unwind(|| { if let Ok(mut l) = molehill_sim::save::load(&b) { for _ in 0..20 { l.step(); } } });
        assert!(r.is_ok(), "case {case} panicked");
    }
}

/// The hotbar spun by itself on the web: `f32::signum(0.0)` is 1.0, so an idle wheel stepped a slot every frame.
#[test] fn wheel_idle_is_still_and_trackpad_flick_is_one_notch() {
    use molehill_sim::view::Wheel;
    let dt = 1. / 60.;
    let mut w = Wheel::default();
    assert_eq!((0..600).map(|_| w.feed(0., false, dt)).sum::<i32>(), 0, "idle wheel: no notches (line units)");
    assert_eq!((0..600).map(|_| w.feed(0., true, dt)).sum::<i32>(), 0, "idle wheel: no notches (pixel units)");
    assert_eq!((0..60).map(|_| w.feed(-0.0, true, dt)).sum::<i32>(), 0, "negative zero is no scroll");
    // a notched mouse wheel: each click is a notch
    let mut w = Wheel::default(); let n: i32 = (0..5).map(|_| { let a = w.feed(1., false, dt); w.feed(0., false, dt); a }).sum(); assert_eq!(n, 5);
    let mut w = Wheel::default(); assert_eq!(w.feed(-1., false, dt), -1);
    // a trackpad flick with ~2 s of momentum: one slot, not dozens
    let mut w = Wheel::default(); let flick: i32 = (0..120).map(|i| w.feed(-30. * (1. - i as f32 / 120.), true, dt)).sum();
    assert_eq!(flick, -1, "flick + momentum is one notch");
    for _ in 0..10 { w.feed(0., true, dt); }
    assert_eq!((0..20).map(|_| w.feed(8., true, dt)).sum::<i32>(), 1, "after a pause the next swipe (other way) counts once");
    // a tiny brush of the pad is not a notch
    let mut w = Wheel::default(); assert_eq!((0..3).map(|_| w.feed(2., true, dt)).sum::<i32>(), 0);
    assert_eq!(w.feed(f32::NAN, true, dt), 0);
}
#[test] fn world_hover_explains_cells() {
    use molehill_sim::{tips::{describe, strain_word}, grid::F_SEEN, world::Farm};
    let (mut w, x0, y0) = arena();
    let see = |w: &mut World, x: usize, y: usize| { let i = w.g.idx(x, y); w.g.flags[i] |= F_SEEN; };
    w.logic.dev.fill(0); w.logic.wire.fill(0); w.buildings.clear(); w.chests.clear(); // the burrow's starting vent etc.
    // open air says nothing; unseen rock says nothing
    see(&mut w, x0, y0 - 5); w.set_wall(x0, y0 - 5, molehill_sim::walls::Wall::None);
    assert!(describe(&w, x0, y0 - 5).is_none(), "{:?}", describe(&w, x0, y0 - 5).map(|t| (t.name, t.lines)));
    assert!(describe(&w, x0, y0 + 2).is_none());
    see(&mut w, x0, y0 + 2); let t = describe(&w, x0, y0 + 2).unwrap(); assert_eq!(t.name, "Granite");
    // a mushroom plot says why it isn't growing
    w.farms.push(Farm { x: x0 + 5, y: y0, growth: 0.5, why: 3 }); see(&mut w, x0 + 5, y0);
    let t = describe(&w, x0 + 5, y0).unwrap(); assert_eq!(t.name, "Mushroom");
    assert!(t.lines.iter().any(|l| l.contains("50%")) && t.lines.iter().any(|l| l.contains("bright")) && t.lines.iter().any(|l| l.contains("dry")));
    // strain wording rises with load, and a crack always wins
    assert_eq!(strain_word(10, false), None);
    assert!(strain_word(100, false).unwrap().contains("Strain"));
    assert!(strain_word(200, false).unwrap().contains("Overloaded"));
    assert!(strain_word(0, true).unwrap().contains("Cracking"));
    // lore: no modern words in anything the hover can say
    let banned = ["radio", "electric", "signal", "sensor", "sky", "sun", "surface", "magic"];
    for (n, l) in [(t.name.clone(), t.lines.join(" "))] { let l = l.to_lowercase(); for b in banned { assert!(!l.contains(b), "{n}: {l}"); } }
}

/// A local stress re-solve (only the region around a change) gives the same answer as a full solve there.
#[test] fn local_stress_resolve_matches_full_solve() {
    let mut w = World::new(512 * S, 256 * S, 11); w.disasters.enabled = false;
    let (x0, y0) = (w.moles[0].x + 20 * S, w.moles[0].y + 12 * S);
    // carve a 6 m hall in the rock below the burrow, then re-solve only around it
    for y in y0..y0 + 8 * S { for x in x0..x0 + 24 * S { let i = w.g.idx(x, y); w.g.set(i, Mat::Empty); } }
    let bx = (x0, y0, x0 + 24 * S - 1, y0 + 8 * S - 1);
    let mut part = w.st.solve_rect(&w.g, Some(bx)); part.sort();
    let mut full = Stress::new(w.g.w * w.g.h); let all = full.solve(&w.g);
    let k = 4 * molehill_sim::mat::CPM; let inside = |i: u32| { let (x, y) = (i as usize % w.g.w, i as usize / w.g.w); x + k >= bx.0 && x <= bx.2 + k && y + k >= bx.1 && y <= bx.3 + k };
    let mut want: Vec<u32> = all.into_iter().filter(|&i| inside(i)).collect(); want.sort();
    assert_eq!(part, want, "failing cells near the change");
    let mut diff = 0; for y in bx.1 - k..=bx.3 + k { for x in bx.0 - k..=bx.2 + k { let i = w.g.idx(x, y); if w.st.util[i] != full.util[i] { diff += 1; } } }
    assert_eq!(diff, 0, "utilisation differs in {diff} cells");
}
/// Shift is sprint: scrolling while sprinting changes the hotbar slot like a plain wheel (it used to zoom).
#[test] fn wheel_modifiers_sprint_does_not_zoom() {
    use molehill_sim::view::{wheel_use, wheel_delta, Mods, WheelUse::*};
    let sprint = Mods { shift: true, ..Default::default() };
    for brush in [false, true] {
        assert_eq!(wheel_use(Mods::default(), brush), Hotbar);
        assert_eq!(wheel_use(sprint, brush), Hotbar, "sprinting keeps the wheel on the hotbar");
        assert_eq!(wheel_use(Mods { ctrl: true, ..Default::default() }, brush), Zoom);
        assert_eq!(wheel_use(Mods { cmd: true, shift: true, ..Default::default() }, brush), Zoom);
    }
    assert_eq!(wheel_use(Mods { alt: true, ..Default::default() }, true), Brush, "brush size has its own modifier");
    assert_eq!(wheel_use(Mods { alt: true, shift: true, ..Default::default() }, true), Brush);
    assert_eq!(wheel_use(Mods { alt: true, ..Default::default() }, false), Hotbar, "no brush in hand: the wheel stays on the hotbar");
    // Shift turns a vertical wheel sideways on macOS / in browsers: it still counts, the same way round
    assert_eq!(wheel_delta(-100., 0., sprint), 100.); assert_eq!(wheel_delta(0., -3., sprint), -3.);
    assert_eq!(wheel_delta(-100., 0., Mods::default()), 0., "a sideways swipe without Shift is not the wheel");
}

// ---- bands: the endless world off-window, and chunked saves ----
mod bands {
    use molehill_sim::{world::*, mat::{S, Mat}, logic::Dev, craft::{Item, Station}, save, band::BH};
    use std::collections::HashMap;
    /// A world with things built near the top of the window, at rows that leave it on a slide down.
    fn built() -> (World, Snap) {
        let mut w = World::new(256 * S, 256 * S, 5); w.disasters.enabled = false;
        let (x, y) = (100, 40);
        for yy in 32..48 { for xx in 96..152 { let i = w.g.idx(xx, yy); w.g.set(i, Mat::Empty); } }
        let b = w.logic.block_of(x, y); let gg = &mut w.g; assert!(w.logic.place(gg, Dev::Lever, b));
        let t = w.logic.block_of(x + 40, y); let gg = &mut w.g; assert!(w.logic.place(gg, Dev::Tnt, t)); w.logic.burn[t] = 50;
        let c = w.logic.block_of(x + 20, y); w.logic.wire[c] = 1;
        w.torches.push((x + 60, y)); w.drops.push(Drop::at((x + 70) as f32 + 0.5, (y + 1) as f32 + 0.5, Item::Coal, 3));
        w.buildings.push(Building { kind: Station::Forge, x: x + 120, y: 50, built: true, claimed: false });
        w.farms.push(Farm { x: x + 200, y: 45, growth: 0.5, why: 0 });
        let a = w.air.block(x, y); w.air.co2[a] = 0.7;
        let s = snap(&w); (w, s)
    }
    #[derive(PartialEq, Debug)] struct Snap { oy: i64, cells: Vec<u8>, devs: Vec<(usize, u8, i16, u8)>, torches: Vec<(usize, usize)>, drops: Vec<(usize, usize, u32)>, blds: Vec<(usize, usize, bool)>, farms: Vec<(usize, usize)>, co2: u8 }
    fn snap(w: &World) -> Snap {
        let top = 64 * w.g.w; let mut t = w.torches.clone(); t.sort();
        Snap { oy: w.oy, cells: w.g.mat[..top].to_vec(),
            devs: (0..w.logic.dev.len()).filter(|&b| w.logic.dev[b] != 0 || w.logic.wire[b] != 0).map(|b| (b, w.logic.dev[b], w.logic.burn[b], w.logic.wire[b])).collect(),
            torches: t, drops: w.drops.iter().map(|d| (d.x, d.y, d.n)).collect(), blds: w.buildings.iter().map(|b| (b.x, b.y, b.built)).collect(),
            farms: w.farms.iter().map(|f| (f.x, f.y)).collect(), co2: (w.air.co2[w.air.block(100, 40)] / 1.5 * 255.) as u8 }
    }
    const SL: i64 = 128 * S as i64;

    /// Devices, copperline, lit fuses, torches, drops, stations, plots and air used to vanish when their rows
    /// left the window; now their band keeps them and brings them back.
    #[test] fn things_survive_leaving_the_window() {
        let (mut w, before) = built();
        w.shift(SL); assert_eq!(w.bands.len(), SL as usize / BH);
        assert!(w.torches.iter().all(|t| t.0 != 160), "the torch left with its band");
        let up: Vec<i64> = w.bands.keys().copied().collect();
        w.shift(-SL); assert!(up.iter().all(|k| !w.bands.contains_key(k)), "back in the window, those bands are no longer stored");
        let after = snap(&w); assert_eq!(after.devs, before.devs); assert_eq!(after, before);
        // a lit charge keeps burning where it was: it really went with the band
        let t = w.logic.block_of(140, 40); assert_eq!(w.logic.burn[t], 50);
    }
    #[test] fn save_keeps_off_window_bands() {
        let (mut w, before) = built(); w.shift(SL); w.shift(SL);
        // one blob
        let mut l = save::load(&save::save(&w)).expect("load");
        assert_eq!(l.oy, w.oy); assert_eq!(l.bands.len(), w.bands.len());
        l.shift(-SL); l.shift(-SL); assert_eq!(snap(&l), before);
        // parts: manifest + blobs by key
        let p = save::save_parts(&w); let store: HashMap<String, Vec<u8>> = p.bands.iter().map(|(k, b)| (k.clone(), b.to_vec())).collect();
        let mut m = p.manifest.clone(); m.push(0);
        let mut l = save::load_with(&m, &mut |k| store.get(k).cloned()).expect("load parts");
        l.shift(-SL); l.shift(-SL); assert_eq!(snap(&l), before);
    }
    /// An autosave writes the window and new bands only: far bands keep their keys while unchanged.
    #[test] fn autosave_writes_only_changed_bands() {
        let (mut w, _) = built(); w.shift(SL);
        let k1: Vec<String> = save::save_parts(&w).bands.iter().map(|b| b.0.clone()).collect();
        let i = w.g.idx(300, 300); w.g.set(i, Mat::IronBeam); // one change in one window band
        let p2 = save::save_parts(&w); let k2: Vec<String> = p2.bands.iter().map(|b| b.0.clone()).collect();
        let new = k2.iter().filter(|k| !k1.contains(k)).count();
        assert_eq!(new, 1, "one changed band, one new key");
        for (wy0, b) in &w.bands { assert!(k1.contains(&save::band_key(*wy0, b.hash)), "far band {wy0} kept its key"); }
        assert!(p2.manifest.len() < 16 * 1024, "the manifest stays small: {} bytes", p2.manifest.len());
        // digging deeper adds bands without rewriting the old ones
        w.shift(SL); let k3: Vec<String> = save::save_parts(&w).bands.iter().map(|b| b.0.clone()).collect();
        assert!(k1.iter().filter(|k| k3.contains(k)).count() >= SL as usize / BH, "old far bands are reused");
    }
    #[test] fn missing_or_damaged_bands() {
        let (mut w, _) = built(); w.shift(SL);
        let p = save::save_parts(&w); let mut store: HashMap<String, Vec<u8>> = p.bands.iter().map(|(k, b)| (k.clone(), b.to_vec())).collect();
        let mut m = p.manifest.clone(); m.push(0);
        let far: Vec<String> = w.bands.iter().map(|(wy0, b)| save::band_key(*wy0, b.hash)).collect();
        store.remove(&far[0]); if let Some(b) = store.get_mut(&far[1]) { let n = b.len() / 2; b[n] ^= 0x55; }
        let l = save::load_with(&m, &mut |k| store.get(k).cloned()).expect("far bands lost is not fatal");
        assert_eq!(l.bands.len(), far.len() - 2, "the missing and the damaged band are dropped");
        assert!(l.log.iter().any(|(_, s)| s.contains("missing")));
        let win = p.bands.iter().map(|b| b.0.clone()).find(|k| !far.contains(k)).unwrap(); store.remove(&win);
        assert!(save::load_with(&m, &mut |k| store.get(k).cloned()).is_err(), "a hole around the mole fails the load");
    }
    /// v3 saves (one blob, a row cache of cells) from before bands still load; the cached rows become bands.
    #[test] fn v3_save_still_loads() {
        let mut w = save::load(include_bytes!("data/v3_cpm8.bin")).expect("v3 loads");
        assert_eq!(w.bands.len(), SL as usize / BH, "the row cache became bands");
        let wy0 = *w.bands.keys().next().unwrap(); let (m, _) = w.stored_cells(wy0).unwrap();
        w.shift(-SL); assert!(!w.bands.contains_key(&wy0)); assert_eq!(&w.g.mat[..m.len()], &m[..], "rows come back from the converted band");
        for _ in 0..30 { w.step(); }
        let l = save::load(&save::save(&w)).expect("and saves again as v4"); assert_eq!(l.oy, w.oy);
    }
}
/// The material table is the one place material behaviour lives: its rows must hang together.
#[test]
fn material_table_is_consistent() {
    use molehill_sim::mat::{Kind, Mat, NMAT, BUILT, OPEN, SOIL};
    for v in 0..NMAT as u8 {
        let m = Mat::from_u8(v); let p = m.p();
        assert!(!p.name.is_empty() && !p.desc.is_empty(), "{m:?} needs a name and a description");
        if p.kind == Kind::Liquid { assert!(p.spread > 0 && p.flow_in_10 > 0, "{m:?}: a liquid must flow"); }
        if p.heat >= 2 { assert!(p.cools_to != Mat::Empty, "{m:?}: something molten must set into something"); }
        if p.ignite > 0 { assert!(p.kind == Kind::Solid || p.kind == Kind::Powder, "{m:?}: only solid things burn"); }
        if m.has(BUILT) { assert_eq!(p.kind, Kind::Solid, "{m:?}: built blocks are solid"); }
        if m.has(SOIL) { assert!(matches!(p.kind, Kind::Solid | Kind::Powder), "{m:?}: soil is ground"); }
        if m.has(OPEN) { assert!(m.passable(), "{m:?}: a block can only displace something you can pass through"); }
        if p.kind == Kind::Solid { assert!(matches!(p.crumbles_to.kind(), Kind::Solid | Kind::Powder), "{m:?} must crumble into rubble"); }
    }
    // heat rules come from the table: lava lights coal and wood, a flame lights wood but not coal
    let (lava, fire) = (Mat::Lava.p().heat, Mat::Fire.p().heat);
    assert!(Mat::Coal.lit_by(lava) && Mat::WoodBeam.lit_by(lava) && Mat::WoodBeam.lit_by(fire) && !Mat::Coal.lit_by(fire) && !Mat::Granite.lit_by(lava));
}

/// After a slide the stress state moves with the rock and only the entering rows are re-solved (in 32 m
/// boxes, a box a tick over the next ticks: `finish_seam` runs the rest at once): the result matches a
/// full solve of the new window (both ways, fresh rows and stored ones): the same cells fail, and no
/// cell's utilisation is off by more than a quarter (a box sees only its own margins, as any region solve).
#[test] fn slide_keeps_stress_state() {
    let mut w = World::new(512 * S, 256 * S, 9); w.disasters.enabled = false;
    for _ in 0..60 { w.step(); }
    for rows in [SLIDE as i64, -(SLIDE as i64)] {
        w.shift(rows); w.finish_seam();
        let mut full = Stress::new(w.g.w * w.g.h); let mut want = full.solve(&w.g); want.sort();
        let diff = (0..w.g.mat.len()).filter(|&i| (w.st.util[i] as i32 - full.util[i] as i32).abs() > 32).count();
        let mut have: Vec<u32> = (0..w.g.mat.len() as u32).filter(|&i| w.st.util[i as usize] > 128 && w.g.mat[i as usize] != Mat::Bedrock as u8).collect(); have.sort();
        assert_eq!(diff, 0, "slide {rows}: utilisation off by more than 0.25 in {diff} cells");
        assert_eq!(have.len(), want.len(), "slide {rows}: failing cells");
    }
}

/// The falling sand runs only in the active region around the mole: sand far away waits mid-air, and
/// falls once the region reaches it.
#[test] fn ca_runs_only_near_the_mole() {
    let mut w = World::new(1024 * S, 512 * S, 11); w.disasters.enabled = false;
    let (mx, my) = (w.moles[0].x, w.moles[0].y);
    let near = (mx + 8 * molehill_sim::mat::CPM, my - 8 * molehill_sim::mat::CPM);
    let far = (if mx > w.g.w / 2 { 8 * S } else { w.g.w - 8 * S }, my - 8 * molehill_sim::mat::CPM);
    let far = (far.0.max(4 * S + 2).min(w.g.w - 4 * S - 2), far.1);
    for &(x, y) in &[near, far] { for yy in y - 6..=y { for xx in x - 2..=x + 2 { let i = w.g.idx(xx, yy); w.g.set(i, Mat::Empty); } } let i = w.g.idx(x, y - 6); w.g.set(i, Mat::Sand); }
    for _ in 0..30 { w.step(); }
    assert_ne!(w.g.get(near.0, near.1 - 6), Mat::Sand, "sand near the mole falls");
    assert_eq!(w.g.get(far.0, far.1 - 6), Mat::Sand, "sand far outside the active region waits");
    w.g.active = None; w.g.step_ca(); w.g.step_ca(); // (region covering it again)
    assert_ne!(w.g.get(far.0, far.1 - 6), Mat::Sand, "and falls once the region covers it");
}

/// Ore by depth (PROGRESSION.md tiers): copper just below the holt, iron in the mid granite, basalt
/// ruling the deep. Sampled straight from the generator over many columns, several seeds.
#[test] fn ore_sits_in_its_depth_bands() {
    use molehill_sim::{gen::Gen, mat::CPM};
    let w = 1024 * S; let band = |d: i64| d.div_euclid(10); // 10 m bands
    let mut tally: std::collections::BTreeMap<i64, [u32; 4]> = Default::default(); // copper, iron, basalt, solid
    for seed in [3u32, 77, 4242] { let g = Gen::new(seed, w);
        for d in (-30..120i64).step_by(2) { let wy = d * CPM as i64; for x in (16 * S..w - 16 * S).step_by(3) {
            let m = Mat::from_u8(g.cell(x, wy, w).0); let t = tally.entry(band(d)).or_default();
            if m == Mat::CopperOre { t[0] += 1; } if m == Mat::IronOre { t[1] += 1; } if m == Mat::Basalt { t[2] += 1; }
            if m.kind() == molehill_sim::mat::Kind::Solid { t[3] += 1; } } } }
    let sum = |b0: i64, b1: i64, k: usize| (b0..=b1).map(|b| tally.get(&b).map_or(0, |t| t[k])).sum::<u32>();
    assert!(sum(0, 3, 0) > 100, "copper seams just below the holt: {}", sum(0, 3, 0));
    assert_eq!(sum(6, 11, 0), 0, "no copper deep down");
    assert_eq!(sum(-3, 1, 1), 0, "no iron near the holt");
    assert!(sum(4, 8, 1) > 100, "iron in the mid granite: {}", sum(4, 8, 1));
    assert!(sum(4, 8, 1) > 3 * sum(0, 3, 1), "iron is a deeper ore");
    let deep = sum(9, 11, 2) as f32 / sum(9, 11, 3).max(1) as f32; assert!(deep > 0.5, "basalt rules below 90 m: {deep}");
    assert_eq!(sum(-3, 1, 2), 0, "no basalt near the holt");
}
/// Grottos are rare and follow the water: commonest in the wet upper layers, rarer with depth, none in
/// the lava deep. Counted over 64 grotto cells per row band and 6 seeds.
#[test] fn grottos_follow_the_layers() {
    use molehill_sim::gen::{Gen, GROTTO_H};
    let w = 1024 * S * 8; let rows = |d: i64| (d * molehill_sim::mat::CPM as i64).div_euclid(GROTTO_H as i64);
    let count = |d0: i64, d1: i64| -> (usize, usize) { let mut n = (0, 0); for seed in 0..6u32 { let g = Gen::new(seed * 7 + 1, w);
        for gy in rows(d0)..rows(d1) { for gx in 0..(w / molehill_sim::gen::GROTTO_W) as i64 { n.1 += 1; if g.grotto_at(gx, gy).is_some() { n.0 += 1; } } } } n };
    let (up, up_n) = count(-120, -24); let (mid, mid_n) = count(24, 48); let (deep, _) = count(72, 200);
    let (fu, fm) = (up as f32 / up_n as f32, mid as f32 / mid_n as f32);
    assert!((0.1..0.3).contains(&fu), "upper layers: a grotto in {:.0}% of cells", fu * 100.);
    assert!(fm < fu * 0.6 && fm > 0., "rarer deeper: {:.0}% vs {:.0}%", fm * 100., fu * 100.);
    assert_eq!(deep, 0, "no grottos in the lava deep");
    // deterministic per seed
    let (a, b) = (Gen::new(5, w), Gen::new(5, w)); assert_eq!(a.grottos_in_rows(-2000, 2000), b.grottos_in_rows(-2000, 2000));
}
/// A generated grotto stands: a fresh holt with grottos in its window keeps its hollows open, its trees
/// up and its pools full for half a minute.
#[test] fn generated_grottos_hold() {
    for seed in [4242u32, 77] {
        let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
        let gs: Vec<_> = w.grottos.iter().filter(|g| g.cy - w.oy > 0 && ((g.cy - w.oy) as usize) < w.g.h).cloned().collect();
        assert!(!gs.is_empty(), "seed {seed}: a grotto in the window");
        let wood = |w: &World| (0..w.g.mat.len()).filter(|&i| w.g.mat[i] == Mat::RootTree as u8).count();
        let (t0, c0) = (wood(&w), w.crumbled); assert!(t0 > 0, "root-trees grew in the grottos");
        // the mole isn't near most grottos: run them all, like a mole that walked by every one
        for _ in 0..1800 { w.step(); w.g.active = None; }
        assert!(w.crumbled - c0 < 200 * (S * S) as u64, "seed {seed}: {} cells crumbled", w.crumbled - c0);
        assert!(wood(&w) * 10 >= t0 * 9, "seed {seed}: trees stand ({} of {t0})", wood(&w));
        for g in &gs { // the hollow (as generated) is still open air, bar a little rubble or a fresh trunk cell
            let (mut n, mut open) = (0, 0);
            for y in 0..w.g.h { for x in g.cx.saturating_sub(g.rx)..(g.cx + g.rx).min(w.g.w) { if g.part(x, w.oy + y as i64) == Some(molehill_sim::gen::Part::Hollow) { n += 1; if !w.g.is_solidish(w.g.idx(x, y)) { open += 1; } } } }
            assert!(n > 0 && open * 10 >= n * 9, "seed {seed}: grotto at {} is {open} of {n} cells open", g.cx); }
    }
}
/// Slides into fresh rows leave nothing about to fail: the old rows next to the new ones are stabilised
/// too, and a roof running up to the window's edge isn't mistaken for a thin one.
#[test] fn slides_leave_nothing_failing() {
    for seed in [777u32, 11] {
        let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
        for rows in [SLIDE as i64, -(SLIDE as i64)] { w.shift(rows); w.finish_seam(); let f = Stress::new(w.g.w * w.g.h).solve(&w.g); assert!(f.is_empty(), "seed {seed}, slide {rows}: {} cells fail", f.len()); }
    }
}
/// Glowmites drift inside their grotto, light it, and can be caught one at a time; a caught one is back later.
#[test] fn glowmites_drift_glow_and_get_caught() {
    use molehill_sim::{craft::Item, gen::Grotto};
    let (mut w, x0, y0) = arena(); let c = molehill_sim::mat::CPM;
    w.grottos = vec![Grotto { id: 99, cx: x0 + 2 * c, cy: w.oy + y0 as i64 - 2 * c as i64, rx: 2 * c, ry: c + c / 2, floor: 3 * c as i64 / 4, pool: (0, 0), trees: [(0, 0); 3], ntrees: 0 }];
    let g = w.grottos[0]; let n0 = w.glowmites().len(); assert!(n0 >= 4, "a swarm: {n0}");
    let inside = |w: &World| w.glowmites().iter().all(|m| ((m.0 - g.cx as f32) / g.rx as f32).abs() < 1. && ((m.1 - (g.cy - w.oy) as f32) / g.ry as f32).abs() < 1.);
    for _ in 0..600 { w.step(); assert!(inside(&w), "glowmites stay in the hollow"); }
    let p = w.glowmites()[0]; let l0 = w.light_sources().len();
    assert!(w.catch_glowmite(p.0 as usize, p.1 as usize), "caught");
    assert_eq!(w.inv[Item::Glowmite as usize], 1); assert_eq!(w.glowmites().len(), n0 - 1); assert_eq!(w.light_sources().len(), l0 - 1, "one light fewer");
    assert!(!w.catch_glowmite(x0 + 30 * c, y0), "nothing to catch out there");
    w.tick += GLOWMITE_BACK; assert_eq!(w.glowmites().len(), n0, "the swarm refills");
}
/// Brush size keys work on any layout: `[` typed as Option+8 (Swedish/German Mac) counts, and Option+digit
/// no longer picks a hotbar slot (it used to jump to slot 8 instead of shrinking the brush).
#[test] fn brush_keys_any_layout() {
    use molehill_sim::view::*; use molehill_sim::world::Brush;
    assert_eq!(brush_key_steps(true, false, false, false), -1, "typed [");
    assert_eq!(brush_key_steps(false, true, false, false), 1, "typed ]");
    assert_eq!(brush_key_steps(true, false, true, false), -1, "US layout: typed and physical [ in the same frame count once");
    assert_eq!(brush_key_steps(false, false, false, true), 1, "physical ] (key right of P)");
    assert!(!digit_selects_slot(Mods { alt: true, ..Default::default() }), "Option+8 is [ on a Swedish Mac, not slot 8");
    assert!(digit_selects_slot(Mods::default()) && digit_selects_slot(Mods { shift: true, ..Default::default() }));
    // every brush item resizes, each keeps its own size, and the label is in metres
    let mut b = BrushSizes::default();
    for br in [Brush::Dig(1), Brush::Dig(2), Brush::Dig(3), Brush::Place(Item::WoodPillar), Brush::Place(Item::Brick), Brush::Place(Item::IronBeam)] {
        let r0 = b.radius(br); b.step(br, -1); b.step(br, 1); b.step(br, 1); assert!(b.radius(br) >= r0, "{br:?} grows");
        assert!(b.label(br).ends_with(" m"), "{}", b.label(br));
    }
    b.snap = true; assert!(b.snapping(Brush::Place(Item::Brick)) && !b.snapping(Brush::Dig(1)), "snap is for building only");
    assert!(b.label(Brush::Place(Item::Brick)).starts_with("grid "));
}
#[test] fn fresh_copperline_reads_idle_before_the_next_tick() {
    // the game draws between sim ticks: copperline laid since the last net rebuild has no net yet and
    // must read as idle, not index out of bounds
    let (mut w, x0, y0) = arena();
    let b = w.logic.block_of(x0 + 8, y0 - 2); assert!(w.logic.place_wire(b));
    assert!(!w.logic.net_lit(b));
    w.step(); assert!(!w.logic.net_lit(b));
}
#[test] fn torch_light_is_blocked_by_a_wall() {
    // a torch on the floor of an open arena; a granite wall 2 m away. The same spot 1 m past the wall
    // is dark, while the mirrored spot on the open side is lit: the torch casts a shadow.
    use molehill_sim::mat::CPM;
    let (mut w, x0, y0) = arena();
    let (tx, ty) = (x0, y0 - CPM); w.torches.push((tx, ty));
    for y in y0 - 3 * CPM..=y0 { for x in tx + 2 * CPM..tx + 2 * CPM + CPM / 2 { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); } }
    w.compute_light((x0 - 30 * CPM / 4, y0 - 7 * CPM, x0 + 30 * CPM / 4, y0));
    let luma = |w: &World, x: usize, y: usize| { let l = w.light.at(w.g.idx(x, y)); l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2 };
    let behind = luma(&w, tx + 3 * CPM, ty); let open = luma(&w, tx - 3 * CPM, ty);
    assert!(open > 0.25, "open side lit ({open})");
    assert!(behind < 0.05, "behind the wall dark ({behind}), open side {open}");
    // and every light source goes through the same rays: the torch is one of the sources
    assert!(w.light_sources().iter().any(|s| (s.x - (tx as f32 + 0.5)).abs() < 0.01 && (s.y - (ty as f32 + 0.5)).abs() < 0.01));
}
#[test] fn furnishing_is_craftable_in_survival() {
    // every decor block, pigment and piece of furniture has a recipe (Survival builds from the ground up;
    // Creative just has them in the pack), and crafting one at a workbench works from raw materials
    use molehill_sim::craft::{ITEMS, RECIPES, Item, Station};
    for it in ITEMS { if it.block().is_some() || it.pigment().is_some() || it.furn().is_some() {
        assert!(RECIPES.iter().any(|r| r.out.0 == it), "{it:?} has no recipe");
        assert!(it.uses().place, "{it:?} can't be placed"); assert!(!it.controls().is_empty(), "{it:?} has no controls line");
    } }
    let (mut w, x0, y0) = arena(); w.inv = [0; molehill_sim::craft::NITEM];
    for &(it, n) in Station::Workbench.cost() { w.inv[it as usize] = n; } w.place_building(Station::Workbench, x0 - 30, y0).unwrap();
    w.inv[Item::Wood as usize] = 4;
    let r = RECIPES.iter().position(|r| r.out.0 == Item::Table).unwrap();
    w.craft_now(r).unwrap(); assert_eq!(w.inv[Item::Table as usize], 1); assert_eq!(w.inv[Item::Wood as usize], 0);
}

// ---- progression: stations, recipes unlocked by touch, tool tiers (PROGRESSION.md) ----
mod progression {
    use molehill_sim::{world::*, craft::*, save};
    /// Short chains: nothing is more than two crafting steps from what the ground gives (ore → ingot → tool).
    #[test] fn recipes_are_short_chains() {
        fn steps(it: Item, seen: &mut Vec<Item>) -> u32 {
            if seen.contains(&it) { return 0; }
            let rs: Vec<&Recipe> = RECIPES.iter().filter(|r| r.out.0 == it).collect();
            if rs.is_empty() { return 0; }
            seen.push(it); let s = rs.iter().map(|r| 1 + r.inputs.iter().map(|&(i, _)| steps(i, seen)).max().unwrap_or(0)).min().unwrap(); seen.pop(); s
        }
        for r in RECIPES { assert!(steps(r.out.0, &mut vec![]) <= 2, "{} is {} steps from the ground", r.name, steps(r.out.0, &mut vec![])); }
        for s in STATIONS { assert!(RECIPES.iter().any(|r| r.station == s), "{s:?} makes something"); }
        // the tool ladder: pickaxe at the workbench (wood and stone, which claws can get), drill at the workshop, bore at the forge
        let at = |n: &str| RECIPES[recipe_by_name(n).unwrap()].station;
        assert_eq!((at("Pickaxe"), at("Drill"), at("Bore")), (Station::Workbench, Station::Workshop, Station::Forge));
        for &(it, _) in RECIPES[recipe_by_name("Pickaxe").unwrap()].inputs { assert!(matches!(it, Item::Wood | Item::Stone)); }
    }
    /// The tool ladder can be climbed from the Survival kit with what each tier can dig: no deadlock like
    /// "iron ore needs the drill, the drill needs iron". The forge burns coal (dug with the pickaxe) when it
    /// has no lava, and a lever (free) powers the workshop's copperline.
    #[test] fn survival_ladder_is_reachable() {
        use molehill_sim::{mat::{Mat, NMAT}, survival::KIT};
        let mut have = [false; NITEM]; for &(it, _) in KIT { have[it as usize] = true; }
        for it in [Item::Grub, Item::Root, Item::Fungus, Item::Spore, Item::Mushroom] { have[it as usize] = true; } // found and farmed
        let mut order = vec![]; let mut iron_before_drill = false;
        for _round in 0..20 {
            if !have[Item::Drill as usize] && (have[Item::IronIngot as usize] || have[Item::IronOre as usize]) { iron_before_drill = true; }
            let tier = tool_tier(&std::array::from_fn(|k| have[k] as u32));
            for k in 0..NMAT { let m = Mat::from_u8(k as u8); if m == Mat::Bedrock { continue; }
                // claws and up take anything of tier 1 or less; harder rock needs the tool
                if m.p().tier <= tier.max(1) { if let Some(it) = drop_of(m) { have[it as usize] = true; } } }
            let built = |s: Station, have: &[bool; NITEM]| s.cost().iter().all(|&(it, _)| have[it as usize]);
            let ready = |s: Station, have: &[bool; NITEM]| match s { Station::Forge => have[Item::Coal as usize] || have[Item::Charcoal as usize], Station::Workshop => have[Item::CopperWire as usize], _ => true };
            for r in RECIPES.iter() {
                if !have[r.out.0 as usize] && built(r.station, &have) && ready(r.station, &have) && r.inputs.iter().all(|&(it, _)| have[it as usize]) { have[r.out.0 as usize] = true; order.push(r.name); }
            }
        }
        for t in [Item::Pickaxe, Item::Drill, Item::Bore, Item::IronIngot, Item::CopperWire, Item::IronBeam] { assert!(have[t as usize], "{t:?} is out of reach in Survival (made: {order:?})"); }
        assert!(!iron_before_drill, "iron ore needs the drill, so the drill can't need iron");
        // bronze is the drill's metal: alloyed after copper, before iron
        let pos = |n: &str| order.iter().position(|&o| o == n);
        let bronze = [pos("Bronze x2"), pos("Bronze x4")].into_iter().flatten().min();
        assert!(bronze.is_some() && pos("Drill") > bronze, "bronze comes before the drill (made: {order:?})");
    }
    /// Dig tempo (`examples/digtempo.rs` prints the table): stone takes work, so a dug room feels earned, but
    /// the first hour stays open. The kit's pickaxe bites every pickaxe rock in at most half a second and soft
    /// soil at once; each better tool clears about twice the room a second of the tool before, or more; and within
    /// 25 m of a new holt's burrow nearly all the rock is the pickaxe's (tiers decide that, not tempo).
    #[test] fn dig_tempo_is_earned_but_the_first_hour_is_open() {
        use molehill_sim::mat::{Mat, Kind, NMAT, CPM, S};
        let mut w = World::new(64 * S, 64 * S, 1); let i = w.g.idx(w.g.w / 2, w.g.h / 2);
        let mut bite = |m: Mat, t: u8| { w.g.set(i, m); w.input.tool = t; w.mine_time(i) };
        let natural = (0..NMAT).map(|k| Mat::from_u8(k as u8)).filter(|m| matches!(m.kind(), Kind::Solid | Kind::Powder) && m.p().dig > 0 && m.p().flags & molehill_sim::mat::BUILT == 0);
        for m in natural { if m.p().tier <= 1 {
            let t = bite(m, 1).unwrap(); assert!(t <= 30, "{} takes {t} ticks a pickaxe bite", m.p().name);
            if m.p().dig <= 2 { assert!(t <= 6, "soft {} takes {t} ticks", m.p().name); } } }
        let rate = |t: u8, ticks: u32| BITES[t as usize] * BITES[t as usize] / ticks as f32;
        let (l, g) = (Mat::Limestone, Mat::Granite);
        let (c, p, d) = (rate(0, bite(l, 0).unwrap()), rate(1, bite(l, 1).unwrap()), rate(2, bite(l, 2).unwrap()));
        let (dg, bg) = (rate(2, bite(g, 2).unwrap()), rate(3, bite(g, 3).unwrap()));
        assert!(p >= 2. * c && d >= 2. * p && bg >= 1.8 * dg, "limestone claws {c:.2} pick {p:.2} drill {d:.2}; granite drill {dg:.2} bore {bg:.2}");
        assert!(bite(l, 1).unwrap() >= 24, "a pickaxe bite of limestone is real work");
        for seed in [1u32, 2, 3] {
            let w = World::new(512 * S, 256 * S, seed * 7919); let (mx, my) = (w.moles[0].x as i64, w.moles[0].y as i64); let r = 25 * CPM as i64;
            let (mut solid, mut cut) = (0u32, 0u32);
            for y in (my - r).max(0)..(my + r).min(w.g.h as i64) { for x in (mx - r).max(0)..(mx + r).min(w.g.w as i64) {
                if (x - mx).pow(2) + (y - my).pow(2) > r * r { continue; } let m = w.g.get(x as usize, y as usize);
                if matches!(m.kind(), Kind::Solid | Kind::Powder) && m != Mat::Bedrock { solid += 1; if m.p().tier <= 1 { cut += 1; } } } }
            let f = cut as f32 / solid as f32; println!("seed {seed}: {:.0}% of the rock within 25 m is the pickaxe's", f * 100.);
            assert!(f >= 0.95, "seed {seed}: only {:.0}% of the rock within 25 m of the burrow is the pickaxe's", f * 100.);
        }
    }
    /// The drill's whole bill from the ground (workbench, forge, a workshop with its copperline, and the drill;
    /// a coal per cold-forge job) is a first-hour haul: no more wood than the kit carries, since wood is scarce
    /// near the burrow, and only what the pickaxe digs. `examples/progression.rs` prints the full ladder.
    #[test] fn the_drill_costs_a_first_hour_haul() {
        use molehill_sim::{survival::KIT, mat::{Mat, NMAT}};
        let mut b = raw_bill(Item::Drill, 1);
        for s in STATIONS { let sb = bill_of_station(s); for k in 0..NITEM { b[k] += sb[k]; } }
        let kit_wood = KIT.iter().find(|k| k.0 == Item::Wood).unwrap().1;
        assert!(b[Item::Wood as usize] <= kit_wood, "the drill needs {} wood, the kit has {kit_wood}", b[Item::Wood as usize]);
        let total: u32 = b.iter().sum(); assert!(total <= 60, "the drill's bill is {total} items: {b:?}");
        for (k, &n) in b.iter().enumerate() { if n == 0 { continue; }
            let pick = (0..NMAT).any(|m| { let m = Mat::from_u8(m as u8); m.p().tier <= 1 && drop_of(m) == Some(ITEMS[k]) });
            assert!(pick, "{:?} in the drill's bill isn't dug with a pickaxe", ITEMS[k]); }
        assert_eq!((b[Item::CopperOre as usize], b[Item::TinOre as usize]), (8, 2), "copper and tin: {b:?}");
    }
    /// The oil press: at a workbench fungus gives more oil than the pack presses on the go, so bringing fungus
    /// home pays; and a mushroom farm's crop can go to the lamp instead of the belly (a choice, both fair:
    /// two mushrooms are 70 of food, about 3.5 minutes; their measure is 3 minutes of light).
    #[test] fn the_bench_presses_oil_better_than_the_pack() {
        use molehill_sim::survival::{FUNGUS_PER_OIL, OIL_SECS};
        let r = &RECIPES[recipe_by_name("Fungus oil x2").unwrap()];
        assert_eq!((r.station, r.out.0), (Station::Workbench, Item::FungusOil));
        let per = r.inputs[0].1 as f32 / r.out.1 as f32; assert!(r.inputs[0].0 == Item::Fungus && per < FUNGUS_PER_OIL as f32, "{per} fungus a measure at the bench");
        let m = &RECIPES[recipe_by_name("Fungus oil (mushroom)").unwrap()];
        assert_eq!((m.station, m.inputs[0].0, m.out.0), (Station::Workbench, Item::Mushroom, Item::FungusOil));
        let food_s = m.inputs[0].1 as f32 * food_value(Item::Mushroom) / 0.33; let light_s = m.out.1 as f32 * OIL_SECS;
        assert!(food_s > light_s * 0.8 && food_s < light_s * 1.6, "mushrooms: {food_s:.0} s of food or {light_s:.0} s of light");
        assert_eq!(raw_bill(Item::FungusOil, 2)[Item::Fungus as usize], 3, "the bill takes the bench's press");
    }
    /// Every metal made in Survival is wanted by something past it: bronze makes the drill, so no ingot is a dead end.
    #[test] fn bronze_makes_the_drill() {
        let drill = &RECIPES[recipe_by_name("Drill").unwrap()];
        assert!(drill.inputs.iter().any(|&(it, _)| it == Item::BronzeIngot), "the drill is bronze");
        assert!(drill.inputs.iter().all(|&(it, _)| !matches!(it, Item::IronIngot | Item::IronOre)), "and needs no iron");
        let ore = &RECIPES[recipe_by_name("Bronze x2").unwrap()];
        assert_eq!((ore.station, ore.inputs.iter().map(|&(it, _)| it).collect::<Vec<_>>()), (Station::Forge, vec![Item::CopperOre, Item::TinOre]), "co-smelted from ore");
        for m in [Item::CopperIngot, Item::TinIngot, Item::BronzeIngot, Item::IronIngot] {
            assert!(RECIPES.iter().any(|r| r.inputs.iter().any(|&(it, _)| it == m)), "{m:?} has no use");
        }
    }
    /// The Survival kit carries a short explore: five minutes with the headlamp lit, eating from the pack,
    /// and the log says each meal without ever running out. The kit's budget is checked too: about 12 minutes
    /// of food before hunger bites and 9 of headlamp oil, so a first trip down can come back for more.
    #[test] fn survival_kit_sustains_a_short_explore() {
        use molehill_sim::survival::{Mode, KIT, OIL_SECS};
        let food = |it: Item| match it { Item::Grub => 45., Item::Mushroom => 35., Item::Root => 30., Item::Fungus => 25., _ => 0. };
        let fed: f32 = KIT.iter().map(|&(it, n)| food(it) * n as f32).sum();
        let oil = KIT.iter().find(|k| k.0 == Item::FungusOil).map_or(0, |k| k.1) as f32;
        let (food_s, oil_s) = ((80. - 40. + fed) / 0.33, OIL_SECS * (1. + oil));
        assert!(food_s > 600. && oil_s > 480., "kit lasts {food_s:.0} s of food, {oil_s:.0} s of oil");
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Survival); w.gear.lamp = true;
        for _ in 0..5 * 60 * 60 { w.step(); }
        let said = |k: &str| w.log.iter().filter(|(_, l)| l.contains(k)).count();
        assert!(w.moles[0].alive && w.moles[0].hp > 90. && w.moles[0].hunger > 40., "hp {} hunger {}", w.moles[0].hp, w.moles[0].hunger);
        assert!(w.lamp_lit(), "the headlamp is still lit");
        assert!(said("Ate a grub") >= 1, "the meal is said: {:?}", w.log);
        assert_eq!((said("nothing to eat"), said("out of oil")), (0, 0), "{:?}", w.log);
    }
    /// A new Survival holt says what is in the pack and what to do first, no early quest is already done by
    /// the kit (so the quest line always points somewhere new), and finishing one names the next.
    #[test] fn survival_start_says_what_next() {
        use molehill_sim::{survival::{Mode, KIT}, craft::{QUESTS, Goal}};
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Survival);
        let said = |w: &World, k: &str| w.log.iter().any(|(_, l)| l.contains(k));
        assert!(said(&w, "Your pack: a pickaxe, wood x6") && said(&w, &format!("Next: {}.", QUESTS[0].text)), "{:?}", w.log);
        // the first hour's quests, in order: workbench, then wood, then copper ore, a forge and copperline
        let pos = |f: &dyn Fn(&Goal) -> bool| QUESTS.iter().position(|q| f(&q.goal)).unwrap();
        let bench = pos(&|g| matches!(g, Goal::Built(Station::Workbench))); let wood = pos(&|g| matches!(g, Goal::Have(Item::Wood, _)));
        let ore = pos(&|g| matches!(g, Goal::Have(Item::CopperOre, _))); let forge = pos(&|g| matches!(g, Goal::Built(Station::Forge)));
        let line = pos(&|g| matches!(g, Goal::Have(Item::CopperWire, _)));
        assert!(bench < wood && wood < ore && ore < forge && forge < line && line <= 6, "the first hour leads wood -> bench -> copper -> copperline");
        // none of them is done by the kit alone (the bench spends 4 of the kit's 6 wood before the wood quest)
        let mut inv = [0u32; NITEM]; for &(it, n) in KIT { inv[it as usize] = n; }
        for (k, q) in QUESTS.iter().enumerate().take(line + 1) {
            if k > bench { inv[Item::Wood as usize] = 2; }
            if let Goal::Have(it, n) = q.goal { assert!(inv[it as usize] < n, "'{}' is done on arrival", q.text); }
        }
        assert!(!w.quest_done(0) && w.quest == 0);
        // finishing one names the next
        w.max_depth = 6.; for _ in 0..31 { w.step(); }
        assert_eq!(w.quest, 1); assert!(said(&w, &format!("Next: {}.", QUESTS[1].text)), "{:?}", w.log);
    }
    /// The first hour isn't walled off by hunger or oil: around the burrow (several seeds) copper ore, fungus
    /// (food, and oil when pressed) and a root tangle (wood and roots) all lie well within what the kit's
    /// ~9 minutes of light and ~12 of food reach and come back from (see `examples/survey.rs`).
    #[test] fn first_hour_finds_are_near_spawn() {
        use molehill_sim::mat::{Mat, CPM, S};
        for seed in [1u32, 2, 3] {
            let w = World::new(512 * S, 256 * S, seed * 7919);
            let (mx, my) = (w.moles[0].x as i64, w.moles[0].y as i64);
            let near = |want: &dyn Fn(Mat) -> bool| { let mut d = f32::MAX;
                for y in 0..w.g.h { for x in 0..w.g.w { if want(w.g.get(x, y)) { d = d.min((((x as i64 - mx).pow(2) + (y as i64 - my).pow(2)) as f32).sqrt() / CPM as f32); } } } d };
            let (cu, fu, rt) = (near(&|m| m == Mat::CopperOre), near(&|m| m == Mat::Fungus), near(&|m| m == Mat::Roots));
            assert!(cu <= 15. && fu <= 20. && rt <= 30., "seed {seed}: copper {cu:.0} m, fungus {fu:.0} m, root tangle {rt:.0} m");
        }
    }
    /// A reason to come back with better gear: after the first hour the quests send you to find iron the
    /// pickaxe can't cut, then make the drill, then go back for it, and no earlier reward hands over the iron.
    /// The refusal and the discovery line name what to come back with.
    #[test] fn quests_send_you_back_with_a_drill() {
        use molehill_sim::{craft::{QUESTS, Goal}, mat::Mat, survival::Mode};
        let pos = |f: &dyn Fn(&Goal) -> bool| QUESTS.iter().position(|q| f(&q.goal)).unwrap();
        let find = pos(&|g| matches!(g, Goal::Found(Mat::IronOre))); let drill = pos(&|g| matches!(g, Goal::Have(Item::Drill, _)));
        let back = pos(&|g| matches!(g, Goal::Have(Item::IronOre, _)));
        assert!(find >= 7 && find < drill && drill < back, "find {find}, drill {drill}, go back {back}");
        assert!(QUESTS[..back].iter().all(|q| q.reward.iter().all(|&(it, _)| it != Item::IronOre)), "no reward gives the iron away");
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Survival);
        assert!(w.dig_refusal(Mat::Granite).unwrap().contains("come back with a drill (bronze, made at a powered workshop)"));
        assert!(w.dig_refusal(Mat::Basalt).unwrap().contains("come back with a bore"));
        assert!(w.come_back_with(Mat::Limestone).is_none() && w.come_back_with(Mat::Bedrock).is_none(), "nothing to come back with for soft rock or bedrock");
        w.inv[Item::Drill as usize] = 1; assert!(w.come_back_with(Mat::IronOre).is_none() && w.dig_refusal(Mat::Basalt).unwrap().contains("your drill: come back with a bore"));
    }
    /// The holt as home: right after the first hour's seven, a quest asks for furniture by the workbench
    /// (only pieces near the bench count, and none without one), pays timber wall and says how to panel with
    /// it. The wooden pieces are listed from the start, since the kit carries wood.
    #[test] fn holt_quest_asks_for_furniture_by_the_bench() {
        use molehill_sim::{craft::{QUESTS, Goal, HOME_M}, survival::Mode, world::{Furniture, Building}, mat::CPM};
        let k = QUESTS.iter().position(|q| matches!(q.goal, Goal::Furnished(_))).unwrap();
        assert!(k == 7 && matches!(QUESTS[k - 1].goal, Goal::Have(Item::CopperWire, _)), "right after the first hour's copperline");
        assert!(QUESTS[k].reward.iter().any(|&(it, _)| it == Item::TimberWall) && QUESTS[k].then.contains("Panel"), "pays wall and says what for");
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Survival); w.step();
        for n in ["Stool", "Table", "Nest", "Shelf", "Timber wall x8"] { assert!(w.recipe_known(recipe_by_name(n).unwrap()), "{n} listed from the kit"); }
        let (x, y) = (w.moles[0].x, w.moles[0].y);
        w.furniture.push(Furniture { item: Item::Stool, x, y, tint: 0 });
        assert_eq!(w.home_furniture(), 0, "no workbench: no holt yet");
        w.buildings.push(Building { kind: Station::Workbench, x, y, built: true, claimed: false });
        assert_eq!(w.home_furniture(), 1);
        let far = x + (HOME_M * CPM as f32) as usize + 4 * CPM; w.furniture.push(Furniture { item: Item::Table, x: far, y, tint: 0 });
        assert_eq!(w.home_furniture(), 1, "a table out past {HOME_M} m isn't the holt's");
    }
    /// Grottos visited are saved (v11), so a reload doesn't announce them again.
    #[test] fn grottos_seen_survive_a_save() {
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3);
        w.grottos_seen.insert(0xABCD); w.grottos_seen.insert(7);
        let l = save::load(&save::save(&w)).unwrap();
        assert!(l.grottos_seen.contains(&0xABCD) && l.grottos_seen.contains(&7) && l.grottos_seen.len() == 2);
    }
    /// A grotto above the burrow says "up", not "0 m down" (the polish playtest saw "0 m down" on 4 of 6 seeds).
    #[test] fn grotto_notice_says_up_or_down() {
        use molehill_sim::world::grotto_notice;
        assert!(grotto_notice(22.).starts_with("A grotto, 22 m down:"));
        assert!(grotto_notice(-6.).starts_with("A grotto, 6 m up:"));
        assert!(grotto_notice(0.2).starts_with("A grotto, level with the burrow:"));
    }
    /// The log reads in order: what killed the mole, then where its pack lies.
    #[test] fn death_line_comes_before_the_pack_line() {
        use molehill_sim::survival::Mode;
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Survival); w.step();
        w.moles[0].hunger = 0.; for f in molehill_sim::world::FOODS { w.inv[f as usize] = 0; } w.moles[0].hp = 0.5;
        for _ in 0..120 { w.step(); if !w.moles[0].alive { break; } }
        let at = |k: &str| w.log.iter().position(|(_, l)| l.contains(k)).unwrap_or_else(|| panic!("no '{k}' in {:?}", w.log));
        assert!(at("starved") < at("Your pack lies"), "{:?}", w.log);
    }
    #[test] fn survival_starts_bare_and_learns() {
        use molehill_sim::survival::Mode;
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Survival); w.step();
        assert!(w.rules.crafting && tool_tier(&w.inv) == 1, "the kit's pickaxe only");
        w.input.tool = 3; assert_eq!(w.dig_power(), 1, "a tool in hand you don't own doesn't dig");
        let pick = recipe_by_name("Pickaxe").unwrap(); let cu = recipe_by_name("Copper ingot").unwrap();
        assert!(w.recipe_known(pick), "the kit's wood shows the pickaxe (and the workbench's other wood recipes)");
        assert!(!w.recipe_known(cu), "copper ore isn't in the kit");
        assert!(w.craft_now(cu).is_err(), "an unknown recipe can't be made");
        w.inv[Item::CopperOre as usize] = 2; w.step(); assert!(w.known[Item::CopperOre as usize] && w.recipe_known(cu));
        assert!(w.log.iter().any(|(_, l)| l.contains("New recipes") && l.contains("Copper ingot")), "the log says what opened");
        w.inv[Item::Stone as usize] = 4; assert!(w.craft_now(pick).unwrap_err().contains("workbench"), "survival crafts at a station");
        // saved and loaded: mode and what you know
        let l = save::load(&save::save(&w)).unwrap(); assert_eq!(l.mode, Mode::Survival); assert_eq!(l.known, w.known);
        // Creative: free access, anywhere
        let mut c = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); c.set_mode(Mode::Creative);
        let d0 = c.inv[Item::Drill as usize]; assert!(c.craft_now(recipe_by_name("Drill").unwrap()).is_ok() && c.inv[Item::Drill as usize] == d0 + 1, "Creative crafts anywhere");
        c.input.tool = 3; assert_eq!(c.dig_power(), 3);
    }
}
/// A blast's knock carries through the rock like a thrum: a listener in solid granite hears it (on any
/// channel) for a moment.
#[test] fn blast_is_heard_by_listeners() {
    use molehill_sim::logic::Dev;
    let (mut w, x0, y0) = arena(); let c = molehill_sim::mat::CPM;
    // a listener sealed into the granite floor 6 m away; the charge goes off on the floor
    let (lx, ly) = (x0 + 6 * c, y0 + 2 * c);
    let b = w.logic.block_of(lx, ly); for y in (b / w.logic.bw) * c..(b / w.logic.bw + 1) * c { for x in (b % w.logic.bw) * c..(b % w.logic.bw + 1) * c { let i = w.g.idx(x, y); w.g.set(i, Mat::Empty); } }
    w.logic.place(&mut w.g, Dev::Rx, b); w.step(); assert_eq!(w.logic.dev_s[b], 0);
    assert!(w.logic.dev[b] == Dev::Rx as u8, "listener placed");
    w.blast(x0 - 4 * c, y0, molehill_sim::blast::TNT_KG); w.step(); w.step(); assert_eq!(w.logic.dev_s[b], 1, "the listener heard the blast");
    for _ in 0..40 { w.step(); } assert_eq!(w.logic.dev_s[b], 0, "for a moment");
}
/// The blast model's numbers: Kinney-Graham falls off with scaled distance, and is deterministic.
#[test] fn blast_overpressure_curve() {
    use molehill_sim::blast::overpressure;
    assert!(overpressure(1.) > 500. && overpressure(1.) < 1500., "Z = 1: about 1 MPa ({})", overpressure(1.));
    assert!(overpressure(5.) < 40. && overpressure(5.) > 15., "Z = 5: tens of kPa ({})", overpressure(5.));
    let mut last = f32::MAX; for k in 1..100 { let p = overpressure(k as f32 * 0.2); assert!(p < last); last = p; }
    let (mut a, x0, y0) = arena(); let (mut b, _, _) = arena(); let c = molehill_sim::mat::CPM;
    let ra = a.blast(x0 + 3 * c, y0 - c, 0.25); let rb = b.blast(x0 + 3 * c, y0 - c, 0.25);
    assert_eq!(a.g.mat, b.g.mat); assert_eq!((ra.cells, ra.shattered, ra.cracked), (rb.cells, rb.shattered, rb.cracked));
    assert!(ra.cells < molehill_sim::blast::BLAST_CELLS, "bounded");
}

/// Thrown ground flies deterministically and every cell lands again: a sand bed blasted in two identical
/// arenas ends identical, with the same amount of sand as it started with.
#[test] fn blast_debris_deterministic_and_conserved() {
    let c = molehill_sim::mat::CPM;
    let mk = || { let (mut w, x0, y0) = arena(); for y in y0 - c / 2..=y0 { for x in x0 + 2 * c..x0 + 6 * c { let i = w.g.idx(x, y); w.g.set(i, Mat::Sand); } } (w, x0, y0) };
    let (mut a, x0, y0) = mk(); let (mut b, _, _) = mk();
    let sand = |w: &World| w.g.mat.iter().filter(|&&m| m == Mat::Sand as u8).count() + w.debris_of(Mat::Sand);
    let n0 = sand(&a);
    a.blast(x0 + c, y0, 0.5); b.blast(x0 + c, y0, 0.5);
    assert!(a.debris_of(Mat::Sand) > 40, "sand flies ({} cells)", a.debris_of(Mat::Sand));
    for _ in 0..240 { a.step(); b.step(); }
    assert!(a.debris.is_empty(), "all landed"); assert_eq!(a.g.mat, b.g.mat, "deterministic"); assert_eq!(sand(&a), n0, "none lost");
}

/// Grottos may open up within sight of the starting holt, but never break into its burrow: every grotto's
/// box (shell included) stays clear of the burrow's footprint, and some now lie within view of the spawn.
/// The showcase seed 14085 (found by the `grottoseed` example) keeps its burrow whole with a root-tree on screen.
#[test]
fn grottos_near_spawn_never_touch_the_burrow() {
    use molehill_sim::gen::{Gen, GROTTO_SHELL};
    use molehill_sim::mat::{Mat, CPM};
    let (w, c) = (1024 * S, CPM as i64); let mut near = 0;
    for seed in 0..3000u32 {
        let gen = Gen::new(seed, w);
        for g in gen.grottos_in_rows(-40 * c, 40 * c) {
            let (sh, dx) = (GROTTO_SHELL as i64, g.cx as i64 - (w / 2) as i64);
            let (x0, x1, y0, y1) = (dx - g.rx as i64 - sh, dx + g.rx as i64 + sh, g.cy - g.ry as i64 - sh, g.cy + g.ry as i64 + sh);
            assert!(!(x1 > -6 * c && x0 < 6 * c && y1 > -5 * c && y0 < 3 * c), "seed {seed}: grotto at {dx},{} touches the burrow", g.cy);
            if x0.abs().min(x1.abs()) < 13 * c && y0 < 7 * c && y1 > -9 * c { near += 1; }
        }
    }
    assert!(near > 0, "no grotto ever opens within view of the spawn");
    let wd = World::new(1024 * S, 512 * S, 14085); let m = &wd.moles[0];
    assert!(wd.fits(m.x, m.y), "the burrow is whole");
    let tree = (0..26 * CPM).flat_map(|dx| (0..15 * CPM).map(move |dy| (dx, dy)))
        .any(|(dx, dy)| { let (x, y) = (m.x - 13 * CPM + dx, m.y - 9 * CPM + dy); wd.g.get(x, y) == Mat::RootTree });
    assert!(tree, "seed 14085 shows a root-tree on screen from the spawn");
}

/// A new holt never starts boxed in by ground the Survival Pickaxe can't dig: within `START_SAFE` (25 m) of
/// the burrow there's nothing of tier 2 or above (granite, basalt, iron, quartz, crystal), and from the burrow
/// a Delver with only a Pickaxe can always dig through to copper (an escape and the first tier up).
#[test]
fn spawn_zone_is_pickaxe_ground_with_a_way_to_copper() {
    use molehill_sim::gen::START_SAFE;
    use molehill_sim::mat::{Kind, Mat, CPM};
    for seed in 0..30u32 {
        let w = World::new(256 * S, 256 * S, seed); let (gw, gh) = (w.g.w, w.g.h);
        let (sx, sy0) = (gw / 2, (-w.oy) as usize); // the burrow: the window's middle column, depth 0
        let r = START_SAFE * CPM as f32;
        for y in 0..gh { for x in 0..gw {
            let (dx, dy) = (x as f32 - sx as f32, y as f32 - sy0 as f32); if dx * dx + dy * dy >= r * r { continue; }
            let m = w.g.get(x, y);
            assert!(m == Mat::Bedrock || m.p().tier <= 1, "seed {seed}: {m:?} at {:.1} m, {:.1} m from the burrow", dx / CPM as f32, dy / CPM as f32);
        } }
        // walk/dig out from the mole with a Pickaxe: open, liquid (not lava) or tier ≤ 1 ground
        let ok = |m: Mat| m != Mat::Lava && m != Mat::Bedrock && (m.passable() || matches!(m.kind(), Kind::Solid | Kind::Powder) && m.p().tier <= 1);
        let start = w.g.idx(w.moles[0].x, w.moles[0].y); let mut seen = vec![false; gw * gh]; seen[start] = true;
        let mut q = std::collections::VecDeque::from([start]); let mut copper = false;
        while let Some(i) = q.pop_front() {
            if w.g.m(i) == Mat::CopperOre { copper = true; break; }
            let (x, y) = (i % gw, i / gw);
            for (nx, ny) in [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)] {
                if nx >= gw || ny >= gh { continue; } let j = ny * gw + nx; if seen[j] || !ok(w.g.m(j)) { continue; } seen[j] = true; q.push_back(j);
            }
        }
        assert!(copper, "seed {seed}: a Pickaxe can't dig through to any copper");
    }
}

#[test] fn lid_clues_rise_with_height_and_stay_lore_safe() {
    use molehill_sim::lore::{clue_rate, ClueKind, FRAGMENTS, clues_in_band, next_fragment};
    // more common higher up, never zero-crossing, monotone
    let mut last = f32::MAX; for d in (-200..=200).step_by(10) { let r = clue_rate(d as f32); assert!(r <= last && r > 0.); last = r; }
    assert!(clue_rate(-90.) > 4. * clue_rate(0.) && clue_rate(90.) < 0.25 * clue_rate(0.));
    // enough fragments of every kind, each a short lore-safe paragraph; nothing confirms the Lid
    for k in [ClueKind::Carving, ClueKind::Note, ClueKind::Draft, ClueKind::Relic] { assert!(FRAGMENTS.iter().filter(|f| f.kind == k).count() >= 5, "{k:?}"); }
    let banned = ["sky", "sun", "surface", "magic", "spell", "radio", "electric", "signal", "wifi", "antenna", "laser", "is real", "proof", "proves", "we saw the lid", "reached the lid"];
    for f in FRAGMENTS { let t = format!("{} {}", f.title, f.text).to_lowercase(); for b in banned { assert!(!t.contains(b), "{}: '{b}'", f.title); } assert!(f.text.len() <= 160, "{}", f.title); }
    // every placed clue sits in open rock (not inside a wall), and drafts and odd finds only above the start
    let (w, _, _) = arena(); let mut n = 0;
    for band in -8..8i64 { for (kind, x, wy, _) in clues_in_band(&w.gen, w.gen.seed, band, w.g.w) { n += 1;
        assert!(w.gen.open(x, wy), "clue in rock at {x},{wy}"); if band >= 0 { assert!(!matches!(kind, ClueKind::Draft | ClueKind::Relic)); } } }
    assert!(n > 10);
    // the journal reads each fragment once, in order per kind
    let mut j = vec![]; while let Some(f) = next_fragment(ClueKind::Note, &j) { j.push(f); } assert_eq!(j.len(), FRAGMENTS.iter().filter(|f| f.kind == ClueKind::Note).count());
}

/// The Ancients' ruin carvings join the Lid journal: every carving has a journal entry that shows its own
/// text, none of them names or confirms the Lid, a carving two ruins share goes in once, and a save from
/// before the carvings joined (ruins visited, journal without them) gets them added on load.
#[test] fn ruin_carvings_join_the_journal() {
    use molehill_sim::lore::{entry, ruin_entry, RUIN_BASE};
    use molehill_sim::ruins::{fragment_of, FRAGMENTS};
    let banned = ["sky", "sun", "surface", "magic", "spell", "radio", "electric", "signal", "the lid", "is real", "proof", "proves"];
    for (k, f) in FRAGMENTS.iter().enumerate() {
        let id = ruin_entry(f).expect("every carving has an entry"); assert_eq!(id, RUIN_BASE + k as u16);
        let (label, title, text) = entry(id); assert_eq!(text, *f); assert_eq!(label, "Ancient carving"); assert!(!title.is_empty());
        let t = format!("{title} {text}").to_lowercase(); for b in banned { assert!(!t.contains(b), "{title}: '{b}'"); }
    }
    let (mut w, _, _) = arena(); w.journal = vec![0]; // one clue fragment read
    let a = 5u32 << 7; let b = a + ((FRAGMENTS.len() as u32) << 7); assert_eq!(fragment_of(a), fragment_of(b));
    w.ruins_seen.insert(a); w.ruins_seen.insert(b); w.ruins_seen.insert(a + (1 << 7));
    let w2 = molehill_sim::save::load(&molehill_sim::save::save(&w)).expect("load");
    assert_eq!(w2.journal[0], 0, "the clue fragment keeps its place");
    assert_eq!(w2.journal.len(), 3, "two different carvings, the shared one once: {:?}", w2.journal);
    let w3 = molehill_sim::save::load(&molehill_sim::save::save(&w2)).expect("load"); assert_eq!(w3.journal, w2.journal);
}

/// Ruins of the Ancients are rare and deep: none near the holt layers or the start, none in a grotto, a few
/// per kilometre of depth, all three kinds. A generated tube station keeps its waygate (iron frame round a
/// sealed gate) and its tube's bore open.
#[test]
fn ruins_are_rare_deep_and_clear_of_the_start_and_grottos() {
    use molehill_sim::gen::{Gen, GROTTO_SHELL};
    use molehill_sim::mat::{Mat, CPM};
    use molehill_sim::ruins::Kind;
    let (w, c) = (1024 * S, CPM as i64); let (mut n, mut kinds) = (0, [0; 3]);
    for seed in 0..300u32 { let gen = Gen::new(seed, w);
        for r in gen.ruins_in_rows(-200 * c, 1000 * c) { n += 1; kinds[r.kind as usize] += 1;
            assert!(r.y0 >= 5 * c, "seed {seed}: a ruin at {} m, up in the holt layers", r.y0 / c);
            let (rx0, rx1, ry0, ry1) = (r.x0 as i64, (r.x0 + r.w) as i64, r.y0, r.y0 + r.h as i64);
            assert!(!(rx1 > (w / 2) as i64 - 30 * c && rx0 < (w / 2) as i64 + 30 * c && ry0 < 30 * c), "seed {seed}: a ruin by the start");
            for g in gen.grottos_in_rows(ry0 - 30 * c, ry1 + 30 * c) { let sh = GROTTO_SHELL as i64;
                let (gx0, gx1, gy0, gy1) = (g.cx as i64 - g.rx as i64 - sh, g.cx as i64 + g.rx as i64 + sh, g.cy - g.ry as i64 - sh, g.cy + g.ry as i64 + sh);
                assert!(!(gx1 > rx0 && gx0 < rx1 && gy1 > ry0 && gy0 < ry1), "seed {seed}: a ruin inside a grotto"); }
            match r.kind { Kind::SealedVault => assert!(r.chest.is_some()), Kind::TubeStation => assert!(r.gate.is_some()), _ => {} }
        } }
    let per_km = n as f32 / 300. / 1.2;
    assert!((1.0..8.0).contains(&per_km), "{per_km} ruins per seed per km");
    assert!(kinds.iter().all(|&k| k > 20), "every kind turns up: {kinds:?}");
    // seed 66 has a tube station in its first window (about 45 m down): generated, its waygate and bore are there
    let wd = World::new(1024 * S, 512 * S, 66);
    let r = wd.ruins.iter().find(|r| r.kind == Kind::TubeStation).expect("seed 66: a tube station in the window");
    let (gx, gwy) = r.gate.unwrap(); let gy = (gwy - wd.oy) as usize;
    assert_eq!(wd.g.get(gx, gy), Mat::Door, "the waygate's sealed gate");
    assert_eq!(wd.g.get(gx, gy - 2 * CPM - 2), Mat::IronBeam, "its iron frame");
}

/// The relics do what they say: the Ancient bit digs a quarter faster; the Ground lens halves the oil.
#[test]
fn relics_bite_faster_and_save_oil() {
    use molehill_sim::craft::Item;
    use molehill_sim::survival::Mode;
    let (mut w, x, y) = arena(); w.input.tool = 1;
    let i = w.g.idx(x + 6, y); w.g.set(i, molehill_sim::mat::Mat::Limestone);
    let t0 = w.mine_time(i).unwrap(); w.inv[Item::AncientBit as usize] = 1; let t1 = w.mine_time(i).unwrap();
    assert!(t1 < t0 && t1 >= t0 * 3 / 4 - 1, "bit: {t0} -> {t1} ticks");
    let burn = |lens: bool| { let (mut w, _, _) = arena(); w.set_mode(Mode::Survival); w.gear.lamp = true; if lens { w.inv[Item::GroundLens as usize] = 1; }
        let o0 = w.lamp_oil; for _ in 0..600 { w.step(); } o0 - w.lamp_oil };
    let (plain, lensed) = (burn(false), burn(true));
    assert!((lensed - plain / 2.).abs() < 0.2, "oil burned in 10 s: {plain} plain, {lensed} with the lens");
}

// ---- background walls and the seven found-site families ----
mod enrichment {
    use super::{arena, Mat, S};
    use molehill_sim::craft::Item;
    use molehill_sim::gen::Gen;
    use molehill_sim::logic::Dev;
    use molehill_sim::mat::CPM;
    use molehill_sim::save;
    use molehill_sim::sites::{self, Kind, SITE_MIN_W};
    use molehill_sim::walls::Wall;
    use molehill_sim::world::World;

    #[test]
    fn back_walls_line_place_scrape_and_save() {
        let (mut w, x, y) = arena();
        // a dug granite floor leaves rock lining; placing never edits the foreground
        let i = w.g.idx(x + 4, y + 1);
        assert_eq!(w.g.m(i), Mat::Granite);
        w.set_wall(x + 4, y + 1, Wall::None);
        w.input.tool = 2; w.input.mine = Some((x + 4, y + 1));
        let mut t = 0; while w.g.mat[i] != 0 && t < 2000 { w.step(); t += 1; }
        w.input.mine = None;
        assert_eq!(w.g.m(i), Mat::Empty, "dig opened the cell");
        assert_eq!(w.walls.at(x + 4, y + 1), Wall::Rock, "the bite left a lining");
        w.inv[Item::TimberWall as usize] = 20;
        w.set_wall(x + 6, y - 2, Wall::None);
        let n = w.place_walls(x + 6, y - 2, Item::TimberWall, 0.5).expect("panel");
        assert!(n > 0);
        assert_eq!(w.walls.at(x + 6, y - 2), Wall::Timber);
        assert_eq!(w.g.get(x + 6, y - 2), Mat::Empty, "panelling never places a block");
        w.scrape_walls(x + 6, y - 2, 0.5).expect("scrape");
        assert_eq!(w.walls.at(x + 6, y - 2), Wall::None);
        assert_eq!(w.g.get(x + 6, y - 2), Mat::Empty, "scrape never digs the rock in front");
        w.set_wall(x + 8, y - 3, Wall::Carving);
        w.sites_seen.insert(0x51FE);
        let w2 = save::load(&save::save(&w)).expect("load");
        assert_eq!(w2.walls.at(x + 8, y - 3), Wall::Carving);
        assert!(w2.sites_seen.contains(&0x51FE));
        // an older holt with no wall bytes gets inferred lining
        let mut w3 = arena().0; w3.walls.cell.fill(0); w3.infer_back();
        assert!(w3.walls.count(Wall::Timber) + w3.walls.count(Wall::Rock) > 100, "infer_back lines open cave");
    }

    #[test]
    fn back_walls_are_not_in_the_ca_or_the_solver() {
        let (mut w, x, y) = arena();
        w.set_wall(x + 4, y - 2, Wall::Timber);
        w.set_wall(x + 4, y - 1, Wall::Timber);
        let above = w.g.idx(x + 4, y - 4); w.g.set(above, Mat::Sand);
        for _ in 0..80 { w.g.step_ca(); }
        assert_eq!(w.walls.at(x + 4, y - 2), Wall::Timber, "falling sand does not take the lining with it");
        assert_eq!(w.walls.at(x + 4, y - 1), Wall::Timber);
        // a 6 m dirt roof fails with or without lining: walls hold no roof
        let (g, f0) = super::slab(Mat::Dirt, 24, 4);
        assert!(!f0.is_empty(), "6 m dirt span fails");
        // lining lives off the grid, so the same roof still fails
        let mut st = molehill_sim::stress::Stress::new(g.w * g.h);
        let f1 = st.solve(&g);
        assert!(!f1.is_empty(), "lining is not a material; the solver never sees it");
        let _ = (x, y, g);
    }

    #[test]
    fn wall_items_are_place_only_and_lore_safe() {
        for it in [Item::RockWall, Item::TimberWall, Item::MasonryWall, Item::BeamWall, Item::PipeWall, Item::CarvedWall] {
            assert!(it.is_wall());
            assert!(it.uses().place && !it.uses().throw);
            let t = format!("{} {}", it.name(), it.desc()).to_lowercase();
            for b in ["sky", "sun", "surface", "magic", "lorawan", "wifi", "radio"] {
                assert!(!t.contains(b), "{it:?}: {b}");
            }
        }
        assert!(molehill_sim::craft::recipe_by_name("Rock wall x8").is_some());
        assert!(molehill_sim::craft::recipe_by_name("Masonry wall x8").is_some());
        assert!(molehill_sim::craft::recipe_by_name("Pipe wall x4").is_some());
    }

    #[test]
    fn sites_are_rare_deterministic_and_clear_of_start_ruins_and_each_other() {
        use molehill_sim::gen::{GROTTO_SHELL, START_SAFE};
        let w = 1024 * S; assert!(w >= SITE_MIN_W);
        let c = CPM as i64;
        let a = Gen::new(21, w); let b = Gen::new(21, w); let d = Gen::new(22, w);
        let rows = |g: &Gen| g.sites_in_rows(-40 * c, 120 * c);
        let sa = rows(&a); let sb = rows(&b); let sd = rows(&d);
        assert_eq!(sa.iter().map(|s| s.id).collect::<Vec<_>>(), sb.iter().map(|s| s.id).collect::<Vec<_>>());
        assert_ne!(sa.iter().map(|s| s.id).collect::<Vec<_>>(), sd.iter().map(|s| s.id).collect::<Vec<_>>());
        let (mut n, mut kinds, mut linked) = (0usize, [0usize; 7], 0usize);
        for seed in 0..180u32 {
            let gen = Gen::new(seed, w);
            let sites = rows(&gen);
            for (i, s) in sites.iter().enumerate() {
                n += 1; kinds[s.kind as usize] += 1; if s.link.is_some() { linked += 1; }
                let box_s = (s.x0 as i64, s.y0, s.x0 as i64 + s.w as i64, s.y0 + s.h as i64);
                let (cx, cy) = ((w / 2) as i64, 0i64);
                let (mx, my) = ((box_s.0 + box_s.2) / 2 - cx, (box_s.1 + box_s.3) / 2 - cy);
                assert!(mx * mx + my * my > ((START_SAFE as i64) * c).pow(2) || box_s.1.abs() > 30 * c,
                    "seed {seed}: a site in the start burrow");
                for t in &sites[i + 1..] {
                    assert!(!sites::boxes_overlap(box_s, (t.x0 as i64, t.y0, t.x0 as i64 + t.w as i64, t.y0 + t.h as i64)),
                        "seed {seed}: overlapping sites");
                }
                for r in gen.ruins_in_rows(s.y0 - 8 * c, s.y0 + s.h as i64 + 8 * c) {
                    assert!(!sites::boxes_overlap(box_s, (r.x0 as i64, r.y0, r.x0 as i64 + r.w as i64, r.y0 + r.h as i64)),
                        "seed {seed}: a site overlaps a ruin");
                }
                for g in gen.grottos_in_rows(s.y0 - 8 * c, s.y0 + s.h as i64 + 8 * c) {
                    let sh = GROTTO_SHELL as i64;
                    let gb = (g.cx as i64 - g.rx as i64 - sh, g.cy - g.ry as i64 - sh, g.cx as i64 + g.rx as i64 + sh, g.cy + g.ry as i64 + sh);
                    assert!(!sites::boxes_overlap(box_s, gb), "seed {seed}: a site overlaps a grotto");
                }
                let (cx, wy) = s.centre();
                if let Some(m) = s.cell(cx, wy) {
                    assert_eq!(gen.cell(cx, wy, w).0, m as u8, "site_cell is what gen.cell stamps");
                }
            }
        }
        assert!(n > 40, "too few sites over 180 seeds: {n}");
        assert!(kinds.iter().all(|&k| k > 0), "every family turns up: {kinds:?}");
        let mut near_pair = linked > 0;
        if !near_pair {
            use molehill_sim::ruins::Kind as RuinKind;
            for seed in 0..400u32 {
                let gen = Gen::new(seed, w);
                let stations: Vec<_> = gen.ruins_in_rows(-40 * c, 140 * c).into_iter().filter(|r| r.kind == RuinKind::TubeStation).collect();
                for s in rows(&gen).iter().filter(|s| s.kind == Kind::Halt) {
                    for r in &stations {
                        let (dx, dy) = ((s.centre().0 as i64 - (r.x0 + r.w / 2) as i64).abs(), (s.centre().1 - (r.y0 + r.h as i64 / 2)).abs());
                        if dx < 50 * c && dy < 40 * c {
                            near_pair = true;
                            assert_eq!(s.link, Some(r.id), "a halt next to a tube station faces it");
                        }
                    }
                }
            }
        }
        assert!(near_pair || sites::notice(Kind::Halt, true).contains("Ancients"),
            "a cargo halt faces a nearby Ancient tube station (or the linked notice is ready)");
        let banned = ["sky", "sun", "surface", "magic", "lorawan", "wifi", "radio", "the lid"];
        for k in 0..7 { let t = sites::notice(Kind::from_u8(k), false).to_lowercase();
            for b in banned { assert!(!t.contains(b), "{k}: {b}"); }
            for &(it, _) in &sites::site_loot(Kind::from_u8(k), 1) {
                assert!(!matches!(it, Item::AncientBit | Item::GroundLens), "relics stay in vaults");
            }
        }
    }

    fn stamp(kind: Kind) -> (World, usize, usize, usize, usize) {
        let (mut w, x, y) = arena();
        let (sw, sh) = kind.size();
        let x0 = (x + 8).min(w.g.w - sw - 8);
        let y0 = y.saturating_add(2).saturating_sub(sh).max(8);
        w.stamp_site(kind, x0, y0);
        (w, x0, y0, sw, sh)
    }

    #[test]
    fn all_seven_site_families_stamp_with_purpose() {
        use Mat::*;
        let has = |w: &World, x0: usize, y0: usize, sw: usize, sh: usize, m: Mat| {
            (y0..y0 + sh).any(|y| (x0..x0 + sw).any(|x| w.g.get(x, y) == m))
        };
        // abandoned mine: timber props, rail fragments, a lining
        let (w, x0, y0, sw, sh) = stamp(Kind::Mine);
        assert!(has(&w, x0, y0, sw, sh, WoodBeam) && has(&w, x0, y0, sw, sh, Rail));
        assert!(w.walls.count(Wall::Soot) + w.walls.count(Wall::Beams) > 20);
        // abandoned holt: a nest, a farm, timber lining
        let (w, x0, y0, sw, sh) = stamp(Kind::Holt);
        assert!(w.furniture.iter().any(|f| f.item == Item::Nest));
        assert!(!w.farms.is_empty());
        assert!(has(&w, x0, y0, sw, sh, Fungus), "holt nests and garden grow as cells, not empty air");
        // flooded pumpworks: a held sump, a closed sluice, a floodgate
        let (mut w, x0, y0, sw, sh) = stamp(Kind::Pumpworks);
        assert!(has(&w, x0, y0, sw, sh, Water) && has(&w, x0, y0, sw, sh, Door));
        assert!(has(&w, x0, y0, sw, sh, Planks), "dry machine hall has crates on the floor");
        assert!(w.logic.dev.iter().any(|&d| d == Dev::Floodgate as u8));
        let n0 = w.g.count(Water);
        for _ in 0..180 { w.step(); }
        assert!(w.g.count(Water) * 2 > n0, "the closed sluice holds the sump");
        // fissure: a drop and a rope
        let (w, x0, y0, sw, sh) = stamp(Kind::Fissure);
        assert!(has(&w, x0, y0, sw, sh, Empty) && !w.tr.ladders.is_empty());
        // fossil cave: mineralised bone
        let (w, x0, y0, sw, sh) = stamp(Kind::Fossil);
        assert!(has(&w, x0, y0, sw, sh, Carved));
        // salt / crystal hall: quartz or crystal, a shallow pool
        let (w, x0, y0, sw, sh) = stamp(Kind::Crystal);
        assert!(has(&w, x0, y0, sw, sh, Quartz) || has(&w, x0, y0, sw, sh, Crystal));
        assert!(has(&w, x0, y0, sw, sh, Water));
        // buried cargo halt: track and a wrecked cart, tubes, never a second waygate
        let (w, x0, y0, sw, sh) = stamp(Kind::Halt);
        assert!(has(&w, x0, y0, sw, sh, Rail) || has(&w, x0, y0, sw, sh, Tube));
        assert!(has(&w, x0, y0, sw, sh, Planks), "warehouse crates sit on the halt floor");
        assert!(!w.tr.carts.is_empty());
        assert!(!has(&w, x0, y0, sw, sh, Door), "halts do not grow a waygate");
    }

    #[test]
    fn site_chest_is_old_kit_and_a_visit_is_a_line_in_the_log() {
        let (mut w, x0, y0, sw, sh) = stamp(Kind::Mine);
        let id = w.sites.last().unwrap().id;
        if let Some((cx, cwy)) = w.sites.last().unwrap().chest {
            let cy = (cwy - w.oy) as usize;
            w.moles[0].x = cx; w.moles[0].y = cy;
            for _ in 0..40 { w.step(); }
            assert!(w.chests_opened >= 1 || w.inv[Item::Rail as usize] >= 6 || w.inv[Item::Coal as usize] >= 4,
                "the mine chest gives old kit, not relics");
            assert_eq!(w.inv[Item::AncientBit as usize], 0);
        }
        w.moles[0].x = x0 + sw / 2; w.moles[0].y = y0 + sh - 16;
        w.sites_seen.clear();
        for _ in 0..40 { w.step(); }
        assert!(w.sites_seen.contains(&id), "coming within 8 m reads the place");
        assert!(w.log.iter().any(|(_, s)| s.contains("old mine")));
        let _ = (x0, y0, sw, sh);
    }
}

/// First hour: every wide holt has root-trees (wood) within a fair dig of the burrow, without dumping wood
/// at it. A grotto with two or more trees lies within `NEAR_GROTTO.1` m of the start (the seed's own, or the
/// one `Gen` places in an empty grotto cell beside it); it's never inside the burrow (see
/// `grottos_near_spawn_never_touch_the_burrow`) and is the same grotto every time for a seed.
#[test]
fn a_grotto_with_trees_is_within_reach_of_every_new_holt() {
    use molehill_sim::gen::{Gen, NEAR_GROTTO};
    use molehill_sim::mat::CPM;
    let (w, c) = (1024 * S, CPM as f32);
    for seed in 0..400u32 {
        let gen = Gen::new(seed, w);
        let best = gen.grottos_in_rows(-40 * CPM as i64, 40 * CPM as i64).iter().filter(|g| g.ntrees >= 2)
            .map(|g| { let (dx, dy) = ((g.cx as f32 - (w / 2) as f32) / c, g.cy as f32 / c); (dx * dx + dy * dy).sqrt() })
            .fold(f32::MAX, f32::min);
        assert!(best <= NEAR_GROTTO.1, "seed {seed}: nearest grotto with trees is {best:.0} m away");
        assert_eq!(gen.near_grotto, Gen::new(seed, w).near_grotto, "seed {seed}: deterministic");
    }
    // a small test window keeps its old shape: no extra grotto
    assert!(Gen::new(7, 96 * S).near_grotto.is_none());
}

/// An abandoned mine reads as a mine at play zoom: its track is a thin strap on sleepers in grey ballast (not
/// a floor-wide band of brown), its props are slim posts, and the seam it followed is a worked face of ore at
/// the far end with a spill at its foot (copper, the shallow ore, three times in four).
#[test]
fn abandoned_mine_track_and_ore_face_read() {
    use molehill_sim::mat::{Mat, CPM};
    use molehill_sim::sites::{Kind, Site, MINE_RAIL};
    let s = Site::new(1, Kind::Mine, 0, 0, 0, false, None);
    let (w, h, c) = (s.w as i64, s.h as i64, CPM as i64);
    let at = |dx: i64, dy: i64| s.cell(dx as usize, dy).unwrap_or(Mat::Rock);
    let col = |dx: i64| (0..h).map(|dy| at(dx, dy)).collect::<Vec<_>>();
    // the track: rail on top of the floor, sleepers and ballast under it, rock beneath
    let (mut rail, mut sleeper, mut ballast) = (0, 0, 0);
    for dx in 0..w { for dy in h - c..h { match at(dx, dy) { Mat::Rail => rail += 1, Mat::WoodBeam => sleeper += 1, Mat::Gravel => ballast += 1, _ => {} } } }
    assert!(rail > 0 && rail <= MINE_RAIL * w, "the strap is at most {MINE_RAIL} cells thick ({rail} cells)");
    assert!(sleeper > 0 && ballast > sleeper, "sleepers in ballast ({sleeper} / {ballast})");
    assert!((0..w).all(|dx| (h - c + MINE_RAIL + 3..h).all(|dy| at(dx, dy) == Mat::Rock)), "rock under the ballast");
    // slim props: no column of timber wider than half a metre
    let mut run = 0; let mut widest = 0;
    for dx in 0..w { if col(dx)[(h / 2) as usize] == Mat::WoodBeam { run += 1; widest = widest.max(run); } else { run = 0; } }
    assert!(widest > 0 && widest <= c / 2, "posts {widest} cells wide");
    // the worked face: a big block of ore at the far end, and plenty of open drift in front of it
    let ore = s.mine_ore(); assert_eq!(ore, Mat::CopperOre);
    let face = (0..3 * c).flat_map(|dx| (0..h).map(move |dy| (dx, dy))).filter(|&(dx, dy)| at(dx, dy) == ore).count();
    assert!(face as i64 >= 8 * c * c, "the face shows at least 8 m² of ore ({face} cells)");
    let open = (0..w).flat_map(|dx| (0..h).map(move |dy| (dx, dy))).filter(|&(dx, dy)| at(dx, dy) == Mat::Empty).count();
    assert!(open as i64 * 2 > w * h, "the drift is mostly open ({open} of {})", w * h);
    let kinds = (0..64u32).filter(|&v| Site::new(1, Kind::Mine, 0, 0, v, false, None).mine_ore() == Mat::CopperOre).count();
    assert_eq!(kinds, 48, "copper three times in four");
}

/// Found places must read as different buildings: chunk-scale mines, pumpworks and halts, each
/// family's own lining, at least two ways in on the large ones, and no overlap with the 25 m spawn zone.
#[test]
fn found_places_differ_in_size_lining_and_stay_clear_of_the_spawn() {
    use molehill_sim::gen::{Gen, START_SAFE};
    use molehill_sim::mat::{Mat, CPM};
    use molehill_sim::sites::{Kind, Site};
    use molehill_sim::walls::Wall;
    let sizes: Vec<_> = (0..7).map(|k| Kind::from_u8(k).size()).collect();
    let large = sizes.iter().filter(|&&(w, h)| w >= 40 * CPM && h >= 18 * CPM).count();
    assert!(large >= 3, "at least three families are chunk-scale ({large} of {:?})", sizes);
    assert!(Kind::Mine.size().0 >= 48 * CPM && Kind::Halt.size().0 >= 64 * CPM && Kind::Pumpworks.size().0 >= 40 * CPM);

    let dominant = |kind: Kind, want: Wall| {
        let site = Site::new(1, kind, 0, 0, 0, false, None);
        let (mut n, mut tot) = (0usize, 0usize);
        for dy in 0..site.h as i64 {
            for dx in 0..site.w {
                if let Some(wl) = site.wall(site.x0 + dx, site.y0 + dy) {
                    tot += 1;
                    if wl == want { n += 1; }
                }
            }
        }
        assert!(n * 3 > tot, "{:?} lining should be mostly {:?} ({n}/{tot})", kind, want);
    };
    dominant(Kind::Holt, Wall::Timber);
    dominant(Kind::Mine, Wall::Soot);
    dominant(Kind::Pumpworks, Wall::WetStone);
    dominant(Kind::Fissure, Wall::Basalt);
    dominant(Kind::Halt, Wall::Ashlar);
    let fossil = Site::new(1, Kind::Fossil, 0, 0, 0, false, None);
    assert!((0..fossil.w).any(|dx| (0..fossil.h as i64).any(|dy| fossil.wall(dx, fossil.y0 + dy) == Some(Wall::Fossil))));
    let crystal = Site::new(1, Kind::Crystal, 0, 0, 0, false, None);
    assert!((0..crystal.w).any(|dx| (0..crystal.h as i64).any(|dy| matches!(crystal.wall(dx, crystal.y0 + dy), Some(Wall::Salt | Wall::Crystal)))));

    let openings = |kind: Kind| {
        let site = Site::new(1, kind, 0, 0, 0, true, None);
        let open = |m: Option<Mat>| matches!(m, Some(Mat::Empty) | Some(Mat::Tube) | Some(Mat::Water));
        let mut sides = [false; 4];
        for dy in 0..site.h as i64 {
            if open(site.cell(site.x0, site.y0 + dy)) { sides[0] = true; }
            if open(site.cell(site.x0 + site.w - 1, site.y0 + dy)) { sides[1] = true; }
        }
        for dx in 0..site.w {
            if open(site.cell(site.x0 + dx, site.y0)) { sides[2] = true; }
            if open(site.cell(site.x0 + dx, site.y0 + site.h as i64 - 1)) { sides[3] = true; }
        }
        sides.iter().filter(|&&b| b).count()
    };
    assert!(openings(Kind::Mine) >= 2, "mine shaft and adits");
    assert!(openings(Kind::Holt) >= 2, "holt door and bolt-hole");
    assert!(openings(Kind::Pumpworks) >= 2, "pump canal and service door");
    assert!(openings(Kind::Halt) >= 2, "halt tube and warehouse door");
    assert!(openings(Kind::Fissure) >= 2, "fissure mouths");
    assert!(openings(Kind::Fossil) >= 2, "fossil mouths");
    assert!(openings(Kind::Crystal) >= 2, "crystal mouths");

    let (ww, c) = (2048usize, CPM as i64);
    for seed in 0..80u32 {
        let gen = Gen::new(seed, ww);
        let safe = START_SAFE as i64 * c;
        let mid = (ww / 2) as i64;
        for site in gen.sites_in_rows(-120 * c, 200 * c) {
            let (x0, x1, y0, y1) = (site.x0 as i64, site.x0 as i64 + site.w as i64, site.y0, site.y0 + site.h as i64);
            assert!(!(x0 < mid + safe && x1 > mid - safe && y0 < safe && y1 > -safe),
                "seed {seed} {:?} overlaps the spawn zone", site.kind);
        }
    }
}

/// First hour, exploring: a new holt has a found place with a chest (an abandoned mine, by preference) whose
/// nearest cell is a real trek from the burrow, 35-90 m (`sites::NEAR_FIND`), inside the starting window.
/// The seed's own site when it placed one there, else one put in an empty site cell by the usual rules (clear
/// of the start, grottos and ruins: see `sites_are_rare_deterministic_and_clear_of_start_ruins_and_each_other`).
/// Guarded on at least 80 % of 300 seeds (and on the survey's seeds 1-50); deterministic per seed.
#[test]
fn a_rewarding_find_is_a_first_hour_trek_from_the_burrow() {
    use molehill_sim::sites::NEAR_FIND; use molehill_sim::gen::Gen;
    use molehill_sim::mat::CPM;
    let (w, c) = (1024 * S, CPM as i64);
    let ok = |seed: u32| { let gen = Gen::new(seed, w);
        gen.sites_in_rows(-(NEAR_FIND.2 as i64) * c, NEAR_FIND.2 as i64 * c).iter()
            .any(|s| s.chest.is_some() && (NEAR_FIND.0..=NEAR_FIND.1).contains(&gen.site_trek(s))) };
    let hits = (0..300u32).filter(|&s| ok(s)).count();
    assert!(hits * 10 >= 300 * 8, "a first-hour find on only {hits} of 300 seeds");
    let survey = (1..=50u32).filter(|&s| ok(s * 7919)).count();
    assert!(survey * 10 >= 50 * 8, "a first-hour find on only {survey} of the survey's 50 seeds");
    let (a, b) = (Gen::new(77, w), Gen::new(77, w));
    assert_eq!(a.near_site.map(|n| (n.0, n.1, n.2.id)), b.near_site.map(|n| (n.0, n.1, n.2.id)));
    assert!(Gen::new(77, 96 * S).near_site.is_none(), "small test windows keep their shape");
}

/// Mood light in found places: glowing rock (a crystal face in a salt hall) lights a room cold and even, a
/// soft glow that carries a few metres, never a lamp's blaze, while a lava pool's light stays hard and bright
/// (`light::SOFT_GLOW`, `SOFT_REACH_M`). No torches, the mole out of the way.
#[test]
fn crystal_glow_is_soft_mood_light_and_lava_stays_hard() {
    use molehill_sim::mat::CPM;
    let lum = |l: [f32; 3]| 0.3 * l[0] + 0.5 * l[1] + 0.2 * l[2];
    let room = |glow: Mat, floor: bool| {
        let (mut w, x0, y0) = arena(); let c = CPM;
        w.moles[0].alive = false;
        // a 6 m wide, 4 m high room left of the spawn; the glowing material is its left wall (or floor pool)
        let ry = y0 - 4 * c;
        let (ax, bx) = (x0 - 7 * c, x0 - c);
        for y in ry - c..=y0 { for x in ax - c..=bx + c { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); w.g.flags[i] = 0; } }
        for y in ry..y0 { for x in ax..bx { let i = w.g.idx(x, y); w.g.set(i, Mat::Empty); } }
        if floor { for y in y0 - c / 2..y0 { for x in ax..bx { let i = w.g.idx(x, y); w.g.set(i, glow); } } }
        else { for y in ry..y0 { for x in ax - c / 2..ax { let i = w.g.idx(x, y); w.g.set(i, glow); } } }
        w.compute_light((ax - 2 * c, ry - 2 * c, bx + 2 * c, y0 + 2 * c));
        let at = |dx: usize, up: usize| lum(w.light.at(w.g.idx(ax + dx, y0 - up)));
        let peak = (ry..y0).flat_map(|y| (ax..bx).map(move |x| (x, y))).filter(|&(x, y)| w.g.get(x, y) == Mat::Empty)
            .map(|(x, y)| lum(w.light.at(w.g.idx(x, y)))).fold(0f32, f32::max);
        (peak, at(c, 2 * c), at(4 * c, 2 * c))
    };
    let (peak, near, far) = room(Mat::Crystal, false);
    assert!(peak < 0.9, "a crystal face glows, it doesn't blaze (peak luma {peak:.2})");
    assert!(near > 0.1, "1 m from the crystal is lit ({near:.2})");
    assert!(far > 0.02 && far < near, "4 m out it still glows faintly ({far:.2})");
    let (lpeak, ..) = room(Mat::Lava, true);
    assert!(lpeak > peak * 1.3, "lava's light stays hard and bright ({lpeak:.2} vs crystal {peak:.2})");
}

/// Found places must not stamp a solid rectangle of building material into their bounding box:
/// host rock (None) sits above a holt dome, a pump roof and a halt warehouse, and outside a
/// fossil or crystal shell, so each silhouette reads as a carved place rather than a crate.
#[test]
fn found_places_have_open_silhouettes_not_filled_boxes() {
    use molehill_sim::mat::{Mat, CPM};
    use molehill_sim::sites::{Kind, Site};
    let c = CPM as i64;
    let holt = Site::new(1, Kind::Holt, 0, 0, 0, false, None);
    assert!(holt.cell(holt.w / 2, 0).is_none(), "host above the holt dome");
    assert!(matches!(holt.cell(holt.w / 2, 4 * c), Some(Mat::Limestone) | Some(Mat::Empty) | None));
    let pump = Site::new(1, Kind::Pumpworks, 0, 0, 0, false, None);
    assert!(pump.cell(pump.w / 2, 0).is_none(), "host above the pumpworks roof");
    let halt = Site::new(1, Kind::Halt, 0, 0, 0, false, None);
    assert!(halt.cell(8 * CPM, 0).is_none(), "host above the warehouse");
    let fossil = Site::new(1, Kind::Fossil, 0, 0, 0, false, None);
    assert!(fossil.cell(0, 0).is_none(), "fossil box corners stay host");
    let crystal = Site::new(1, Kind::Crystal, 0, 0, 0, false, None);
    assert!(crystal.cell(0, 0).is_none(), "crystal box corners stay host");
    let mine = Site::new(1, Kind::Mine, 0, 0, 0, false, None);
    assert!(mine.cell(8 * CPM, 0).is_none(), "host above the mine drift roof");
}

/// Every way into a found place reaches its chest (or the fissure floor) and the rest of its
/// rooms, with starting movement only: the mole's collision box flood-fills the stamped layout
/// (gravity off, as the mole climbs walls and rope ladders). Chests stand in open space so they
/// open. Walked across seeds, layout variants and both halt sides. Coordinate with the movement
/// agent's `site_*_traverse` scenarios: this test owns the positive layout check; those own the
/// climb feel.
#[test]
fn found_places_entrances_reach_every_room_and_the_chest() {
    use molehill_sim::mat::{Mat, CPM};
    use molehill_sim::sites::Kind;
    use molehill_sim::world::World;

    fn regions(w: &World, x0: usize, y0: usize, bw: usize, bh: usize) -> (Vec<u32>, Vec<usize>) {
        let fits = |x: usize, y: usize| !w.box_hits((x0 + x) as f32, (y0 + y) as f32);
        let mut reg = vec![0u32; bw * bh];
        let mut sizes = vec![0usize];
        for s in 0..bw * bh {
            if reg[s] != 0 || !fits(s % bw, s / bw) { continue; }
            let id = sizes.len() as u32;
            let mut n = 0;
            let mut st = vec![s];
            reg[s] = id;
            while let Some(i) = st.pop() {
                n += 1;
                let (x, y) = (i % bw, i / bw);
                for (nx, ny) in [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)] {
                    if nx < bw && ny < bh && reg[ny * bw + nx] == 0 && fits(nx, ny) {
                        reg[ny * bw + nx] = id;
                        st.push(ny * bw + nx);
                    }
                }
            }
            sizes.push(n);
        }
        (reg, sizes)
    }

    let kinds = [
        Kind::Holt, Kind::Mine, Kind::Pumpworks, Kind::Fissure,
        Kind::Fossil, Kind::Crystal, Kind::Halt,
    ];
    let mut bad = vec![];
    for seed in [1u32, 11, 4242] {
        for &kind in &kinds {
            let lefts: &[bool] = if kind == Kind::Halt { &[false, true] } else { &[false] };
            for &left in lefts {
                let var = seed & 7;
                let mut w = World::new(1024, 512, seed);
                w.disasters.enabled = false;
                let (sw, sh) = kind.size();
                let m = 2 * CPM;
                let (x0, y0) = (64, 512 - sh - 3 * m);
                for y in y0 - m..y0 + sh + m {
                    for x in x0 - m..x0 + sw + m {
                        let i = w.g.idx(x, y);
                        w.g.set(i, Mat::Granite);
                        w.g.flags[i] = 0;
                    }
                }
                w.tr.ladders.clear();
                w.chests.clear();
                w.stamp_site_var(kind, x0, y0, var, left);
                let name = kind.name();
                let tag = format!("{name} seed={seed} var={var} left={left}");
                let (bx0, by0, bw, bh) = (x0 - m / 2, y0 - m / 2, sw + m, sh + m);
                let (reg, sizes) = regions(&w, bx0, by0, bw, bh);
                let mut edge = vec![false; sizes.len()];
                let mut goal = vec![false; sizes.len()];
                let chest = w.chests.first().map(|c| (c.x, c.y));
                let buried = chest.map_or(false, |(cx, cy)| w.g.is_solidish(w.g.idx(cx, cy)));
                if buried {
                    bad.push(format!("{tag}: chest is inside a solid at {:?}", chest));
                }
                for (i, &r) in reg.iter().enumerate() {
                    if r == 0 { continue; }
                    let (gx, gy) = (bx0 + i % bw, by0 + i / bw);
                    if gx < x0 + CPM / 2 || gx + CPM / 2 >= x0 + sw || gy < y0 + CPM {
                        edge[r as usize] = true;
                    }
                    let at_goal = match chest {
                        Some((cx, cy)) => gx.abs_diff(cx) <= CPM && gy.abs_diff(cy) <= CPM,
                        None => gy + 2 * CPM >= y0 + sh,
                    };
                    if at_goal { goal[r as usize] = true; }
                }
                let where_goal = if kind == Kind::Fissure { "its floor" } else { "its chest" };
                let edge_ids: Vec<_> = (1..sizes.len()).filter(|&k| edge[k]).collect();
                if edge_ids.is_empty() {
                    bad.push(format!("{tag}: no way in (no entrance the body fits)"));
                    continue;
                }
                for &k in &edge_ids {
                    if !goal[k] {
                        bad.push(format!(
                            "{tag}: a way in (region {k}, {} body positions) does not reach {where_goal}",
                            sizes[k]
                        ));
                    }
                }
                let main = edge_ids.iter().copied().max_by_key(|&k| sizes[k]);
                let big = 4 * CPM * CPM;
                let cut: Vec<_> = (1..sizes.len())
                    .filter(|&k| Some(k) != main && sizes[k] >= big)
                    .map(|k| sizes[k])
                    .collect();
                if !cut.is_empty() {
                    bad.push(format!("{tag}: rooms cut off from the way in ({cut:?} body positions)"));
                }
                if matches!(kind, Kind::Crystal | Kind::Fossil) {
                    let (mx, my) = (x0 + sw - 2 * CPM, y0 + sh - 2 * CPM);
                    if !w.moles.is_empty() {
                        w.moles[0].x = mx;
                        w.moles[0].y = my;
                        let hp0 = w.moles[0].hp;
                        for _ in 0..180 { w.step(); }
                        if w.moles[0].hp + 5. < hp0 {
                            bad.push(format!(
                                "{tag}: east-mouth lintel buried the mole ({hp0:.0} -> {:.0} hp)",
                                w.moles[0].hp
                            ));
                        }
                        if let Some(c) = w.chests.first() {
                            if w.g.is_solidish(w.g.idx(c.x, c.y)) {
                                bad.push(format!("{tag}: chest was buried after 180 ticks"));
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}





/// A new holt sees its own burrow and a skin of its walls, not a 30 m box of rock: caves and ore beyond stay
/// dark until vision, digging or the headlamp finds them (the start used to reveal ±15 m straight through rock).
#[test]
fn spawn_reveals_only_burrow_neighbourhood() {
    use molehill_sim::grid::F_SEEN;
    use molehill_sim::mat::{Mat, CPM};
    use molehill_sim::world::SPAWN_SIGHT_M;
    for seed in [1u32, 7, 42, 4242, 90210] {
        let w = World::new(512 * S, 256 * S, seed); let (gw, gh) = (w.g.w, w.g.h);
        let (mx, my) = (w.moles[0].x, w.moles[0].y);
        let seen = |x: usize, y: usize| w.g.flags[y * gw + x] & F_SEEN != 0;
        // inside: where the mole stands, and the chamber's air a metre up on both sides
        assert!(seen(mx, my - 1) && seen(mx - 2 * CPM, my - CPM) && seen(mx + 2 * CPM, my - CPM), "seed {seed}: the burrow itself is dark");
        // nothing far out is seen, open or not: 10 m and more from the mole in any direction
        let far = (SPAWN_SIGHT_M + 4) * CPM; let mut n_seen = 0;
        for y in 0..gh { for x in 0..gw {
            if !seen(x, y) { continue; } n_seen += 1;
            let (dx, dy) = (x as i64 - mx as i64, y as i64 - my as i64);
            assert!(dx * dx + dy * dy < (far * far) as i64, "seed {seed}: {:?} seen {:.1} m, {:.1} m from the mole", w.g.get(x, y), dx as f32 / CPM as f32, dy as f32 / CPM as f32);
        } }
        // the old box was 30 × 23.5 m (~45 000 cells); the burrow and its skin are a small fraction of that
        assert!(n_seen < 12_000, "seed {seed}: {n_seen} cells seen at the start");
        // materials only count as discovered if they're in sight: no far ore in the journal of materials
        for m in [Mat::IronOre, Mat::Granite, Mat::Basalt, Mat::Lava] { assert!(!w.discovered[m as usize] || w.g.mat.iter().zip(&w.g.flags).any(|(&c, &f)| c == m as u8 && f & F_SEEN != 0), "seed {seed}: {m:?} discovered unseen"); }
    }
}

/// A new holt built in steps (the game's loading line) is the same world as `World::new`, its progress
/// only climbs, it passes through every stage, and no single step is a large share of the build.
#[test]
fn staged_new_holt_matches_and_climbs() {
    let (w, h, seed) = (512 * S, 256 * S, 4242);
    let whole = World::new(w, h, seed);
    let mut b = NewHolt::new(w, h, seed); let (mut last, mut steps) = (b.progress(), 0); let mut labels = vec![b.label()];
    while b.step().is_none() { steps += 1; let p = b.progress(); assert!(p >= last && p <= 1., "progress went {last} -> {p}"); last = p;
        if labels.last() != Some(&b.label()) { labels.push(b.label()); } assert!(steps < 5000, "build never finished"); }
    assert_eq!(b.progress(), 1.); assert!(steps >= 20, "only {steps} steps: the line would barely move");
    for l in ["Laying down the rock", "Hollowing the burrow", "Shoring up the caves", "Letting the water find its level"] { assert!(labels.contains(&l), "no '{l}' stage in {labels:?}"); }
    assert_eq!(molehill_sim::save::save(&b.take()), molehill_sim::save::save(&whole), "a staged build must give the same holt");
}
