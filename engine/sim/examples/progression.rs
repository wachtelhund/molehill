//! First-to-mid progression, headless, per seed: evidence for pacing, shortages and repetition.
//! `cargo run --release -p molehill-sim --example progression -- [SEEDS] [MINUTES] [ONE_SEED]`
//!
//! Two parts, at the game's own window (1024 × 512 cells), Survival, hunger on, headlamp on:
//! * **Routes:** for each first-hour target (the nearest stone, root tangle, copper, tin, clay, coal, fungus,
//!   grotto, site, iron), its distance from the burrow and a straight-line estimate of the time to reach it
//!   with the kit's pickaxe (bites through solid at `mine_time`, walks through open at run speed), and how
//!   many cells on that line the pickaxe can't cut (a gate: the way needs a detour, or a better tool).
//! * **Bot:** a naive Delver that digs down, then tunnels sideways (turning at what it can't cut), with the
//!   sim's own pickup, auto-eating, oil, discoveries and quests running. It prints the log as a timeline
//!   (tick, depth) and what the pack holds at the end; long stretches with no new log line are "empty".
//! * **Ladder:** what the ground has to give for each rung of the tool ladder (stations and the thing, made
//!   from nothing the cheapest way, a coal per cold-forge job: `craft::raw_bill`), printed once; then, per
//!   bot, the first time its pack held enough for each rung (it never crafts: this is what a player carrying
//!   that pack could have made), and when the light first went out and hunger first hit zero.
//! Headless only: it says nothing about what the player sees.
use molehill_sim::{world::{World, Event, BITES, TUNE}, mat::{Mat, Kind, CPM, S, NMAT}, craft::{drop_of, raw_bill, bill_of_station, Item, Station, ITEMS, NITEM}, survival::Mode};

/// The rungs of the ladder, each with the stations and things it adds (cumulative down the list).
const RUNGS: &[(&str, &[Station], &[(Item, u32)])] = &[
    ("bench", &[Station::Workbench], &[]),
    ("forge+line", &[Station::Forge], &[(Item::CopperWire, 8)]),
    ("drill", &[Station::Workshop], &[(Item::Drill, 1)]),
    ("thrummer", &[], &[(Item::Thrummer, 1)]),
    ("bore", &[], &[(Item::Bore, 1)]),
];
/// The cumulative raw bill of each rung.
fn rung_bills() -> Vec<[u32; NITEM]> {
    let mut acc = [0u32; NITEM]; let mut v = vec![];
    for &(_, st, its) in RUNGS {
        for &s in st { let b = bill_of_station(s); for k in 0..NITEM { acc[k] += b[k]; } }
        for &(it, n) in its { let b = raw_bill(it, n); for k in 0..NITEM { acc[k] += b[k]; } }
        v.push(acc);
    }
    v
}
fn bill_str(b: &[u32; NITEM]) -> String { ITEMS.iter().filter(|&&it| b[it as usize] > 0).map(|&it| format!("{} {}", b[it as usize], it.name().to_lowercase())).collect::<Vec<_>>().join(", ") }
fn ladder() {
    println!("ladder (raw, cumulative; a coal per cold-forge job):");
    for (k, b) in rung_bills().iter().enumerate() { println!("  {:<10} {}", RUNGS[k].0, bill_str(b)); }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let mins: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(15);
    // a third argument runs only that seed (the bot alone)
    if let Some(one) = a.get(3).and_then(|s| s.parse::<u32>().ok()) { bot(one, mins); return; }
    ladder();
    for seed in 1..=n { routes(seed * 7919); }
    for seed in 1..=n { bot(seed * 7919, mins); }
}

fn new_world(seed: u32) -> World {
    let mut w = World::new(1024 * S, 512 * S, seed); w.disasters.enabled = false; w.set_mode(Mode::Survival); w.gear.lamp = true; w
}

