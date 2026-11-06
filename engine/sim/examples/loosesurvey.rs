//! Loose ground around a fresh holt: how much of what the generator placed near the burrow is loose or
//! slump-prone, and how much falls on a Delver tunnelling out of it.
//! `cargo run --release -p molehill-sim --example loosesurvey -- [SEEDS] [RADIUS_M] [--nodig]`
//!
//! Static, per seed, within RADIUS_M (default 30) of the burrow in the game's window, as a share of the solid
//! and powder ground there: sand and fungus (powder: it pours as soon as it is undercut), gravel (rubble that
//! slumps when cut), cells already loose (`F_LOOSE`), and the total of those ("loose"). Dirt and wet
//! (aquifer, `F_WET`) ground are listed beside it.
//!
//! Dig bot (unless `--nodig`): from the burrow floor, a 1.5 m shaft down to 20 m, then 1 m tall tunnels 30 m
//! out each way at the burrow floor, 10 m and 20 m down, carved one cell column every 2 ticks with the mole kept
//! at the face (so the sim runs there). Reported: cells that crumbled (stress cave-ins, `World::crumbled`) and
//! the loose fill left inside the carved tunnels after 5 s of settling ("rain"), and how much of the ground
//! the generator placed loose (sand, gravel, fungus) within 4 m of the works was displaced ("genloose"), all
//! per metre of tunnel. `WHY=1` also prints the displaced ground by its original material.
use molehill_sim::{grid::F_LOOSE, grid::F_WET, mat::{Kind, Mat, CPM, S}, world::World};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
    let rad: f32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(30.);
    let dig = !a.iter().any(|s| s == "--nodig");
    let c = CPM as f32;
    let mut tot = [0f64; 8]; let mut tot_dig = [0f64; 4];
    println!("seed   loose%  sand%  grav%  fung%  flag%  dirt%  wet%  | crumbled/m  rain/m  genloose/m");
    for seed in 0..n {
        let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false;
        let (mx, my) = (w.moles[0].x as f32, w.moles[0].y as f32);
        let mut k = [0u32; 8]; // ground, sand, gravel, fungus, flagged, dirt, wet, loose total
        for y in 0..w.g.h { for x in 0..w.g.w {
            if ((x as f32 - mx).powi(2) + (y as f32 - my).powi(2)).sqrt() / c > rad { continue; }
            let i = w.g.idx(x, y); let m = w.g.m(i); if !matches!(m.kind(), Kind::Solid | Kind::Powder) || m == Mat::Bedrock { continue; }
            k[0] += 1; let fl = w.g.flags[i];
            let (sand, grav, fung, lf) = (m == Mat::Sand, m == Mat::Gravel, m == Mat::Fungus, fl & F_LOOSE != 0);
            k[1] += sand as u32; k[2] += grav as u32; k[3] += fung as u32; k[4] += lf as u32; k[5] += (m == Mat::Dirt) as u32; k[6] += (fl & F_WET != 0) as u32;
            k[7] += (sand || grav || fung || lf) as u32;
        } }
        let pc = |v: u32| v as f64 / k[0].max(1) as f64 * 100.;
        let (cr, rain, gl, len) = if dig { dig_bot(&mut w) } else { (0., 0., 0., 1.) };
        println!("{seed:>4}  {:6.2} {:6.2} {:6.2} {:6.2} {:6.2} {:6.2} {:5.2}  | {:9.2} {:7.2} {:9.2}", pc(k[7]), pc(k[1]), pc(k[2]), pc(k[3]), pc(k[4]), pc(k[5]), pc(k[6]), cr / len, rain / len, gl / len);
        for j in 0..7 { tot[j] += pc([k[7], k[1], k[2], k[3], k[4], k[5], k[6]][j]); }
        tot_dig[0] += cr / len; tot_dig[1] += rain / len; tot_dig[3] += gl / len; tot_dig[2] += 1.;
    }
    let m = |j: usize| tot[j] / n as f64;
    println!("mean  {:6.2} {:6.2} {:6.2} {:6.2} {:6.2} {:6.2} {:5.2}  | {:9.2} {:7.2} {:9.2}", m(0), m(1), m(2), m(3), m(4), m(5), m(6), tot_dig[0] / tot_dig[2], tot_dig[1] / tot_dig[2], tot_dig[3] / tot_dig[2]);
}

