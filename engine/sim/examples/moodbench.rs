//! Found-place mood diagnostic: each family stamped in plain host rock, then
//! * light: the open cells of the site lit by its own torches and glowing materials (no mole): mean, 90th
//!   percentile and max brightness, and the share left dark (< 0.05) or blown out (> 1.2);
//! * TNT (mine, pumpworks, halt): one charge on the floor in the middle: cells changed in the site box,
//!   crumbled cells and debris over 10 s, how far the furthest changed cell is, and the events heard;
//! * water (pumpworks): the lever flipped, water in the box and in the world over 30 s (conservation).
//!
//! `cargo run --release -p molehill-sim --example moodbench [kind…]`
use molehill_sim::{mat::{Mat, CPM, S}, sites::Kind, world::{Event, World}};

fn lum(c: [f32; 3]) -> f32 { 0.3 * c[0] + 0.5 * c[1] + 0.2 * c[2] }

fn setup(kind: Kind) -> (World, usize, usize, usize, usize) {
    let mut w = World::new(1024 * S, 512 * S, 4242); w.disasters.enabled = false;
    let (sw, sh) = kind.size(); let c = CPM;
    let (mx, my) = (w.moles[0].x, w.moles[0].y);
    let (x0, y0) = (mx - sw / 2, my + 4 * c);
    for y in y0 - 10 * c..y0 + sh + 10 * c { for x in x0 - 10 * c..x0 + sw + 10 * c { let i = y * w.g.w + x; w.g.set(i, Mat::Rock); w.g.flags[i] = 0; } }
    w.torches.clear(); w.drops.clear(); w.grottos.clear();
    w.stamp_site(kind, x0, y0);
    w.moles[0].alive = false;
    w.g.wake_all();
    for _ in 0..60 { w.step(); }
    (w, x0, y0, sw, sh)
}

fn light(kind: Kind) {
    let (mut w, x0, y0, sw, sh) = setup(kind);
    w.compute_light((x0, y0, x0 + sw - 1, y0 + sh - 1));
    let mut v: Vec<f32> = vec![];
    for y in y0..y0 + sh { for x in x0..x0 + sw { let i = y * w.g.w + x; if w.g.get(x, y) == Mat::Empty { v.push(lum(w.light.at(i))); } } }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len().max(1) as f32; let mean = v.iter().sum::<f32>() / n;
    let dark = v.iter().filter(|&&l| l < 0.05).count() as f32 / n; let hot = v.iter().filter(|&&l| l > 1.2).count() as f32 / n;
    if std::env::var("TORCHES").is_ok() { for &(tx, ty) in &w.torches { let c = CPM;
        let at = |dx: i64| { let x = (tx as i64 + dx) as usize; format!("{:.2}{}", lum(w.light.at(ty * w.g.w + x)), if w.g.get(x, ty) == Mat::Empty { "" } else { "#" }) };
        println!("  torch at site {:.1},{:.1} m: {:?} there; luma at 0/1/3/5 m right: {} {} {} {}; left 3 m {}", (tx - x0) as f32 / c as f32, (ty - y0) as f32 / c as f32, w.g.get(tx, ty), at(0), at(c as i64), at(3 * c as i64), at(5 * c as i64), at(-3 * c as i64)); } }
    // the edge of each torch's pool (5.5-8.5 m out, in its own room): how much of it is pitch dark
    let c = CPM as f32; let (mut ring, mut ring_dark) = (0usize, 0usize);
    for y in y0..y0 + sh { for x in x0..x0 + sw { if w.g.get(x, y) != Mat::Empty { continue; }
        if w.torches.iter().any(|&(tx, ty)| { let d = (((x as f32 - tx as f32).powi(2) + (y as f32 - ty as f32).powi(2)).sqrt()) / c; d > 5.5 && d < 8.5 }) {
            ring += 1; if lum(w.light.at(y * w.g.w + x)) < 0.05 { ring_dark += 1; } } } }
    let blown_where: usize = (y0..y0 + sh).flat_map(|y| (x0..x0 + sw).map(move |x| (x, y))).filter(|&(x, y)| lum(w.light.at(y * w.g.w + x)) > 1.2 && w.g.get(x, y) == Mat::Empty
        && w.torches.iter().all(|&(tx, ty)| (((x as f32 - tx as f32).powi(2) + (y as f32 - ty as f32).powi(2)).sqrt()) / c > 1.5)).count();
    println!("{:<18} pool edge 5.5-8.5 m: {} cells, {:.0}% dark;  blown cells more than 1.5 m from any torch: {}", "", ring, ring_dark as f32 * 100. / ring.max(1) as f32, blown_where);
    println!("{:<18} light: open {:>6}  mean {:.2}  p90 {:.2}  max {:.2}  dark {:>3.0}%  blown {:>3.0}%  torches {}",
        kind.name(), v.len(), mean, v[(v.len() * 9 / 10).min(v.len() - 1)], v.last().unwrap_or(&0.), dark * 100., hot * 100., w.torches.len());
}

