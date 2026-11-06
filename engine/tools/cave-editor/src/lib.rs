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
use molehill_sim::templates::{self, default_char, rle_decode, rle_encode, Placement, SiteFile, Template, ThingDef, FAMILIES, HOST};
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

/// What an edit can change, kept small: the grids as bytes (palette characters are ASCII), the things,
/// the chest, and the palette and lining mixes (strokes add entries to them).
struct Snap { cells: Vec<u8>, lining: Vec<u8>, things: Vec<ThingDef>, chest: Option<[usize; 2]>,
    palette: std::collections::BTreeMap<String, String>, linings: std::collections::BTreeMap<String, Vec<String>> }
/// Undo steps kept (on the largest piece, the mine, about 0.2 MB each).
pub const UNDO_STEPS: usize = 100;

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
    fn snap(&self) -> Snap {
        let b = |g: &[char]| g.iter().map(|&c| c as u8).collect();
        Snap { cells: b(&self.cells), lining: b(&self.lining), things: self.file.things.clone(), chest: self.file.chest, palette: self.file.palette.clone(), linings: self.file.linings.clone() }
    }
    fn restore(&mut self, s: Snap) {
        self.cells = s.cells.into_iter().map(char::from).collect(); self.lining = s.lining.into_iter().map(char::from).collect();
        self.file.things = s.things; self.file.chest = s.chest; self.file.palette = s.palette; self.file.linings = s.linings; self.dirty = true;
    }
    /// Call once at the start of each stroke or action, so one undo takes it back whole.
    pub fn begin(&mut self) { self.undo.push(self.snap()); if self.undo.len() > UNDO_STEPS { self.undo.remove(0); } self.redo.clear(); }
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
pub fn stamp(t: &Template, p: Placement) -> Result<Stamped, String> {
    let (ww, wh) = (((t.w + 4 * M + 127) / 64 * 64).max(1024), ((t.h + 6 * M + 127) / 64 * 64).max(512));
    let mut w = World::new(ww, wh, 1);
    w.disasters.enabled = false;
    let (x0, y0) = (64, wh - t.h - 3 * M);
    for y in y0 - M..y0 + t.h + M { for x in x0 - M..x0 + t.w + M { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); w.g.flags[i] = 0; } }
    w.tr.ladders.clear(); w.chests.clear(); w.torches.clear(); w.furniture.clear();
    let chest = w.stamp_template(t, x0, y0, p, 0x51FE);
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
pub fn reach(t: &Template, p: Placement) -> Result<Reach, String> {
    let s = stamp(t, p)?;
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
pub fn stress(t: &Template, p: Placement) -> Result<Vec<bool>, String> {
    let mut s = stamp(t, p)?;
    let fails = s.w.st.solve(&s.w.g);
    let mut over = vec![false; t.w * t.h];
    for i in fails { let (x, y) = (i as usize % s.w.g.w, i as usize / s.w.g.w);
        if x >= s.x0 && y >= s.y0 && x < s.x0 + t.w && y < s.y0 + t.h { over[(y - s.y0) * t.w + x - s.x0] = true; } }
    Ok(over)
}

/// Run the piece for `ticks` with a mole standing in it (at the chest, else the middle of the floor):
/// what it looks like after, and what happened.
pub struct Sim { pub cells: Vec<Mat>, pub changed: usize, pub crumbled: u64, pub hp: f32, pub ticks: u32 }
pub fn simulate(t: &Template, p: Placement, ticks: u32) -> Result<Sim, String> {
    let mut s = stamp(t, p)?;
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
pub fn draw(t: &Template, p: Placement, sim: Option<&[Mat]>) -> Vec<u8> {
    let mut px = vec![0u8; t.w * t.h * 4];
    for y in 0..t.h { for x in 0..t.w {
        let i = y * t.w + x;
        let m = match sim { Some(c) => Some(c[i]), None => t.placed_cell(x, y, p) };
        let c: [u8; 3] = match m {
            None => if (x + y) % 6 == 0 { [40, 36, 34] } else { [26, 23, 22] },
            Some(Mat::Empty) => match t.placed_lining(x, y, p, molehill_sim::grid::hash3(x as u32, y as u32, 0x51FE)) {
                Some(w) => { let k = w.color(); [k[0] / 2 + 10, k[1] / 2 + 10, k[2] / 2 + 10] } None => [14, 12, 12] },
            Some(m) => molehill_sim::mat::PROPS[m as usize].color,
        };
        px[i * 4..i * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    } }
    draw_markers(t, p, &mut px);
    px
}
/// The chest and things as markers, over whatever is drawn (so a check's tint doesn't hide them).
pub fn draw_markers(t: &Template, p: Placement, px: &mut [u8]) {
    let mut dot = |x: i64, y: i64, r: i64, c: [u8; 3]| for yy in y - r..=y + r { for xx in x - r..=x + r {
        if xx >= 0 && yy >= 0 && (xx as usize) < t.w && (yy as usize) < t.h { let i = (yy as usize * t.w + xx as usize) * 4; px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]); } } };
    for th in t.placed_things(p) {
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
    if let Some((cx, cy)) = t.placed_chest(p) { dot(cx as i64, cy as i64, 3, [240, 190, 40]); }
}

/// Lay an overlay over drawn pixels: `tint` where `mask` is set, mixed by `a` (0..1).
pub fn overlay(px: &mut [u8], mask: impl Iterator<Item = bool>, tint: [u8; 3], a: f32) {
    for (i, on) in mask.enumerate() { if !on { continue; }
        for k in 0..3 { let p = &mut px[i * 4 + k]; *p = (*p as f32 * (1. - a) + tint[k] as f32 * a) as u8; } }
}

/// The families in key order 1–7.
pub fn families() -> [Kind; 7] { FAMILIES.map(|f| f.0) }
pub fn kind_label(k: Kind) -> &'static str { templates::file_name(k) }

// ---------------------------------------------------------------- the editor's state and panel (no Bevy)

/// Panel geometry the window lays rows out with, and clicks are mapped back through.
pub const PANEL_W: f32 = 330.;
pub const ROW_TOP: f32 = 8.;
pub const LINE: f32 = 17.;
/// Rows the window lays out (the panel never needs more: see `panel_fits`); the window is never shorter
/// than they need (`MIN_WINDOW_H`).
pub const ROWS: usize = 53;
pub const MIN_WINDOW_H: f32 = ROW_TOP * 2. + ROWS as f32 * LINE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool { Material, Lining, Thing, Chest }
pub enum Overlay { None, Reach(Reach), Stress(Vec<bool>), Sim(Sim) }
/// Everything a panel row or a key does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act { Family(usize), Tool(Tool), Pal(usize), PalStep(i64), Var(i64), Side, Depth, Brush(f32), Reach, Stress, Simulate, Clear, Undo, Redo, Save, Fit }
/// A panel row: its text, what clicking it does, and whether it shows as the current choice.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PanelRow { pub text: String, pub act: Option<Act>, pub on: bool }
/// Mouse buttons on the canvas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Press { Left, LeftHeld, Right }

pub struct Editor {
    pub fam: usize, pub doc: Doc, pub var: u32, pub left: bool, pub tool: Tool,
    /// the depth (m) the preview places the piece at (depth remaps)
    pub depth: f32,
    pub mat_i: usize, pub wall_i: usize, pub thing_i: usize, pub brush: f32,
    pub overlay: Overlay, pub msg: String,
    /// the canvas needs redrawing / the view refitting (the window clears these)
    pub redraw: bool, pub fit: bool,
    /// where saves go (assets/sites/ normally; tests point it elsewhere)
    pub save_dir: PathBuf,
    stroke: bool, discard: Option<usize>,
    pub mats: Vec<Option<Mat>>, pub walls: Vec<Option<Wall>>, pub things: Vec<ThingDef>,
}

impl Editor {
    pub fn new(fam: usize) -> Result<Editor, String> {
        let doc = Doc::open(families()[fam])?;
        let mut mats = vec![None]; mats.extend(paint_mats().into_iter().map(Some));
        let mut walls = vec![None]; walls.extend(paint_walls().into_iter().map(Some));
        let mat_i = mats.iter().position(|m| *m == Some(Mat::Rock)).unwrap_or(0);
        Ok(Editor { fam, doc, var: 0, left: false, depth: DEPTHS[1], tool: Tool::Material, mat_i, wall_i: 1, thing_i: 0, brush: 2., overlay: Overlay::None,
            msg: "Opened. Paint with the left mouse; the right mouse picks or removes.".into(), redraw: true, fit: true, save_dir: sites_dir(),
            stroke: false, discard: None, mats, walls, things: thing_kinds() })
    }
    pub fn kind(&self) -> Kind { families()[self.fam] }
    /// How the preview places the piece: the variant, the side and the depth.
    pub fn placement(&self) -> Placement { Placement { var: self.var, left: self.left, depth: self.depth } }
    /// A canvas column in the template's own cells (a flipped preview is painted where the template has it).
    fn to_template(&self, x: usize) -> usize {
        match self.doc.template() { Ok(t) if t.mirrored(self.var) => t.w - 1 - x.min(t.w - 1), _ => x }
    }
    pub fn pal_len(&self) -> usize { match self.tool { Tool::Material | Tool::Chest => self.mats.len(), Tool::Lining => self.walls.len(), Tool::Thing => self.things.len() } }
    pub fn pal_cur(&self) -> usize { match self.tool { Tool::Material | Tool::Chest => self.mat_i, Tool::Lining => self.wall_i, Tool::Thing => self.thing_i } }
    fn set_pal(&mut self, i: usize) { match self.tool { Tool::Material | Tool::Chest => self.mat_i = i, Tool::Lining => self.wall_i = i, Tool::Thing => self.thing_i = i } }
    pub fn pal_label(&self, i: usize) -> String {
        match self.tool {
            Tool::Material | Tool::Chest => self.mats[i].map_or("(host rock)".into(), |m| format!("{m:?}")),
            Tool::Lining => self.walls[i].map_or("(no lining)".into(), |w| format!("{w:?}")),
            Tool::Thing => thing_label(&self.things[i]),
        }
    }
    fn open(&mut self, fam: usize) {
        if fam == self.fam && !self.doc.dirty { return; }
        if self.doc.dirty && self.discard != Some(fam) { self.discard = Some(fam); self.msg = "Unsaved changes: press again to discard them, or Ctrl+S first.".into(); return; }
        match Doc::open(families()[fam]) {
            Ok(d) => { self.doc = d; self.fam = fam; self.var = 0; self.left = false; self.overlay = Overlay::None; self.redraw = true; self.fit = true; self.discard = None;
                self.msg = format!("Opened {}.", kind_label(self.kind())); }
            Err(e) => self.msg = e,
        }
    }
    pub fn run(&mut self, a: Act) {
        match a {
            Act::Family(k) => self.open(k),
            Act::Tool(t) => { self.tool = t; self.msg = match t {
                Tool::Material => "Paint materials: left paints, right picks the one under the cursor.",
                Tool::Lining => "Paint the back-wall lining (behind open space; it holds nothing up).",
                Tool::Thing => "Set things down: left sets the one picked, right takes the nearest away.",
                Tool::Chest => "Left sets the chest (in open space, just above a floor); right removes it." }.into(); }
            Act::Pal(i) => { if i < self.pal_len() { self.set_pal(i); } }
            Act::PalStep(d) => { let n = self.pal_len().max(1) as i64; let i = (self.pal_cur() as i64 + d).rem_euclid(n) as usize; self.set_pal(i); }
            Act::Var(d) => { self.var = (self.var as i64 + d).clamp(0, (1 << 20) - 1) as u32; self.overlay = Overlay::None; self.redraw = true; }
            Act::Side => { self.left = !self.left; self.overlay = Overlay::None; self.redraw = true; }
            Act::Depth => { let i = DEPTHS.iter().position(|&d| d == self.depth).map_or(0, |i| (i + 1) % DEPTHS.len()); self.depth = DEPTHS[i]; self.overlay = Overlay::None; self.redraw = true; }
            Act::Brush(d) => self.brush = (self.brush + d).clamp(0.5, 24.),
            Act::Reach | Act::Stress | Act::Simulate => {
                let t = match self.doc.template() { Ok(t) => t, Err(e) => { self.msg = e; return; } };
                let r = match a {
                    Act::Reach => reach(&t, self.placement()).map(|r| {
                        self.msg = if r.problems.is_empty() { format!("Walkable: every way in reaches the {} (green; orange is open space the mole can't get to). Gravity is ignored: G runs it for real.", if t.placed_chest(self.placement()).is_some() { "chest" } else { "floor" }) } else { r.problems.join(" / ") };
                        Overlay::Reach(r) }),
                    Act::Stress => stress(&t, self.placement()).map(|o| { let n = o.iter().filter(|&&b| b).count();
                        self.msg = if n == 0 { "Sound: nothing over capacity.".into() } else { format!("{n} cells over capacity (red): they crack and fall in play.") }; Overlay::Stress(o) }),
                    _ => simulate(&t, self.placement(), 600).map(|s| {
                        self.msg = format!("After 10 s with a mole at the chest: {} cells changed, {} crumbled, hp {:.0}.", s.changed, s.crumbled, s.hp); Overlay::Sim(s) }),
                };
                match r { Ok(o) => { self.overlay = o; self.redraw = true; } Err(e) => self.msg = e }
            }
            Act::Clear => { self.overlay = Overlay::None; self.redraw = true; }
            Act::Undo => { if self.doc.undo() { self.overlay = Overlay::None; self.redraw = true; self.msg = "Undone.".into(); } else { self.msg = "Nothing to undo.".into(); } }
            Act::Redo => { if self.doc.redo() { self.overlay = Overlay::None; self.redraw = true; self.msg = "Redone.".into(); } else { self.msg = "Nothing to redo.".into(); } }
            Act::Save => {
                let p = self.save_dir.join(format!("{}.json", kind_label(self.kind())));
                self.msg = match self.doc.save(&p) { Ok(()) => format!("Saved {}. It is the editor's now: export_sites keeps it.", p.display()), Err(e) => format!("Not saved: {e}") };
            }
            Act::Fit => self.fit = true,
        }
    }
    /// A key (the letter or symbol as typed, `^` prefix for Ctrl/Cmd, upper case for Shift on letters).
    pub fn key(&mut self, k: &str) {
        let a = match k {
            "1" | "2" | "3" | "4" | "5" | "6" | "7" => Act::Family(k.parse::<usize>().unwrap() - 1),
            "m" => Act::Tool(Tool::Material), "l" => Act::Tool(Tool::Lining), "t" => Act::Tool(Tool::Thing), "c" => Act::Tool(Tool::Chest),
            "[" => Act::Brush(-0.5), "]" => Act::Brush(0.5), "," => Act::PalStep(-1), "." => Act::PalStep(1),
            "v" => Act::Var(-1), "b" => Act::Var(1), "V" => Act::Var(-256), "B" => Act::Var(256), "n" => Act::Side, "e" => Act::Depth,
            "r" => Act::Reach, "p" => Act::Stress, "g" => Act::Simulate, "esc" => Act::Clear, "f" => Act::Fit,
            "^s" => Act::Save, "^z" => Act::Undo, "^Z" | "^y" => Act::Redo,
            _ => return,
        };
        self.run(a);
    }
    /// The left panel, top to bottom.
    pub fn rows(&self, slots: &[(String, String)]) -> Vec<PanelRow> {
        let mut r: Vec<PanelRow> = vec![];
        let mut row = |text: String, act: Option<Act>, on: bool| r.push(PanelRow { text, act, on });
        row(format!("CAVE EDITOR  (dev only, not shipped){}", if self.doc.dirty { "  *unsaved*" } else { "" }), None, false);
        row(String::new(), None, false);
        for (k, &f) in families().iter().enumerate() { row(format!("{}  {}", k + 1, kind_label(f)), Some(Act::Family(k)), k == self.fam); }
        row(String::new(), None, false);
        let t = self.tool;
        for (key, label, tool) in [("M", "paint material", Tool::Material), ("L", "paint lining", Tool::Lining), ("T", "set things (torch, furniture, rope...)", Tool::Thing), ("C", "move the chest", Tool::Chest)] {
            row(format!("{key}  {label}"), Some(Act::Tool(tool)), t == tool);
        }
        row(format!("[ ]  brush radius {:.1} cells ({:.2} m)", self.brush, self.brush / CPM as f32), None, false);
        row("      smaller".into(), Some(Act::Brush(-0.5)), false);
        row("      bigger".into(), Some(Act::Brush(0.5)), false);
        row(String::new(), None, false);
        if t != Tool::Chest {
            let (n, cur) = (self.pal_len(), self.pal_cur());
            let first = cur.saturating_sub(5).min(n.saturating_sub(PALETTE_ROWS));
            row(format!(",  .  palette ({} of {n})", cur + 1), None, false);
            for i in first..(first + PALETTE_ROWS).min(n) { row(format!("{} {}", if i == cur { ">" } else { " " }, self.pal_label(i)), Some(Act::Pal(i)), i == cur); }
        } else { row("Left sets the chest, right removes it.".into(), None, false); }
        row(String::new(), None, false);
        row(format!("V / B  variant var = {}   (Shift: 256)", self.var), None, false);
        row("      -1".into(), Some(Act::Var(-1)), false);
        row("      +1".into(), Some(Act::Var(1)), false);
        row(format!("N  side: {}", if self.left { "west (left)" } else { "east" }), Some(Act::Side), self.left);
        for (name, opt) in slots { row(format!("      {name}: {opt}"), None, false); }
        row(format!("E  depth {:.0} m (remaps)", self.depth), Some(Act::Depth), false);
        row(String::new(), None, false);
        row("R  can the mole walk it?".into(), Some(Act::Reach), matches!(self.overlay, Overlay::Reach(_)));
        row("P  will it stand? (stress)".into(), Some(Act::Stress), matches!(self.overlay, Overlay::Stress(_)));
        row("G  run it 10 s with a mole in it".into(), Some(Act::Simulate), matches!(self.overlay, Overlay::Sim(_)));
        row("Esc  clear the check".into(), Some(Act::Clear), false);
        row(String::new(), None, false);
        row("Ctrl/Cmd+Z  undo".into(), Some(Act::Undo), false);
        row("Ctrl/Cmd+Shift+Z or Y  redo".into(), Some(Act::Redo), false);
        row("Ctrl/Cmd+S  save to assets/sites/".into(), Some(Act::Save), false);
        row("F  fit   (wheel or +/- zoom, arrows/WASD pan)".into(), Some(Act::Fit), false);
        row(String::new(), None, false);
        for l in wrap(&self.msg, 44).into_iter().take(MSG_ROWS) { row(l, None, false); }
        r
    }
    /// The row under a cursor at window y (logical pixels).
    pub fn row_at(y: f32) -> Option<usize> { let i = ((y - ROW_TOP) / LINE).floor(); (i >= 0.).then_some(i as usize) }
    /// A click on the panel: runs that row's command, if it has one.
    pub fn click_panel(&mut self, y: f32, slots: &[(String, String)]) -> bool {
        let Some(i) = Editor::row_at(y) else { return false };
        match self.rows(slots).get(i).and_then(|r| r.act) { Some(a) => { self.run(a); true } None => false }
    }
    /// The mouse on the canvas at cell (x, y).
    pub fn press(&mut self, x: usize, y: usize, p: Press) {
        let (var, left) = (self.var, self.left);
        let x = self.to_template(x);
        match (self.tool, p) {
            (Tool::Material | Tool::Lining, Press::Left | Press::LeftHeld) => {
                if p == Press::Left || !self.stroke { self.doc.begin(); self.stroke = true; }
                let b = if self.tool == Tool::Material { Brush::Mat(self.mats[self.mat_i]) } else { Brush::Lining(self.walls[self.wall_i]) };
                if self.doc.paint(x as i64, y as i64, self.brush, b) > 0 { self.overlay = Overlay::None; self.redraw = true; }
            }
            (Tool::Material, Press::Right) => {
                if let Some(Brush::Mat(m)) = self.doc.pick(x, y) { if let Some(i) = self.mats.iter().position(|&q| q == m) { self.mat_i = i; self.msg = format!("Picked {}.", self.pal_label(i)); } }
            }
            (Tool::Lining, Press::Right) => {}
            (Tool::Thing, Press::Left) => { self.doc.begin(); let t = self.things[self.thing_i].clone(); self.doc.place_thing(&t, x, y, var, left); self.msg = format!("Set down a {} at ({x}, {y}).", thing_label(&t)); self.redraw = true; }
            (Tool::Thing, Press::Right) => { self.doc.begin(); self.msg = if self.doc.remove_thing_near(x, y, 6.) { self.redraw = true; "Took the nearest thing away.".into() } else { "No thing within 6 cells.".into() }; }
            (Tool::Chest, Press::Left) => { self.doc.begin(); self.doc.set_chest(Some([x, y])); self.msg = format!("Chest at ({x}, {y})."); self.redraw = true; }
            (Tool::Chest, Press::Right) => { self.doc.begin(); self.doc.set_chest(None); self.msg = "No chest.".into(); self.redraw = true; }
            (Tool::Thing | Tool::Chest, Press::LeftHeld) => {}
        }
    }
    /// The mouse button let go: the stroke ends (the next press starts a new undo step).
    pub fn release(&mut self) { self.stroke = false; }
    /// The variant's slot choices, for the panel: (slot, option) names.
    pub fn slot_choices(&self, t: &Template) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = t.file.slots.iter().map(|s| { let o = s.pick.choose(self.var, self.left).min(s.options.len() - 1); (s.name.clone(), s.options[o].name.clone()) }).collect();
        // the seeded variation (from var 256 up), as the game would place it
        let mut on = vec![];
        if t.mirrored(self.var) { on.push("flipped".to_string()); }
        for (z, zone) in t.variation.zones.iter().enumerate() { if t.zone_on(z, self.var) { on.push(zone.tag.clone()); } else if zone.tag == "chest" && self.var >= templates::VARY_FROM { on.push("no chest".into()); } }
        if !t.variation.is_empty() { v.push(("varied".into(), if self.var < templates::VARY_FROM { "no (var under 256)".into() } else if on.is_empty() { "nothing this time".into() } else { on.join(", ") })); }
        v
    }
    /// The canvas pixels with the current check laid over.
    pub fn pixels(&self, t: &Template) -> Vec<u8> {
        let sim = if let Overlay::Sim(s) = &self.overlay { Some(s.cells.as_slice()) } else { None };
        let mut px = draw(t, self.placement(), sim);
        match &self.overlay {
            Overlay::Reach(r) => {
                overlay(&mut px, r.region.iter().map(|&k| k != 0 && r.ways_in.contains(&k)), [80, 220, 120], 0.35);
                overlay(&mut px, r.region.iter().map(|&k| k != 0 && !r.ways_in.contains(&k)), [240, 140, 40], 0.5);
            }
            Overlay::Stress(o) => overlay(&mut px, o.iter().copied(), [255, 40, 40], 0.75),
            _ => {}
        }
        draw_markers(t, self.placement(), &mut px);
        px
    }
}
/// Depths the preview cycles through with `E` (metres; the wet up, the holt layers, the hot deep).
pub const DEPTHS: [f32; 3] = [10., 50., 80.];
/// Palette entries shown at once, and message lines kept.
pub const PALETTE_ROWS: usize = 9;
pub const MSG_ROWS: usize = 4;
fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut out = vec![]; let mut line = String::new();
    for w in s.split(' ') { if !line.is_empty() && line.len() + 1 + w.len() > width { out.push(std::mem::take(&mut line)); } if !line.is_empty() { line.push(' '); } line.push_str(w); }
    if !line.is_empty() { out.push(line); }
    out
}
/// The longest the panel gets (the halt and mine have the most slot rows), against the rows the window has.
pub fn panel_fits() -> bool {
    // the most slot rows any piece shows (the mine's three) and the variation row
    let slots = vec![(String::new(), String::new()); 4];
    let mut e = Editor::new(0).unwrap();
    e.msg = "x ".repeat(200);
    let n = e.rows(&slots).len();
    if n > ROWS { eprintln!("the panel needs {n} rows, the window has {ROWS}"); }
    n <= ROWS
}

/// Write pixels as a 24-bit BMP (`--canvas`, `--shot`).
pub fn write_bmp(path: &Path, w: usize, h: usize, rgba: &[u8], bgra: bool) -> std::io::Result<()> {
    let row = (w * 3 + 3) / 4 * 4;
    let mut out = Vec::with_capacity(54 + row * h);
    let le = |v: u32| v.to_le_bytes();
    out.extend_from_slice(b"BM"); out.extend_from_slice(&le((54 + row * h) as u32)); out.extend_from_slice(&[0; 4]); out.extend_from_slice(&le(54));
    out.extend_from_slice(&le(40)); out.extend_from_slice(&le(w as u32)); out.extend_from_slice(&le(h as u32)); out.extend_from_slice(&[1, 0, 24, 0]);
    out.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() {
        for x in 0..w { let i = (y * w + x) * 4; let (r, g, b) = if bgra { (rgba[i + 2], rgba[i + 1], rgba[i]) } else { (rgba[i], rgba[i + 1], rgba[i + 2]) }; out.extend_from_slice(&[b, g, r]); }
        out.extend(std::iter::repeat(0).take(row - w * 3));
    }
    std::fs::write(path, out)
}
