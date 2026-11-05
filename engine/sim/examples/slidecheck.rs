//! After generation and slides, how many cells would fail a full stress solve, and where (by depth band)?
use molehill_sim::{mat::{S, CPM}, stress::Stress, world::{World, SLIDE}};
fn report(w: &World, tag: &str) {
    let mut st = Stress::new(w.g.w * w.g.h); let f = st.solve(&w.g);
    let mut bands = std::collections::BTreeMap::new(); for &i in &f { let y = i as usize / w.g.w; *bands.entry(((w.oy + y as i64) as f32 / CPM as f32 / 8.).floor() as i64 * 8).or_insert(0) += 1; }
    let mut mats = std::collections::BTreeMap::new(); for &i in &f { *mats.entry(format!("{:?}", w.g.m(i as usize))).or_insert(0) += 1; }
    println!("{tag}: {} failing; by depth (m): {:?}; by material: {:?}", f.len(), bands, mats);
}
fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4242);
    let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
    report(&w, "after World::new");
    w.shift(SLIDE as i64); report(&w, "after a slide down (fresh rows)");
    w.shift(-(SLIDE as i64)); report(&w, "after sliding back");
}
