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
