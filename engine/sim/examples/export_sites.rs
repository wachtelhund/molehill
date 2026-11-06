//! Export the seven found-place families from the procedural reference layouts in `sites.rs` into the
//! templates under `assets/sites/` (see `templates.rs`). Each layout knob a site's `var` selects becomes a
//! slot (sparse patches over the base) or a role (the mine's ore); a halt's side is a slot picked by
//! `left`. Before writing, every combination of options is composed from the data and compared with the
//! reference cell for cell, so a file is only written when it reproduces the layout exactly.
//!
//! `cargo run --release -p molehill-sim --example export_sites` (from `engine/`), or `-- --check` to only
//! compare the files on disk with a fresh export (exit 1 if they differ).
use std::collections::BTreeMap;

use molehill_sim::mat::{Mat, CPM};
use molehill_sim::sites::{Kind, Knobs, Site};
use molehill_sim::templates::{self, default_char, rle_encode, Connector, OptionDef, Pick, RoleDef, SiteFile, SlotDef, Template, When, FAMILIES, FORMAT, HOST};
use molehill_sim::walls::Wall;

/// The knob a slot sets, and how a site's `var` (or side) picks it: the same arithmetic as `Knobs::of`.
struct SlotSpec { name: &'static str, pick: Pick, n: usize, set: fn(&mut Knobs, &mut bool, usize), label: fn(usize) -> String }
fn var_pick(shift: u32, modulo: u32) -> Pick { Pick { from: "var".into(), shift, modulo, table: vec![] } }

fn slots_of(kind: Kind) -> Vec<SlotSpec> {
    match kind {
        Kind::Mine => vec![
            SlotSpec { name: "slump", pick: var_pick(0, 6), n: 6, set: |k, _, o| k.slump = o as i64, label: |o| format!("bay {} m slumped", 20 + o) },
            SlotSpec { name: "rails", pick: var_pick(2, 4), n: 4, set: |k, _, o| k.rails = o as i64, label: |o| format!("rusted run {o}") },
            SlotSpec { name: "band", pick: var_pick(0, 8), n: 8, set: |k, _, o| k.band = o as i64, label: |o| format!("face bands +{o}") },
        ],
        Kind::Crystal => vec![SlotSpec { name: "hang", pick: var_pick(0, 4), n: 4, set: |k, _, o| k.hang = o as i64, label: |o| format!("clusters +{o}") }],
        Kind::Halt => vec![SlotSpec { name: "side", pick: Pick { from: "left".into(), shift: 0, modulo: 0, table: vec![] }, n: 2, set: |_, l, o| *l = o == 1, label: |o| if o == 1 { "tube west".into() } else { "tube east".into() } }],
        _ => vec![],
    }
}
/// The mine's ore is a role: `var % 4 == 3` follows coal, else copper.
const ORE_CHAR: char = '*';
fn roles_of(kind: Kind) -> Vec<RoleDef> {
    if kind != Kind::Mine { return vec![]; }
    vec![RoleDef { name: "ore".into(), char: ORE_CHAR.to_string(), pick: Pick { from: "var".into(), shift: 0, modulo: 4, table: vec![0, 0, 0, 1] }, options: vec!["CopperOre".into(), "Coal".into()] }]
}
const ORES: [Mat; 2] = [Mat::CopperOre, Mat::Coal];

fn site(kind: Kind, left: bool) -> Site {
    let (w, h) = kind.size();
    Site { id: 0, kind, x0: 0, y0: 0, w, h, var: 0, left, chest: None, link: None }
}
/// The reference grid for knobs `k` as palette characters (the ore role where the two ores differ).
fn encoded(kind: Kind, k: &Knobs, left: bool) -> Vec<char> {
    let s = site(kind, left); let (w, h) = (s.w, s.h);
    let mut g = Vec::with_capacity(w * h);
    for dy in 0..h { for dx in 0..w {
        let at = |ore: Mat| s.cell_reference(dx, dy as i64, &Knobs { ore, ..*k });
        let (a, b) = (at(ORES[0]), at(ORES[1]));
        g.push(if a == b { a.map_or(HOST, default_char) } else {
            assert!(a == Some(ORES[0]) && b == Some(ORES[1]), "{kind:?} ({dx},{dy}): the ore knob changes {a:?} into {b:?}");
            ORE_CHAR
        });
    } }
    g
}
fn spans(w: usize, base: &[char], g: &[char]) -> Vec<(usize, usize, String)> {
    let mut v = vec![]; let mut i = 0;
    while i < g.len() {
        if g[i] == base[i] { i += 1; continue; }
        let (y, x0) = (i / w, i % w); let mut j = i;
        while j < g.len() && j / w == y && g[j] != base[j] { j += 1; }
        v.push((x0, y, rle_encode(g[i..j].iter().copied())));
        i = j;
    }
    v
}
/// Box-edge openings in a grid: `entrance` where open floor reaches the side, `tube` where a pressure tube does.
fn connectors(w: usize, h: usize, g: &[char], when: Option<When>) -> Vec<Connector> {
    let mut v = vec![];
    let open = |c: char| c == default_char(Mat::Empty) || c == default_char(Mat::Water);
    let tube = |c: char| c == default_char(Mat::Tube);
    let sides: [(&str, usize, Box<dyn Fn(usize) -> usize>); 4] = [
        ("west", h, Box::new(|t| t * w)), ("east", h, Box::new(|t| t * w + w - 1)),
        ("north", w, Box::new(|t| t)), ("south", w, Box::new(|t| (h - 1) * w + t)),
    ];
    for (side, n, idx) in sides.iter() {
        for (tag, f) in [("entrance", &open as &dyn Fn(char) -> bool), ("tube", &tube)] {
            let mut t = 0;
            while t < *n {
                if !f(g[idx(t)]) { t += 1; continue; }
                let s = t; while t < *n && f(g[idx(t)]) { t += 1; }
                let tag = if tag == "entrance" && (*side == "north" || *side == "south") { "shaft" } else { tag };
                if t - s >= CPM { v.push(Connector { tag: tag.into(), side: side.to_string(), at: s, len: t - s, when: when.clone() }); }
            }
        }
    }
    v
}

fn export(kind: Kind) -> SiteFile {
    let (w, h) = kind.size();
    let specs = slots_of(kind);
    let base_k = Knobs { slump: 0, rails: 0, band: 0, hang: 0, ore: ORES[0] };
    let base = encoded(kind, &base_k, false);
    // every cell any variant stamps needs a lining mix
    let mut used = base.iter().map(|&c| c != HOST).collect::<Vec<_>>();
    let mut slots = vec![];
    for sp in &specs {
        let mut options = vec![];
        for o in 0..sp.n {
            let (mut k, mut left) = (base_k, false);
            (sp.set)(&mut k, &mut left, o);
            let g = encoded(kind, &k, left);
            for (u, &c) in used.iter_mut().zip(&g) { *u |= c != HOST; }
            let chest = if left { Site::chest_reference(kind, true).map(|c| [c.0, c.1]) } else { None };
            options.push(OptionDef { name: (sp.label)(o), patch: spans(w, &base, &g), lining: vec![], chest });
        }
        slots.push(SlotDef { name: sp.name.into(), pick: sp.pick.clone(), options });
    }
    // linings: a cell's walls depend on its position and the low 4 bits of its hash; store the shortest period
    let s0 = site(kind, false);
    let mut mixes: Vec<Vec<Wall>> = vec![]; let mut lining = vec![HOST; w * h];
    let keys: Vec<char> = ('a'..='z').chain('A'..='Z').collect();
    for i in 0..w * h {
        if !used[i] { continue; }
        let (dx, dy) = ((i % w) as i64, (i / w) as i64);
        let t: Vec<Wall> = (0..16u32).map(|hh| s0.lining_reference(dx, dy, hh)).collect();
        for probe in [0x1234_5670u32, 0xDEAD_BEE0, 0xFFFF_FFF0, 0x8000_0000] {
            for hh in 0..16 { assert_eq!(s0.lining_reference(dx, dy, probe | hh), t[hh as usize], "{kind:?} lining reads more than 4 hash bits"); }
        }
        let mut p = 16; while p > 1 && (0..16).all(|j| t[j] == t[j % (p / 2)]) { p /= 2; }
        let mix = t[..p].to_vec();
        let id = mixes.iter().position(|m| *m == mix).unwrap_or_else(|| { mixes.push(mix); mixes.len() - 1 });
        lining[i] = keys[id];
    }
    let mut palette = BTreeMap::new();
    for &c in base.iter().chain(slots.iter().flat_map(|s: &SlotDef| s.options.iter()).flat_map(|o| o.patch.iter()).flat_map(|p| p.2.chars()).collect::<Vec<_>>().iter()) {
        if c == HOST || c == ORE_CHAR || c.is_ascii_digit() { continue; }
        let m = (0..molehill_sim::mat::NMAT as u8).map(Mat::from_u8).find(|&m| default_char(m) == c).unwrap();
        palette.insert(c.to_string(), format!("{m:?}"));
    }
    let roles = roles_of(kind);
    for r in &roles { palette.insert(r.char.clone(), format!("@{}", r.name)); }
    let linings = mixes.iter().enumerate().map(|(i, m)| (keys[i].to_string(), m.iter().map(|w| format!("{w:?}")).collect())).collect();
    // things: index-aligned across the halt's sides; entries that differ are kept per side
    let side_slot = specs.iter().position(|s| s.name == "side");
    let lists: Vec<_> = (0..if side_slot.is_some() { 2 } else { 1 }).map(|l| Site::things_reference(&site(kind, l == 1), 0, 0)).collect();
    let mut things = vec![];
    if lists.len() == 1 { things = lists[0].iter().map(|t| t.to_def(None)).collect(); } else {
        assert_eq!(lists[0].len(), lists[1].len(), "{kind:?}: things differ in count by side");
        for (a, b) in lists[0].iter().zip(&lists[1]) {
            if a == b { things.push(a.to_def(None)); } else {
                things.push(a.to_def(Some(When { slot: "side".into(), option: 0 })));
                things.push(b.to_def(Some(When { slot: "side".into(), option: 1 })));
            }
        }
    }
    let mut conns = connectors(w, h, &base, side_slot.map(|_| When { slot: "side".into(), option: 0 }));
    if side_slot.is_some() { conns.extend(connectors(w, h, &encoded(kind, &base_k, true), Some(When { slot: "side".into(), option: 1 }))); }
    let rows = |g: &[char]| g.chunks(w).map(|r| rle_encode(r.iter().copied())).collect::<Vec<_>>();
    SiteFile {
        format: FORMAT, source: "export".into(), family: templates::file_name(kind).into(), cpm: CPM, size: [w, h], palette,
        cells: rows(&base), linings, lining: rows(&lining), roles, slots,
        chest: Site::chest_reference(kind, false).map(|c| [c.0, c.1]), things, connectors: conns,
    }
}

/// Check the data against the reference: slots patch disjoint cells (so any mix of options is well
/// defined), and every variant a site can be today (var 0..96 covers each knob's period; both sides)
/// comes out the same cell for cell, with the same linings, chest and things.
fn verify(kind: Kind, f: &SiteFile) -> Result<(), String> {
    let t = Template::from_file(f.clone())?;
    let (w, h) = kind.size();
    let mut owner = vec![usize::MAX; w * h];
    for (si, s) in f.slots.iter().enumerate() {
        for o in &s.options { for (x, y, run) in &o.patch {
            for k in 0..templates::rle_decode(run)?.len() {
                let i = y * w + x + k;
                if owner[i] != usize::MAX && owner[i] != si { return Err(format!("slots {} and {} both patch ({},{y})", f.slots[owner[i]].name, s.name, x + k)); }
                owner[i] = si;
            }
        } }
    }
    let lefts: &[bool] = if kind == Kind::Halt { &[false, true] } else { &[false] };
    for &left in lefts { for var in (0..96u32).chain([1 << 19, 123_457, (1 << 20) - 1]) {
        let k = Knobs::of(var);
        let s = site(kind, left);
        for dy in 0..h { for dx in 0..w {
            let (a, b) = (t.cell(dx, dy, var, left), s.cell_reference(dx, dy as i64, &k));
            if a != b { return Err(format!("var {var} left {left}: ({dx},{dy}) template {a:?}, reference {b:?}")); }
            if b.is_some() { for hh in [0u32, 5, 11, 0xABCD_EF07] {
                if t.lining_with(dx, dy, var, left, hh) != Some(s.lining_reference(dx as i64, dy as i64, hh)) { return Err(format!("lining at ({dx},{dy})")); }
            } }
        } }
        if t.chest(var, left) != Site::chest_reference(kind, left) { return Err("chest".into()); }
        if t.things(var, left) != Site::things_reference(&s, 0, 0) { return Err(format!("things (left {left})")); }
    } }
    Ok(())
}

fn main() {
    let check = std::env::args().any(|a| a == "--check");
    let force = std::env::args().any(|a| a == "--force");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sites");
    std::fs::create_dir_all(&dir).unwrap();
    let mut stale = 0;
    for &(kind, name) in &FAMILIES {
        let f = export(kind);
        if let Err(e) = verify(kind, &f) { eprintln!("FAIL {name}: the export doesn't reproduce the reference: {e}"); std::process::exit(1); }
        let json = templates::to_json(&f);
        let path = dir.join(format!("{name}.json"));
        let owned = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<SiteFile>(&t).ok()).is_some_and(|f| f.from_editor());
        if owned && !force {
            println!("KEPT {name} (painted in the cave editor; `-- --force` overwrites it with the sites.rs layout)");
            continue;
        }
        if check {
            let same = std::fs::read_to_string(&path).map_or(false, |old| old == json);
            println!("{} {name}", if same { "SAME" } else { "STALE" });
            if !same { stale += 1; }
        } else {
            std::fs::write(&path, &json).unwrap();
            let slots: Vec<String> = f.slots.iter().map(|s| format!("{}×{}", s.name, s.options.len())).collect();
            println!("wrote {} ({} KB, {}×{} cells, {} linings, slots [{}], {} things, {} connectors)", path.display(), json.len() / 1024, f.size[0], f.size[1], f.linings.len(), slots.join(", "), f.things.len(), f.connectors.len());
        }
    }
    if stale > 0 { std::process::exit(1); }
}
