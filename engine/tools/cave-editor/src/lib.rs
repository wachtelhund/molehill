//! The cave editor's model: one found-place template open for editing, the edits a painter makes, and
//! the checks that run the real simulation on it. No Bevy here, so all of it is tested headless
//! (`cargo test --release -p molehill-cave-editor`); `main.rs` is the window around it.
//!
//! Dev only. This crate is not a dependency of the game, is not a default workspace member, and is never
//! built by `build-web.sh`, so nothing a player downloads contains it (see `tests/packaging.rs` and
//! `docs/wiki/Cave-Editor.md`).
use std::path::{Path, PathBuf};

use molehill_sim::craft::ITEMS;
use molehill_sim::logic::Dev;
use molehill_sim::mat::{Mat, CPM, NMAT};
use molehill_sim::sites::Kind;
use molehill_sim::templates::{self, default_char, rle_decode, rle_encode, SiteFile, Template, ThingDef, FAMILIES, HOST};
use molehill_sim::walls::{Wall, NWALL};
use molehill_sim::world::World;

/// Where the site files live in the repo (`assets/sites/`).
pub fn sites_dir() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sites") }
pub fn path_of(kind: Kind) -> PathBuf { sites_dir().join(format!("{}.json", templates::file_name(kind))) }

/// What a brush stroke writes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brush {
    /// a material, or None to leave the host rock
    Mat(Option<Mat>),
    /// a back-wall lining (a single wall), or None for no lining of its own
    Lining(Option<Wall>),
}

/// The kinds of thing a painter can set down, in the order the palette cycles them.
pub fn thing_kinds() -> Vec<ThingDef> {
    let at = [0., 0.];
    let mut v = vec![ThingDef { what: "torch".into(), item: None, at, anchor: None, when: None }];
    for &it in ITEMS.iter() { if it.furn().is_some() { v.push(ThingDef { what: "furniture".into(), item: Some(format!("{it:?}")), at, anchor: None, when: None }); } }
    v.push(ThingDef { what: "farm".into(), item: None, at, anchor: None, when: None });
    v.push(ThingDef { what: "ladder".into(), item: None, at, anchor: None, when: None });
    v.push(ThingDef { what: "cart".into(), item: None, at, anchor: None, when: None });
    for d in [Dev::Lever, Dev::Floodgate, Dev::Pump, Dev::Lamp, Dev::Door, Dev::Plate] { v.push(ThingDef { what: "device".into(), item: Some(format!("{d:?}")), at, anchor: None, when: None }); }
    v.push(ThingDef { what: "wire".into(), item: None, at, anchor: None, when: None });
    v.push(ThingDef { what: "on".into(), item: None, at, anchor: None, when: None });
    v
}
pub fn thing_label(t: &ThingDef) -> String { match &t.item { Some(i) => format!("{} {i}", t.what), None => t.what.clone() } }
/// Every material a painter can pick (the short-lived ones, steam and fire, left out).
pub fn paint_mats() -> Vec<Mat> { (0..NMAT as u8).map(Mat::from_u8).filter(|m| !matches!(m, Mat::Steam | Mat::Fire)).collect() }
pub fn paint_walls() -> Vec<Wall> { (1..NWALL as u8).map(Wall::from_u8).collect() }

#[derive(Clone, PartialEq)]
struct Snap { cells: Vec<char>, lining: Vec<char>, things: Vec<ThingDef>, chest: Option<[usize; 2]>, file: SiteFile }

/// One template open for editing. Painting changes the base layer; slot patches (the variants) are kept
/// and laid over it in the preview, so a cell a variant patches shows the variant's value.
pub struct Doc {
    pub kind: Kind,
    pub w: usize,
    pub h: usize,
    pub file: SiteFile,
    pub cells: Vec<char>,
    pub lining: Vec<char>,
    pub dirty: bool,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
}

