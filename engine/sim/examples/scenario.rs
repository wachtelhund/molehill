//! Headless, deterministic scenario runner (no window, no GPU).
//! Usage: cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/*.scn [--json]
//! See TESTING.md for the file format. Exit code 1 if any assertion fails.
//! Scenarios are written in tiles of 0.25 m (the original cell size): every map character is stamped as
//! a K×K block of cells (K = mat::S), offsets and radii are scaled up, and positions, speeds, counts and
//! cell lookups are reported back in tiles, so a scenario means the same thing at any resolution.
use molehill_sim::mat::{Mat, NMAT, S as K};
use molehill_sim::world::World;

fn item_by_name(s: &str) -> Option<molehill_sim::craft::Item> { molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(s)) }
fn mat_by_name(s: &str) -> Option<Mat> { (0..NMAT as u8).map(Mat::from_u8).find(|m| format!("{m:?}").eq_ignore_ascii_case(s)) }
fn mat_char(c: char) -> Option<Mat> { Some(match c { '.' | '@' => Mat::Empty, '#' => Mat::Granite, 'B' => Mat::Bedrock, 'r' => Mat::Rock, 'd' => Mat::Dirt, 'c' => Mat::Clay, 's' => Mat::Sand, 'g' => Mat::Gravel, 'w' => Mat::Water, 'l' => Mat::Lava, 'o' => Mat::Coal, 'i' => Mat::IronOre, 'b' => Mat::WoodBeam, 'k' => Mat::StoneBrick, 'I' => Mat::IronBeam, 'a' => Mat::Basalt, _ => return None }) }
fn ch_of(m: Mat) -> char { match m { Mat::Empty => '.', Mat::Granite => '#', Mat::Bedrock => 'B', Mat::Rock => 'r', Mat::Dirt => 'd', Mat::Clay => 'c', Mat::Sand => 's', Mat::Gravel => 'g', Mat::Water => 'w', Mat::Lava => 'l', Mat::Coal => 'o', Mat::IronOre => 'i', Mat::WoodBeam => 'b', Mat::StoneBrick => 'k', Mat::IronBeam => 'I', Mat::Basalt => 'a', Mat::Steam => '~', _ => '?' } }

enum Act { Throw(u64, String, f32, f32, f32), Loose(u64, String, i32, i32, f32, f32), Brush(u64, f32), Place(u64, i32, i32, String), Put(u64, i32, i32, String), Hold(u64, u64, Vec<String>), Mine(u64, u64, i32, i32), Dig(u64, u64, i32, i32), Tool(u64, u8), Explode(u64, i32, i32, f32), Dump(u64), Ascii(u64), Device(u64, i32, i32, String), Use(u64, i32, i32), Pickup(u64, i32, i32), Wire(u64, i32, i32, i32, i32), SaveLoad(u64), Slide(u64), Expect(u64, String, String, f64, usize) }

