//! Underground regions (`gen::Biome`, `Gen::biome_of`): where each lies, what ground and growth it has, and its
//! back walls. Mostly sampled straight from the generator (no stabilisation, no window), several seeds, over
//! world columns well east and west of the home columns (the Deepholt has no sides); the last test checks the
//! regions' linings in tiles the sliding window brings in.
use molehill_sim::gen::{Biome, Gen, NBIOME};
use molehill_sim::mat::{Kind, Mat, CPM, S};
use molehill_sim::walls::{natural_at, Wall};

/// The generator's home width (the game window) and the world columns sampled: four home widths, from one
/// west of home to two east of it.
const W: usize = 1024 * S;
const X0: i64 = -(W as i64); const X1: i64 = 3 * W as i64;
const SEEDS: [u32; 4] = [3, 77, 4242, 9001];

/// Per 10 m depth band, how many samples fell in each region.
fn region_tally(d0: i64, d1: i64) -> std::collections::BTreeMap<i64, [u32; NBIOME]> {
    let mut t: std::collections::BTreeMap<i64, [u32; NBIOME]> = Default::default();
    for seed in SEEDS { let g = Gen::new(seed, W);
        for d in (d0..d1).step_by(2) { for x in (X0..X1).step_by(5 * CPM) {
            t.entry(d.div_euclid(10)).or_default()[g.biome_of(x, d * CPM as i64) as usize] += 1;
        } } }
    t
}

/// Regions follow the layers: loam and karst up where it is wet, fungal hollows only in the middle layers,
/// the ashlands only deep down and all of the very deep. Every region turns up; each is deterministic.
#[test]
fn regions_follow_the_layers() {
    let t = region_tally(-80, 160);
    let sum = |b0: i64, b1: i64, r: Biome| (b0..=b1).map(|b| t.get(&b).map_or(0, |v| v[r as usize])).sum::<u32>();
    let all = |b0: i64, b1: i64| (b0..=b1).map(|b| t.get(&b).map_or(0, |v| v.iter().sum::<u32>())).sum::<u32>();
    for r in [Biome::Loam, Biome::Karst, Biome::Crags, Biome::Ashlands, Biome::Fungal] {
        assert!(sum(-8, 15, r) > 0, "{} never turns up", r.name());
    }
    assert_eq!(sum(-8, 2, Biome::Ashlands), 0, "no ashlands at or above 30 m");
    assert_eq!(sum(-8, -1, Biome::Fungal) + sum(9, 15, Biome::Fungal), 0, "fungal hollows only in the middle layers");
    let deep = sum(11, 15, Biome::Ashlands) as f32 / all(11, 15) as f32; assert!(deep > 0.95, "the deep below 110 m is ashlands: {deep:.2}");
    let up = (sum(-8, -3, Biome::Loam) + sum(-8, -3, Biome::Karst)) as f32 / all(-8, -3) as f32; assert!(up > 0.6, "loam and karst rule the wet upper layers: {up:.2}");
    let f = sum(2, 5, Biome::Fungal) as f32 / all(2, 5) as f32; assert!((0.04..0.35).contains(&f), "fungal hollows are pockets, not a layer: {f:.2} of 20-60 m");
    let (a, b) = (Gen::new(5, W), Gen::new(5, W));
    for x in (X0..X1).step_by(97) { for wy in (-400..1200i64).step_by(37) { assert_eq!(a.biome_of(x, wy), b.biome_of(x, wy)); } }
}

/// Material tallies by region over depths [d0, d1) m (solid and powder ground; fluids separately).
struct Tally { n: std::collections::HashMap<(Biome, Mat), u32>, tot: std::collections::HashMap<Biome, u32>, fluid: std::collections::HashMap<(Biome, Mat), u32> }
impl Tally {
    fn new(d0: i64, d1: i64) -> Tally {
        let mut t = Tally { n: Default::default(), tot: Default::default(), fluid: Default::default() };
        for seed in SEEDS { let g = Gen::new(seed, W);
            for d in d0..d1 { let wy = d * CPM as i64; for x in (X0..X1).step_by(7) {
                let b = g.biome_of(x, wy); let m = Mat::from_u8(g.cell(x, wy).0);
                if m.kind() == Kind::Liquid { *t.fluid.entry((b, m)).or_default() += 1; }
                if m == Mat::Empty || m.kind() == Kind::Liquid { continue; }
                *t.n.entry((b, m)).or_default() += 1; *t.tot.entry(b).or_default() += 1;
            } } }
        t
    }
    fn frac(&self, b: Biome, ms: &[Mat]) -> f32 { ms.iter().map(|&m| self.n.get(&(b, m)).copied().unwrap_or(0)).sum::<u32>() as f32 / self.tot.get(&b).copied().unwrap_or(1).max(1) as f32 }
    fn wet(&self, b: Biome, m: Mat) -> f32 { self.fluid.get(&(b, m)).copied().unwrap_or(0) as f32 / self.tot.get(&b).copied().unwrap_or(1).max(1) as f32 }
    /// `b` has more of `ms` than every other region in `among` with enough samples to count.
    fn leads(&self, b: Biome, among: &[Biome], ms: &[Mat], what: &str) {
        let f = self.frac(b, ms);
        for &o in among { if o == b || self.tot.get(&o).copied().unwrap_or(0) < 2000 { continue; }
            let fo = self.frac(o, ms); assert!(f > fo, "{} should have the most {what}: {f:.3} vs {} {fo:.3}", b.name(), o.name()); }
    }
}