impl Doc {
    pub fn from_file(f: SiteFile) -> Result<Doc, String> {
        let t = Template::from_file(f.clone())?; // validates it
        let grid = |rows: &[String]| -> Result<Vec<char>, String> { let mut g = vec![]; for r in rows { g.extend(rle_decode(r)?); } Ok(g) };
        Ok(Doc { kind: t.kind, w: t.w, h: t.h, cells: grid(&f.cells)?, lining: grid(&f.lining)?, file: f, dirty: false, undo: vec![], redo: vec![] })
    }
    /// Open a family's file from `assets/sites/`.
    pub fn open(kind: Kind) -> Result<Doc, String> { Doc::load(&path_of(kind)) }
    pub fn load(path: &Path) -> Result<Doc, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let f: SiteFile = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Doc::from_file(f)
    }
    fn snap(&self) -> Snap { Snap { cells: self.cells.clone(), lining: self.lining.clone(), things: self.file.things.clone(), chest: self.file.chest, file: self.file.clone() } }
    fn restore(&mut self, s: Snap) { self.cells = s.cells; self.lining = s.lining; self.file = s.file; self.file.things = s.things; self.file.chest = s.chest; self.dirty = true; }
    /// Call once at the start of each stroke or action, so one undo takes it back whole.
    pub fn begin(&mut self) { self.undo.push(self.snap()); if self.undo.len() > 200 { self.undo.remove(0); } self.redo.clear(); }
    pub fn undo(&mut self) -> bool { let Some(s) = self.undo.pop() else { return false }; self.redo.push(self.snap()); self.restore(s); true }
    pub fn redo(&mut self) -> bool { let Some(s) = self.redo.pop() else { return false }; self.undo.push(self.snap()); self.restore(s); true }

    /// The palette character for a material, adding it to the file's palette if it's new there.
    fn mat_char(&mut self, m: Option<Mat>) -> char {
        let Some(m) = m else { return HOST };
        let name = format!("{m:?}");
        if let Some((k, _)) = self.file.palette.iter().find(|(_, v)| **v == name) { return k.chars().next().unwrap(); }
        let c = default_char(m);
        self.file.palette.insert(c.to_string(), name);
        c
    }
    /// The lining character for a single wall, adding a one-wall mix if the file has none.
    fn wall_char(&mut self, w: Option<Wall>) -> char {
        let Some(w) = w else { return HOST };
        let one = vec![format!("{w:?}")];
        if let Some((k, _)) = self.file.linings.iter().find(|(_, v)| **v == one) { return k.chars().next().unwrap(); }
        let used: Vec<char> = self.file.linings.keys().filter_map(|k| k.chars().next()).collect();
        let c = ('a'..='z').chain('A'..='Z').find(|c| !used.contains(c)).expect("out of lining keys");
        self.file.linings.insert(c.to_string(), one);
        c
    }
    /// Paint a round brush of radius `r` cells centred on (cx, cy). Returns how many cells changed.
    pub fn paint(&mut self, cx: i64, cy: i64, r: f32, b: Brush) -> usize {
        let (layer_mat, ch) = match b { Brush::Mat(m) => (true, self.mat_char(m)), Brush::Lining(w) => (false, self.wall_char(w)) };
        let ri = r.ceil() as i64; let mut n = 0;
        for y in cy - ri..=cy + ri { for x in cx - ri..=cx + ri {
            if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h { continue; }
            let (fx, fy) = ((x - cx) as f32, (y - cy) as f32);
            if fx * fx + fy * fy > r * r + 0.25 { continue; }
            let i = y as usize * self.w + x as usize;
            let g = if layer_mat { &mut self.cells } else { &mut self.lining };
            if g[i] != ch { g[i] = ch; n += 1; }
        } }
        if n > 0 { self.dirty = true; }
        n
    }
    /// What the base layer holds at (x, y): a material (None = host) for the eyedropper.
    pub fn pick(&self, x: usize, y: usize) -> Option<Brush> {
        let c = *self.cells.get(y * self.w + x)?;
        if c == HOST { return Some(Brush::Mat(None)); }
        let name = self.file.palette.get(&c.to_string())?;
        Some(Brush::Mat(templates::mat_named(name)))
    }
    /// Set a thing down at cell (x, y); a ladder ties to the nearest solid cell beside or above its head.
    pub fn place_thing(&mut self, kind: &ThingDef, x: usize, y: usize, var: u32, left: bool) {
        let mut t = kind.clone();
        t.at = [x as f32, y as f32];
        if t.what == "cart" { t.at[0] += 0.5; }
        if t.what == "ladder" {
            let tpl = self.template().ok();
            let solid = |ax: i64, ay: i64| tpl.as_ref().map_or(false, |t| ax >= 0 && ay >= 0 && (ax as usize) < t.w && (ay as usize) < t.h
                && t.cell(ax as usize, ay as usize, var, left).map_or(true, |m| !m.passable()));
            let (xi, yi) = (x as i64, y as i64);
            let a = [(xi + 1, yi), (xi - 1, yi), (xi, yi - 1)].into_iter().find(|&(ax, ay)| solid(ax, ay)).unwrap_or((xi, yi - 1));
            t.anchor = Some([a.0.max(0) as usize, a.1.max(0) as usize]);
        }
        self.file.things.push(t);
        self.dirty = true;
    }
    /// Take away the thing nearest (x, y) within `r` cells. Returns whether one went.
    pub fn remove_thing_near(&mut self, x: usize, y: usize, r: f32) -> bool {
        let d = |t: &ThingDef| ((t.at[0] - x as f32).powi(2) + (t.at[1] - y as f32).powi(2)).sqrt();
        let best = self.file.things.iter().enumerate().filter(|(_, t)| d(t) <= r).min_by(|a, b| d(a.1).total_cmp(&d(b.1))).map(|(i, _)| i);
        if let Some(i) = best { self.file.things.remove(i); self.dirty = true; true } else { false }
    }
    pub fn set_chest(&mut self, at: Option<[usize; 2]>) { self.file.chest = at; self.dirty = true; }
    /// The document as a site file (rows re-coded; marked as the editor's from now on).
    pub fn to_file(&self) -> SiteFile {
        let mut f = self.file.clone();
        f.cells = self.cells.chunks(self.w).map(|r| rle_encode(r.iter().copied())).collect();
        f.lining = self.lining.chunks(self.w).map(|r| rle_encode(r.iter().copied())).collect();
        if self.dirty { f.source = "editor".into(); }
        // drop palette entries nothing uses any more (roles stay)
        let used: std::collections::BTreeSet<char> = self.cells.iter().copied()
            .chain(f.slots.iter().flat_map(|s| s.options.iter()).flat_map(|o| o.patch.iter()).flat_map(|p| rle_decode(&p.2).unwrap_or_default())).collect();
        f.palette.retain(|k, v| v.starts_with('@') || k.chars().next().is_some_and(|c| used.contains(&c)));
        f
    }
    pub fn template(&self) -> Result<Template, String> { Template::from_file(self.to_file()) }
    /// Write the file (canonical form, see `templates::to_json`).
    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        let f = self.to_file();
        Template::from_file(f.clone())?;
        std::fs::write(path, templates::to_json(&f)).map_err(|e| e.to_string())?;
        self.file = f; self.dirty = false;
        Ok(())
    }
}