fn tnt(kind: Kind) {
    let (mut w, x0, y0, sw, sh) = setup(kind);
    let c = CPM;
    let snap: Vec<u8> = (y0..y0 + sh).flat_map(|y| (x0..x0 + sw).map(move |x| (x, y))).map(|(x, y)| w.g.mat[y * w.g.w + x]).collect();
    // the charge: the first open cell above the floor at the middle
    let bx = x0 + sw / 2; let mut by = y0 + sh - 2; while by > y0 && w.g.get(bx, by) != Mat::Empty { by -= 1; }
    while by + 1 < y0 + sh && w.g.get(bx, by + 1) == Mat::Empty { by += 1; }
    let calm = std::env::var("CALM").is_ok();
    let cr0 = w.crumbled; if !calm { w.blast_tnt(bx, by); } else { println!("(calm: no charge; at {},{} {:?})", bx - x0, by - y0, w.g.get(bx, by)); }
    let (mut booms, mut rumbles, mut debris) = (0, 0, 0usize);
    for _ in 0..600 { w.step(); debris = debris.max(w.debris.len()); for e in &w.events { match e { Event::Explode => booms += 1, Event::Crumble => rumbles += 1, _ => {} } } }
    let (mut changed, mut far) = (0usize, 0f32);
    for (k, &m) in snap.iter().enumerate() { let (x, y) = (x0 + k % sw, y0 + k / sw); if w.g.mat[y * w.g.w + x] != m {
        changed += 1; let d = (((x as f32 - bx as f32).powi(2) + (y as f32 - by as f32).powi(2)).sqrt()) / c as f32; far = far.max(d); } }
    println!("{:<18} tnt: changed {:>5} cells ({:.1} m²)  crumbled {:>5}  debris peak {:>4}  furthest {:>4.1} m  booms {} rumbles {}",
        kind.name(), changed, changed as f32 / (c * c) as f32, w.crumbled - cr0, debris, far, booms, rumbles);
}

fn water() {
    let (mut w, x0, y0, sw, sh) = setup(Kind::Pumpworks);
    let c = CPM;
    // the dry side: east of the sluice (sites.rs: the split wall at 20 m, 1 m thick)
    let inbox = |w: &World| (y0..y0 + sh).flat_map(|y| (x0 + 21 * c..x0 + sw).map(move |x| (x, y))).filter(|&(x, y)| w.g.get(x, y) == Mat::Water).count();
    let (b0, t0) = (inbox(&w), w.g.count(Mat::Water));
    w.moles[0].alive = true; w.moles[0].x = x0 + 22 * c; w.moles[0].y = y0 + sh - 2 * c;
    let ok = w.use_device(x0 + 22 * c, y0 + sh - 4 * c);
    w.moles[0].alive = false;
    print!("pumpworks water: lever {}  dry side {} world {}", if ok { "flipped" } else { "NOT flipped" }, b0, t0);
    for s in [2, 5, 10, 20, 30] { while w.tick < s as u64 * 60 + 60 { w.step(); } print!("  | {s}s dry {} world {}", inbox(&w), w.g.count(Mat::Water)); }
    println!();
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kinds: Vec<Kind> = if a.is_empty() { (0..7).map(Kind::from_u8).collect() } else { a.iter().filter_map(|s| Kind::from_name(s)).collect() };
    for &k in &kinds { light(k); }
    for &k in kinds.iter().filter(|k| matches!(k, Kind::Mine | Kind::Pumpworks | Kind::Halt)) { tnt(k); }
    if kinds.contains(&Kind::Pumpworks) { water(); }
}
