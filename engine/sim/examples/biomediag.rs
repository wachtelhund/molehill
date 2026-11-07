//! Region diagnostics: `--example biomediag -- [SEED]` prints, for the ground 27-50 m from a fresh holt's
//! burrow (below it), the material mix and the regions there; and a column of regions under the burrow.
use molehill_sim::{mat::{Kind, CPM, S}, world::World, gen::START_SAFE};
fn main() {
    let seed: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let w = World::new(256 * S, 256 * S, seed);
    let (sx, sy) = (w.g.w / 2, (-w.oy) as usize); let c = CPM as f32;
    let mut mats: std::collections::BTreeMap<String, u32> = Default::default(); let mut regs: std::collections::BTreeMap<String, u32> = Default::default();
    for y in sy..w.g.h { for x in 0..w.g.w {
        let d = (((x as f32 - sx as f32) / c).powi(2) + ((y as f32 - sy as f32) / c).powi(2)).sqrt();
        if d < START_SAFE + 2. || d > 50. { continue; }
        let m = w.g.get(x, y); if m.kind() != Kind::Solid { continue; }
        *mats.entry(format!("{m:?}")).or_default() += 1; *regs.entry(w.gen.biome_of(w.ox + x as i64, w.oy + y as i64).name().into()).or_default() += 1;
    } }
    println!("seed {seed}: column leaning {:?}", w.gen.biome_x(w.ox + sx as i64)); println!("mats {mats:?}"); println!("regions {regs:?}");
    for d in (-40..140).step_by(10) { print!("{d}:{} ", w.gen.biome_of(w.ox + sx as i64, d * CPM as i64).name()); } println!();
}