// ---------------------------------------------------------------- checks against the real sim

/// A piece stamped alone in solid granite, as the found-places test and `sitewalk` do.
pub struct Stamped { pub w: World, pub x0: usize, pub y0: usize, pub chest: Option<(usize, usize)> }
const M: usize = 2 * CPM;
pub fn stamp(t: &Template, var: u32, left: bool) -> Result<Stamped, String> {
    let (ww, wh) = (((t.w + 4 * M + 127) / 64 * 64).max(1024), ((t.h + 6 * M + 127) / 64 * 64).max(512));
    let mut w = World::new(ww, wh, 1);
    w.disasters.enabled = false;
    let (x0, y0) = (64, wh - t.h - 3 * M);
    for y in y0 - M..y0 + t.h + M { for x in x0 - M..x0 + t.w + M { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); w.g.flags[i] = 0; } }
    w.tr.ladders.clear(); w.chests.clear(); w.torches.clear(); w.furniture.clear();
    let chest = w.stamp_template(t, x0, y0, var, left, 0x51FE);
    Ok(Stamped { w, x0, y0, chest })
}

/// Can the mole's body get from every way in to the chest (or, with none, the floor), and is any big room
/// cut off? Same rules as `found_places_entrances_reach_every_room_and_the_chest`.
pub struct Reach {
    /// per box cell (w × h): the region whose body positions cover it (0 = none)
    pub region: Vec<u32>,
    pub sizes: Vec<usize>,
    pub ways_in: Vec<u32>,
    pub problems: Vec<String>,
}
pub fn reach(t: &Template, var: u32, left: bool) -> Result<Reach, String> {
    let s = stamp(t, var, left)?;
    let (bx0, by0, bw, bh) = (s.x0 - M / 2, s.y0 - M / 2, t.w + M, t.h + M);
    let (reg, sizes) = s.w.body_regions(bx0, by0, bw, bh);
    let mut edge = vec![false; sizes.len()]; let mut goal = vec![false; sizes.len()];
    for (i, &r) in reg.iter().enumerate() {
        if r == 0 { continue; }
        let (gx, gy) = (bx0 + i % bw, by0 + i / bw);
        if gx < s.x0 + CPM / 2 || gx + CPM / 2 >= s.x0 + t.w || gy < s.y0 + CPM { edge[r as usize] = true; }
        let at_goal = match s.chest { Some((cx, cy)) => gx.abs_diff(cx) <= CPM && gy.abs_diff(cy) <= CPM, None => gy + 2 * CPM >= s.y0 + t.h };
        if at_goal { goal[r as usize] = true; }
    }
    let mut problems = vec![];
    if let Some((cx, cy)) = s.chest { if s.w.g.is_solidish(s.w.g.idx(cx, cy)) { problems.push(format!("the chest at ({}, {}) is inside a solid", cx - s.x0, cy - s.y0)); } }
    let ways_in: Vec<u32> = (1..sizes.len() as u32).filter(|&k| edge[k as usize]).collect();
    if ways_in.is_empty() { problems.push("no way in: no entrance the body fits".into()); }
    for &k in &ways_in { if !goal[k as usize] { problems.push(format!("a way in ({} body positions) doesn't reach {}", sizes[k as usize], if s.chest.is_some() { "the chest" } else { "the floor" })); } }
    let main = ways_in.iter().copied().max_by_key(|&k| sizes[k as usize]);
    for k in 1..sizes.len() as u32 { if Some(k) != main && sizes[k as usize] >= 4 * CPM * CPM && !ways_in.contains(&k) { problems.push(format!("a room of {} body positions is cut off", sizes[k as usize])); } }
    // project the body positions (feet) onto the box's cells so the overlay shows where the mole can be
    let mut region = vec![0u32; t.w * t.h];
    let (hw, bhh) = ((molehill_sim::world::BODY_HW.ceil()) as usize, (molehill_sim::world::BODY_H.ceil()) as usize);
    for (i, &r) in reg.iter().enumerate() {
        if r == 0 { continue; }
        let (fx, fy) = (bx0 + i % bw, by0 + i / bw);
        for y in fy.saturating_sub(bhh)..fy { for x in fx.saturating_sub(hw)..fx + hw {
            if x < s.x0 || y < s.y0 || x >= s.x0 + t.w || y >= s.y0 + t.h { continue; }
            region[(y - s.y0) * t.w + x - s.x0] = r;
        } }
    }
    Ok(Reach { region, sizes, ways_in, problems })
}