fn routes(seed: u32) {
    let w = new_world(seed); let c = CPM as f32;
    let (mx, my) = (w.moles[0].x as i64, w.moles[0].y as i64);
    let mut near = [(f32::MAX, 0usize, 0usize); NMAT];
    for y in 0..w.g.h { for x in 0..w.g.w { let m = w.g.get(x, y); let k = m as usize;
        let d = (((x as i64 - mx).pow(2) + (y as i64 - my).pow(2)) as f32).sqrt() / c; if d < near[k].0 { near[k] = (d, x, y); } } }
    let mut line = format!("seed {seed:>6} routes:");
    let mut add = |name: &str, t: Option<(f32, usize, usize)>| { match t {
        Some((d, x, y)) if d < f32::MAX => { let (s, gate) = cost(&w, (mx as usize, my as usize), (x, y)); line += &format!("  {name} {d:.0}m/{s:.0}s{}", if gate > 0 { format!("!{gate}") } else { String::new() }); }
        _ => line += &format!("  {name} -"),
    } };
    // stone: the nearest rock the pickaxe digs into stone
    let stone = (0..NMAT).filter(|&k| drop_of(Mat::from_u8(k as u8)) == Some(Item::Stone)).map(|k| near[k]).fold((f32::MAX, 0, 0), |a, b| if b.0 < a.0 { b } else { a });
    add("stone", Some(stone));
    for (name, m) in [("roots", Mat::Roots), ("copper", Mat::CopperOre), ("tin", Mat::TinOre), ("clay", Mat::Clay), ("coal", Mat::Coal), ("fungus", Mat::Fungus), ("iron", Mat::IronOre), ("lava", Mat::Lava)] { add(name, Some(near[m as usize])); }
    let g = w.grottos.iter().map(|g| { let (x, y) = ((g.cx - w.ox).max(0) as usize, (g.cy - w.oy).max(0) as usize); ((((x as i64 - mx).pow(2) + (y as i64 - my).pow(2)) as f32).sqrt() / c, x, y) }).fold((f32::MAX, 0, 0), |a, b| if b.0 < a.0 { b } else { a });
    add("grotto", Some(g));
    let s = w.sites.iter().map(|s| { let (x, y) = ((s.x0 - w.ox).max(0) as usize + s.w / 2, (s.y0 - w.oy).max(0) as usize + s.h / 2); ((((x as i64 - mx).pow(2) + (y as i64 - my).pow(2)) as f32).sqrt() / c, x, y) }).fold((f32::MAX, 0, 0), |a, b| if b.0 < a.0 { b } else { a });
    add("site", Some(s));
    println!("{line}");
}

/// Straight-line time (s) from `a` to `b` with the kit's pickaxe, and the cells on the way it can't cut.
fn cost(w: &World, a: (usize, usize), b: (usize, usize)) -> (f32, u32) {
    let (dx, dy) = (b.0 as f32 - a.0 as f32, b.1 as f32 - a.1 as f32); let len = dx.hypot(dy).max(1.);
    let r = BITES[1]; let (mut t, mut gate, mut seg_max, mut seg) = (0f32, 0u32, 0u32, 0f32);
    for k in 0..len as usize {
        let (x, y) = ((a.0 as f32 + dx * k as f32 / len) as usize, (a.1 as f32 + dy * k as f32 / len) as usize);
        let i = w.g.idx(x, y); let m = w.g.m(i);
        if m.kind() == Kind::Solid || m.kind() == Kind::Powder {
            match w.mine_time(i) { Some(mt) => seg_max = seg_max.max(mt), None => gate += 1 }
            seg += 1.; if seg >= r { t += seg_max as f32 / 60.; seg = 0.; seg_max = 0; }
        } else { t += 1. / TUNE.run; }
    }
    (t + seg_max as f32 / 60., gate)
}