/// Each region has its own ground, compared at the same depths. Up high (-30 to 10 m): the loam is the most
/// soil, the karst the most limestone, the crags the most loose gravel. In the middle layers (15-55 m): the
/// crags are the most granite, the fungal hollows the most fungus, moss and mycelium, the karst still the most
/// limestone and the wettest. Deep down (90-140 m): the ashlands are basalt, their pockets lava, nothing grows.
#[test]
fn each_region_has_its_own_ground() {
    use Biome::*;
    let top = Tally::new(-30, 10); let up = [Loam, Karst, Crags];
    top.leads(Loam, &up, &[Mat::Dirt, Mat::Clay, Mat::Roots], "soil");
    top.leads(Karst, &up, &[Mat::Limestone], "limestone");
    top.leads(Crags, &up, &[Mat::Gravel], "gravel");
    let mid = Tally::new(15, 55); let midr = [Loam, Karst, Crags, Fungal];
    mid.leads(Crags, &midr, &[Mat::Granite], "granite");
    mid.leads(Fungal, &midr, &[Mat::Fungus, Mat::Moss, Mat::Mycelium], "growth");
    mid.leads(Karst, &midr, &[Mat::Limestone], "limestone");
    assert!(mid.frac(Fungal, &[Mat::Fungus]) > 0.01, "fungus banks in the fungal hollows: {:.3}", mid.frac(Fungal, &[Mat::Fungus]));
    let (kw, cw) = (top.wet(Karst, Mat::Water) + mid.wet(Karst, Mat::Water), top.wet(Crags, Mat::Water) + mid.wet(Crags, Mat::Water));
    assert!(kw > 1.5 * cw, "the karst floods, the crags are dry: {kw:.4} vs {cw:.4}");
    let deep = Tally::new(90, 140);
    assert!(deep.frac(Ashlands, &[Mat::Basalt]) > 0.5, "ashlands are basalt: {:.2}", deep.frac(Ashlands, &[Mat::Basalt]));
    assert_eq!(deep.frac(Ashlands, &[Mat::Fungus, Mat::Moss, Mat::Roots, Mat::Mycelium]), 0., "nothing grows in the ashlands");
    let (lava, water) = (deep.wet(Ashlands, Mat::Lava), deep.wet(Ashlands, Mat::Water));
    assert!(lava > 3. * water, "the ashlands' pockets are lava: {lava:.4} lava, {water:.4} water");
}

/// A region's caves are lined in its own way: fungal threads only in the fungal hollows, split basalt in the
/// ashlands, salt crust and seep in the karst, roots in the loam, cracks in the crags.
#[test]
fn each_region_lines_its_caves() {
    use std::collections::HashMap;
    let mut n: HashMap<(Biome, Wall), u32> = HashMap::new(); let mut tot: HashMap<Biome, u32> = HashMap::new();
    for seed in SEEDS { let g = Gen::new(seed, W);
        for d in (-40..140i64).step_by(1) { let wy = d * CPM as i64; for x in (X0..X1).step_by(11) {
            let b = g.biome_of(x, wy); *n.entry((b, natural_at(&g, x, wy))).or_default() += 1; *tot.entry(b).or_default() += 1;
        } } }
    let f = |b: Biome, wl: Wall| n.get(&(b, wl)).copied().unwrap_or(0) as f32 / tot[&b] as f32;
    for b in [Biome::Loam, Biome::Karst, Biome::Crags, Biome::Ashlands] { assert_eq!(f(b, Wall::Mycel), 0., "fungal threads in {}", b.name()); }
    assert!(f(Biome::Fungal, Wall::Mycel) > 0.3, "fungal hollows are threaded: {:.2}", f(Biome::Fungal, Wall::Mycel));
    assert!(f(Biome::Ashlands, Wall::Basalt) > 0.3, "ashlands are split basalt: {:.2}", f(Biome::Ashlands, Wall::Basalt));
    let top = |wl: Wall| [Biome::Loam, Biome::Karst, Biome::Crags, Biome::Ashlands, Biome::Fungal].into_iter().max_by(|&a, &b| f(a, wl).total_cmp(&f(b, wl))).unwrap();
    assert_eq!(top(Wall::Root), Biome::Loam); assert_eq!(top(Wall::Salt), Biome::Karst); assert_eq!(top(Wall::Crack), Biome::Crags); assert_eq!(top(Wall::Damp), Biome::Karst);
}