/// Which cells the stress solver says are over capacity (they crack and fall in play), box-relative.
pub fn stress(t: &Template, var: u32, left: bool) -> Result<Vec<bool>, String> {
    let mut s = stamp(t, var, left)?;
    let fails = s.w.st.solve(&s.w.g);
    let mut over = vec![false; t.w * t.h];
    for i in fails { let (x, y) = (i as usize % s.w.g.w, i as usize / s.w.g.w);
        if x >= s.x0 && y >= s.y0 && x < s.x0 + t.w && y < s.y0 + t.h { over[(y - s.y0) * t.w + x - s.x0] = true; } }
    Ok(over)
}

/// Run the piece for `ticks` with a mole standing in it (at the chest, else the middle of the floor):
/// what it looks like after, and what happened.
pub struct Sim { pub cells: Vec<Mat>, pub changed: usize, pub crumbled: u64, pub hp: f32, pub ticks: u32 }
pub fn simulate(t: &Template, var: u32, left: bool, ticks: u32) -> Result<Sim, String> {
    let mut s = stamp(t, var, left)?;
    let read = |w: &World| -> Vec<Mat> { let mut v = Vec::with_capacity(t.w * t.h); for y in 0..t.h { for x in 0..t.w { v.push(w.g.get(s.x0 + x, s.y0 + y)); } } v };
    let before = read(&s.w);
    let (mx, my) = s.chest.unwrap_or((s.x0 + t.w / 2, s.y0 + t.h - CPM - 1));
    if let Some(m) = s.w.moles.first_mut() { m.x = mx; m.y = my; }
    s.w.pl.px = mx as f32; s.w.pl.py = (my + 1) as f32;
    let c0 = s.w.crumbled;
    for _ in 0..ticks { s.w.step(); }
    let after = read(&s.w);
    let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    Ok(Sim { cells: after, changed, crumbled: s.w.crumbled - c0, hp: s.w.moles.first().map_or(0., |m| m.hp), ticks })
}

