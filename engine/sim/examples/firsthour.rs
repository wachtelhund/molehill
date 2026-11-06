//! First-hour traversal traps: around the burrow of a fresh holt, the drops a careful mole can fall down.
//! `cargo run --release -p molehill-sim --example firsthour -- [SEEDS] [RADIUS_M]`
//! For each seed: the worst drop off a ledge (walk off an edge into open air, landing on rock, not water),
//! and the worst dig-through (a floor no thicker than 1 m with open air under it), in metres, plus how many
//! of each would hurt (over the safe fall) or kill (over ~13 m) and the nearest lethal one.
use molehill_sim::{world::{World, TUNE}, mat::{CPM, S, Kind}};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(40);
    let rad: f32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(30.);
    let c = CPM as f32; let safe = TUNE.safe_fall as f32 / c; let lethal = safe + 100. / (3. / molehill_sim::mat::SF) / c;
    println!("safe fall {safe:.1} m, lethal ~{lethal:.1} m, radius {rad} m");
    if let Some(k) = a.iter().position(|s| s == "--col") { column(a[k + 1].parse().unwrap()); return; }
    if let Some(k) = a.iter().position(|s| s == "--show") { show(a[k + 1].parse().unwrap()); return; }
    let (mut tot_l, mut tot_d) = (0, 0);
    for seed in 0..n {
        let w = World::new(256 * S, 256 * S, seed); let (gw, gh) = (w.g.w, w.g.h);
        let (mx, my) = (w.moles[0].x as f32, w.moles[0].y as f32);
        let open = |x: usize, y: usize| w.g.get(x, y).passable() && w.g.get(x, y).kind() != Kind::Liquid;
        let solid = |x: usize, y: usize| matches!(w.g.get(x, y).kind(), Kind::Solid | Kind::Powder);
        // drop from (x, y) (an open cell) straight down: metres to the first non-open cell, and whether it's water
        let drop = |x: usize, y: usize| { let mut k = y; while k + 1 < gh && open(x, k + 1) { k += 1; } ((k - y) as f32 / c, w.g.get(x, (k + 1).min(gh - 1)).kind() == Kind::Liquid) };
        let (mut worst_l, mut worst_d, mut nl, mut nd, mut near) = (0f32, 0f32, 0, 0, f32::MAX);
        for y in 1..gh - 1 { for x in 1..gw - 1 {
            let d = ((x as f32 - mx).powi(2) + (y as f32 - my).powi(2)).sqrt() / c; if d > rad { continue; }
            // a standing place: open, floor under it, head room
            if !open(x, y) || !solid(x, y + 1) || (1..6).any(|k| y < k || !open(x, y - k)) { continue; }
            for nx in [x - 1, x + 1] { if open(nx, y) && open(nx, y + 1) { let (f, wet) = drop(nx, y); if !wet && f > worst_l { worst_l = f; } if !wet && f > lethal { nl += 1; near = near.min(d); } } }
            // dig down: floor thickness up to 1 m, then air
            let mut t = 1; while t <= CPM && y + t < gh && solid(x, y + t) { t += 1; }
            if t <= CPM && y + t < gh && open(x, y + t) { let (f, wet) = drop(x, y + t); let f = f + t as f32 / c; if !wet && f > worst_d { worst_d = f; } if !wet && f > lethal { nd += 1; near = near.min(d); } }
        } }
        tot_l += (nl > 0) as u32; tot_d += (nd > 0) as u32;
        println!("seed {seed:>5}: ledge worst {worst_l:5.1} m ({nl:>4} lethal)   dig-through worst {worst_d:5.1} m ({nd:>4} lethal)   nearest lethal {}", if near < f32::MAX { format!("{near:.0} m") } else { "-".into() });
    }
    println!("seeds with a lethal ledge drop: {tot_l}/{n}; with a lethal dig-through: {tot_d}/{n}");
}

/// ASCII map around the spawn at 0.5 m per character: # solid, . open, ~ water, ! lava, @ the mole, and
/// what the window's grottos, sites and ruins are (with their boxes, in metres from the mole).
fn show(seed: u32) {
    let w = World::new(256 * S, 256 * S, seed); let c = CPM as i64; let (mx, my) = (w.moles[0].x as i64, w.moles[0].y as i64);
    for g in &w.grottos { println!("grotto {:?}", g); }
    for s in &w.sites { println!("site {:?} at {:.0},{:.0} m (w {} h {} m)", s.kind, (s.x0 as i64 - mx) as f32 / c as f32, (s.y0 as i64 - (my - w.oy * 0)) as f32 / c as f32, s.w / CPM, s.h / CPM); }
    for y in (-10i64..40).map(|r| my + r * c / 2) { let mut l = String::new(); for x in (-40i64..40).map(|q| mx + q * c / 2) {
        if x < 0 || y < 0 || x as usize >= w.g.w || y as usize >= w.g.h { l.push(' '); continue; }
        let m = w.g.get(x as usize, y as usize);
        l.push(if (x - mx).abs() < c / 2 && (y - my).abs() < c / 2 { '@' } else { match m.kind() { Kind::Solid | Kind::Powder => '#', Kind::Liquid => if m == molehill_sim::mat::Mat::Lava { '!' } else { '~' }, _ => '.' } }); }
        println!("{:>4} {l}", (y - my) / (c / 2)); }
}

/// The cells right under the spawn, by material initial, one row per cell (then every 2 m).
fn column(seed: u32) {
    let w = World::new(256 * S, 256 * S, seed); let (mx, my) = (w.moles[0].x, w.moles[0].y);
    for dy in 0..20 * CPM { let y = my + dy; if dy >= 12 && dy % 16 != 0 { continue; }
        let row: String = (mx - 6..mx + 7).map(|x| { let m = w.g.get(x, y); if m.passable() { '.' } else { format!("{m:?}").chars().next().unwrap() } }).collect();
        println!("{:>4} {row}", dy); }
}
