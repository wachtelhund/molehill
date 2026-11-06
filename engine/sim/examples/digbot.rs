//! Cave-in rate while tunnelling: a Survival mole digs like a player (pickaxe, the bite aimed 1.5 m ahead at
//! body height so it clears a passage, walking into it), cycling every 30 s through a tunnel right, a
//! staircase down-right, a tunnel left and a staircase down-left. Per seed and per minute of digging:
//! * `cracks`: ticks a crack telegraph starts (stress failures begin to creak; `Event::Crack` with cells cracking);
//! * `episodes`: cave-ins, counted as runs of crumbling separated by at least 1 s of quiet, and `big` ones
//!   (≥ 1 m² of rock let go);
//! * `crumbled`: cells that gave way (m² per minute);
//! * `buried`: ticks the mole was hurt by being buried; `warned`: share of episodes that had a crack telegraph
//!   at least 0.5 s before they began; `dug`: metres of tunnel the bot advanced.
//! Disasters are off (the game's default), so everything counted comes from digging.
//!
//! `cargo run --release -p molehill-sim --example digbot -- [minutes] [seeds…]`
use molehill_sim::{mat::{CPM, S}, survival::Mode, world::{Event, World}};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mins: f32 = a.first().and_then(|s| s.parse().ok()).unwrap_or(3.);
    let seeds: Vec<u32> = if a.len() > 1 { a[1..].iter().filter_map(|s| s.parse().ok()).collect() } else { vec![1, 7, 11, 42, 4242] };
    let c = CPM as f32; let ticks = (mins * 3600.) as u64;
    let mut tot = [0f32; 8]; let mut all_sizes = [0u32; 3]; let (mut all_big, mut all_big_warned) = (0u32, 0u32);
    let size_bin = |n: u64| if n < (S * S) as u64 { 0 } else if n < (CPM * CPM) as u64 { 1 } else { 2 };
    println!("{:>6} {:>7} {:>9} {:>6} {:>11} {:>7} {:>7} {:>6} {:>9}", "seed", "cracks", "episodes", "big", "crumbled m²", "buried", "warned", "far m", "drift/s");
    for &seed in &seeds {
        let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false; w.set_mode(Mode::Survival);
        w.rules.no_hunger = true; w.lamp_oil = 1e9;
        for _ in 0..30 { w.step(); } // the player body syncs to the mole on the first steps
        let (sx, sy) = (w.pl.px, w.oy as f32 + w.pl.py); // absolute rows: the window slides as the bot goes down
        let (mut cracks, mut episodes, mut big, mut buried) = (0u32, 0u32, 0u32, 0u32);
        let (mut quiet, mut ep_cells, mut ep_warned, mut warned) = (u64::MAX, 0u64, false, 0u32);
        let mut crack_ticks: std::collections::VecDeque<u64> = Default::default();
        let mut by_mat = std::collections::BTreeMap::<String, u64>::new(); let (mut hang, mut over) = (0u64, 0u64);
        let (mut far, mut stuck, mut best) = (0f32, 0u64, 0f32); let mut drift = 0u64; let mut drift_kind = [0u64; 3]; let mut drift_mat = std::collections::BTreeMap::<String, u64>::new(); let mut deaths = 0u64; let mut sizes = [0u32; 3]; let mut big_warned = 0u32;
        let mut dir = 0usize;
        for t in 0..ticks {
            if t % (30 * 60) == 0 { dir = (t / (30 * 60)) as usize % 4; stuck = 0; best = 0.; }
            let (dx, dy) = [(1., 0.), (1., 0.7), (-1., 0.), (-1., 0.7)][dir];
            let l = ((dx * dx + dy * dy) as f32).sqrt();
            let (px, py) = (w.pl.px, w.pl.py - 1.4 * S as f32);
            let aim = ((px + dx / l * 1.5 * c).max(0.) as usize, (py + dy / l * 1.5 * c).max(0.) as usize);
            w.input = Default::default(); w.input.tool = 1; w.input.mine = Some(aim);
            w.input.right = dx > 0.; w.input.left = dx < 0.;
            // a stuck bot (a wall it can't dig, a pit) hops, then turns after 5 s without progress
            let d = (w.pl.px - sx).abs() / c; if d > best + 0.25 { best = d; stuck = 0; } else { stuck += 1; }
            // stay a tunnel-builder near home: turn back 40 m out or 25 m down (the spawn's layers, not a dive)
            if ((w.pl.px - sx) / c > 40. && dx > 0.) || ((sx - w.pl.px) / c > 40. && dx < 0.) || (w.oy as f32 + w.pl.py - sy) / c > 25. && dy > 0. { dir = (dir + 2) % 4; best = 0.; stuck = 0; if (w.oy as f32 + w.pl.py - sy) / c > 25. { dir &= 2; } }
            if stuck > 120 && stuck % 40 < 6 { w.input.jump = true; }
            if stuck > 300 { dir = (dir + 1) % 4; stuck = 0; best = 0.; }
            let before = w.crumbled; let hp0 = w.moles[0].hp;
            let (mx, my, r) = (w.moles[0].x, w.moles[0].y, 8 * CPM);
            let near: Vec<usize> = (my.saturating_sub(r)..(my + r).min(w.g.h)).flat_map(|y| (mx.saturating_sub(r)..(mx + r).min(w.g.w)).map(move |x| (x, y))).map(|(x, y)| y * w.g.w + x).collect();
            let mv0: Vec<u8> = near.iter().map(|&i| w.g.moved[i]).collect();
            let pend: Vec<(usize, u8, molehill_sim::mat::Mat)> = w.st.cracking.iter().map(|&i| (i as usize, w.st.pending[i as usize], w.g.m(i as usize))).filter(|p| p.1 == 1).collect();
            w.step();
            for (i, _, m) in pend { if w.g.m(i) != m { *by_mat.entry(format!("{m:?}")).or_default() += 1; if w.st.reach.get(i) == Some(&false) { hang += 1; } else { over += 1; } } }
            let crumbled = w.crumbled - before;
            for (&i, &m0) in near.iter().zip(&mv0) { if w.g.moved[i] != m0 { let m = w.g.m(i); match m.kind() {
                molehill_sim::mat::Kind::Powder => drift_kind[0] += 1, molehill_sim::mat::Kind::Solid if w.g.loose(i) => drift_kind[1] += 1, molehill_sim::mat::Kind::Solid => drift_kind[2] += 1, _ => {} };
                if matches!(m.kind(), molehill_sim::mat::Kind::Powder | molehill_sim::mat::Kind::Solid) { *drift_mat.entry(format!("{m:?}")).or_insert(0u64) += 1; } } }
            drift += near.iter().zip(&mv0).filter(|(&i, &m0)| w.g.moved[i] != m0 && matches!(w.g.m(i).kind(), molehill_sim::mat::Kind::Solid | molehill_sim::mat::Kind::Powder)).count() as u64;
            if w.events.iter().any(|e| *e == Event::Crack) && !w.st.cracking.is_empty() { cracks += 1; crack_ticks.push_back(t); }
            while crack_ticks.front().is_some_and(|&c| c + 600 < t) { crack_ticks.pop_front(); }
            if crumbled > 0 {
                if quiet >= 60 { if episodes > 0 { sizes[size_bin(ep_cells)] += 1; } episodes += 1; ep_cells = 0; ep_warned = crack_ticks.iter().any(|&lc| lc + 30 <= t); if ep_warned { warned += 1; } }
                ep_cells += crumbled; if ep_cells >= (CPM * CPM) as u64 && ep_cells - crumbled < (CPM * CPM) as u64 { big += 1; if ep_warned { big_warned += 1; } }
                quiet = 0;
            } else { quiet = quiet.saturating_add(1); }
            let m = &w.moles[0];
            let buried_now = (0..molehill_sim::world::MOLE_H).any(|j| w.g.is_solidish(w.g.idx(m.x, m.y - j)));
            if buried_now && m.hp < hp0 { buried += 1; }
            // (the mole's cell, not the player body: that reads (0, 0) for a tick after a respawn)
            let (mxf, myf) = (w.moles[0].x as f32, (w.oy + w.moles[0].y as i64) as f32);
            if w.moles[0].alive && w.pl.dead_t == 0 { far = far.max((((mxf - sx).powi(2) + (myf - sy).powi(2)).sqrt()) / c); } else { deaths += 1; }
            let _ = ep_warned;
        }
        if std::env::var("WHY").is_ok() { println!("   crumbled by material {:?}; no load path {hang}, overloaded {over}; dead ticks {deaths}, deaths {}", by_mat, w.pl.deaths); }
        if episodes > 0 { sizes[size_bin(ep_cells)] += 1; }
        if std::env::var("DRIFT").is_ok() { println!("   drift: powder {} loose rubble {} other solid {}; by material {:?}", drift_kind[0], drift_kind[1], drift_kind[2], drift_mat); }
        if std::env::var("WHY").is_ok() { println!("   episode sizes: <1 tile {}, <1 m² {}, ≥1 m² {}", sizes[0], sizes[1], sizes[2]); }
        for k in 0..3 { all_sizes[k] += sizes[k]; }
        all_big += big; all_big_warned += big_warned;
        let pm = |v: f32| v / mins;
        let cm2 = (w.crumbled as f32) / (c * c);
        let ds = drift as f32 / (mins * 60.);
        println!("{seed:>6} {:>7.1} {:>9.1} {:>6.1} {:>11.2} {:>7.0} {:>6.0}% {:>6.0} {:>9.0}", pm(cracks as f32), pm(episodes as f32), pm(big as f32), pm(cm2), buried, warned as f32 * 100. / episodes.max(1) as f32, far, ds);
        for (k, v) in [cracks as f32, episodes as f32, big as f32, cm2, buried as f32, warned as f32, far, ds].into_iter().enumerate() { tot[k] += v; }
    }
    let n = seeds.len() as f32;
    println!("{:>6} {:>7.1} {:>9.1} {:>6.1} {:>11.2} {:>7.0} {:>6.0}% {:>6.0} {:>9.0}", "mean", tot[0] / n / mins, tot[1] / n / mins, tot[2] / n / mins, tot[3] / n / mins, tot[4] / n, tot[5] * 100. / tot[1].max(1.), tot[6] / n, tot[7] / n);
    println!("big cave-ins (≥ 1 m²) warned by a creak 0.5-10 s before: {all_big_warned} of {all_big}");
    println!("episode sizes over all seeds: under one 0.25 m tile {}, under 1 m² {}, 1 m² or more {}", all_sizes[0], all_sizes[1], all_sizes[2]);
    println!("(per minute of digging, {mins} min per seed; buried = ticks hurt by burial, total; far = furthest the bot got;");
    println!(" drift/s = cells of loose ground or rubble moving within 8 m of the mole, per second)");
}
