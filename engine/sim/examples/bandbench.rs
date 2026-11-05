//! Cost of the band store on a game-sized window: a slide, an autosave (parts), a full save, a load.
use molehill_sim::{world::{World, SLIDE}, mat::S, save};
use std::time::Instant;
fn main() {
    let mut w = World::new(1024 * S, 512 * S, 7); w.disasters.enabled = false;
    let t = Instant::now(); for _ in 0..6 { w.shift(SLIDE as i64); } println!("6 slides down: {:.1} ms each, {} bands stored", t.elapsed().as_secs_f64() * 1e3 / 6., w.bands.len());
    let t = Instant::now(); let p = save::save_parts(&w); let el = t.elapsed();
    let win: usize = p.bands.iter().filter(|b| matches!(b.1, std::borrow::Cow::Owned(_))).map(|b| b.1.len()).sum();
    let far: usize = p.bands.iter().filter(|b| matches!(b.1, std::borrow::Cow::Borrowed(_))).map(|b| b.1.len()).sum();
    println!("save_parts: {:.1} ms; manifest {} B, window bands {} KB, far bands {} KB", el.as_secs_f64() * 1e3, p.manifest.len(), win / 1024, far / 1024);
    let t = Instant::now(); let b = save::save(&w); println!("save (one blob): {:.1} ms, {} KB", t.elapsed().as_secs_f64() * 1e3, b.len() / 1024);
    let t = Instant::now(); let l = save::load(&b).unwrap(); println!("load: {:.1} ms (oy {})", t.elapsed().as_secs_f64() * 1e3, l.oy);
}