/// The regions keep the start rules: whatever region the burrow lies in, the ground under it just past the
/// start zone (35-60 m down, within 25 m to the sides) holds granite or harder, and the ore bands stay put
/// (see `ore_sits_in_its_depth_bands` and `spawn_zone_is_pickaxe_ground_with_a_way_to_copper` in sim.rs).
#[test]
fn hard_ground_lies_just_past_the_start_zone() {
    let w = 1024 * S; let c = CPM as i64;
    for seed in 0..40u32 { let g = Gen::new(seed, w);
        let (mut hard, mut all) = (0u32, 0u32);
        for wy in (35 * c..60 * c).step_by(3) { for x in ((w / 2 - 25 * CPM) as i64..(w / 2 + 25 * CPM) as i64).step_by(3) {
            let m = Mat::from_u8(g.cell(x, wy).0); if m.kind() != Kind::Solid { continue; }
            all += 1; if m.p().tier >= 2 { hard += 1; }
        } }
        assert!(hard as f32 > all as f32 * 0.15, "seed {seed}: only {hard} of {all} solid cells 35-60 m under the burrow need more than a pickaxe");
    }
}

/// Under the sliding window: tiles that slide in (sideways, west of home, and down) are lined by the regions of
/// their own world cells, exactly as the generator would line them, and the fungal threads appear only where
/// the region is the fungal hollows.
#[test]
fn regions_line_the_tiles_the_window_brings_in() {
    use molehill_sim::{band::TW, world::{World, SLIDE}};
    let mut w = World::new(1024 * S, 512 * S, 4242); w.disasters.enabled = false;
    for _ in 0..3 { w.shift_x(-(TW as i64)); } w.shift(SLIDE as i64); w.shift(SLIDE as i64);
    assert!(w.ox < 0, "west of the home columns");
    let (gw, gh) = (w.g.w, w.g.h); let (mut lined, mut mycel, mut checked) = (0u32, 0u32, 0u32);
    // the columns that came in sideways (the west 96 m) and the rows that came in from below (the bottom 64 m)
    for y in 0..gh { for x in 0..gw { if x >= 3 * TW && y < gh - 2 * SLIDE { continue; }
        let (wx, wy) = (w.ox + x as i64, w.oy + y as i64); let i = y * gw + x; let wl = w.walls.get(i);
        if wl == Wall::None || w.gen.site_wall(wx, wy).is_some() || w.gen.grotto_of(wx, wy).is_some_and(|g| g.is_void(wx, wy)) || w.gen.in_ruin_void(wx, wy) { continue; }
        lined += 1; if wl == Wall::Mycel { mycel += 1; assert_eq!(w.gen.biome_of(wx, wy), Biome::Fungal, "fungal threads outside the hollows at {wx},{wy}"); }
        if w.desig[i] == 0 { checked += 1; assert_eq!(wl, natural_at(&w.gen, wx, wy), "lining at {wx},{wy} is the region's"); }
    } }
    assert!(lined > 1000 && checked > 1000, "open cave was lined: {lined} ({checked} checked)");
    println!("lined {lined}, of them fungal threads {mycel}");
}

/// A region's sideways edge wanders as it goes down: of the cells where the region (loam, karst, crags) changes
/// from one column to the next, few have the same change straight below them 2 m down (a straight seam has all).
#[test]
fn sideways_region_edges_wander_with_depth() {
    let (mut edges, mut straight) = (0u32, 0u32); let dy = 2 * CPM as i64;
    for seed in SEEDS { let g = Gen::new(seed, W);
        for d in (-30..70i64).step_by(1) { let wy = d * CPM as i64; for x in X0..X1 {
            let (a, b) = (g.biome_of(x, wy), g.biome_of(x + 1, wy)); if a == b { continue; }
            if [a, b].iter().any(|r| matches!(r, Biome::Fungal | Biome::Ashlands)) { continue; } // (their edges are depth edges, wandering already)
            edges += 1; if g.biome_of(x, wy + dy) == a && g.biome_of(x + 1, wy + dy) == b { straight += 1; }
        } } }
    let share = straight as f32 / edges.max(1) as f32;
    println!("sideways region edges: {edges}, straight 2 m down: {share:.2}");
    assert!(edges > 100 && share < 0.5, "sideways region edges run straight down: {share:.2} of {edges}");
}

/// Natural linings come in irregular patches, not a 1 m grid: where the lining changes from one cell to the
/// next along a row, that change falls on a 1 m block edge about as often as on any other column.
#[test]
fn linings_have_no_block_grid() {
    let (mut changes, mut on_grid) = (0u32, 0u32); let c = CPM as i64;
    for seed in SEEDS { let g = Gen::new(seed, W);
        for d in (-40..140i64).step_by(3) { let wy = d * CPM as i64 + 3; let mut prev = natural_at(&g, X0, wy);
            for x in X0 + 1..X0 + 4000 { let wl = natural_at(&g, x, wy); if wl != prev { changes += 1; if x.rem_euclid(c) == 0 { on_grid += 1; } } prev = wl; } } }
    let share = on_grid as f32 / changes.max(1) as f32;
    println!("lining changes: {changes}, on 1 m block edges: {share:.2} (no grid: {:.2})", 1. / CPM as f32);
    assert!(changes > 1000 && share < 1.6 / CPM as f32, "linings change on the 1 m grid: {share:.2} of {changes}");
}
