//! How long a new holt and a save load block (game-sized window). Builds a large long-running save by
//! sliding the window deep and back, then times save, load and the load's phases. Usage: loadbench [seed] [slides]
use molehill_sim::{mat::S, save, world::World};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(4242); let slides: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(24);
    let (w, h) = (1024 * S, 512 * S); let ms = |t: std::time::Instant| t.elapsed().as_secs_f64() * 1e3;
    let t = std::time::Instant::now(); let mut wd = World::new(w, h, seed); println!("new holt {:.0} ms", ms(t));
    // staged: the longest single step is what a frame can stall on
    let t = std::time::Instant::now(); let mut b = molehill_sim::world::NewHolt::new(w, h, seed); let (mut n, mut worst, mut wl) = (0, 0f64, "");
    loop { let (s, l) = (std::time::Instant::now(), b.label()); let d = b.step().is_some(); let e = ms(s); n += 1; if e > 40. { println!("  step {n} {l}: {e:.0} ms"); } if e > worst { worst = e; wl = l; } if d { break; } }
    println!("staged new holt {:.0} ms in {n} steps, longest step {worst:.0} ms ({wl})", ms(t)); assert!(b.take().g.mat == wd.g.mat);
    let sl = molehill_sim::world::SLIDE as i64;
    for _ in 0..slides { wd.shift(sl); } for _ in 0..slides / 2 { wd.shift(-sl); }
    for _ in 0..600 { wd.step(); }
    let t = std::time::Instant::now(); let b = save::save(&wd); println!("save {:.0} ms, {} KB, {} stored bands", ms(t), b.len() / 1024, wd.bands.len());
    let t = std::time::Instant::now(); let l = save::load(&b).unwrap(); println!("load {:.0} ms", ms(t));
    assert_eq!(l.g.mat, wd.g.mat);
}
