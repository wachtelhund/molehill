//! Found-place templates: the seven site families as data files under `assets/sites/` (JSON).
//!
//! A template is one piece of a found place at cell resolution (`cpm` cells per metre, checked against
//! `mat::CPM`): a grid of materials (or "leave the host rock"), a grid of linings, the things it owns
//! (torches, furniture, farms, ladders, carts, devices and copperline), a chest, and tagged connectors
//! (its ways in). Variation is data too:
//!
//! * a **slot** has alternatives (options), each a sparse patch over the base grid (plus a chest move),
//!   picked from the site's `var` (`(var >> shift) % modulo`, then through `table` if there is one) or
//!   from its side (`left`). Slots patch disjoint cells, so their order doesn't matter.
//! * a **role** is a material decided per site (a mine's ore), written as one palette character.
//! * a **lining** mix is a list of walls; a cell takes entry `hash % len` of its own mix, from the same
//!   per-cell hash the old procedural linings used.
//!
//! Rows are run-length coded: `12.3#o` is twelve of `.`, three of `#`, one `o`. `_` leaves the host.
//! The game reads the files compiled in (`include_str!`), so web and native builds see the same data.
//! The external cave editor (`tools/cave-editor`, never shipped) writes them; `examples/export_sites.rs`
//! regenerates them from the procedural reference layouts in `sites.rs`, and the golden test
//! `templates_match_the_procedural_sites` keeps both equal cell for cell.
use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::craft::{Item, ITEMS};
use crate::logic::{Dev, DEVS};
use crate::mat::{Mat, CPM, NMAT};
use crate::sites::Kind;
use crate::walls::{Wall, NWALL};

pub const FORMAT: u32 = 1;
/// Palette character for "leave the host rock" (and, in the lining grid, "no lining of its own").
pub const HOST: char = '_';
/// Grid codes: below NMAT a material, then roles, then host / keep.
const ROLE0: u8 = 200;
const HOST_CODE: u8 = 254;
const KEEP: u8 = 255;

/// The seven families, in `Kind::from_u8` order, with their file names (`assets/sites/<file>.json`).
pub const FAMILIES: [(Kind, &str); 7] = [
    (Kind::Mine, "mine"), (Kind::Holt, "holt"), (Kind::Pumpworks, "pumpworks"), (Kind::Fissure, "fissure"),
    (Kind::Fossil, "fossil"), (Kind::Crystal, "crystal"), (Kind::Halt, "halt"),
];
pub fn file_name(kind: Kind) -> &'static str { FAMILIES.iter().find(|f| f.0 == kind).unwrap().1 }

/// Default palette characters (the exporter and editor use these; a file may choose its own).
/// Mostly the scenario map legend (TESTING.md), plus the materials it doesn't have.
pub fn default_char(m: Mat) -> char {
    use Mat::*;
    match m {
        Empty => '.', Bedrock => 'B', Granite => '#', Rock => 'r', Limestone => 'L', Basalt => 'a', Dirt => 'd', Clay => 'c',
        Sand => 's', Gravel => 'g', Coal => 'o', CopperOre => 'u', TinOre => 'n', IronOre => 'i', Quartz => 'Q', Crystal => 'K',
        Roots => 't', Fungus => 'f', Grass => 'v', WoodBeam => 'b', StoneBrick => 'k', IronBeam => 'I', Door => 'D', Water => 'w',
        Lava => 'l', Steam => '~', Fire => '^', Glass => 'G', RootTree => 'T', TreeRoot => 'R', Moss => 'm', Mycelium => 'y',
        Planks => 'p', Polished => 'q', Tiles => 'x', Carved => 'C', Rail => '=', Tube => '%',
        Chute => 'h', Screen => 'H', Bumper => '|', Riffle => 'z',
        Shaft => 'j', Gear => '*', Wheel => 'O', Belt => '>', BeltL => '<', Drum => 'U', Engine => 'E', SteamPipe => 'P', Boiler => 'K',
        StampMill => 'M', Washer => 'W', Smelter => 'S', Bellows => 'Y', Mould => 'V', MouldHot => 'X',
    }
}
pub fn mat_named(s: &str) -> Option<Mat> { (0..NMAT as u8).map(Mat::from_u8).find(|m| format!("{m:?}") == s) }
pub fn wall_named(s: &str) -> Option<Wall> { (0..NWALL as u8).map(Wall::from_u8).find(|w| format!("{w:?}") == s) }
pub fn item_named(s: &str) -> Option<Item> { ITEMS.iter().copied().find(|i| format!("{i:?}") == s) }
pub fn dev_named(s: &str) -> Option<Dev> { DEVS.iter().copied().find(|d| format!("{d:?}") == s) }

