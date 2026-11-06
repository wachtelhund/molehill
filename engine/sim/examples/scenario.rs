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
fn mat_char(c: char) -> Option<Mat> { Some(match c { '.' | '@' => Mat::Empty, 't' => Mat::Roots, 'f' => Mat::Fungus, '#' => Mat::Granite, 'B' => Mat::Bedrock, 'r' => Mat::Rock, 'd' => Mat::Dirt, 'c' => Mat::Clay, 's' => Mat::Sand, 'g' => Mat::Gravel, 'w' => Mat::Water, 'l' => Mat::Lava, 'o' => Mat::Coal, 'i' => Mat::IronOre, 'b' => Mat::WoodBeam, 'k' => Mat::StoneBrick, 'I' => Mat::IronBeam, 'a' => Mat::Basalt, 'T' => Mat::RootTree, 'R' => Mat::TreeRoot, 'm' => Mat::Moss, 'y' => Mat::Mycelium, '=' => Mat::Rail, '%' => Mat::Tube, 'p' => Mat::Planks, 'q' => Mat::Polished, 'x' => Mat::Tiles, 'C' => Mat::Carved, 'h' => Mat::Chute, 'H' => Mat::Screen, '|' => Mat::Bumper, _ => return None }) }
fn ch_of(m: Mat) -> char { match m { Mat::Empty => '.', Mat::Roots => 't', Mat::Fungus => 'f', Mat::Granite => '#', Mat::Bedrock => 'B', Mat::Rock => 'r', Mat::Dirt => 'd', Mat::Clay => 'c', Mat::Sand => 's', Mat::Gravel => 'g', Mat::Water => 'w', Mat::Lava => 'l', Mat::Coal => 'o', Mat::IronOre => 'i', Mat::WoodBeam => 'b', Mat::StoneBrick => 'k', Mat::IronBeam => 'I', Mat::Basalt => 'a', Mat::Steam => '~', Mat::RootTree => 'T', Mat::TreeRoot => 'R', Mat::Moss => 'm', Mat::Mycelium => 'y', Mat::Rail => '=', Mat::Tube => '%', Mat::Planks => 'p', Mat::Polished => 'q', Mat::Tiles => 'x', Mat::Carved => 'C', Mat::Chute => 'h', Mat::Screen => 'H', Mat::Bumper => '|', _ => '?' } }

enum Act { Load(u64, u32, String), Ruin(u64, String, i32, i32, bool), Clue(u64, String, i32, i32), Furnish(u64, String, i32, i32), Paint(u64, String, i32, i32), Set(u64, String, f32), PlaceTorch(u64, i32, i32), Grotto(u64, i32, i32, f32, f32), Catch(u64, i32, i32), Throw(u64, String, f32, f32, f32), Loose(u64, String, i32, i32, f32, f32), Brush(u64, f32), Place(u64, i32, i32, String), Put(u64, i32, i32, String), Hold(u64, u64, Vec<String>), Mine(u64, u64, i32, i32), Dig(u64, u64, i32, i32), Tool(u64, u8), Explode(u64, i32, i32, f32), Dump(u64), Ascii(u64), Device(u64, i32, i32, String), Use(u64, i32, i32), Pickup(u64, i32, i32), Wire(u64, i32, i32, i32, i32), SaveLoad(u64), Lamp(u64, bool), Aim(u64, Option<(i32, i32)>), Wield(u64, String), Torch(u64, i32, i32), Slide(u64), SizeKey(u64, String), SizeWheel(u64, String, i32), Hand(u64, String), Snap(u64, bool), Line(u64, i32, i32, i32, i32, String), Mode(u64, bool), Pour(u64, i32, i32, String), Craft(u64, String), Station(u64, i32, i32, String), Scrape(u64, i32, i32), SetWall(u64, i32, i32, String), Site(u64, String, i32, i32), Expect(u64, String, String, f64, usize) }
fn wall_code(s: &str) -> Option<u8> {
    Some(match s.to_ascii_lowercase().as_str() {
        "none" => 0, "rock" => 1, "timber" => 2, "masonry" => 3, "beams" | "beam" => 4, "pipes" | "pipe" => 5,
        "carving" | "carved" => 6, "damp" => 7, "crack" => 8, "root" => 9, "gap" => 10, "salt" => 11, "crystal" => 12, "fossil" => 13,
        "soot" => 14, "wetstone" | "wet_stone" => 15, "basalt" => 16, "ashlar" => 17, "ironclad" => 18, _ => return None,
    })
}

