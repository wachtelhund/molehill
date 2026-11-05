//! Load a save file and play it for a while (crash hunt for old saves): `loadsave FILE [TICKS]`.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let b = std::fs::read(&a[1]).unwrap(); let n: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(3000);
    let mut w = match molehill_sim::save::load(&b) { Ok(w) => w, Err(e) => { println!("load error: {e}"); return } };
    println!("loaded {}x{} oy {} mole {:?}", w.g.w, w.g.h, w.oy, (w.moles[0].x, w.moles[0].y));
    for t in 0..n { w.input.right = (t / 150) % 2 == 0; w.input.left = !w.input.right; w.input.jump = t % 40 < 4; w.input.down = t % 200 < 80;
        let (x, y) = (w.moles[0].x, w.moles[0].y); w.input.mine = if t % 2 == 0 { Some((x + 2, y + 1)) } else { None }; w.step();
        if t % 2 == 0 { w.compute_light((x.saturating_sub(80), y.saturating_sub(50), (x + 80).min(w.g.w - 1), (y + 50).min(w.g.h - 1))); } }
    println!("ok: mole {:?} oy {} hp {}", (w.moles[0].x, w.moles[0].y), w.oy, w.moles[0].hp);
}