// ---------------------------------------------------------------- the file

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SiteFile {
    pub format: u32,
    /// who owns the layout: `export` (written by `export_sites` from the procedural reference in sites.rs,
    /// and checked against it) or `editor` (painted in the cave editor; the exporter leaves it alone and
    /// the golden check skips it). Exported files leave the field out.
    #[serde(default = "export_source", skip_serializing_if = "is_export")]
    pub source: String,
    /// the family this piece belongs to (`mine`, `holt`, `pump`, `fissure`, `fossil`, `crystal`, `halt`)
    pub family: String,
    pub cpm: usize,
    /// width, height in cells
    pub size: [usize; 2],
    /// palette character → material name (`mat::Mat`), or `@role` for a role
    pub palette: BTreeMap<String, String>,
    /// material rows, run-length coded, top to bottom
    pub cells: Vec<String>,
    /// lining character → its mix of walls (`walls::Wall` names); a cell takes entry `hash % len`
    pub linings: BTreeMap<String, Vec<String>>,
    /// lining rows, run-length coded
    pub lining: Vec<String>,
    #[serde(default)]
    pub roles: Vec<RoleDef>,
    #[serde(default)]
    pub slots: Vec<SlotDef>,
    /// the chest's cell (from the box's top-left), if the place has one
    #[serde(default)]
    pub chest: Option<[usize; 2]>,
    #[serde(default)]
    pub things: Vec<ThingDef>,
    #[serde(default)]
    pub connectors: Vec<Connector>,
    /// How each placed site of this piece varies (see `Variation`). Not part of the exported layout: the
    /// exporter carries it over unchanged and the golden check doesn't read it.
    #[serde(default, skip_serializing_if = "Variation::is_empty")]
    pub variation: Variation,
}

/// Seeded variation laid over a template when a site is placed: never at `var` below `VARY_FROM` (the
/// scripted stamps and the golden range), on almost every site the generator places (its `var` is 20
/// random bits). Each roll is a hash of `var >> 8` and the roll's index, so a site always comes out the same.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Variation {
    /// half the sites are flipped east-west (cells, linings, chest, things)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mirror: bool,
    /// materials swapped by the depth of the site's middle (metres; + is down)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remap: Vec<Remap>,
    /// tagged random zones, in template (unmirrored) cells
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<Zone>,
}
impl Variation { pub fn is_empty(&self) -> bool { *self == Variation::default() } }
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Remap {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deeper_than: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shallower_than: Option<f64>,
}
/// A tagged random zone. `tag`: `chest` (the chest is there with `chance`), `furniture` (one of `sets` of
/// thing indices is kept, the other sets' things are dropped), `may-collapse` (with `chance`, the open
/// cells in `rect` [x, y, w, h] fill with `fill`, rubble that settles as loose ground).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Zone {
    pub tag: String,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub chance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<[usize; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sets: Vec<Vec<usize>>,
}
fn one() -> f64 { 1. }
fn is_one(v: &f64) -> bool { *v == 1. }
/// Variation starts at this `var` (below it a site is stamped exactly as its template).
pub const VARY_FROM: u32 = 1 << 8;
/// Where and how a site is placed: its layout knobs and the depth (m) of its middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement { pub var: u32, pub left: bool, pub depth: f32 }

fn export_source() -> String { "export".into() }
fn is_export(s: &String) -> bool { s == "export" }
impl SiteFile {
    /// Painted in the cave editor (not exported from sites.rs).
    pub fn from_editor(&self) -> bool { self.source == "editor" }
}

