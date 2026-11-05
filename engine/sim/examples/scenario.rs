//! Headless, deterministic scenario runner (no window, no GPU).
//! Usage: cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/*.scn [--json]
//! See TESTING.md for the file format. Exit code 1 if any assertion fails.
use molehill_sim::mat::{Mat, NMAT};
use molehill_sim::world::World;

fn mat_by_name(s: &str) -> Option<Mat> { (0..NMAT as u8).map(Mat::from_u8).find(|m| format!("{m:?}").eq_ignore_ascii_case(s)) }
fn mat_char(c: char) -> Option<Mat> { Some(match c { '.' | '@' => Mat::Empty, '#' => Mat::Granite, 'B' => Mat::Bedrock, 'r' => Mat::Rock, 'd' => Mat::Dirt, 'c' => Mat::Clay, 's' => Mat::Sand, 'g' => Mat::Gravel, 'w' => Mat::Water, 'l' => Mat::Lava, 'o' => Mat::Coal, 'i' => Mat::IronOre, 'b' => Mat::WoodBeam, 'k' => Mat::StoneBrick, 'I' => Mat::IronBeam, _ => return None }) }
fn ch_of(m: Mat) -> char { match m { Mat::Empty => '.', Mat::Granite => '#', Mat::Bedrock => 'B', Mat::Rock => 'r', Mat::Dirt => 'd', Mat::Clay => 'c', Mat::Sand => 's', Mat::Gravel => 'g', Mat::Water => 'w', Mat::Lava => 'l', Mat::Coal => 'o', Mat::IronOre => 'i', Mat::WoodBeam => 'b', Mat::StoneBrick => 'k', Mat::IronBeam => 'I', Mat::Steam => '~', _ => '?' } }

enum Act { Brush(u64, f32), Place(u64, i32, i32, String), Hold(u64, u64, Vec<String>), Mine(u64, u64, i32, i32), Tool(u64, u8), Explode(u64, i32, i32, f32), Dump(u64), Ascii(u64), Expect(u64, String, String, f64, usize) }

struct Run { w: World, ox: usize, oy: usize, mw: usize, mh: usize, ax: usize, ay: usize, sx: f32, sy: f32, top: f32, events: Vec<String> }
impl Run {
    fn var(&self, v: &str) -> Option<f64> {
        let p = &self.w.pl; let m = &self.w.moles[0];
        Some(match v {
            "px" => p.px as f64, "py" => p.py as f64, "vx" => p.vx as f64, "vy" => p.vy as f64,
            "dx" => (p.px - self.sx) as f64, "dy" => (p.py - self.sy) as f64, "rise" => (self.sy - self.top) as f64,
            "grounded" => p.grounded as u8 as f64, "cling" => p.cling as f64, "hp" => m.hp as f64, "breath" => m.breath as f64,
            "deaths" => p.deaths as f64, "crumbled" => self.w.crumbled as f64, "tick" => self.w.tick as f64,
            _ if v.starts_with("count:") => { let mt = mat_by_name(&v[6..])?; let mut n = 0; for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { if self.w.g.get(x, y) == mt { n += 1; } } } n as f64 }
            _ if v.starts_with("static:") => { let mt = mat_by_name(&v[7..])?; let mut n = 0; for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { let i = self.w.g.idx(x, y); if self.w.g.get(x, y) == mt && !self.w.g.loose(i) { n += 1; } } } n as f64 }
            _ if v.starts_with("util:") => { // util:DX,DY  stress utilisation of a cell (1.0 = at capacity)
                let a: Vec<&str> = v[5..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                self.w.st.util[self.w.g.idx((self.ax as i32 + dx) as usize, (self.ay as i32 + dy) as usize)] as f64 / 128. }
            _ if v.starts_with("is:") => { // is:DX,DY,Mat  (relative to @)
                let a: Vec<&str> = v[3..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                (self.w.g.get((self.ax as i32 + dx) as usize, (self.ay as i32 + dy) as usize) == mat_by_name(a.get(2)?)?) as u8 as f64 }
            _ if v.starts_with("inv:") => { let i = molehill_sim::craft::ITEMS.iter().position(|it| format!("{it:?}").eq_ignore_ascii_case(&v[4..]))?; self.w.inv[i] as f64 }
            _ => return None })
    }
    fn json(&self) -> String {
        let p = &self.w.pl; let m = &self.w.moles[0];
        let mut counts = vec![0u32; NMAT]; for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { counts[self.w.g.get(x, y) as usize] += 1; } }
        let c: Vec<String> = counts.iter().enumerate().filter(|(_, n)| **n > 0).map(|(i, n)| format!("\"{:?}\":{n}", Mat::from_u8(i as u8))).collect();
        let inv: Vec<String> = molehill_sim::craft::ITEMS.iter().enumerate().filter(|(i, _)| self.w.inv[*i] > 0).map(|(i, it)| format!("\"{it:?}\":{}", self.w.inv[i])).collect();
        format!("{{\"tick\":{},\"player\":{{\"px\":{:.3},\"py\":{:.3},\"vx\":{:.3},\"vy\":{:.3},\"grounded\":{},\"cling\":{},\"hp\":{:.1},\"breath\":{:.1},\"deaths\":{}}},\"inv\":{{{}}},\"mats\":{{{}}},\"events\":[{}]}}",
            self.w.tick, p.px, p.py, p.vx, p.vy, p.grounded, p.cling, m.hp, m.breath, p.deaths, inv.join(","), c.join(","), self.events.iter().map(|e| format!("\"{e}\"")).collect::<Vec<_>>().join(","))
    }
    fn ascii(&self) -> String {
        let mut s = String::new(); let (mx, my) = (self.w.moles[0].x, self.w.moles[0].y);
        for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { s.push(if x == mx && (my - 2..=my).contains(&y) { 'M' } else { ch_of(self.w.g.get(x, y)) }); } s.push('\n'); } s
    }
}