// ---------------------------------------------------------------- drawing (CPU pixels, used by the window)

/// RGBA pixels for the piece as the variant shows it, one pixel per cell: materials in their table
/// colour, host rock dark and hatched, the lining a dim back behind open cells, and markers for the
/// chest and things.
pub fn draw(t: &Template, var: u32, left: bool, sim: Option<&[Mat]>) -> Vec<u8> {
    let mut px = vec![0u8; t.w * t.h * 4];
    for y in 0..t.h { for x in 0..t.w {
        let i = y * t.w + x;
        let m = match sim { Some(c) => Some(c[i]), None => t.cell(x, y, var, left) };
        let c: [u8; 3] = match m {
            None => if (x + y) % 6 == 0 { [40, 36, 34] } else { [26, 23, 22] },
            Some(Mat::Empty) => match t.lining_with(x, y, var, left, molehill_sim::grid::hash3(x as u32, y as u32, 0x51FE)) {
                Some(w) => { let k = w.color(); [k[0] / 2 + 10, k[1] / 2 + 10, k[2] / 2 + 10] } None => [14, 12, 12] },
            Some(m) => molehill_sim::mat::PROPS[m as usize].color,
        };
        px[i * 4..i * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    } }
    let mut dot = |x: i64, y: i64, r: i64, c: [u8; 3]| for yy in y - r..=y + r { for xx in x - r..=x + r {
        if xx >= 0 && yy >= 0 && (xx as usize) < t.w && (yy as usize) < t.h { let i = (yy as usize * t.w + xx as usize) * 4; px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]); } } };
    for th in t.things(var, left) {
        use molehill_sim::templates::Thing::*;
        match th {
            Torch { at } => dot(at[0] as i64, at[1] as i64, 1, [255, 200, 80]),
            Furn { at, .. } => dot(at[0] as i64, at[1] as i64, 2, [120, 170, 230]),
            Farm { at } => dot(at[0] as i64, at[1] as i64, 2, [120, 220, 120]),
            Ladder { at, .. } => for k in 0..(4 * CPM) as i64 { dot(at[0] as i64, at[1] as i64 + k, 0, [200, 150, 90]); },
            Cart { at } => dot(at[0] as i64, at[1] as i64 - 2, 2, [190, 190, 200]),
            Device { at, .. } => dot(at[0] as i64, at[1] as i64, 2, [230, 90, 80]),
            Wire { at } => dot(at[0] as i64, at[1] as i64, 1, [210, 120, 60]),
            SwitchOn { at } => dot(at[0] as i64, at[1] as i64, 1, [255, 255, 255]),
        }
    }
    if let Some((cx, cy)) = t.chest(var, left) { dot(cx as i64, cy as i64, 3, [240, 190, 40]); }
    px
}

/// Lay an overlay over drawn pixels: `tint` where `mask` is set, mixed by `a` (0..1).
pub fn overlay(px: &mut [u8], mask: impl Iterator<Item = bool>, tint: [u8; 3], a: f32) {
    for (i, on) in mask.enumerate() { if !on { continue; }
        for k in 0..3 { let p = &mut px[i * 4 + k]; *p = (*p as f32 * (1. - a) + tint[k] as f32 * a) as u8; } }
}

/// The families in key order 1–7.
pub fn families() -> [Kind; 7] { FAMILIES.map(|f| f.0) }
pub fn kind_label(k: Kind) -> &'static str { templates::file_name(k) }
