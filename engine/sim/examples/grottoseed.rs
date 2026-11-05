//! Find seeds whose new holt spawns closest to a grotto (headless, deterministic, generator only).
//! Usage: cargo run --release -p molehill-sim --example grottoseed -- [SEEDS] [--show SEED]
//! Scores each seed by the distance (m) from the burrow (window centre, depth 0) to the nearest open
//! air of a grotto with trees, using the generator's own grotto layout (`Gen::grotto_at`, `Grotto::part`).
//! `--show SEED` generates that world for real and prints an ASCII map around the spawn.
use molehill_sim::gen::{Gen, Part};
use molehill_sim::mat::{CPM, S};
use molehill_sim::world::World;

const W: usize = 1024 * S; const H: usize = 512 * S; // the game's window (256 × 128 m)

/// (metres to the nearest grotto air, dx m, dy m (+ = down), trees, glow) for the best grotto near the spawn.
fn score(seed: u32) -> Option<(f32, f32, f32, usize)> {
    let gen = Gen::new(seed, W); let (sx, sy) = ((W / 2) as f32, 0f32);
    let mut best: Option<(f32, f32, f32, usize)> = None;
    for g in gen.grottos_in_rows(-48 * CPM as i64, 48 * CPM as i64) {
        if g.ntrees == 0 { continue; }
        let (x0, x1) = (g.cx.saturating_sub(g.rx + 8), g.cx + g.rx + 8); let (y0, y1) = (g.cy - g.ry as i64 - 8, g.cy + g.ry as i64 + 8);
        for y in (y0..=y1).step_by(2) { for x in (x0..=x1).step_by(2) {
            if !matches!(g.part(x, y), Some(Part::Hollow)) { continue; }
            let (dx, dy) = ((x as f32 - sx) / CPM as f32, (y as f32 - sy) / CPM as f32); let d = (dx * dx + dy * dy).sqrt();
            if best.map_or(true, |b| d < b.0 || (d - b.0).abs() < 0.01 && g.ntrees > b.3) { best = Some((d, dx, dy, g.ntrees)); }
        } }
    }
    best
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--show") { show(args[i + 1].parse().unwrap()); return; }
    let n: u32 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(20000);
    let mut all: Vec<(u32, (f32, f32, f32, usize))> = (0..n).filter_map(|s| score(s).map(|v| (s, v))).collect();
    all.sort_by(|a, b| a.1 .0.total_cmp(&b.1 .0).then(b.1 .3.cmp(&a.1 .3)));
    println!("seed   dist_m   dx_m    dy_m   trees");
    for (s, (d, dx, dy, t)) in all.iter().take(12) { println!("{s:<6} {d:6.1}  {dx:+6.1}  {dy:+6.1}   {t}"); }
}

/// Generate the world for real and print the area round the spawn, one char per 0.5 m.
fn show(seed: u32) {
    let w = World::new(W, H, seed); let m = &w.moles[0];
    println!("seed {seed}: mole at x {} (window centre {}), depth {:.1} m; grottos in window: {}", m.x, W / 2, w.depth_m(m.x, m.y), w.grottos.len());
    let k = CPM / 2; let (cx, cy) = (m.x as i64, m.y as i64);
    for ty in -12i64..=84 { let mut line = String::new(); for tx in -45i64..=45 {
        let (x, y) = (cx + tx * k as i64, cy + ty * k as i64);
        if tx == 0 && (ty == 0 || ty == -1) { line.push('M'); continue; }
        if x < 0 || y < 0 || x as usize >= W || y as usize >= H { line.push(' '); continue; }
        use molehill_sim::mat::Mat::*;
        line.push(match w.g.get(x as usize, y as usize) { Empty => '.', Water => 'w', Lava => 'l', RootTree => 'T', TreeRoot => 'R', Moss => 'm', Mycelium => 'y', Limestone => 'L', Dirt => 'd', Clay => 'c', Sand => 's', Gravel => 'g', Granite => '#', Rock => 'r', _ => '+' });
    } println!("{line}"); }
    println!("(one char = 0.5 m; M = the mole; T root-tree, y mycelium, m moss, w water, . open)");
}