struct Run { w: World, sizes: molehill_sim::view::BrushSizes, held: molehill_sim::world::Brush, ox: usize, oy: usize, mw: usize, mh: usize, ax: usize, ay: usize, sx: f32, sy: f32, top: f32, events: Vec<String> }
impl Run {
    fn var(&self, v: &str) -> Option<f64> {
        let p = &self.w.pl; let m = &self.w.moles[0]; let kf = K as f64;
        Some(match v {
            // px, py: tiles from the map's top-left corner; velocities in tiles/s
            "px" => (p.px - self.ox as f32) as f64 / kf, "py" => (p.py - self.oy as f32) as f64 / kf, "vx" => p.vx as f64 / kf, "vy" => p.vy as f64 / kf,
            "dx" => (p.px - self.sx) as f64 / kf, "dy" => (p.py - self.sy) as f64 / kf, "rise" => (self.sy - self.top) as f64 / kf,
            "grounded" => p.grounded as u8 as f64, "cling" => p.cling as f64, "hp" => m.hp as f64, "breath" => m.breath as f64,
            "deaths" => p.deaths as f64, "tier" => self.w.dig_power() as f64, "buildings" => self.w.buildings.len() as f64, "brush" => self.sizes.radius(self.held) as f64 / kf, "snapcells" => self.sizes.snap_cells() as f64 / kf, "torches" => self.w.torches.len() as f64, "awake" => self.awake() as f64, "crumbled" => self.w.crumbled as f64 / (kf * kf), "tick" => self.w.tick as f64,
            "hunger" => m.hunger as f64, "oil" => self.w.lamp_oil as f64, "quest" => self.w.quest as f64, "lamp" => self.w.lamp_lit() as u8 as f64, "mode" => self.w.mode as u8 as f64, "relics" => self.w.relics as f64, "ruins_seen" => self.w.ruins_seen.len() as f64, "hungeron" => !self.w.rules.no_hunger as u8 as f64,
            "flash" => p.flash as f64, "shake" => self.w.shake as f64,
            // the last blast's wave and what it threw: cells in flight (tiles), the drawn front's radius (m),
            // the tiles in it now, and wave:DX,DY the path distance (m) the front ran to reach a tile (-1: never)
            // transport: carts (count), the first cart's position (tiles from the map's corner, like px/py),
            // speed along the track (tiles/s), cargo (items), on the rails, and whether the mole rides one
            "carts" => self.w.tr.carts.len() as f64,
            "cartx" => self.w.tr.carts.first().map_or(-1., |c| (c.x - self.ox as f32) as f64 / kf),
            "carty" => self.w.tr.carts.first().map_or(-1., |c| (c.y - self.oy as f32) as f64 / kf),
            "cartv" => self.w.tr.carts.first().map_or(0., |c| c.v as f64 / kf),
            "cargo" => self.w.tr.carts.first().map_or(0., |c| c.load() as f64),
            "cartrail" => self.w.tr.carts.first().map_or(0., |c| c.on_rail as u8 as f64),
            "cartgap" => self.w.tr.carts.first().map_or(0., |c| (c.x - self.w.pl.px) as f64 / kf),
            "cartpow" => self.w.tr.carts.first().map_or(0., |c| self.w.rail_powered(c.x.max(0.) as usize, c.y.max(0.) as usize) as u8 as f64),
            // ladders and lifts: how many; whether the mole is on a ladder; the first lift's cage floor (tiles from
            // the map top), its speed (tiles/s, + = down) and its counterweight's ballast (stones)
            "ladders" => self.w.tr.ladders.len() as f64, "climbing" => self.w.tr.climbing as u8 as f64, "lifts" => self.w.tr.lifts.len() as f64,
            "lifty" => self.w.tr.lifts.first().map_or(-1., |l| (l.y - self.oy as f32) as f64 / kf),
            "liftv" => self.w.tr.lifts.first().map_or(0., |l| l.v as f64 / kf),
            "ballast" => self.w.tr.lifts.first().map_or(0., |l| l.ballast as f64),
            // pressure tubes: hoppers, capsules in the pipe, whether the mole is sealed in one, and whether the
            // first hopper's line holds pressure
            "capat" => self.w.tr.capsules.first().map_or(-1., |c| c.at as f64 / (c.path.len() - 1).max(1) as f64), // how far along its pipe (0..1)
            "hoppers" => self.w.tr.hoppers.len() as f64, "capsules" => self.w.tr.capsules.len() as f64, "intube" => self.w.tr.in_tube.is_some() as u8 as f64,
            "pressure" => if self.w.tr.hoppers.is_empty() { 0. } else { self.w.line_pressurised(0) as u8 as f64 },
            "riding" => self.w.tr.riding.is_some() as u8 as f64,
            // production: ore hoppers, the first one's load and whether its gate is open; carts tipped at a bumper so far;
            // cargoof:K the K-th cart's cargo; cargo:Item the first cart's count of that item
            "bins" => self.w.tr.bins.len() as f64, "binload" => self.w.tr.bins.first().map_or(0., |b| b.load() as f64),
            "binopen" => self.w.tr.bins.first().map_or(0., |b| b.open as u8 as f64), "tips" => self.w.tr.tips as f64,
            _ if v.starts_with("cargoof:") => { let k: usize = v[8..].parse().ok()?; self.w.tr.carts.get(k).map_or(0., |c| c.load() as f64) }
            _ if v.starts_with("cargo:") => { let it = item_by_name(&v[6..])?; self.w.tr.carts.first().map_or(0., |c| c.cargo.iter().filter(|x| x.0 == it).map(|x| x.1).sum::<u32>() as f64) }
            // shaking:DX,DY  1 if the tile is a shaker screen with current in its block
            _ if v.starts_with("shaking:") => { let a: Vec<&str> = v[8..].split(',').collect(); let (x, y) = self.at(a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?); self.w.shaking(x, y) as u8 as f64 }
            // zone:Name,DX0,DX1[,DY0,DY1]  loose items of that kind whose centre lies in those tile columns (and rows) from the '@' tile
            _ if v.starts_with("zone:") => { let a: Vec<&str> = v[5..].split(',').collect(); let it = item_by_name(a.first()?)?;
                let n: Vec<f32> = a[1..].iter().filter_map(|t| t.parse().ok()).collect(); let (x0, x1) = (*n.first()?, *n.get(1)?); let (y0, y1) = (n.get(2).copied().unwrap_or(-1e9), n.get(3).copied().unwrap_or(1e9));
                let (cx, cy) = (self.ax as f32, self.ay as f32); let k = K as f32;
                self.w.drops.iter().filter(|d| d.item == it && { let (tx, ty) = (((d.px - cx) / k).floor(), ((d.py - cy) / k).floor()); tx >= x0 && tx <= x1 && ty >= y0 && ty <= y1 }).map(|d| d.n).sum::<u32>() as f64 }
            "debris" => self.w.debris.len() as f64 / (K * K) as f64,
            "front" => self.w.waves.last().map_or(-1., |v| v.front_m(self.w.tick) as f64),
            "frontcells" => self.w.waves.last().map_or(0., |v| v.front(self.w.tick).count() as f64 / (K * K) as f64),
            _ if v.starts_with("debris:") => { let m = mat_by_name(&v[7..])?; self.w.debris_of(m) as f64 / (K * K) as f64 }
            _ if v.starts_with("wave:") => {
                let a: Vec<&str> = v[5..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                let Some(wv) = self.w.waves.last() else { return Some(-1.) }; let t: Vec<usize> = self.tile(dx, dy).collect();
                wv.cells.iter().filter(|c| t.contains(&(c.i as usize))).map(|c| c.d * molehill_sim::mat::CELL_M).reduce(f32::min).map_or(-1., |d| d as f64) }
            _ if v.starts_with("p:") => { // p:DX,DY  the last blast's peak overpressure (kPa) on that tile (its maximum)
                let a: Vec<&str> = v[2..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                if self.w.blast_field.is_empty() { 0. } else { self.tile(dx, dy).map(|i| self.w.blast_field[i]).fold(0f32, f32::max) as f64 } }
            // glowmites drifting over the map
            "glowmites" => self.w.glowmites().iter().filter(|g| g.0 >= self.ox as f32 && g.0 < (self.ox + self.mw) as f32 && g.1 >= self.oy as f32 && g.1 < (self.oy + self.mh) as f32).count() as f64,
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
            // log:WORD = lines in the message log containing WORD (case-insensitive; _ stands for a space)
            _ if v.starts_with("log:") => { let k = v[4..].replace('_', " ").to_lowercase(); self.w.log.iter().filter(|(_, l)| l.to_lowercase().contains(&k)).count() as f64 }
            _ if v.starts_with("is:") => { // is:DX,DY,Mat  (relative to @)
                let a: Vec<&str> = v[3..].split(',').collect(); let (dx, dy): (i32, i32) = (a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                let mt = mat_by_name(a.get(2)?)?; (2 * self.tile(dx, dy).filter(|&i| self.w.g.m(i) == mt).count() >= K * K) as u8 as f64 } // most of the tile
            _ if v.starts_with("known:") => { let i = molehill_sim::craft::ITEMS.iter().position(|it| format!("{it:?}").eq_ignore_ascii_case(&v[6..]))?; self.w.known[i] as u8 as f64 }
            _ if v.starts_with("recipe:") => { let k = molehill_sim::craft::recipe_by_name(&v[7..].replace('_', " "))?; self.w.recipe_known(k) as u8 as f64 }
            _ if v.starts_with("inv:") => { let i = molehill_sim::craft::ITEMS.iter().position(|it| format!("{it:?}").eq_ignore_ascii_case(&v[4..]))?; self.w.inv[i] as f64 }
            _ if v.starts_with("dev:") || v.starts_with("lit:") || v.starts_with("wire:") || v.starts_with("on:") => { // dev:DX,DY,Name  lit:DX,DY  wire:DX,DY  on:DX,DY (powered)
                let (k, rest) = v.split_once(':')?; let a: Vec<&str> = rest.split(',').collect(); let (x, y) = self.at(a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                let b = self.w.logic.block_of(x, y); let l = &self.w.logic;
                match k { "dev" => (format!("{:?}", molehill_sim::logic::Dev::from_u8(l.dev[b])).eq_ignore_ascii_case(a.get(2)?)) as u8 as f64, "lit" => l.lit(b) as u8 as f64, "wire" => l.wire[b] as f64, _ => l.powered[b] as f64 } }
            // furn:Item  how many of that furniture stand in the world; tint:Item  the first one's dye (0 undyed)
            // journal: fragments read; clues: unread clues in the map; clues_upper / clues_lower: every clue in the upper / lower half of the window
            "journal" => self.w.journal.len() as f64,
            // clues_bands:A,B  how many clues the world places in 32 m bands A..=B (absolute; negative = higher up)
            _ if v.starts_with("clues_bands:") => { let a: Vec<i64> = v[12..].split(',').filter_map(|t| t.parse().ok()).collect(); let (b0, b1) = (*a.first()?, *a.get(1)?);
                (b0..=b1).map(|b| molehill_sim::lore::clues_in_band(&self.w.gen, self.w.gen.seed, b, self.w.g.w).len()).sum::<usize>() as f64 }
            "clues" => self.w.clues.iter().filter(|c| !c.found && c.x >= self.ox && c.x < self.ox + self.mw && c.y >= self.oy && c.y < self.oy + self.mh).count() as f64,
            "clues_upper" => self.w.clues.iter().filter(|c| c.y < self.w.g.h / 2).count() as f64,
            "clues_lower" => self.w.clues.iter().filter(|c| c.y >= self.w.g.h / 2).count() as f64,
            _ if v.starts_with("furn:") => { let it = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(&v[5..]))?; self.w.furniture.iter().filter(|f| f.item == it).count() as f64 }
            _ if v.starts_with("tint:") => { let it = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(&v[5..]))?; self.w.furniture.iter().find(|f| f.item == it)?.tint as f64 }
            // paint:DX,DY  the pigment on the tile's centre cell (0 unpainted)
            _ if v.starts_with("paint:") => { let a: Vec<&str> = v[6..].split(',').collect(); let (x, y) = self.at(a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?); self.w.g.paint(y * self.w.g.w + x) as f64 }
            _ if v.starts_with("light:") => { // light:DX,DY  brightness at the tile's centre (luma; ~1 = fully lit)
                let a: Vec<&str> = v[6..].split(',').collect(); let (x, y) = self.at(a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                let l = self.w.light.at(y * self.w.g.w + x); (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2) as f64 }
            // a loose item (the first of that kind): ix/iy its centre in tiles from the '@' tile's centre,
            // ivx/ivy tiles/s, irest 1 if asleep, ilit 1 if a lit charge; items:Name how many lie loose
            _ if v.starts_with("items:") => { let it = item_by_name(&v[6..])?; self.w.drops.iter().filter(|d| d.item == it).map(|d| d.n).sum::<u32>() as f64 }
            _ if ["ix:", "iy:", "ivx:", "ivy:", "irest:", "ilit:"].iter().any(|p| v.starts_with(p)) => {
                let (k, name) = v.split_once(':')?; let it = item_by_name(name)?; let d = self.w.drops.iter().find(|d| d.item == it)?;
                let (cx, cy) = (self.ax as f32 + K as f32 / 2., self.ay as f32 + K as f32 / 2.);
                match k { "ix" => ((d.px - cx) as f64) / kf, "iy" => ((d.py - cy) as f64) / kf, "ivx" => d.vx as f64 / kf, "ivy" => d.vy as f64 / kf,
                    "irest" => d.asleep() as u8 as f64, _ => (d.fuse > 0) as u8 as f64 } }
            _ if v.starts_with("ev:") => self.events.iter().filter(|e| e.split(':').nth(1) == Some(&v[3..])).count() as f64,
            "sites" => self.w.sites.len() as f64, "sites_seen" => self.w.sites_seen.len() as f64,
            _ if v.starts_with("walls:") => {
                let wl = molehill_sim::walls::Wall::from_u8(wall_code(&v[6..])?); self.w.walls.count(wl) as f64 }
            _ if v.starts_with("wall:") => {
                let a: Vec<&str> = v[5..].split(',').collect(); let (x, y) = self.at(a.get(0)?.parse().ok()?, a.get(1)?.parse().ok()?);
                let want = wall_code(a.get(2)?)?; (self.w.walls.at(x, y) as u8 == want) as u8 as f64 }
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
    /// The last blast's wave: where its drawn front is now (path metres from the charge), how far it reached,
    /// the tiles in the front (relative to the mole's start, at most 48), and the cells in flight.
    fn blast_json(&self) -> String {
        let Some(wv) = self.w.waves.last() else { return format!("{{\"debris\":{}}}", self.w.debris.len() as f32 / (K * K) as f32) };
        let k = K as i64; let w = self.w.g.w;
        let mut tiles: Vec<(i64, i64)> = wv.front(self.w.tick).map(|c| (((c.i as usize % w) as i64 - self.ax as i64).div_euclid(k), ((c.i as usize / w) as i64 - self.ay as i64).div_euclid(k))).collect();
        tiles.sort(); tiles.dedup(); let nt = tiles.len(); tiles.truncate(48);
        format!("{{\"age_s\":{:.3},\"front_m\":{:.2},\"reach_m\":{:.2},\"wave_tiles\":{},\"front_tiles\":{},\"front\":[{}],\"debris\":{}}}",
            wv.age(self.w.tick), wv.front_m(self.w.tick), wv.reach * molehill_sim::mat::CELL_M, wv.cells.len() as f32 / (K * K) as f32, nt,
            tiles.iter().map(|(x, y)| format!("[{x},{y}]")).collect::<Vec<_>>().join(","), self.w.debris.len() as f32 / (K * K) as f32)
    }
    fn json(&self) -> String {
        let p = &self.w.pl; let m = &self.w.moles[0];
        let mut counts = vec![0u32; NMAT]; for y in self.oy..self.oy + self.mh { for x in self.ox..self.ox + self.mw { counts[self.w.g.get(x, y) as usize] += 1; } }
        let c0: Vec<String> = vec![];
        let inv: Vec<String> = molehill_sim::craft::ITEMS.iter().enumerate().filter(|(i, _)| self.w.inv[*i] > 0).map(|(i, it)| format!("\"{it:?}\":{}", self.w.inv[i])).collect();
        let c: Vec<String> = counts.iter().enumerate().filter(|(_, n)| **n > 0).map(|(i, n)| format!("\"{:?}\":{}", Mat::from_u8(i as u8), *n as f32 / (K * K) as f32)).collect(); let _ = c0;
        format!("{{\"tick\":{},\"player\":{{\"px\":{:.3},\"py\":{:.3},\"vx\":{:.3},\"vy\":{:.3},\"grounded\":{},\"cling\":{},\"hp\":{:.1},\"breath\":{:.1},\"deaths\":{}}},\"inv\":{{{}}},\"mats\":{{{}}},\"blast\":{},\"events\":[{}]}}",
            self.w.tick, self.var("px").unwrap(), self.var("py").unwrap(), self.var("vx").unwrap(), self.var("vy").unwrap(), p.grounded, p.cling, m.hp, m.breath, p.deaths, inv.join(","), c.join(","), self.blast_json(), self.events.iter().map(|e| format!("\"{e}\"")).collect::<Vec<_>>().join(","))
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
    let (mut seed, mut map, mut acts, mut gives, mut ticks) = (11u32, Vec::<String>::new(), Vec::new(), Vec::<(String, u32)>::new(), 0u64); let mut mode = None; let mut nohunger = false; let mut undisc: Vec<String> = vec![];
    let mut lines = src.lines().enumerate();
    let rng = |s: &str| -> (u64, u64) { match s.split_once('-') { Some((a, b)) => (a.parse().unwrap(), b.parse().unwrap()), None => { let a = s.parse().unwrap(); (a, a) } } };
    while let Some((ln, l)) = lines.next() {
        let l = l.split("//").next().unwrap().trim(); if l.is_empty() { continue; }
        let t: Vec<&str> = l.split_whitespace().collect();
        match t[0] {
            "seed" => seed = t[1].parse().unwrap(),
            "map" => { for (_, ml) in lines.by_ref() { if ml.trim() == "end" { break; } map.push(ml.trim_end().to_string()); } }
            // mode survival|creative: start in that game mode (its rules and its pack; `give` lines apply after)
            // ruin T hall|vault|tube DX DY [left]: stamp a ruin of the Ancients with its top-left on tile (DX, DY);
            // a vault's chest is registered (ancient loot), and the ruin is known to the world (its lore)
            "ruin" => acts.push(Act::Ruin(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap(), t.get(5) == Some(&"left"))),
            "mode" => mode = Some(match t[1] { "survival" => molehill_sim::survival::Mode::Survival, "creative" => molehill_sim::survival::Mode::Creative, o => panic!("{path}: unknown mode {o}") }),
            // nohunger: the world's hunger option off (food never drains), applied after `mode`
            "nohunger" => nohunger = true,
            // set T hp|hunger|breath|oil V: set a vital (or the headlamp's oil, in seconds) at tick T
            "set" => acts.push(Act::Set(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap())),
            // placetorch T DX DY: place a torch from the pack, as a click (refused: NOTE)
            "placetorch" => acts.push(Act::PlaceTorch(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            "give" => gives.push((t[1].into(), t[2].parse().unwrap())),
            // undiscovered MAT: this material hasn't been seen yet (all others count as discovered), so first
            // sight of it logs a discovery
            "undiscovered" => undisc.push(t[1].to_string()),
            "ticks" => ticks = t[1].parse().unwrap(),
            "hold" => { let (a, b) = rng(t[1]); acts.push(Act::Hold(a, b, t[2..].iter().map(|s| s.to_string()).collect())); }
            "mine" => { let (a, b) = rng(t[1]); acts.push(Act::Mine(a, b, t[2].parse().unwrap(), t[3].parse().unwrap())); }
            "dig" => { let (a, b) = rng(t[1]); acts.push(Act::Dig(a, b, t[2].parse().unwrap(), t[3].parse().unwrap())); }
            "tool" => acts.push(Act::Tool(t[1].parse().unwrap(), t[2].parse().unwrap())),
            "explode" => acts.push(Act::Explode(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].parse().unwrap())),
            "lamp" => acts.push(Act::Lamp(t[1].parse().unwrap(), t[2] == "on")),
            "aim" => acts.push(Act::Aim(t[1].parse().unwrap(), if t[2] == "none" { None } else { Some((t[2].parse().unwrap(), t[3].parse().unwrap())) })),
            "wield" => acts.push(Act::Wield(t[1].parse().unwrap(), t[2].into())),
            "torch" => acts.push(Act::Torch(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            // throw T Item DX DY POWER: throw one from the pack toward (DX, DY) tiles, POWER 0..1 of a full throw
            "throw" => acts.push(Act::Throw(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap(), t[5].parse().unwrap())),
            // drop T Item DX DY [VX VY]: a loose item centred on tile (DX, DY) from '@', moving at (VX, VY) tiles/s
            "drop" => acts.push(Act::Loose(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap(), t.get(5).map_or(0., |v| v.parse().unwrap()), t.get(6).map_or(0., |v| v.parse().unwrap()))),
            "brush" => acts.push(Act::Brush(t[1].parse().unwrap(), t[2].parse().unwrap())),
            "put" => acts.push(Act::Put(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            // furnish T Item DX DY: set a piece of furniture down at tile (DX, DY) (it finds the floor or ceiling)
            // clue T Kind DX DY: put a Lid clue (Carving, Note, Draft, Relic) on the floor of tile (DX, DY)
            "clue" => acts.push(Act::Clue(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap())),
            "furnish" => acts.push(Act::Furnish(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap())),
            // paint T Pigment DX DY: paint a brush (see `brush`) of cells around tile (DX, DY), or dye the furniture there
            "paint" => acts.push(Act::Paint(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap())),
            "place" => acts.push(Act::Place(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "device" => acts.push(Act::Device(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "use" => acts.push(Act::Use(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            "pickup" => acts.push(Act::Pickup(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            "wire" => { let a: Vec<i32> = t[2..].iter().map(|v| v.parse().unwrap()).collect(); let (x1, y1) = if a.len() >= 4 { (a[2], a[3]) } else { (a[0], a[1]) }; acts.push(Act::Wire(t[1].parse().unwrap(), a[0], a[1], x1, y1)); }
            "saveload" => acts.push(Act::SaveLoad(t[1].parse().unwrap())),
            "grotto" => acts.push(Act::Grotto(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].parse().unwrap(), t[5].parse().unwrap())),
            "catch" => acts.push(Act::Catch(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            "slide" => acts.push(Act::Slide(t[1].parse().unwrap())),
            // brush size and grid snap (the game's rules, from view::BrushSizes): `hand T Pickaxe|Drill|Bore|<block item>`,
            // `sizekey T [` or `]` (the typed character), `sizewheel T none|alt|ctrl|cmd|shift N`, `snap T on|off`,
            // `line T DX1 DY1 DX2 DY2 Item` (a grid-snapped drag from one tile to another)
            // progression: `switch T survival|creative` changes mode mid-run (as `mode` does at the start), `craft T <recipe name>`
            // (as at the station, "_" for spaces), `station T DX DY Workbench|Forge|Workshop` (built with its bottom-left on that tile)
            "pour" => acts.push(Act::Pour(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())), // set a tile's material (lava, fire, ...)
            "switch" => acts.push(Act::Mode(t[1].parse().unwrap(), t[2] == "creative")),
            "craft" => acts.push(Act::Craft(t[1].parse().unwrap(), t[2].replace('_', " "))),
            "station" => acts.push(Act::Station(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "hand" => acts.push(Act::Hand(t[1].parse().unwrap(), t[2].into())),
            "sizekey" => acts.push(Act::SizeKey(t[1].parse().unwrap(), t[2].into())),
            "sizewheel" => acts.push(Act::SizeWheel(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap())),
            "snap" => acts.push(Act::Snap(t[1].parse().unwrap(), t[2] == "on")),
            "line" => acts.push(Act::Line(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].parse().unwrap(), t[5].parse().unwrap(), t[6].into())),
            "scrape" => acts.push(Act::Scrape(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap())),
            "wall" => acts.push(Act::SetWall(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].parse().unwrap(), t[4].into())),
            "site" => acts.push(Act::Site(t[1].parse().unwrap(), t[2].into(), t[3].parse().unwrap(), t[4].parse().unwrap())),
            "dump" => acts.push(Act::Dump(t[1].parse().unwrap())),
            "load" => acts.push(Act::Load(t[1].parse().unwrap(), t[2].parse().unwrap(), t[3].into())),
            "ascii" => acts.push(Act::Ascii(t[1].parse().unwrap())),
            "expect" => acts.push(Act::Expect(t[1].parse().unwrap(), t[2].into(), t[3].into(), t[4].parse().unwrap(), ln + 1)),
            o => panic!("{path}:{}: unknown directive {o}", ln + 1),
        }
    }
    for a in &acts { ticks = ticks.max(match a { Act::Hold(_, b, _) | Act::Mine(_, b, _, _) | Act::Dig(_, b, _, _) => *b, Act::Load(t, ..) | Act::Grotto(t, ..) | Act::Catch(t, ..) | Act::Tool(t, _) | Act::Ruin(t, ..) | Act::Clue(t, ..) | Act::Furnish(t, ..) | Act::Paint(t, ..) | Act::Set(t, ..) | Act::PlaceTorch(t, ..) | Act::Throw(t, ..) | Act::Loose(t, ..) | Act::Brush(t, _) | Act::Place(t, ..) | Act::Put(t, ..) | Act::Explode(t, ..) | Act::Device(t, ..) | Act::Use(t, ..) | Act::Pickup(t, ..) | Act::Wire(t, ..) | Act::SaveLoad(t) | Act::Lamp(t, _) | Act::Aim(t, _) | Act::Wield(t, _) | Act::Torch(t, ..) | Act::Slide(t) | Act::SizeKey(t, ..) | Act::SizeWheel(t, ..) | Act::Hand(t, ..) | Act::Snap(t, ..) | Act::Line(t, ..) | Act::Dump(t) | Act::Ascii(t) | Act::Mode(t, _) | Act::Pour(t, ..) | Act::Craft(t, _) | Act::Station(t, ..) | Act::Scrape(t, ..) | Act::SetWall(t, ..) | Act::Site(t, ..) | Act::Expect(t, ..) => *t }); }
    let mut w = World::new(512 * K, 256 * K, seed); w.disasters.enabled = false; w.drops.clear(); w.torches.clear(); w.farms.clear(); w.grottos.clear(); w.keep_blast_field = true; // (the generated world's own grottos aren't part of the scenario)
    // every material counts as already discovered: a discovery's reward would depend on what the generated world showed
    w.discovered = [true; NMAT]; w.ruins.clear();
    for n in &undisc { let m = (0..NMAT).map(|k| Mat::from_u8(k as u8)).find(|m| format!("{m:?}").eq_ignore_ascii_case(n)).unwrap_or_else(|| panic!("{path}: unknown material {n}")); w.discovered[m as usize] = false; }
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
        for j in 0..K * K { let (cx, cy) = (ox + x * K + j % K, oy + y * K + j / K); let i = w.g.idx(cx, cy); w.g.set(i, m); w.g.flags[i] = 0; w.walls.set_at(cx, cy, molehill_sim::walls::Wall::None); } } }
    if let Some(m) = mode { w.set_mode(m); }
    if nohunger { w.set_hunger(false); }
    for (n, k) in gives { let i = molehill_sim::craft::ITEMS.iter().position(|it| format!("{it:?}").eq_ignore_ascii_case(&n)).unwrap_or_else(|| panic!("unknown item {n}")); w.inv[i] = k; }
    // the mole stands centred on the '@' tile, feet on its bottom edge
    let (ax, ay) = (ox + ax * K, oy + ay * K); // '@' tile's top-left cell
    let (fx, fy) = (ax + K / 2, ay + K - 1); // feet cell
    w.moles[0].x = fx; w.moles[0].y = fy; w.pl = Default::default(); w.pl.fall_from = fy; w.moles[0].hp = 100.; w.pl.spawn = (fx, w.oy + fy as i64);
    w.pl.px = (ax as f32) + K as f32 / 2.; w.pl.py = (ay + K) as f32; w.pl.ppx = w.pl.px; w.pl.ppy = w.pl.py; w.pl.grounded = true; w.pl.sync = Some((fx, fy));
    let (mw, mh) = (mw * K, mh * K);
    let mut r = Run { sizes: Default::default(), held: molehill_sim::world::Brush::Dig(1), ox, oy, mw, mh, ax, ay, sx: 0., sy: 0., top: f32::MAX, events: vec![], w };
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
                Act::Ruin(t, kind, dx, dy, left) if *t == tick => { use molehill_sim::ruins::{Kind, Ruin};
                    let kd = match kind.as_str() { "hall" => Kind::PumpingHall, "vault" => Kind::SealedVault, "tube" => Kind::TubeStation, o => panic!("{path}: unknown ruin {o}") };
                    let (x0, y0) = (r.ax as i32 + dx * K as i32, r.ay as i32 + dy * K as i32); let oy = r.w.oy;
                    let ru = Ruin::new(0x5EED + tick as u32 * 7 + (*dx as u32) * 131, kd, x0 as usize, oy + y0 as i64, *left, 1, 12);
                    for y in 0..ru.h { for x in 0..ru.w { if let Some(m) = ru.cell(ru.x0 + x, ru.y0 + y as i64) { let i = r.w.g.idx(ru.x0 + x, y0 as usize + y); r.w.g.set(i, m); r.w.g.flags[i] = 0; } } }
                    if let Some((cx, cwy)) = ru.chest { r.w.chests.push(molehill_sim::world::Chest { x: cx, y: (cwy - oy) as usize, opened: false, ancient: Some(ru.id) }); }
                    r.w.ruins.push(ru); }
                Act::Set(t, v, x) if *t == tick => { let m = &mut r.w.moles[0]; match v.as_str() { "hp" => m.hp = *x, "hunger" => m.hunger = *x, "breath" => m.breath = *x, "oil" => r.w.lamp_oil = *x, "quest" => r.w.quest = *x as usize,
                    // set T inv:ITEM N: the pack's count of an item, mid-run (`give` is at the start)
                    o if o.starts_with("inv:") => { let it = item_by_name(&o[4..]).unwrap_or_else(|| panic!("{path}: unknown item {o}")); r.w.inv[it as usize] = *x as u32; }
                    o => panic!("{path}: can't set {o}") } }
                Act::PlaceTorch(t, dx, dy) if *t == tick => { let (x, y) = r.at(*dx, *dy); if let Err(e) = r.w.try_place_torch(x, y) { println!("  NOTE {path} tick {tick}: torch at {dx},{dy} refused: {e}"); r.w.refuse_place("torch", e); } }
                Act::Throw(t, it, dx, dy, pw) if *t == tick => { let item = item_by_name(it).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    if let Err(e) = r.w.throw_item(item, (*dx, *dy), *pw) { println!("  NOTE {path} tick {tick}: throw {it}: {e}"); } }
                Act::Loose(t, it, dx, dy, vx, vy) if *t == tick => { let item = item_by_name(it).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let kf = K as f32; let mut d = molehill_sim::world::Drop::at(r.ax as f32 + (*dx as f32 + 0.5) * kf, r.ay as f32 + (*dy as f32 + 0.5) * kf, item, 1);
                    d.vx = vx * kf; d.vy = vy * kf; r.w.drops.push(d); }
                Act::Brush(t, b) if *t == tick => inp.brush = *b * K as f32,
                Act::Lamp(t, on) if *t == tick => r.w.gear.lamp = *on,
                Act::Aim(t, a) if *t == tick => r.w.gear.aim = a.map(|(dx, dy)| { let (x, y) = r.at(dx, dy); (x as f32 + 0.5, y as f32 + 0.5) }),
                Act::Wield(t, it) if *t == tick => r.w.gear.held = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(it)),
                Act::Torch(t, dx, dy) if *t == tick => { let c = r.at(*dx, *dy); r.w.torches.push(c); }
                Act::Explode(t, dx, dy, kg) if *t == tick => { let (x, y) = r.at(*dx, *dy); r.w.blast(x, y, *kg); } // a blast of kg TNT-equivalent (a TNT charge is 0.5)
                Act::Clue(t, k, dx, dy) if *t == tick => { use molehill_sim::lore::{Clue, ClueKind};
                    let kind = [ClueKind::Carving, ClueKind::Note, ClueKind::Draft, ClueKind::Relic].into_iter().find(|c| format!("{c:?}").eq_ignore_ascii_case(k)).unwrap_or_else(|| panic!("{path}: unknown clue {k}"));
                    let (x, _) = r.at(*dx, *dy); let y = (r.ay as i32 + dy * K as i32 + K as i32 - 1) as usize; let wy = r.w.oy + y as i64;
                    r.w.clues.push(Clue { kind, x, y, wy, found: false, look: 0 }); }
                Act::Furnish(t, it, dx, dy) if *t == tick => { let item = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(it)).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let (x, y) = r.at(*dx, *dy); if let Err(e) = r.w.place_furniture(item, x, y) { println!("  NOTE {path} tick {tick}: furnish {it} at {dx},{dy}: {e}"); r.w.refuse_place(item.name(), e); } }
                Act::Paint(t, it, dx, dy) if *t == tick => { let item = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(it)).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let (x, y) = r.at(*dx, *dy); if let Err(e) = r.w.paint(x, y, item, inp.brush) { println!("  NOTE {path} tick {tick}: paint {it} at {dx},{dy}: {e}"); } }
                Act::Load(t, n, it) if *t == tick => { let item = item_by_name(it).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    if !r.w.tr.carts.is_empty() { r.w.tr.carts[0].cargo.push((item, *n)); } else { println!("  NOTE {path} tick {tick}: no cart to load"); } }
                Act::Place(t, dx, dy, it) if *t == tick => {
                    let item = molehill_sim::craft::ITEMS.iter().copied().find(|i| format!("{i:?}").eq_ignore_ascii_case(it)).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let (x, y) = r.at(*dx, *dy);
                    if item.is_wall() { if let Err(e) = r.w.place_walls(x, y, item, inp.brush) { println!("  NOTE {path} tick {tick}: place {it} at {dx},{dy} failed: {e}"); r.w.refuse_place(item.name(), e); } }
                    else { match r.w.place_transport(item, x, y) { Ok(true) => {} Err(e) => { println!("  NOTE {path} tick {tick}: place {it} at {dx},{dy} failed: {e}"); r.w.refuse_place(item.name(), e); }
                        Ok(false) => if let Err(e) = r.w.place_brush(x, y, item, inp.brush) { println!("  NOTE {path} tick {tick}: place {it} at {dx},{dy} failed: {e}"); r.w.refuse_place(item.name(), e); } } }
                }
                Act::Scrape(t, dx, dy) if *t == tick => { let (x, y) = r.at(*dx, *dy); if let Err(e) = r.w.scrape_walls(x, y, inp.brush) { println!("  NOTE {path} tick {tick}: scrape at {dx},{dy}: {e}"); } }
                Act::SetWall(t, dx, dy, kind) if *t == tick => {
                    let wl = molehill_sim::walls::Wall::from_u8(wall_code(kind).unwrap_or_else(|| panic!("{path}: unknown wall {kind}")));
                    let (gw, cells): (usize, Vec<usize>) = (r.w.g.w, r.tile(*dx, *dy).collect());
                    for i in cells { r.w.set_wall(i % gw, i / gw, wl); } }
                Act::Site(t, kind, dx, dy) if *t == tick => {
                    let kd = molehill_sim::sites::Kind::from_name(kind).unwrap_or_else(|| panic!("{path}: unknown site {kind}"));
                    let (x, y) = (r.ax as i32 + dx * K as i32, r.ay as i32 + dy * K as i32);
                    r.w.stamp_site(kd, x as usize, y as usize); }
                Act::Device(t, dx, dy, n) if *t == tick => { let (x, y) = r.at(*dx, *dy);
                    let d = molehill_sim::logic::DEVS.iter().copied().find(|d| format!("{d:?}").eq_ignore_ascii_case(n)).unwrap_or_else(|| panic!("unknown device {n}"));
                    if let Err(e) = r.w.place_device(d, x, y) { println!("  NOTE {path} tick {tick}: device {n} at {dx},{dy}: {e}"); r.w.refuse_place(d.name(), e); } }
                Act::Use(t, dx, dy) if *t == tick => { let (x, y) = r.at(*dx, *dy); if !r.w.use_device(x, y) { println!("  NOTE {path} tick {tick}: nothing to use at {dx},{dy}"); } }
                Act::Grotto(t, dx, dy, rx, ry) if *t == tick => { // a glowmite swarm of a grotto centred on that tile (radii in tiles)
                    let (x, y) = r.at(*dx, *dy); let (rx, ry) = ((*rx * K as f32) as usize, (*ry * K as f32) as usize);
                    let id = 0x5CE7 + r.w.grottos.len() as u32; let cy = r.w.oy + y as i64;
                    r.w.grottos.push(molehill_sim::gen::Grotto { id, cx: x, cy, rx, ry, floor: ry as i64 / 2, pool: (0, 0), trees: [(0, 0); 3], ntrees: 0 }); }
                Act::Catch(t, dx, dy) if *t == tick => { let (x, y) = r.at(*dx, *dy); if !r.w.catch_glowmite(x, y) { println!("  NOTE {path} tick {tick}: no glowmite to catch at {dx},{dy}"); } }
                Act::Pickup(t, dx, dy) if *t == tick => { let (x, y) = r.at(*dx, *dy); r.w.pick_up(x, y); }
                Act::Wire(t, x0, y0, x1, y1) if *t == tick => { // a straight run of copperline, one per 1 m block
                    let n = ((x1 - x0).abs().max((y1 - y0).abs()) / 4).max(0);
                    for k in 0..=n { let f = |a: i32, b: i32| if n == 0 { a } else { a + (b - a) * k / n }; let (x, y) = r.at(f(*x0, *x1), f(*y0, *y1)); r.w.lay_wire(x, y); } }
                // the window slides away until only its bottom band is left, then back: the scenario's rows go
                // into stored bands and return (the mole is carried along, as if it had walked away and back)
                Act::Slide(t) if *t == tick => { let (m0, p0) = (r.w.moles[0].clone(), r.w.pl.clone()); let bh = molehill_sim::band::BH; let rows = ((r.w.g.h - bh) / bh * bh) as i64;
                    r.w.shift(rows); r.w.shift(-rows); r.w.moles[0] = m0; r.w.pl = p0; }
                Act::Pour(t, dx, dy, m) if *t == tick => { let mt = mat_by_name(m).unwrap_or_else(|| panic!("{path}: unknown material {m}"));
                    let cells: Vec<usize> = r.tile(*dx, *dy).collect(); for i in cells { r.w.g.set(i, mt); } }
                Act::Mode(t, c) if *t == tick => r.w.set_mode(if *c { molehill_sim::survival::Mode::Creative } else { molehill_sim::survival::Mode::Survival }),
                Act::Craft(t, n) if *t == tick => { let k = molehill_sim::craft::recipe_by_name(n).unwrap_or_else(|| panic!("{path}: unknown recipe {n}"));
                    if let Err(e) = r.w.craft_now(k) { println!("  NOTE {path} tick {tick}: craft {n}: {e}"); } }
                Act::Station(t, dx, dy, n) if *t == tick => { let kind = *molehill_sim::craft::STATIONS.iter().find(|s| s.name().eq_ignore_ascii_case(n)).unwrap_or_else(|| panic!("{path}: unknown station {n}"));
                    let (x, y) = r.at(*dx, *dy); let (x, y) = (x - K / 2, y - K / 2 + K - 1); // bottom-left cell of the tile: the station stands on the row below
                    if let Err(e) = r.w.place_building(kind, x, y) { println!("  NOTE {path} tick {tick}: station {n}: {e}"); } }
                Act::Hand(t, h) if *t == tick => { use molehill_sim::world::Brush;
                    r.held = match h.as_str() { "Pickaxe" => Brush::Dig(1), "Drill" => Brush::Dig(2), "Bore" => Brush::Dig(3), it => Brush::Place(item_by_name(it).unwrap_or_else(|| panic!("{path}: unknown item {it}"))) };
                    if let Brush::Dig(n) = r.held { inp.tool = n; } inp.brush = r.sizes.radius(r.held); }
                Act::SizeKey(t, c) if *t == tick => { let st = molehill_sim::view::brush_key_steps(c == "[", c == "]", false, false); let h = r.held; r.sizes.step(h, st); inp.brush = r.sizes.radius(h); }
                Act::SizeWheel(t, m, n) if *t == tick => { use molehill_sim::view::*;
                    let mods = Mods { alt: m == "alt", ctrl: m == "ctrl", cmd: m == "cmd", shift: m == "shift" };
                    if wheel_use(mods, true) == WheelUse::Brush { let h = r.held; r.sizes.step(h, *n); inp.brush = r.sizes.radius(h); } }
                Act::Snap(t, on) if *t == tick => r.sizes.snap = *on,
                Act::Line(t, x1, y1, x2, y2, it) if *t == tick => { let item = item_by_name(it).unwrap_or_else(|| panic!("{path}: unknown item {it}"));
                    let (a, b) = (r.at(*x1, *y1), r.at(*x2, *y2)); let n = r.sizes.snap_cells();
                    let rline = if item.is_wall() { r.w.place_wall_line(a, b, n, item) } else { r.w.place_line(a, b, n, item) };
                    if let Err(e) = rline { println!("  NOTE {path} tick {tick}: line failed: {e}"); r.w.refuse_place(item.name(), e); } }
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
            { let (x1, y1) = ((r.ox + r.mw).min(r.w.g.w - 1), (r.oy + r.mh).min(r.w.g.h - 1)); r.w.compute_light((r.ox, r.oy, x1, y1)); }
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