/// Which option a site takes: from its `var` (`(var >> shift) % modulo`, then `table[...]` if given) or
/// its side (`left`: 0 east, 1 west).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pick {
    pub from: String,
    #[serde(default)]
    pub shift: u32,
    #[serde(default)]
    pub modulo: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table: Vec<u32>,
}
impl Pick {
    pub fn choose(&self, var: u32, left: bool) -> usize {
        let i = match self.from.as_str() { "left" => left as u32, _ => (var >> self.shift) % self.modulo.max(1) };
        if self.table.is_empty() { i as usize } else { self.table[i as usize % self.table.len()] as usize }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoleDef {
    pub name: String,
    /// its palette character
    pub char: String,
    pub pick: Pick,
    /// material names, one per choice
    pub options: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SlotDef {
    pub name: String,
    pub pick: Pick,
    pub options: Vec<OptionDef>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct OptionDef {
    #[serde(default)]
    pub name: String,
    /// row spans laid over the base: (x, y, run-length coded cells)
    #[serde(default)]
    pub patch: Vec<(usize, usize, String)>,
    /// lining spans, the same way (cells this option opens need a lining)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lining: Vec<(usize, usize, String)>,
    /// moves the chest
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chest: Option<[usize; 2]>,
}

/// Something a site owns besides cells, at a cell from the box's top-left. `what`: torch, furniture
/// (`item`), farm, ladder (`anchor`: the rock it hangs from), cart (`at` may be fractional), device
/// (`item` is the `logic::Dev` name), wire, on (switch the device there on).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ThingDef {
    pub what: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    pub at: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<[usize; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}
/// Only in this option of a slot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct When { pub slot: String, pub option: usize }

/// A tagged way in or out of a piece: `tag` (entrance, tube, shaft, …), the side it opens on (west, east,
/// north, south), where along that side it starts (cells) and how long it is.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Connector {
    pub tag: String,
    pub side: String,
    pub at: usize,
    pub len: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

// ---------------------------------------------------------------- run-length rows

pub fn rle_encode(row: impl IntoIterator<Item = char>) -> String {
    let mut out = String::new();
    let mut run: Option<(char, usize)> = None;
    let flush = |out: &mut String, r: (char, usize)| { if r.1 > 1 { out.push_str(&r.1.to_string()); } out.push(r.0); };
    for c in row {
        match run { Some((p, n)) if p == c => run = Some((p, n + 1)), Some(r) => { flush(&mut out, r); run = Some((c, 1)); } None => run = Some((c, 1)) }
    }
    if let Some(r) = run { flush(&mut out, r); }
    out
}
pub fn rle_decode(s: &str) -> Result<Vec<char>, String> {
    let mut out = vec![]; let mut n = 0usize; let mut has = false;
    for c in s.chars() {
        if let Some(d) = c.to_digit(10) { n = n * 10 + d as usize; has = true; continue; }
        for _ in 0..if has { n } else { 1 } { out.push(c); }
        n = 0; has = false;
    }
    if has { return Err(format!("row ends in a count: {s:?}")); }
    Ok(out)
}

/// The file as JSON the way the exporter and editor write it: pretty, but each row, span, thing and
/// connector on one line, so a diff shows what changed.
pub fn to_json(f: &SiteFile) -> String {
    // inline what fits on a line: a span, a thing, a short list; rows and long lists get a line each
    fn flat(v: &serde_json::Value) -> bool {
        let short = serde_json::to_string(v).map_or(false, |t| t.len() <= 110);
        match v {
            serde_json::Value::Array(a) => short && a.iter().all(|e| !e.is_object() && (!e.is_array() || flat(e))),
            serde_json::Value::Object(o) => short && o.values().all(|e| !e.is_array() || flat(e)),
            _ => true,
        }
    }
    fn put(v: &serde_json::Value, ind: usize, out: &mut String) {
        let pad = |n: usize| "  ".repeat(n);
        match v {
            serde_json::Value::Array(a) if !a.is_empty() && !flat(v) => {
                out.push_str("[\n");
                for (i, e) in a.iter().enumerate() { out.push_str(&pad(ind + 1)); put(e, ind + 1, out); if i + 1 < a.len() { out.push(','); } out.push('\n'); }
                out.push_str(&pad(ind)); out.push(']');
            }
            serde_json::Value::Object(o) if !o.is_empty() && (!flat(v) || ind == 0) => {
                out.push_str("{\n");
                for (i, (k, e)) in o.iter().enumerate() {
                    out.push_str(&pad(ind + 1)); out.push_str(&serde_json::to_string(k).unwrap()); out.push_str(": "); put(e, ind + 1, out);
                    if i + 1 < o.len() { out.push(','); } out.push('\n');
                }
                out.push_str(&pad(ind)); out.push('}');
            }
            // a whole-cell position reads 64, not 64.0
            serde_json::Value::Number(n) if n.as_f64().is_some_and(|f| f.fract() == 0. && f.abs() < 1e15) && !n.is_i64() && !n.is_u64() => {
                out.push_str(&(n.as_f64().unwrap() as i64).to_string())
            }
            serde_json::Value::Array(a) if flat(v) => {
                out.push('[');
                for (i, e) in a.iter().enumerate() { if i > 0 { out.push(','); } put(e, ind + 1, out); }
                out.push(']');
            }
            serde_json::Value::Object(o) if flat(v) => {
                out.push('{');
                for (i, (k, e)) in o.iter().enumerate() { if i > 0 { out.push(','); } out.push_str(&serde_json::to_string(k).unwrap()); out.push(':'); put(e, ind + 1, out); }
                out.push('}');
            }
            _ => out.push_str(&serde_json::to_string(v).unwrap()),
        }
    }
    let mut out = String::new();
    put(&serde_json::to_value(f).unwrap(), 0, &mut out);
    out.push('\n');
    out
}

// ---------------------------------------------------------------- runtime

/// A thing at a window (or box-relative) cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Thing {
    Torch { at: [usize; 2] },
    Furn { item: Item, at: [usize; 2] },
    Farm { at: [usize; 2] },
    Ladder { at: [usize; 2], anchor: [usize; 2] },
    Cart { at: [f32; 2] },
    Device { dev: Dev, at: [usize; 2] },
    Wire { at: [usize; 2] },
    SwitchOn { at: [usize; 2] },
}
impl Thing {
    pub fn offset(self, x0: usize, y0: usize) -> Thing {
        let o = |a: [usize; 2]| [a[0] + x0, a[1] + y0];
        match self {
            Thing::Torch { at } => Thing::Torch { at: o(at) },
            Thing::Furn { item, at } => Thing::Furn { item, at: o(at) },
            Thing::Farm { at } => Thing::Farm { at: o(at) },
            Thing::Ladder { at, anchor } => Thing::Ladder { at: o(at), anchor: o(anchor) },
            Thing::Cart { at } => Thing::Cart { at: [at[0] + x0 as f32, at[1] + y0 as f32] },
            Thing::Device { dev, at } => Thing::Device { dev, at: o(at) },
            Thing::Wire { at } => Thing::Wire { at: o(at) },
            Thing::SwitchOn { at } => Thing::SwitchOn { at: o(at) },
        }
    }
    pub fn to_def(self, when: Option<When>) -> ThingDef {
        let p = |a: [usize; 2]| [a[0] as f32, a[1] as f32];
        let (what, item, at, anchor) = match self {
            Thing::Torch { at } => ("torch", None, p(at), None),
            Thing::Furn { item, at } => ("furniture", Some(format!("{item:?}")), p(at), None),
            Thing::Farm { at } => ("farm", None, p(at), None),
            Thing::Ladder { at, anchor } => ("ladder", None, p(at), Some(anchor)),
            Thing::Cart { at } => ("cart", None, at, None),
            Thing::Device { dev, at } => ("device", Some(format!("{dev:?}")), p(at), None),
            Thing::Wire { at } => ("wire", None, p(at), None),
            Thing::SwitchOn { at } => ("on", None, p(at), None),
        };
        ThingDef { what: what.into(), item, at, anchor, when }
    }
    pub fn from_def(d: &ThingDef) -> Result<Thing, String> {
        let at = [d.at[0] as usize, d.at[1] as usize];
        let item = || d.item.as_deref().and_then(item_named).ok_or_else(|| format!("{}: unknown item {:?}", d.what, d.item));
        Ok(match d.what.as_str() {
            "torch" => Thing::Torch { at },
            "furniture" => Thing::Furn { item: item()?, at },
            "farm" => Thing::Farm { at },
            "ladder" => Thing::Ladder { at, anchor: d.anchor.ok_or("ladder without an anchor")? },
            "cart" => Thing::Cart { at: d.at },
            "device" => Thing::Device { dev: d.item.as_deref().and_then(dev_named).ok_or_else(|| format!("unknown device {:?}", d.item))?, at },
            "wire" => Thing::Wire { at },
            "on" => Thing::SwitchOn { at },
            w => return Err(format!("unknown thing {w:?}")),
        })
    }
}

struct Slot {
    name: String,
    pick: Pick,
    n: usize,
    /// per cell: u32::MAX if no option touches it, else the row into `vals` (n per row)
    at: Vec<u32>,
    vals: Vec<u8>,
    lining: Vec<u8>,
    chest: Vec<Option<(usize, usize)>>,
}

/// A loaded template, ready to stamp.
pub struct Template {
    pub kind: Kind,
    pub w: usize,
    pub h: usize,
    pub file: SiteFile,
    cells: Vec<u8>,
    lining: Vec<u8>,
    mixes: Vec<Vec<Wall>>,
    roles: Vec<(Pick, Vec<Mat>)>,
    slots: Vec<Slot>,
    chest: Option<(usize, usize)>,
    things: Vec<(Thing, Option<(usize, usize)>)>,
    pub variation: Variation,
    zone_fill: Vec<Option<Mat>>,
    remap_to: Vec<(Mat, Mat)>,
}

impl Template {
    pub fn parse(json: &str) -> Result<Template, String> {
        let f: SiteFile = serde_json::from_str(json).map_err(|e| e.to_string())?;
        Template::from_file(f)
    }
    pub fn from_file(f: SiteFile) -> Result<Template, String> {
        if f.format != FORMAT { return Err(format!("format {} (this build reads {FORMAT})", f.format)); }
        if f.cpm != CPM { return Err(format!("drawn at {} cells/m, the game runs at {CPM}", f.cpm)); }
        let kind = Kind::from_name(&f.family).ok_or_else(|| format!("unknown family {:?}", f.family))?;
        let [w, h] = f.size;
        let one = |k: &String| { let mut c = k.chars(); match (c.next(), c.next()) { (Some(ch), None) if !ch.is_ascii_digit() => Ok(ch), _ => Err(format!("palette key {k:?} must be one non-digit character")) } };
        let mut codes: BTreeMap<char, u8> = BTreeMap::new();
        codes.insert(HOST, HOST_CODE);
        let mut roles = vec![];
        for (i, r) in f.roles.iter().enumerate() {
            let mats = r.options.iter().map(|m| mat_named(m).ok_or_else(|| format!("role {}: unknown material {m}", r.name))).collect::<Result<Vec<_>, _>>()?;
            codes.insert(one(&r.char)?, ROLE0 + i as u8);
            roles.push((r.pick.clone(), mats));
        }
        for (k, v) in &f.palette {
            if v.starts_with('@') { if !f.roles.iter().any(|r| r.name == v[1..]) { return Err(format!("palette {k}: no role {v}")); } continue; }
            codes.insert(one(k)?, mat_named(v).ok_or_else(|| format!("palette {k}: unknown material {v}"))? as u8);
        }
        let mut lcodes: BTreeMap<char, u8> = BTreeMap::new();
        lcodes.insert(HOST, HOST_CODE);
        let mut mixes = vec![];
        for (k, v) in &f.linings {
            let ws = v.iter().map(|n| wall_named(n).ok_or_else(|| format!("lining {k}: unknown wall {n}"))).collect::<Result<Vec<_>, _>>()?;
            if ws.is_empty() { return Err(format!("lining {k} is empty")); }
            lcodes.insert(one(k)?, mixes.len() as u8); mixes.push(ws);
        }
        let code = |map: &BTreeMap<char, u8>, c: char| map.get(&c).copied().ok_or_else(|| format!("character {c:?} isn't in the palette"));
        let grid = |rows: &[String], map: &BTreeMap<char, u8>, what: &str| -> Result<Vec<u8>, String> {
            if rows.len() != h { return Err(format!("{what}: {} rows, the size says {h}", rows.len())); }
            let mut g = Vec::with_capacity(w * h);
            for (y, r) in rows.iter().enumerate() {
                let row = rle_decode(r)?;
                if row.len() != w { return Err(format!("{what} row {y}: {} cells, the size says {w}", row.len())); }
                for c in row { g.push(code(map, c)?); }
            }
            Ok(g)
        };
        let cells = grid(&f.cells, &codes, "cells")?;
        let lining = grid(&f.lining, &lcodes, "lining")?;
        let mut slots = vec![];
        for s in &f.slots {
            let n = s.options.len();
            let mut at = vec![u32::MAX; w * h]; let mut vals = vec![]; let mut lin = vec![];
            let mut chest = vec![];
            for (o, opt) in s.options.iter().enumerate() {
                for (layer, spans, map) in [(0, &opt.patch, &codes), (1, &opt.lining, &lcodes)] {
                    for (x, y, run) in spans {
                        for (k, c) in rle_decode(run)?.into_iter().enumerate() {
                            if *y >= h || x + k >= w { return Err(format!("slot {}: span outside the piece", s.name)); }
                            let i = y * w + x + k;
                            if at[i] == u32::MAX { at[i] = (vals.len() / n) as u32; vals.extend(std::iter::repeat(KEEP).take(n)); lin.extend(std::iter::repeat(KEEP).take(n)); }
                            let row = at[i] as usize * n + o;
                            let v = code(map, c)?;
                            if layer == 0 { vals[row] = v } else { lin[row] = v }
                        }
                    }
                }
                chest.push(opt.chest.map(|c| (c[0], c[1])));
            }
            slots.push(Slot { name: s.name.clone(), pick: s.pick.clone(), n, at, vals, lining: lin, chest });
        }
        let mut things = vec![];
        for t in &f.things {
            let when = match &t.when {
                None => None,
                Some(wh) => Some((slots.iter().position(|s| s.name == wh.slot).ok_or_else(|| format!("thing: no slot {}", wh.slot))?, wh.option)),
            };
            things.push((Thing::from_def(t)?, when));
        }
        let chest = f.chest.map(|c| (c[0], c[1]));
        let v = f.variation.clone();
        let named = |s: &str, what: &str| mat_named(s).ok_or_else(|| format!("variation {what}: unknown material {s}"));
        let remap_to = v.remap.iter().map(|r| Ok((named(&r.from, "remap")?, named(&r.to, "remap")?))).collect::<Result<Vec<_>, String>>()?;
        let mut zone_fill = vec![];
        for z in &v.zones {
            if !matches!(z.tag.as_str(), "chest" | "furniture" | "may-collapse") { return Err(format!("variation: unknown zone tag {:?}", z.tag)); }
            if !(0. ..=1.).contains(&z.chance) { return Err(format!("variation {}: chance {} is not 0..1", z.tag, z.chance)); }
            if z.tag == "may-collapse" { let Some([x, y, rw, rh]) = z.rect else { return Err("variation may-collapse: no rect".into()) };
                if x + rw > w || y + rh > h { return Err("variation may-collapse: rect outside the piece".into()); } }
            for set in &z.sets { for &i in set { if i >= things.len() { return Err(format!("variation furniture: no thing {i}")); } } }
            zone_fill.push(match &z.fill { Some(m) => Some(named(m, "fill")?), None => None });
        }
        Ok(Template { kind, w, h, file: f, cells, lining, mixes, roles, slots, chest, things, variation: v, zone_fill, remap_to })
    }
    fn choices(&self, var: u32, left: bool) -> impl Iterator<Item = usize> + '_ { self.slots.iter().map(move |s| s.pick.choose(var, left).min(s.n - 1)) }
    /// Material at box cell (dx, dy) for a site with this `var` and side; None leaves the host rock.
    pub fn cell(&self, dx: usize, dy: usize, var: u32, left: bool) -> Option<Mat> {
        let i = dy * self.w + dx;
        let mut c = self.cells[i];
        for (s, o) in self.slots.iter().zip(self.choices(var, left)) {
            let a = s.at[i]; if a != u32::MAX { let v = s.vals[a as usize * s.n + o]; if v != KEEP { c = v; } }
        }
        match c {
            HOST_CODE => None,
            c if c >= ROLE0 => { let (p, m) = &self.roles[(c - ROLE0) as usize]; Some(m[p.choose(var, left).min(m.len() - 1)]) }
            c => Some(Mat::from_u8(c)),
        }
    }
    /// Lining at box cell (dx, dy) for a cell hash `h` (only where the piece stamps a cell; the caller checks).
    pub fn lining_with(&self, dx: usize, dy: usize, var: u32, left: bool, h: u32) -> Option<Wall> {
        let i = dy * self.w + dx;
        let mut c = self.lining[i];
        for (s, o) in self.slots.iter().zip(self.choices(var, left)) {
            let a = s.at[i]; if a != u32::MAX { let v = s.lining[a as usize * s.n + o]; if v != KEEP { c = v; } }
        }
        if c == HOST_CODE { return None; }
        let m = &self.mixes[c as usize];
        Some(m[h as usize % m.len()])
    }
    /// The chest cell (from the box's top-left) for this variant.
    pub fn chest(&self, var: u32, left: bool) -> Option<(usize, usize)> {
        let mut c = self.chest;
        for (s, o) in self.slots.iter().zip(self.choices(var, left)) { if let Some(m) = s.chest[o] { c = Some(m); } }
        c
    }
    /// Things for this variant, box-relative, in order.
    pub fn things(&self, var: u32, left: bool) -> Vec<Thing> {
        let ch: Vec<usize> = self.choices(var, left).collect();
        self.things.iter().filter(|(_, w)| w.map_or(true, |(s, o)| ch[s] == o)).map(|(t, _)| *t).collect()
    }

    // ---- placed: the template with the site's seeded variation (see `Variation`)

    /// A roll in [0, 1) for this site, from `var`'s high bits and the roll's index `k`.
    fn roll(var: u32, k: u32) -> f32 { (crate::grid::hash3(var >> 8, k, 0xA55E) >> 8) as f32 / (1u32 << 24) as f32 }
    fn varies(var: u32) -> bool { var >= VARY_FROM }
    /// Is this site flipped east-west?
    pub fn mirrored(&self, var: u32) -> bool { self.variation.mirror && Self::varies(var) && Self::roll(var, 0) < 0.5 }
    /// Does zone `z` fire on this site?
    pub fn zone_on(&self, z: usize, var: u32) -> bool { Self::varies(var) && (Self::roll(var, 16 + z as u32) as f64) < self.variation.zones[z].chance }
    fn tx(&self, dx: usize, var: u32) -> usize { if self.mirrored(var) { self.w - 1 - dx } else { dx } }
    /// Material at box cell (dx, dy) of a placed site.
    pub fn placed_cell(&self, dx: usize, dy: usize, p: Placement) -> Option<Mat> {
        let tx = self.tx(dx, p.var);
        let mut m = self.cell(tx, dy, p.var, p.left)?;
        if Self::varies(p.var) {
            for (z, zone) in self.variation.zones.iter().enumerate() {
                if zone.tag != "may-collapse" || m != Mat::Empty { continue; }
                let Some([x, y, w, h]) = zone.rect else { continue };
                if tx >= x && tx < x + w && dy >= y && dy < y + h && self.zone_on(z, p.var) { m = self.zone_fill[z].unwrap_or(Mat::Gravel); }
            }
            for (r, to) in self.variation.remap.iter().zip(&self.remap_to) {
                if to.0 == m && r.deeper_than.map_or(true, |d| p.depth as f64 > d) && r.shallower_than.map_or(true, |d| (p.depth as f64) < d) { m = to.1; }
            }
        }
        Some(m)
    }
    /// Lining at box cell (dx, dy) of a placed site, for a cell hash `h`.
    pub fn placed_lining(&self, dx: usize, dy: usize, p: Placement, h: u32) -> Option<Wall> { self.lining_with(self.tx(dx, p.var), dy, p.var, p.left, h) }
    /// The chest of a placed site, if it has one.
    pub fn placed_chest(&self, p: Placement) -> Option<(usize, usize)> {
        let (x, y) = self.chest(p.var, p.left)?;
        if Self::varies(p.var) { for (z, zone) in self.variation.zones.iter().enumerate() { if zone.tag == "chest" && !self.zone_on(z, p.var) { return None; } } }
        Some((if self.mirrored(p.var) { self.w - 1 - x } else { x }, y))
    }
    /// The things of a placed site, box-relative, in order.
    pub fn placed_things(&self, p: Placement) -> Vec<Thing> {
        let ch: Vec<usize> = self.choices(p.var, p.left).collect();
        let mut dropped = vec![false; self.things.len()];
        if Self::varies(p.var) {
            for (z, zone) in self.variation.zones.iter().enumerate() {
                if zone.tag != "furniture" || zone.sets.is_empty() { continue; }
                let keep = (Self::roll(p.var, 16 + z as u32) * zone.sets.len() as f32) as usize % zone.sets.len();
                for (k, set) in zone.sets.iter().enumerate() { if k != keep { for &i in set { if i < dropped.len() { dropped[i] = true; } } } }
            }
        }
        let mirror = self.mirrored(p.var);
        self.things.iter().enumerate().filter(|(i, (_, w))| !dropped[*i] && w.map_or(true, |(s, o)| ch[s] == o))
            .map(|(_, (t, _))| if mirror { t.mirrored(self.w) } else { *t }).collect()
    }
}

impl Thing {
    /// The same thing in a box `w` cells wide flipped east-west.
    pub fn mirrored(self, w: usize) -> Thing {
        let m = |a: [usize; 2]| [w - 1 - a[0].min(w - 1), a[1]];
        match self {
            Thing::Torch { at } => Thing::Torch { at: m(at) },
            Thing::Furn { item, at } => Thing::Furn { item, at: m(at) },
            Thing::Farm { at } => Thing::Farm { at: m(at) },
            Thing::Ladder { at, anchor } => Thing::Ladder { at: m(at), anchor: m(anchor) },
            Thing::Cart { at } => Thing::Cart { at: [w as f32 - at[0], at[1]] },
            Thing::Device { dev, at } => Thing::Device { dev, at: m(at) },
            Thing::Wire { at } => Thing::Wire { at: m(at) },
            Thing::SwitchOn { at } => Thing::SwitchOn { at: m(at) },
        }
    }
}

impl crate::world::World {
    /// Stamp a template (any, not only the built-in one for its family) with its box's top-left at window
    /// (x0, y0), placed as `p` (its variation included): cells, linings (hashed with `id`, as a placed site's
    /// are), the chest and its things. Returns
    /// the chest's window cell. The cave editor's checks use this; the game stamps through `sites.rs`.
    pub fn stamp_template(&mut self, t: &Template, x0: usize, y0: usize, p: Placement, id: u32) -> Option<(usize, usize)> {
        if x0 + t.w >= self.g.w || y0 + t.h >= self.g.h { return None; }
        for dy in 0..t.h { for dx in 0..t.w {
            let (x, y) = (x0 + dx, y0 + dy);
            if let Some(m) = t.placed_cell(dx, dy, p) {
                let i = y * self.g.w + x; self.g.set(i, m); self.g.flags[i] = 0;
                let wy = self.oy + y as i64;
                if let Some(wl) = t.placed_lining(dx, dy, p, crate::grid::hash3(x as u32, wy as u32, id)) { self.walls.set_at(x, y, wl); }
            }
        } }
        let chest = t.placed_chest(p).map(|(cx, cy)| (x0 + cx, y0 + cy));
        if let Some((cx, cy)) = chest {
            if !self.chests.iter().any(|c| c.x == cx && c.y == cy) { self.chests.push(crate::world::Chest { x: cx, y: cy, opened: false, ancient: None }); }
        }
        for th in t.placed_things(p) { self.set_thing(th.offset(x0, y0)); }
        chest
    }
}

/// The site files compiled into the game.
const BUILTIN: [&str; 7] = [
    include_str!("../../../assets/sites/mine.json"),
    include_str!("../../../assets/sites/holt.json"),
    include_str!("../../../assets/sites/pumpworks.json"),
    include_str!("../../../assets/sites/fissure.json"),
    include_str!("../../../assets/sites/fossil.json"),
    include_str!("../../../assets/sites/crystal.json"),
    include_str!("../../../assets/sites/halt.json"),
];

/// The template for a family (parsed once, on first use).
pub fn get(kind: Kind) -> &'static Template {
    static ALL: OnceLock<Vec<Template>> = OnceLock::new();
    let all = ALL.get_or_init(|| {
        FAMILIES.iter().zip(BUILTIN).map(|(&(k, name), json)| {
            let t = Template::parse(json).unwrap_or_else(|e| panic!("assets/sites/{name}.json: {e}"));
            assert!(t.kind == k && (t.w, t.h) == k.size(), "assets/sites/{name}.json is a {:?} of {}x{}, expected {:?} {:?}", t.kind, t.w, t.h, k, k.size());
            t
        }).collect()
    });
    &all[FAMILIES.iter().position(|f| f.0 == kind).unwrap()]
}
