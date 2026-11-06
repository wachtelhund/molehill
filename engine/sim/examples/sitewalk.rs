//! Found-place traversal survey: can the mole's body get everywhere a site's layout opens up?
//! `cargo run --release -p molehill-sim --example sitewalk -- [KIND ...]` (default: all seven)
//!
//! Each site is stamped alone in solid granite, and every place the mole's body fits is flood-filled into
//! connected regions (`World::body_regions`: the sim's own collision box, gravity ignored, since the rope
//! gun takes the mole up any face under rock and rope ladders take it down: no wall climbing). The map is drawn at 0.5 m a character: `#` solid, a letter per
//! region (A is the biggest), `x` open space no body position reaches (too tight), `|` a rope ladder, `C`
//! the chest. A region that never comes near the site's edge has no way in; two big regions mean the
//! layout is cut in two. `tests/found_places.rs` checks the same thing.
use molehill_sim::{world::{World, BODY_H, BODY_HW}, mat::{Mat, CPM}, sites::Kind};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kinds: Vec<&str> = if a.is_empty() { vec!["holt", "mine", "pump", "fissure", "fossil", "crystal", "halt"] } else { a.iter().map(|s| s.as_str()).collect() };
    for k in kinds { survey(Kind::from_name(k).unwrap_or_else(|| panic!("unknown site {k}"))); }
}
fn survey(kind: Kind) {
    let mut w = World::new(1024, 512, 1); w.disasters.enabled = false;
    let (sw, sh) = kind.size(); let m = 2 * CPM; let (x0, y0) = (64, 512 - sh - 3 * m);
    for y in y0 - m..y0 + sh + m { for x in x0 - m..x0 + sw + m { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); w.g.flags[i] = 0; } }
    w.tr.ladders.clear(); w.chests.clear(); w.stamp_site(kind, x0, y0);
    let (bx0, by0, bw, bh) = (x0 - m / 2, y0 - m / 2, sw + m, sh + m);
    let (reg, sizes) = w.body_regions(bx0, by0, bw, bh);
    // a way in: the region comes within half a metre of the site's sides or a metre of its top
    let mut edge = vec![false; sizes.len()];
    for (i, &r) in reg.iter().enumerate() { let (gx, gy) = (bx0 + i % bw, by0 + i / bw);
        if r != 0 && (gx < x0 + CPM / 2 || gx + CPM / 2 >= x0 + sw || gy < y0 + CPM) { edge[r as usize] = true; } }
    let mut order: Vec<usize> = (1..sizes.len()).collect(); order.sort_by_key(|&k| std::cmp::Reverse(sizes[k]));
    let mut letter = vec![b'?'; sizes.len()]; for (r, &k) in order.iter().enumerate() { letter[k] = if r < 26 { b'A' + r as u8 } else { b'z' }; }
    // which region's body covers each open cell (the biggest, where several do)
    let mut cover = vec![0u32; bw * bh]; let (hw, h) = (BODY_HW.ceil() as usize, BODY_H.ceil() as usize);
    for (i, &id) in reg.iter().enumerate() { if id == 0 { continue; } let (sx, sy) = (i % bw, i / bw);
        for y in sy.saturating_sub(h)..sy { for x in sx.saturating_sub(hw)..(sx + hw).min(bw) { let c = &mut cover[y * bw + x]; if *c == 0 || sizes[id as usize] > sizes[*c as usize] { *c = id; } } } }
    println!("== {} ({} x {} m): {} regions", kind.name(), sw / CPM, sh / CPM, order.len());
    for &k in order.iter().take(8) { println!("  {} {:>6} positions{}", letter[k] as char, sizes[k], if edge[k] { ", reaches the edge (a way in)" } else { ", sealed: no way in" }); }
    let q = CPM / 2; let mut tight = 0;
    for ty in 0..bh / q { let mut s = String::new(); for tx in 0..bw / q {
        let (cx, cy) = (bx0 + tx * q, by0 + ty * q);
        let cells: Vec<(usize, usize)> = (0..q * q).map(|j| (cx + j % q, cy + j / q)).collect();
        let open = cells.iter().filter(|&&(x, y)| w.g.get(x, y).passable()).count();
        let ch = if w.chests.iter().any(|c| c.x / q == cx / q && c.y / q == cy / q) { 'C' }
            else if w.tr.ladders.iter().any(|l| l.x / q == cx / q && cy + q > l.top && cy <= l.top + l.len) { '|' }
            else if open * 2 < q * q { '#' }
            else { let best = cells.iter().map(|&(x, y)| cover[(y - by0) * bw + x - bx0]).max_by_key(|&c| sizes[c as usize]).unwrap_or(0);
                if best == 0 { tight += 1; 'x' } else { letter[best as usize] as char } };
        s.push(ch); } println!("{s}"); }
    println!("  {tight} half-metre squares of open space no body position reaches");
}