struct Run { w: World, ox: usize, oy: usize, mw: usize, mh: usize, ax: usize, ay: usize, sx: f32, sy: f32, top: f32, events: Vec<String> }
impl Run {
    fn var(&self, v: &str) -> Option<f64> {
        let p = &self.w.pl; let m = &self.w.moles[0]; let kf = K as f64;
        Some(match v {
            // px, py: tiles from the map's top-left corner; velocities in tiles/s
            "px" => (p.px - self.ox as f32) as f64 / kf, "py" => (p.py - self.oy as f32) as f64 / kf, "vx" => p.vx as f64 / kf, "vy" => p.vy as f64 / kf,
            "dx" => (p.px - self.sx) as f64 / kf, "dy" => (p.py - self.sy) as f64 / kf, "rise" => (self.sy - self.top) as f64 / kf,
            "grounded" => p.grounded as u8 as f64, "cling" => p.cling as f64, "hp" => m.hp as f64, "breath" => m.breath as f64,
            "deaths" => p.deaths as f64, "awake" => self.awake() as f64, "crumbled" => self.w.crumbled as f64 / (kf * kf), "tick" => self.w.tick as f64,
            "flash" => p.flash as f64, "shake" => self.w.shake as f64,
            // how far the body moved this tick, in tiles (catches teleport pops a velocity check misses)
            "step" => ((p.px - p.ppx).powi(2) + (p.py - p.ppy).powi(2)).sqrt() as f64 / kf,
            _ if v.starts_with("count:") => { let mt = mat_by_name(&v[6..])?; let mut n = 0; for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { if self.w.g.get(x, y) == mt { n += 1; } } } n as f64 / (kf * kf) }
            _ if v.starts_with("static:") => { let mt = mat_by_name(&v[7..])?; let mut n = 0; for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { let i = self.w.g.idx(x, y); if self.w.g.get(x, y) == mt && !self.w.g.loose(i) { n += 1; } } } n as f64 / (kf * kf) }
            _ if v.starts_with("util:") => { // util:DX,DY  stress utilisation of a cell (1.0 = at capacity)
                let a: Vec<&str> = v[5..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                self.tile(dx, dy).map(|i| self.w.st.util[i]).max()? as f64 / 128. }
            _ if v.starts_with("strain:") => { // strain:DX,DY  hover strain level: 0 idle, 1 bearing, 2 straining, 3 overloaded, 4 cracking
                let a: Vec<&str> = v[7..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                self.tile(dx, dy).map(|i| molehill_sim::tips::strain_at(&self.w, i)).max()? as f64 }
            _ if v.starts_with("is:") => { // is:DX,DY,Mat  (relative to @)
                let a: Vec<&str> = v[3..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                let mt = mat_by_name(a.get(2)?)?; (2 * self.tile(dx, dy).filter(|&i| self.w.g.m(i) == mt).count() >= K * K) as u8 as f64 } // most of the tile
            _ if v.starts_with("inv:") => { let i = molehill_sim::craft::ITEMS.iter().position(|it| format!("{it:?}").eq_ignore_ascii_case(&v[4..]))?; self.w.inv[i] as f64 }
            _ if v.starts_with("dev:") || v.starts_with("lit:") || v.starts_with("wire:") || v.starts_with("on:") => { // dev:DX,DY,Name  lit:DX,DY  wire:DX,DY  on:DX,DY (powered)
                let (k, rest) = v.split_once(':')?; let a: Vec<&str> = rest.split(',').collect(); let (x, y) = self.at(a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                let b = self.w.logic.block_of(x, y); let l = &self.w.logic;
                match k { "dev" => (format!("{:?}", molehill_sim::logic::Dev::from_u8(l.dev[b])).eq_ignore_ascii_case(a.get(2)?)) as u8 as f64, "lit" => l.lit(b) as u8 as f64, "wire" => l.wire[b] as f64, _ => l.powered[b] as f64 } }
            // a loose item (the first of that kind): ix/iy its centre in tiles from the '@' tile's centre,
            // ivx/ivy tiles/s, irest 1 if asleep, ilit 1 if a lit charge; items:Name how many lie loose
            _ if v.starts_with("items:") => { let it = item_by_name(&v[6..])?; self.w.drops.iter().filter(|d| d.item == it).map(|d| d.n).sum::<u32>() as f64 }
            _ if ["ix:", "iy:", "ivx:", "ivy:", "irest:", "ilit:"].iter().any(|p| v.starts_with(p)) => {
                let (k, name) = v.split_once(':')?; let it = item_by_name(name)?; let d = self.w.drops.iter().find(|d| d.item == it)?;
                let (cx, cy) = (self.ax as f32 + K as f32 / 2., self.ay as f32 + K as f32 / 2.);
                match k { "ix" => ((d.px - cx) as f64) / kf, "iy" => ((d.py - cy) as f64) / kf, "ivx" => d.vx as f64 / kf, "ivy" => d.vy as f64 / kf,
                    "irest" => d.asleep() as u8 as f64, _ => (d.fuse > 0) as u8 as f64 } }
            _ if v.starts_with("ev:") => self.events.iter().filter(|e| e.split(':').nth(1) == Some(&v[3..])).count() as f64,
            _ => return None })
    }
    /// The cell nearest the centre of the tile (dx, dy) from '@'.
    fn at(&self, dx: i32, dy: i32) -> (usize, usize) { let k = K as i32; ((self.ax as i32 + dx * k + k / 2) as usize, (self.ay as i32 + dy * k + k / 2) as usize) }
    /// Every cell of the tile (dx, dy) from '@'.
    fn tile(&self, dx: i32, dy: i32) -> impl Iterator<Item = usize> + '_ { let k = K as i32; let (x0, y0) = (self.ax as i32 + dx * k, self.ay as i32 + dy * k);
        (0..k * k).map(move |j| self.w.g.idx((x0 + j % k) as usize, (y0 + j / k) as usize)) }
    /// CA chunks overlapping the map rectangle that are still awake.
    fn awake(&self) -> usize {
        use molehill_sim::grid::CHUNK; let g = &self.w.g; let mut n = 0;
        for cy in self.oy / CHUNK..=(self.oy + self.mh - 1) / CHUNK { for cx in self.ox / CHUNK..=(self.ox + self.mw - 1) / CHUNK {
            if g.awake[cy * g.cx + cx].load(std::sync::atomic::Ordering::Relaxed) != 0 { n += 1; } } } n
    }
    fn json(&self) -> String {
        let p = &self.w.pl; let m = &self.w.moles[0];
        let mut counts = vec![0u32; NMAT]; for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { counts[self.w.g.get(x, y) as usize] += 1; } }
        let c0: Vec<String> = vec![];
        let inv: Vec<String> = molehill_sim::craft::ITEMS.iter().enumerate().filter(|(i, _)| self.w.inv[*i] > 0).map(|(i, it)| format!("\"{it:?}\":{}", self.w.inv[i])).collect();
        let c: Vec<String> = counts.iter().enumerate().filter(|(_, n)| **n > 0).map(|(i, n)| format!("\"{:?}\":{}", Mat::from_u8(i as u8), *n as f32 / (K * K) as f32)).collect(); let _ = c0;
        format!("{{\"tick\":{},\"player\":{{\"px\":{:.3},\"py\":{:.3},\"vx\":{:.3},\"vy\":{:.3},\"grounded\":{},\"cling\":{},\"hp\":{:.1},\"breath\":{:.1},\"deaths\":{}}},\"inv\":{{{}}},\"mats\":{{{}}},\"events\":[{}]}}",
            self.w.tick, self.var("px").unwrap(), self.var("py").unwrap(), self.var("vx").unwrap(), self.var("vy").unwrap(), p.grounded, p.cling, m.hp, m.breath, p.deaths, inv.join(","), c.join(","), self.events.iter().map(|e| format!("\"{e}\"")).collect::<Vec<_>>().join(","))
    }
    /// The map region, one character per tile (its most common material; M = the mole's body).
    fn ascii(&self) -> String { let kf = K as f64;
        let mut s = String::new(); let p = &self.w.pl;
        let (bx0, bx1) = (((p.px - molehill_sim::world::BODY_HW) / kf as f32).floor() as i64, ((p.px + molehill_sim::world::BODY_HW - 1e-3) / kf as f32).floor() as i64);
        let (by0, by1) = (((p.py - molehill_sim::world::BODY_H) / kf as f32).floor() as i64, ((p.py - 1e-3) / kf as f32).floor() as i64);
        let _ = (bx0, bx1);
        let mx = (p.px / kf as f32).floor() as i64;
        for ty in 0..self.mh / K { for tx in 0..self.mw / K {
            let (x0, y0) = (self.ox + tx * K, self.oy + ty * K); let (ax, ay) = ((x0 / K) as i64, (y0 / K) as i64);
            if ax == mx && (by0..=by1).contains(&ay) { s.push('M'); continue; }
            let mut n = [0u16; NMAT]; for j in 0..K * K { n[self.w.g.get(x0 + j % K, y0 + j / K) as usize] += 1; }
            let m = (0..NMAT).max_by_key(|&k| (n[k], std::cmp::Reverse(k))).unwrap(); s.push(ch_of(Mat::from_u8(m as u8)));
        } s.push('\n'); } s
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
            "dig" => { let (a, b) = rng(t[1]); acts.push(Act::Dig(a, b, t[2].parse().unwrap(), t[3].parse().unwrap())); }
            "tool" => acts.push(Act::Tool(t[1].parse().unwrap(), t[2].parse().unwrap())),
            "explode" => acts.push(Act::Explode(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].parse().unwrap())),
            // throw T Item DX DY POWER: throw one from the pack toward (DX, DY) tiles, POWER 0..1 of a full throw
            "throw" => acts.push(Act::Throw(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap(), t[5].parse().unwrap())),
            // drop T Item DX DY [VX VY]: a loose item centred on tile (DX, DY) from '@', moving at (VX, VY) tiles/s
            "drop" => acts.push(Act::Loose(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap(), t.get(5).map_or(0., |v| v.parse().unwrap()), t.get(6).map_or(0., |v| v.parse().unwrap()))),
            "brush" => acts.push(Act::Brush(t[1].parse().unwrap(), t[2].parse().unwrap())),
            "put" => acts.push(Act::Put(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "place" => acts.push(Act::Place(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "device" => acts.push(Act::Device(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "use" => acts.push(Act::Use(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            "pickup" => acts.push(Act::Pickup(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            "wire" => { let a: Vec<i32> = t[2..].iter().map(|v| v.parse().unwrap()).collect(); let (x1, y1) = if a.len() >= 4 { (a[2], a[3]) } else { (a[0], a[1]) }; acts.push(Act::Wire(t[1].parse().unwrap(), a[0], a[1], x1, y1)); }
            "saveload" => acts.push(Act::SaveLoad(t[1].parse().unwrap())),
            "slide" => acts.push(Act::Slide(t[1].parse().unwrap())),
            "dump" => acts.push(Act::Dump(t[1].parse().unwrap())),
            "ascii" => acts.push(Act::Ascii(t[1].parse().unwrap())),
            "expect" => acts.push(Act::Expect(t[1].parse().unwrap(), t[2].into(), t[3].into(), t[4].parse().unwrap(), ln + 1)),
            o => panic!("{path}:{}: unknown directive {o}", ln + 1),
        }
    }
    for a in &acts { ticks = ticks.max(match a { Act::Hold(_, b, _) | Act::Mine(_, b, _, _) | Act::Dig(_, b, _, _) => *b, Act::Tool(t, _) | Act::Throw(t, ..) | Act::Loose(t, ..) | Act::Brush(t, _) | Act::Place(t, ..) | Act::Put(t, ..) | Act::Explode(t, ..) | Act::Device(t, ..) | Act::Use(t, ..) | Act::Pickup(t, ..) | Act::Wire(t, ..) | Act::SaveLoad(t) | Act::Slide(t) | Act::Dump(t) | Act::Ascii(t) | Act::Expect(t, ..) => *t }); }
    let mut w = World::new(512 * K, 256 * K, seed); w.disasters.enabled = false; w.drops.clear(); w.torches.clear(); w.farms.clear();
    // tile-aligned spawn tile: the map is laid out in K×K tiles from here
    let (sx0, sy0) = (w.moles[0].x / K, w.moles[0].y / K);
    let mh = map.len(); let mw = map.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let (mut ax, mut ay) = (0, 0); for (y, l) in map.iter().enumerate() { if let Some(x) = l.chars().position(|c| c == '@') { ax = x; ay = y; } }
    let (tox, toy) = (sx0 - ax, sy0 - ay); let (ox, oy) = (tox * K, toy * K);
    // solid granite frame around the map so the generated world can't crumble into the scenario
    let fr = 20 * K;
    for y in oy.saturating_sub(fr)..(oy + mh * K + fr).min(w.g.h) { for x in ox.saturating_sub(fr)..(ox + mw * K + fr).min(w.g.w) { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); w.g.flags[i] = 0; } }
    for (y, l) in map.iter().enumerate() { let cs: Vec<char> = l.chars().collect(); for x in 0..mw { let c = *cs.get(x).unwrap_or(&'#');
        let m = mat_char(c).unwrap_or_else(|| panic!("{path}: unknown map char '{c}'"));
        for j in 0..K * K { let i = w.g.idx(ox + x * K + j % K, oy + y * K + j / K); w.g.set(i, m); w.g.flags[i] = 0; } } }
    for (n, k) in gives { let i = molehill_sim::craft::ITEMS.iter().position(|it| format!("{it:?}").eq_ignore_ascii_case(&n)).unwrap_or_else(|| panic!("unknown item {n}")); w.inv[i] = k; }
    // the mole stands centred on the '@' tile, feet on its bottom edge
    let (ax, ay) = (ox + ax * K, oy + ay * K); // '@' tile's top-left cell
    let (fx, fy) = (ax + K / 2, ay + K - 1); // feet cell
    w.moles[0].x = fx; w.moles[0].y = fy; w.pl = Default::default(); w.pl.fall_from = fy; w.moles[0].hp = 100.; w.pl.spawn = (fx, w.oy + fy as i64);
    w.pl.px = (ax as f32) + K as f32 / 2.; w.pl.py = (ay + K) as f32; w.pl.ppx = w.pl.px; w.pl.ppy = w.pl.py; w.pl.grounded = true; w.pl.sync = Some((fx, fy));
    let (mw, mh) = (mw * K, mh * K);
    let mut r = Run { ox, oy, mw, mh, ax, ay, sx: 0., sy: 0., top: f32::MAX, events: vec![], w };
    let (mut ok, mut fails) = (0, 0);
    let t0 = r.w.tick;
    for tick in 0..=ticks {
        if tick > 0 {
            let mut inp = molehill_sim::world::Input::default(); inp.tool = r.w.input.tool; inp.brush = r.w.input.brush;
            for a in &acts { match a {
                Act::Hold(a, b, ks) if (*a..=*b).contains(&tick) => for k in ks { match k.as_str() { "left" => inp.left = true, "right" => inp.right = true, "jump" => inp.jump = true, "up" => inp.up = true, "down" => inp.down = true, "sprint" => inp.sprint = true, o => panic!("unknown key {o}") } },
                Act::Mine(a, b, dx, dy) if (*a..=*b).contains(&tick) => inp.mine = Some(r.at(*dx, *dy)),
                Act::Dig(a, b, dx, dy) if (*a..=*b).contains(&tick) => { let m = &r.w.moles[0]; let k = K as i32; inp.mine = Some(((m.x as i32 + dx * k) as usize, (m.y as i32 + dy * k) as usize)); }
                Act::Tool(t, n) if *t == tick => inp.tool = *n,
                Act::Throw(t, it, dx, dy, pw) if *t == tick => { let item = item_by_name(it).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    if let Err(e) = r.w.throw_item(item, (*dx, *dy), *pw) { println!("  NOTE {path} tick {tick}: throw {it}: {e}"); } }
                Act::Loose(t, it, dx, dy, vx, vy) if *t == tick => { let item = item_by_name(it).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let kf = K as f32; let mut d = molehill_sim::world::Drop::at(r.ax as f32 + (*dx as f32 + 0.5) * kf, r.ay as f32 + (*dy as f32 + 0.5) * kf, item, 1);
                    d.vx = vx * kf; d.vy = vy * kf; r.w.drops.push(d); }
                Act::Brush(t, b) if *t == tick => inp.brush = *b * K as f32,
                Act::Explode(t, dx, dy, rr) if *t == tick => { let (x, y) = r.at(*dx, *dy); r.w.explode(x, y, *rr * K as f32, 6); }
                Act::Place(t, dx, dy, it) if *t == tick => {
                    let item = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(it)).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let (x, y) = r.at(*dx, *dy);
                    if let Err(e) = r.w.place_brush(x, y, item, inp.brush) { println!("  NOTE {path} tick {tick}: place {it} at {dx},{dy} failed: {e}"); }
                }
                Act::Device(t, dx, dy, n) if *t == tick => { let (x, y) = r.at(*dx, *dy);
                    let d = molehill_sim::logic::DEVS.iter().copied().find(|d| format!("{d:?}").eq_ignore_ascii_case(n)).unwrap_or_else(|| panic!("unknown device {n}"));
                    if let Err(e) = r.w.place_device(d, x, y) { println!("  NOTE {path} tick {tick}: device {n} at {dx},{dy}: {e}"); } }
                Act::Use(t, dx, dy) if *t == tick => { let (x, y) = r.at(*dx, *dy); if !r.w.use_device(x, y) { println!("  NOTE {path} tick {tick}: nothing to use at {dx},{dy}"); } }
                Act::Pickup(t, dx, dy) if *t == tick => { let (x, y) = r.at(*dx, *dy); r.w.pick_up(x, y); }
                Act::Wire(t, x0, y0, x1, y1) if *t == tick => { // a straight run of copperline, one per 1 m block
                    let n = ((x1 - x0).abs().max((y1 - y0).abs()) / 4).max(0);
                    for k in 0..=n { let f = |a: i32, b: i32| if n == 0 { a } else { a + (b - a) * k / n }; let (x, y) = r.at(f(*x0, *x1), f(*y0, *y1)); r.w.lay_wire(x, y); } }
                // the window slides away until only its bottom band is left, then back: the scenario's rows go
                // into stored bands and return (the mole is carried along, as if it had walked away and back)
                Act::Slide(t) if *t == tick => { let (m0, p0) = (r.w.moles[0].clone(), r.w.pl.clone()); let bh = molehill_sim::band::BH; let rows = ((r.w.g.h - bh) / bh * bh) as i64;
                    r.w.shift(rows); r.w.shift(-rows); r.w.moles[0] = m0; r.w.pl = p0; }
                Act::SaveLoad(t) if *t == tick => { let b = molehill_sim::save::save(&r.w); let mut w = molehill_sim::save::load(&b).unwrap_or_else(|e| panic!("{path}: reload failed: {e}")); w.disasters.enabled = false; r.w = w; }
                Act::Put(t, dx, dy, it) if *t == tick => {
                    let item = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(it)).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    // fill the whole tile, bottom row first, so each cell rests on the one below or a neighbour
                    let cells: Vec<usize> = r.tile(*dx, *dy).collect(); let mut err = None; let mut left = cells.clone();
                    for _ in 0..K * K { left.retain(|&i| { let (x, y) = (i % r.w.g.w, i / r.w.g.w); match r.w.place_block_at(x, y, item) { Ok(()) => false, Err(e) => { err = Some(e); true } } }); if left.is_empty() { break; } }
                    if !left.is_empty() { if let Some(e) = err { println!("  NOTE {path} tick {tick}: put {it} at {dx},{dy} failed: {e}"); } }
                }
                _ => {} } }
            r.w.input = inp; let pre = std::mem::take(&mut r.w.events); r.w.step(); // keep events from actions (clicks, places) like the game does
            for e in pre.into_iter().chain(std::mem::take(&mut r.w.events)) { r.events.push(format!("{}:{e:?}", tick)); }
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