fn run_file(path: &str, json: bool) -> bool {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let (mut seed, mut map, mut acts, mut gives, mut ticks) = (11u32, Vec::<String>::new(), Vec::new(), Vec::<(String, u32)>::new(), 0u64);
    let mut lines = src.lines().enumerate();
    let rng = |s: &str| -> (u64, u64) { match s.split_once('-') { Some((a, b)) => (a.parse().unwrap(), b.parse().unwrap()), None => { let a = s.parse().unwrap(); (a, a) } } };
    while let Some((ln, l)) = lines.next() {
        let l = l.split("//").next().unwrap().trim(); if l.is_empty() { continue; }
        let t: Vec<&str> = l.split_whitespace().collect();
        match t[0] {
            "seed" => seed = t[1].parse().unwrap(),
            "map" => { for (_, ml) in lines.by_ref() { if ml.trim() == "end" { break; } map.push(ml.trim_end().to_string()); } }
            "give" => gives.push((t[1].into(), t[2].parse().unwrap())),
            "ticks" => ticks = t[1].parse().unwrap(),
            "hold" => { let (a, b) = rng(t[1]); acts.push(Act::Hold(a, b, t[2..].iter().map(|s| s.to_string()).collect())); }
            "mine" => { let (a, b) = rng(t[1]); acts.push(Act::Mine(a, b, t[2].parse().unwrap(), t[3].parse().unwrap())); }
            "tool" => acts.push(Act::Tool(t[1].parse().unwrap(), t[2].parse().unwrap())),
            "explode" => acts.push(Act::Explode(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].parse().unwrap())),
            "brush" => acts.push(Act::Brush(t[1].parse().unwrap(), t[2].parse().unwrap())),
            "place" => acts.push(Act::Place(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "dump" => acts.push(Act::Dump(t[1].parse().unwrap())),
            "ascii" => acts.push(Act::Ascii(t[1].parse().unwrap())),
            "expect" => acts.push(Act::Expect(t[1].parse().unwrap(), t[2].into(), t[3].into(), t[4].parse().unwrap(), ln + 1)),
            o => panic!("{path}:{}: unknown directive {o}", ln + 1),
        }
    }
    for a in &acts { ticks = ticks.max(match a { Act::Hold(_, b, _) | Act::Mine(_, b, _, _) => *b, Act::Tool(t, _) | Act::Brush(t, _) | Act::Place(t, ..) | Act::Explode(t, ..) | Act::Dump(t) | Act::Ascii(t) | Act::Expect(t, ..) => *t }); }
    let mut w = World::new(512, 256, seed); w.disasters.enabled = false; w.drops.clear(); w.torches.clear(); w.farms.clear();
    let (sx0, sy0) = (w.moles[0].x, w.moles[0].y);
    let mh = map.len(); let mw = map.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let (mut ax, mut ay) = (0, 0); for (y, l) in map.iter().enumerate() { if let Some(x) = l.chars().position(|c| c == '@') { ax = x; ay = y; } }
    let (ox, oy) = (sx0 - ax, sy0 - ay);
    // solid granite frame around the map so the generated world can't crumble into the scenario
    for y in oy.saturating_sub(20)..(oy + mh + 20).min(w.g.h) { for x in ox.saturating_sub(20)..(ox + mw + 20).min(w.g.w) { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); w.g.flags[i] = 0; } }
    for (y, l) in map.iter().enumerate() { let cs: Vec<char> = l.chars().collect(); for x in 0..mw { let c = *cs.get(x).unwrap_or(&'#');
        let m = mat_char(c).unwrap_or_else(|| panic!("{path}: unknown map char '{c}'")); let i = w.g.idx(ox + x, oy + y); w.g.set(i, m); w.g.flags[i] = 0; } }
    for (n, k) in gives { let i = molehill_sim::craft::ITEMS.iter().position(|it| format!("{it:?}").eq_ignore_ascii_case(&n)).unwrap_or_else(|| panic!("unknown item {n}")); w.inv[i] = k; }
    w.moles[0].x = ox + ax; w.moles[0].y = oy + ay; w.pl.sync = None; w.pl = Default::default(); w.pl.fall_from = oy + ay; w.moles[0].hp = 100.; w.pl.spawn = (ox + ax, w.oy + (oy + ay) as i64);
    let mut r = Run { ox, oy, mw, mh, ax: ox + ax, ay: oy + ay, sx: 0., sy: 0., top: f32::MAX, events: vec![], w };
    let (mut ok, mut fails) = (0, 0);
    let t0 = r.w.tick;
    for tick in 0..=ticks {
        if tick > 0 {
            let mut inp = molehill_sim::world::Input::default(); inp.tool = r.w.input.tool; inp.brush = r.w.input.brush;
            for a in &acts { match a {
                Act::Hold(a, b, ks) if (*a..=*b).contains(&tick) => for k in ks { match k.as_str() { "left" => inp.left = true, "right" => inp.right = true, "jump" => inp.jump = true, "up" => inp.up = true, "down" => inp.down = true, o => panic!("unknown key {o}") } },
                Act::Mine(a, b, dx, dy) if (*a..=*b).contains(&tick) => inp.mine = Some(((r.ax as i32 + dx) as usize, (r.ay as i32 + dy) as usize)),
                Act::Tool(t, n) if *t == tick => inp.tool = *n,
                Act::Brush(t, b) if *t == tick => inp.brush = *b,
                Act::Explode(t, dx, dy, rr) if *t == tick => { let (x, y) = ((r.ax as i32 + dx) as usize, (r.ay as i32 + dy) as usize); r.w.explode(x, y, *rr, 6); }
                Act::Place(t, dx, dy, it) if *t == tick => {
                    let item = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(it)).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let (x, y) = ((r.ax as i32 + dx) as usize, (r.ay as i32 + dy) as usize);
                    if let Err(e) = r.w.place_brush(x, y, item, inp.brush) { println!("  NOTE {path} tick {tick}: place {it} at {dx},{dy} failed: {e}"); }
                }
                _ => {} } }
            r.w.input = inp; r.w.step();
            for e in std::mem::take(&mut r.w.events) { r.events.push(format!("{}:{e:?}", tick)); }
            r.top = r.top.min(r.w.pl.py);
        } else { r.w.step(); r.w.events.clear(); r.sx = r.w.pl.px; r.sy = r.w.pl.py; r.top = r.sy; }
        for a in &acts { match a {
            Act::Dump(t) if *t == tick => println!("DUMP {path} {}", r.json()),
            Act::Ascii(t) if *t == tick => println!("ASCII {path} tick {tick}\n{}", r.ascii()),
            Act::Expect(t, v, op, n, ln) if *t == tick => {
                let val = r.var(v).unwrap_or_else(|| panic!("{path}:{ln}: unknown variable {v}"));
                let pass = match op.as_str() { "<" => val < *n, "<=" => val <= *n, ">" => val > *n, ">=" => val >= *n, "==" => (val - n).abs() < 1e-6, "!=" => (val - n).abs() >= 1e-6, o => panic!("bad op {o}") };
                if pass { ok += 1 } else { fails += 1; println!("  FAIL {path}:{ln} tick {tick}: {v} = {val:.3}, expected {op} {n}"); }
            }
            _ => {} } }
    }
    let _ = t0;
    if json { println!("FINAL {path} {}", r.json()); }
    println!("{} {path} ({ok} passed, {fails} failed, {ticks} ticks)", if fails == 0 { "PASS" } else { "FAIL" });
    fails == 0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect(); let json = args.iter().any(|a| a == "--json");
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if files.is_empty() { eprintln!("usage: scenario <file.scn>... [--json]"); std::process::exit(2); }
    let res: Vec<bool> = files.iter().map(|f| run_file(f, json)).collect();
    let bad = res.iter().filter(|r| !**r).count();
    println!("{} scenarios, {} failed", res.len(), bad); if bad > 0 { std::process::exit(1); }
}
