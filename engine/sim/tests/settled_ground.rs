//! The settled ground round the burrow (`gen::SETTLED`): a new holt's first tunnels don't run into a steady
//! rain of loose ground, while the loose lenses stay further out and deeper, as the hazard they are.
//! Numbers from `--example loosesurvey` are in REFINE-worldgen.md.
use molehill_sim::gen::{Gen, SETTLED};
use molehill_sim::grid::F_WET;
use molehill_sim::mat::{Kind, Mat, CPM, S};
use molehill_sim::world::World;

/// Share of the solid and powder ground the generator makes sand or gravel, and soaked sand or gravel, over cells
/// `keep(dx_m, depth_m)`, straight from the generator (before the window's fill pass).
fn lens_share(seeds: std::ops::Range<u32>, keep: &dyn Fn(f32, f32) -> bool) -> (f32, f32) {
    let w = 1024 * S; let c = CPM as f32; let (mut n, mut loose, mut wet) = (0u32, 0u32, 0u32);
    for seed in seeds { let g = Gen::new(seed, w);
        for wy in (-60 * CPM as i64..60 * CPM as i64).step_by(3) { for x in (8 * S..w - 8 * S).step_by(3) {
            if !keep((x as f32 - (w / 2) as f32) / c, wy as f32 / c) { continue; }
            let (m, f) = g.cell(x, wy, w); let m = Mat::from_u8(m);
            if !matches!(m.kind(), Kind::Solid | Kind::Powder) || m == Mat::Bedrock { continue; }
            n += 1; if matches!(m, Mat::Sand | Mat::Gravel) { loose += 1; if f & F_WET != 0 { wet += 1; } } // (a grotto's moss and roots are wet by design)
        } } }
    (loose as f32 / n.max(1) as f32, wet as f32 / n.max(1) as f32)
}

/// Round the burrow (within SETTLED.0 m) lenses are rare and never soaked; well beyond SETTLED.1 m, at the
/// same depths, they are as common as ever. Over 24 seeds.
#[test]
fn loose_lenses_are_rare_round_the_burrow_but_not_beyond() {
    let near = lens_share(0..24, &|dx, d| (dx * dx + d * d).sqrt() < SETTLED.0);
    let far = lens_share(0..24, &|dx, d| dx.abs() > SETTLED.1 + 10. && d.abs() < SETTLED.0);
    assert!(near.0 < 0.012, "loose lenses round the burrow: {:.2} %", near.0 * 100.);
    assert_eq!(near.1, 0., "no soaked lens round the burrow");
    assert!(far.0 > 0.03, "lenses further out stay: {:.2} %", far.0 * 100.);
    assert!(far.0 > 4. * near.0, "far {:.2} % vs near {:.2} %", far.0 * 100., near.0 * 100.);
    // and deeper down, under the start zone's reach, the hazard pockets are still there
    let deep = lens_share(0..24, &|dx, d| dx.abs() > SETTLED.1 && d > 15. && d < 35.);
    assert!(deep.0 > 0.02, "deeper lenses stay: {:.2} %", deep.0 * 100.);
}

/// In a fresh holt (the game's window, after the fill pass and stabilisation) under 2 % of the ground within
/// 30 m of the burrow is loose (sand, gravel, fungus or knocked loose), and no natural cave wall there is sand.
/// Before the settled ground this was 7.5 % (mean of 16 seeds, `loosesurvey`).
#[test]
fn a_new_holt_starts_on_settled_ground() {
    use molehill_sim::grid::F_LOOSE;
    for seed in [0u32, 3, 9, 11, 12] {
        let w = World::new(1024 * S, 512 * S, seed); let c = CPM as f32;
        let (mx, my) = (w.moles[0].x as f32, w.moles[0].y as f32);
        let (mut n, mut loose, mut wall) = (0u32, 0u32, 0u32);
        for y in 1..w.g.h - 1 { for x in 1..w.g.w - 1 {
            if ((x as f32 - mx).powi(2) + (y as f32 - my).powi(2)).sqrt() / c > 30. { continue; }
            let i = w.g.idx(x, y); let m = w.g.m(i); if !matches!(m.kind(), Kind::Solid | Kind::Powder) || m == Mat::Bedrock { continue; }
            n += 1; if matches!(m, Mat::Sand | Mat::Gravel | Mat::Fungus) || w.g.flags[i] & F_LOOSE != 0 { loose += 1; }
            if m == Mat::Sand && [i - 1, i + 1, i - w.g.w, i + w.g.w].iter().any(|&j| w.g.m(j).passable()) { wall += 1; }
        } }
        let f = loose as f32 / n as f32;
        assert!(f < 0.02, "seed {seed}: {:.2} % of the ground round the burrow is loose", f * 100.);
        assert!(wall < 8 * (S * S) as u32, "seed {seed}: {wall} sand cells face open cave round the burrow");
    }
}