/// Carve the shaft and six tunnels; returns (cells crumbled, loose cells left in the tunnels, generator-loose
/// cells displaced, metres of tunnel).
fn dig_bot(w: &mut World) -> (f64, f64, f64, f64) {
    let (sx, fy) = (w.moles[0].x, w.moles[0].y); // the burrow floor row (feet)
    let (hgt, half) = (CPM, 3 * CPM / 4);
    let c0 = w.crumbled;
    let before: Vec<(u8, u8)> = w.g.mat.iter().zip(&w.g.flags).map(|(&m, &f)| (m, f)).collect();
    let mut carved: Vec<usize> = vec![];
    let put = |w: &mut World, x: usize, y: usize| { w.moles[0].x = x; w.moles[0].y = y; w.pl.px = x as f32 + 0.5; w.pl.py = (y + 1) as f32; w.pl.ppx = w.pl.px; w.pl.ppy = w.pl.py; w.pl.vx = 0.; w.pl.vy = 0.; w.pl.sync = Some((x, y)); w.moles[0].hp = 100.; };
    let carve = |w: &mut World, x: usize, y0: usize, y1: usize, carved: &mut Vec<usize>| {
        for y in y0..=y1 { let i = w.g.idx(x, y); let m = w.g.m(i); if m != Mat::Bedrock && m.kind() != Kind::Liquid { w.g.set(i, Mat::Empty); } carved.push(i); }
    };
    // the shaft: rows below the burrow floor down to 20 m, a row of 2·half cells every 2 ticks
    for y in fy + 1..fy + 20 * CPM { for x in sx - half..sx + half { let i = w.g.idx(x, y); if w.g.m(i) != Mat::Bedrock { w.g.set(i, Mat::Empty); } }
        put(w, sx, y); w.step(); w.step(); }
    let mut metres = 0.;
    for depth in [0usize, 10, 20] {
        let floor = fy + depth * CPM;
        for dir in [-1i64, 1] {
            let start = if dir < 0 { sx - half } else { sx + half };
            for k in 0..30 * CPM { let x = (start as i64 + dir * k as i64) as usize; if x < 4 * S || x >= w.g.w - 4 * S { break; }
                carve(w, x, floor + 1 - hgt, floor, &mut carved); put(w, x, floor); w.step(); w.step(); }
            metres += 30.;
            put(w, sx, floor); for _ in 0..300 { w.step(); }
        }
    }
    // what failed: ground near the tunnels (not carved by the bot) that was fixed in place before and is now
    // gone or loose, by its original material
    let mut gl = 0u32; {
        let mut cut = vec![false; w.g.mat.len()]; for &i in &carved { cut[i] = true; }
        for y in fy + 1..fy + 20 * CPM { for x in sx - half..sx + half { cut[w.g.idx(x, y)] = true; } }
        let mut why: std::collections::BTreeMap<String, u32> = Default::default();
        for i in 0..w.g.mat.len() { let (m0, f0) = before[i]; let m0 = Mat::from_u8(m0);
            if cut[i] || f0 & F_LOOSE != 0 || !matches!(m0.kind(), Kind::Solid | Kind::Powder) || m0 == Mat::Bedrock { continue; }
            let (x, y) = (i % w.g.w, i / w.g.w); if (x as i64 - sx as i64).abs() > 34 * CPM as i64 || y + 6 * CPM < fy || y > fy + 26 * CPM { continue; }
            if w.g.mat[i] != m0 as u8 || w.g.flags[i] & F_LOOSE != 0 { *why.entry(format!("{m0:?}")).or_default() += 1; if matches!(m0, Mat::Sand | Mat::Gravel | Mat::Fungus) { gl += 1; } } }
        if std::env::var("WHY").is_ok() { eprintln!("  displaced by original material: {why:?}"); }
    }
    let rain = carved.iter().filter(|&&i| { let m = w.g.m(i); matches!(m.kind(), Kind::Solid | Kind::Powder) }).count() as f64;
    ((w.crumbled - c0) as f64, rain, gl as f64, metres)
}
