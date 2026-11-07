//! Where world generation spends its time (game-sized window). Usage: genbench [seed]
use molehill_sim::{gen, grid::Grid, mat::S, stress::Stress, world::World};
fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4242);
    let (w, h) = (1024 * S, 512 * S); let ms = |t: std::time::Instant| t.elapsed().as_secs_f64() * 1e3;
    let t = std::time::Instant::now(); let mut g = Grid::new(w, h); let gn = gen::Gen::new(seed, w); gn.fill_rows(&mut g, 0, -(h as i64 / 2), 0, h); println!("fill_rows (whole window) {:.0} ms", ms(t));
    let t = std::time::Instant::now(); let mut st = Stress::new(w * h); let n = st.solve(&g).len(); println!("one full solve {:.0} ms ({n} failing)", ms(t));
    let t = std::time::Instant::now(); gen::stabilize(&mut g, &mut st); println!("stabilize {:.0} ms", ms(t));
    let t = std::time::Instant::now(); let mut wd = World::new(w, h, seed); println!("World::new total {:.0} ms", ms(t));
    let sl = molehill_sim::world::SLIDE as i64;
    let t = std::time::Instant::now(); wd.shift(sl); println!("slide into fresh rows {:.0} ms", ms(t));
    let t = std::time::Instant::now(); wd.shift(-sl); println!("slide back into stored rows {:.0} ms", ms(t));
    let t = std::time::Instant::now(); let f = wd.st.solve(&wd.g); println!("  (of which a full stress solve ~{:.0} ms, {} failing)", ms(t), f.len());
    let t = std::time::Instant::now(); let mut g2 = Grid::new(w, h); gn.fill_rows(&mut g2, 0, 0, 0, sl as usize); println!("  fill_rows for one slide {:.0} ms", ms(t));
}
// (appended) slide costs: into fresh rows and back into stored rows
#[allow(dead_code)] fn _slides() {}