fn bot(seed: u32, mins: u32) {
    let mut w = new_world(seed); let c = CPM as f32;
    let ticks = mins as u64 * 3600; let (mut dir, mut last_x, mut stuck) = (1i32, 0usize, 0u32);
    let (mut refusals, mut digs, mut last_log, mut quiet_max, mut quiet_at) = (0u32, 0u32, 0u64, 0u64, 0u64);
    let mut seen_log = 0usize; let mut refused_at = 0u64; let mut was_under = false; let mut bare = 0u32; let mut causes: Vec<String> = vec![];
    // the last 5 s alive: head in water, stale air (peak), head in solid (buried)
    let mut recent: std::collections::VecDeque<(bool, f32, bool)> = Default::default();
    // the first tick the pack could pay each rung; the first tick the light went out, and hunger hit zero
    let bills = rung_bills(); let mut paid: Vec<Option<u64>> = vec![None; bills.len()]; let (mut dark, mut starving) = (None, None);
    println!("\nseed {seed} bot ({mins} min, Survival, hunger on, lamp on):");
    for t in 0..ticks {
        let (x, y) = (w.moles[0].x, w.moles[0].y);
        if !w.moles[0].alive { w.input = Default::default(); w.step(); continue; }
        // dig down for the first 60 s and 15 s of every 2 minutes, unless the floor refused in the last 5 s;
        // otherwise tunnel sideways at body height. Like a careful player: short of breath, swim up.
        let down = (t < 3600 || t % 7200 < 900) && t >= refused_at + 300;
        w.input = Default::default(); w.input.tool = 1;
        let under = w.g.get(x, y - S).kind() == Kind::Liquid;
        if under && !was_under { dir = -dir; } was_under = under;
        if under || w.moles[0].breath < 95. { w.input.up = true; w.input.jump = t % 20 < 10; w.input.right = dir > 0; w.input.left = dir < 0; }
        else if down { w.input.mine = Some((x, y + 2 * S)); }
        else {
            if dir > 0 { w.input.right = true } else { w.input.left = true }
            let ahead = (x as i32 + dir * 3 * S as i32).clamp(1, w.g.w as i32 - 2) as usize;
            w.input.mine = Some((ahead, if (t / 15) % 2 == 0 { y - S } else { y - 3 * S }));
            if t % 240 == 0 { if x.abs_diff(last_x) < S { stuck += 1; if stuck >= 2 { dir = -dir; stuck = 0; } } else { stuck = 0; } last_x = x; }
        }
        if w.moles[0].alive { let hi = w.g.idx(x, y - S); let hm = w.g.m(hi);
            recent.push_back((hm == Mat::Water, w.air.at(x, y - S), hm.kind() == Kind::Solid || hm.kind() == Kind::Powder)); if recent.len() > 300 { recent.pop_front(); } }
        let before = w.events.len(); let was_alive = w.moles[0].alive;
        w.step();
        // a death: where the pack went (in liquid or dry; how far from where the mole died)
        if was_alive && !w.moles[0].alive && !w.drops.is_empty() {
            let wet = w.drops.iter().filter(|d| { let (dx, dy) = (d.px as usize, d.py as usize); dx < w.g.w && dy < w.g.h && w.g.get(dx, dy).kind() == Kind::Liquid }).count();
            let (px, py) = w.drops.iter().fold((0., 0.), |a, d| (a.0 + d.px, a.1 + d.py)); let n = w.drops.len() as f32; let (px, py) = (px / n, py / n);
            let far = (px - w.pl.px).hypot(py - w.pl.py) / c;
            let (back, gate) = cost(&w, (w.g.w / 2, (w.pl.spawn.1 - w.oy).clamp(0, w.g.h as i64 - 1) as usize), (px as usize, py as usize));
            causes.push(format!("pack at t {:.0}s: {} piles, {wet} in liquid, {far:.1} m from the death spot, {:.0} m down; way back ~{back:.0}s{}", t as f32 / 60., w.drops.len(), w.depth_m(px as usize, py as usize), if gate > 0 { format!(" ({gate} gated cells)") } else { String::new() }));
        }
        // a respawn: is the burrow flooded? (water over the head, and how deep before air)
        if !was_alive && w.moles[0].alive {
            let (x, y) = (w.moles[0].x, w.moles[0].y); let head = y - 2 * S;
            let mut k = head; while k > 0 && w.g.get(x, k).kind() == Kind::Liquid { k -= 1; }
            let wet = w.g.get(x, head).kind() == Kind::Liquid || w.g.get(x, y - S).kind() == Kind::Liquid;
            if wet { causes.push(format!("respawn t {:.0}s in water: {:.1} m of water over the head", t as f32 / 60., (head - k) as f32 / c)); }
        }
        let (hp, br, hu) = (w.moles[0].hp, w.moles[0].breath, w.moles[0].hunger);
        for e in &w.events[before.min(w.events.len())..] { match e { Event::Crack => { refusals += 1; if down { refused_at = t; } } Event::Dig => digs += 1,
            Event::Death => { let n = recent.len().max(1) as f32;
                let (wet, air, buried) = (recent.iter().filter(|r| r.0).count() as f32 / n, recent.iter().map(|r| r.1).fold(0f32, f32::max), recent.iter().filter(|r| r.2).count() as f32 / n);
                causes.push(format!("t {:.0}s at {:.0} m: breath {br:.0} hunger {hu:.0} hp {hp:.0}; last 5 s: head in water {:.0}%, stale air peak {air:.2}, buried {:.0}%", t as f32 / 60., w.depth_m(x, y), wet * 100., buried * 100.)); recent.clear(); }
            _ => {} } }
        if w.events.len() > 4096 { w.events.clear(); }
        if t % 600 == 0 { for (k, b) in bills.iter().enumerate() { if paid[k].is_none() && (0..NITEM).all(|i| w.inv[i] >= b[i]) { paid[k] = Some(t); } } }
        if dark.is_none() && w.gear.lamp && !w.lamp_lit() { dark = Some(t); }
        if starving.is_none() && w.moles[0].alive && w.moles[0].hunger <= 0. { starving = Some(t); }
        // the log as a timeline
        while seen_log < w.log.len() {
            let (lt, l) = &w.log[seen_log];
            if *lt > last_log + 1 { let q = lt - last_log; if q > quiet_max { quiet_max = q; quiet_at = last_log; } }
            last_log = *lt; seen_log += 1;
            let d = w.depth_m(x, y);
            if l.contains(" has died.") { bare += 1; }
            if !l.starts_with("Ate a ") || l.contains("(0 left)") { println!("  t {:>5.0}s {:>5.1}m  {l}", *lt as f32 / 60., d); }
        }
        if w.log.len() > 50 { let k = w.log.len() - 50; w.log.drain(..k); seen_log -= k; }
    }
    if ticks > last_log { let q = ticks - last_log; if q > quiet_max { quiet_max = q; quiet_at = last_log; } }
    let inv = |it: Item| w.inv[it as usize];
    println!("  end: alive {} deaths {} depth {:.0}m (max {:.0}m) x {:+.0}m hunger {:.0} oil {:.0}s+{}  quest {}",
        w.moles[0].alive, w.pl.deaths, w.depth_m(w.moles[0].x, w.moles[0].y), w.max_depth, (w.moles[0].x as f32 - w.g.w as f32 / 2.) / c, w.moles[0].hunger, w.lamp_oil, w.oil_measures(), w.quest);
    println!("  pack: stone {} wood {} copper {} tin {} clay {} coal {} fungus {} grub {} root {} iron {}",
        inv(Item::Stone), inv(Item::Wood), inv(Item::CopperOre), inv(Item::TinOre), inv(Item::Clay), inv(Item::Coal), inv(Item::Fungus), inv(Item::Grub), inv(Item::Root), inv(Item::IronOre));
    let at = |t: Option<u64>| t.map_or("never".to_string(), |t| format!("{:.1} min", t as f32 / 3600.));
    println!("  ladder: {}", RUNGS.iter().enumerate().map(|(k, r)| format!("{} {}", r.0, at(paid[k]))).collect::<Vec<_>>().join(", "));
    println!("  light out first at {}, hunger first at zero at {}", at(dark), at(starving));
    for c in &causes { println!("  death {c}"); }
    println!("  bare 'has died' lines: {}", bare);
    println!("  dig strikes {digs}, refused strikes {refusals}, longest quiet log stretch {:.0} s from t {:.0} s", quiet_max as f32 / 60., quiet_at as f32 / 60.);
}
