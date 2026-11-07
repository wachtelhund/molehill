//! Save / load (port of the v2/saveload design, binary instead of JSON).
//! Envelope: magic "MOLEHILL", format version, engine id, dimensions, seed; then length-prefixed
//! sections. Cell layers are byte-RLE. Materials are stored by *name* (a name table travels with
//! the save) so the material enum can grow/reorder. Loading regenerates the world from the seed
//! (biome/surface info, background) and then overwrites all mutable state. Loading never panics:
//! every read is bounds-checked and returns a readable error; values are sanitised.
use crate::craft::{Item, Station, NITEM, RECIPES};
use crate::mat::{Mat, NMAT, PROPS};
use crate::world::{Building, Chest, Drop, Farm, Mole, World};
use std::collections::VecDeque;

/// v5: v4 plus the mode (Creative / Survival) and the items ever picked up (recipes unlock by touch).
/// v4: the world as bands (see `band.rs`): a manifest names every band (window and off-window) by start
/// row and content hash; blobs travel inline (`save`) or as separate keys (`save_parts`). v3: one blob
/// with a row cache of cells (still loads). Older (surface-world) saves are refused.
pub const VERSION: u32 = 12; // 12: the window slides sideways too: its column (`ox`), and the world stored as tiles keyed by (column, row); // 11: grottos visited; // 10: background walls (band trailing RLE) and sites visited; // 9: transport (carts and their cargo; see transport.rs); // 8: Ancient relics found and ruins visited; // 7: the Lid journal and the clues found; 6: the items ever picked up (recipes unlock by touch); 5: the game mode, the headlamp's oil, and hunger on/off
const MAGIC: &[u8; 8] = b"MOLEHILL";
/// The engine tag also names the cell resolution: a save at another cells-per-metre won't load.
const ENGINE: &str = if crate::mat::CPM == 4 { "bevy-v3" } else if crate::mat::CPM == 8 { "bevy-v3-cpm8" } else { "bevy-v3-cpm?" };

use crate::band::{self, Wr, Rd, R, BH};
use std::borrow::Cow;

/// v3 (one blob, row cache of cells only). Read so today's saves still load; never written.
fn load_v3(b: &[u8]) -> R<World> {
    let mut r = Rd { b, p: 0 };
    if r.take(8).map_err(|_| "not a Molehill save".to_string())? != MAGIC { return Err("not a Molehill save".into()); }
    let ver = r.u32()?; if ver > VERSION { return Err(format!("save version {ver} is newer than this build ({VERSION})")); }
    if ver < 3 { return Err("old save from the surface world - start a new holt (N)".into()); }
    let eng = r.str()?;
    if eng != ENGINE && eng.starts_with("bevy-v3") { return Err("save is from an older, coarser Deepholt - start a new holt (N)".into()); }
    if eng != ENGINE { return Err(format!("save is from engine '{eng}', expected '{ENGINE}'")); }
    let (gw, gh) = (r.len(1 << 13)?, r.len(1 << 13)?); if gw % 64 != 0 || gh % 64 != 0 || gw == 0 || gh == 0 || gw * gh > 1 << 23 { return Err("bad world dimensions".into()); }
    let seed = r.u32()?; let tick = r.u64()?; let gtick = r.u64()?;
    let nm = r.len(256)?; let mut remap = vec![0u8; nm];
    for k in 0..nm { let name = r.str()?; remap[k] = PROPS.iter().position(|p| p.name == name).ok_or(format!("unknown material '{name}'"))? as u8; }
    let n = gw * gh;
    let mut mat = r.rle(n)?; for m in mat.iter_mut() { if *m as usize >= nm { return Err("material id out of range".into()); } *m = remap[*m as usize]; }
    let flags = r.rle(n)?; let desig = r.rle(n)?;
    let mut w = World::blank(gw, gh, seed);
    let nb = w.logic.wire.len();
    let wire = r.rle(nb)?; let dev = r.rle(nb)?; let dev_s = r.rle(nb)?; let dp = r.rle(nb * 2)?;
    w.g.mat = mat; w.g.flags = flags; w.desig = desig.into_iter().map(|d| d.min(4)).collect(); w.tick = tick; w.g.tick = gtick;
    w.logic.wire = wire.into_iter().map(|v| v.min(1)).collect(); w.logic.dev = dev.into_iter().map(|v| if v as usize <= crate::logic::DEVS.len() { v } else { 0 }).collect(); w.logic.dev_s = dev_s;
    w.logic.dev_p = dp.chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect(); w.logic.invalidate();
    let ni = r.len(4096)?; let mut inv = [0u32; NITEM]; for k in 0..ni { let v = r.u32()?; if k < NITEM { inv[k] = v; } } w.inv = inv;
    let tmpl = w.moles[0].clone(); w.moles.clear();
    for _ in 0..r.len(1000)? {
        let name = r.str()?; let name: &'static str = Box::leak(name.into_boxed_str());
        let (x, y) = (r.u32()? as usize, r.u32()? as usize); let (hp, breath, hunger, sleep) = (r.f32()?, r.f32()?, r.f32()?, r.f32()?); let alive = r.u8()? != 0; let facing = r.u8()? as i8 - 1;
        if x < 2 || y < 4 || x >= gw - 2 || y >= gh - 2 { continue; }
        w.moles.push(Mole { name, x, y, hp: hp.clamp(0., 100.), breath: breath.clamp(0., 100.), hunger: hunger.clamp(0., 100.), sleep: sleep.clamp(0., 100.), alive, facing: facing.signum().max(-1), path: VecDeque::new(), job: None, work: 0, work_cell: None, carry: None, ..tmpl.clone() });
    }
    w.moles.truncate(1); if w.moles.is_empty() { w.moles.push(tmpl.clone()); } // one mole: the player
    w.drops.clear(); for _ in 0..r.len(1 << 20)? { let (x, y, it, nn) = (r.u32()? as usize, r.u32()? as usize, r.u8()?, r.u32()?); if x < gw && y < gh && (it as usize) < NITEM { w.drops.push(Drop::at(x as f32 + 0.5, y as f32 + 0.5, crate::craft::ITEMS[it as usize], nn)); } }
    w.chests.clear(); for _ in 0..r.len(1 << 16)? { let (x, y, o) = (r.u32()? as usize, r.u32()? as usize, r.u8()? != 0); if x < gw && y < gh { w.chests.push(Chest { x, y, opened: o, ancient: None }); } }
    w.buildings.clear(); for _ in 0..r.len(1 << 16)? { let (k, x, y, b) = (r.u8()?, r.u32()? as usize, r.u32()? as usize, r.u8()? != 0); if let Some(kind) = band::station_from_code(k) { if x + Station::W < gw && y < gh { w.buildings.push(Building { kind, x, y, built: b, claimed: false }); } } }
    w.torches.clear(); for _ in 0..r.len(1 << 20)? { let (x, y) = (r.u32()? as usize, r.u32()? as usize); if x < gw && y < gh { w.torches.push((x, y)); } }
    w.quest = (r.u32()? as usize).min(crate::craft::QUESTS.len()); w.chests_opened = r.u32()?; w.pillars = r.u32()?; w.max_depth = r.f32()?; w.crumbled = r.u64()?;
    let sp: Vec<usize> = (0..4).map(|_| r.u32().map(|v| v as usize)).collect::<R<_>>()?; w.stockpile = (sp[0].min(gw - 1), sp[1].min(gh - 1), sp[2].min(gw - 1), sp[3].min(gh - 1));
    w.craft_queue = (0..r.len(64)?).map(|_| r.u32().map(|v| v as usize)).collect::<R<Vec<_>>>()?.into_iter().filter(|&k| k < RECIPES.len()).collect();
    for k in 0..nm { let v = r.u8()? != 0; w.discovered[remap[k] as usize] |= v; } // by name: saves from before a material was added still load
    w.disasters.rng = r.u64()? | 1; w.disasters.ticks_left = r.u32()?; w.disasters.enabled = r.u8()? != 0;
    w.log.clear(); for _ in 0..r.len(1000)? { let t = r.u64()?; let s = r.str()?; w.log.push((t, s)); }
    w.oy = r.u64()? as i64;
    let mut rows: std::collections::HashMap<i64, (Vec<u8>, Vec<u8>)> = Default::default();
    for _ in 0..r.len(1 << 20)? { let wy = r.u64()? as i64; let mut m = r.rle(gw)?; for v in m.iter_mut() { *v = *remap.get(*v as usize).unwrap_or(&0); } let f = r.rle(gw)?; rows.insert(wy, (m, f)); }
    // the old row cache becomes bands (cells only: v3 lost devices and things off-window anyway)
    w.bands.clear(); let starts: std::collections::BTreeSet<i64> = rows.keys().map(|&wy| band::band_start(w.oy, wy)).collect(); let tw = band::tile_w(gw);
    for s0 in starts { if (0..BH as i64).all(|k| rows.contains_key(&(s0 + k))) {
        let (mut m, mut f) = (Vec::with_capacity(gw * BH), Vec::with_capacity(gw * BH)); for k in 0..BH as i64 { let (a, b) = &rows[&(s0 + k)]; m.extend_from_slice(a); f.extend_from_slice(b); }
        let t = band::Tile::of_cells(m, f, gw); for k in 0..gw / tw { w.bands.insert(((k * tw) as i64, s0), band::Stored::new(t.cut(k * tw, tw).write())); } } }
    w.opened.clear(); for _ in 0..r.len(1 << 20)? { let x = r.u32()? as i64; let wy = r.u64()? as i64; w.opened.insert((x, wy)); }
    w.farms.clear(); for _ in 0..r.len(1 << 16)? { let (x, y, g) = (r.u32()? as usize, r.u32()? as usize, r.f32()?); if x < gw && y + 1 < gh { w.farms.push(Farm { x, y, growth: g.clamp(0., 1.), why: 0 }); } }
    let ab = r.rle(w.air.co2.len())?; w.air.from_bytes(&ab); w.harvested = r.u32()?;
    if r.p < r.b.len() {
        let bn = r.rle(nb * 2)?; w.logic.burn = bn.chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]]).clamp(0, 600)).collect();
        for b in 0..nb { if w.logic.dev[b] == 0 { w.logic.burn[b] = 0; } }
        w.pl.deaths = r.u32()?;
    }
    let _ = (&w.chests, Chest { x: 0, y: 0, opened: false, ancient: None }); w.refresh_chests(); w.infer_back();
    let _ = Item::Dirt; let _ = Mat::Empty;
    // derived state
    w.g.life.iter_mut().for_each(|v| *v = 0); w.g.wake_all(); w.st = crate::stress::Stress::new(n);
    let f = w.st.solve(&w.g); w.st.crack(&w.g, &f); w.g.clear_struct();
    w.say(format!("Loaded save (tick {}).", tick));
    Ok(w)
}

/// The storage key of a band blob before v12: its start row and content hash (a changed band gets a new key).
pub fn band_key(wy0: i64, hash: u64) -> String { format!("b{wy0}_{hash:016x}") }
/// The storage key of a tile blob (v12): its first column and row and its content hash.
pub fn tile_key(x0: i64, wy0: i64, hash: u64) -> String { format!("t{x0}_{wy0}_{hash:016x}") }

/// A save split into parts: a small manifest (globals + the list of bands) and the band blobs, each
/// under `band_key`. Write the blobs first and the manifest last; then a torn save still loads the
/// previous manifest, whose blobs are untouched (a blob never changes under its key).
pub struct Parts<'a> { pub manifest: Vec<u8>, pub bands: Vec<(String, Cow<'a, [u8]>)> }

pub fn save_parts(w: &World) -> Parts<'_> {
    let mut bands: Vec<((i64, i64), u64, Cow<[u8]>)> = vec![]; let tw = band::tile_w(w.g.w);
    for y0 in (0..w.g.h).step_by(BH) { for x0 in (0..w.g.w).step_by(tw) { let blob = band::encode(w, x0, y0); bands.push(((w.ox + x0 as i64, w.oy + y0 as i64), band::hash(&blob), Cow::Owned(blob))); } }
    for (&k, b) in &w.bands { bands.push((k, b.hash, Cow::Borrowed(&b.blob[..]))); }
    let mut o = Wr(vec![]); o.0.extend_from_slice(MAGIC); o.u32(VERSION); o.str(ENGINE);
    o.u32(w.g.w as u32); o.u32(w.g.h as u32); o.u32(w.info.seed); o.u64(w.tick); o.u64(w.g.tick);
    o.u32(NMAT as u32); for p in PROPS.iter() { o.str(p.name); }
    o.u32(NITEM as u32); for v in w.inv { o.u32(v); }
    o.u32(w.moles.len() as u32);
    for m in &w.moles { o.str(m.name); o.u32(m.x as u32); o.u32(m.y as u32); o.f32(m.hp); o.f32(m.breath); o.f32(m.hunger); o.f32(m.sleep); o.u8(m.alive as u8); o.u8((m.facing + 1) as u8); }
    o.u32(w.quest as u32); o.u32(w.chests_opened); o.u32(w.pillars); o.f32(w.max_depth); o.u64(w.crumbled);
    let sp = w.stockpile; for v in [sp.0, sp.1, sp.2, sp.3] { o.u32(v as u32); }
    o.u32(w.craft_queue.len() as u32); for &r in &w.craft_queue { o.u32(r as u32); }
    for d in w.discovered { o.u8(d as u8); }
    o.u64(w.disasters.rng); o.u32(w.disasters.ticks_left.max(60)); o.u8(w.disasters.enabled as u8);
    o.u32(w.log.len() as u32); for (t, s) in &w.log { o.u64(*t); o.str(s); }
    o.u64(w.oy as u64); o.u64(w.ox as u64);
    let mut opened: Vec<(i64, i64)> = w.opened.iter().copied().collect();
    for c in &w.chests { if c.opened { opened.push((w.ox + c.x as i64, w.oy + c.y as i64)); } } opened.sort(); opened.dedup();
    o.u32(opened.len() as u32); for &(x, wy) in &opened { o.u64(x as u64); o.u64(wy as u64); }
    o.u32(w.harvested); o.u32(w.pl.deaths); o.u64(w.pl.spawn.0 as u64); o.u64(w.pl.spawn.1 as u64);
    o.u8(w.mode as u8); o.f32(w.lamp_oil); o.u8(w.rules.no_hunger as u8);
    o.u32(NITEM as u32); for k in w.known { o.u8(k as u8); }
    o.u32(w.journal.len() as u32); for &f in &w.journal { o.u32(f as u32); }
    let mut fc: Vec<(i64, i64)> = w.found_clues.iter().copied().collect(); fc.sort();
    o.u32(fc.len() as u32); for &(x, wy) in &fc { o.u64(x as u64); o.u64(wy as u64); }
    let mut rs: Vec<u32> = w.ruins_seen.iter().copied().collect(); rs.sort(); o.u8(w.relics); o.u32(rs.len() as u32); for id in rs { o.u32(id); }
    o.bytes(&w.encode_transport());
    let mut ss: Vec<u32> = w.sites_seen.iter().copied().collect(); ss.sort(); o.u32(ss.len() as u32); for id in ss { o.u32(id); }
    let mut gs: Vec<u32> = w.grottos_seen.iter().copied().collect(); gs.sort(); o.u32(gs.len() as u32); for id in gs { o.u32(id); }
    o.u32(bands.len() as u32); for ((x0, wy0), h, b) in &bands { o.u64(*x0 as u64); o.u64(*wy0 as u64); o.u64(*h); o.u32(b.len() as u32); }
    Parts { manifest: o.0, bands: bands.into_iter().map(|((x0, wy0), h, b)| (tile_key(x0, wy0, h), b)).collect() }
}

/// The whole save in one blob (manual slots, native files, tests): the manifest, then every band inline.
pub fn save(w: &World) -> Vec<u8> {
    let p = save_parts(w); let mut o = p.manifest; o.push(1); for (_, b) in &p.bands { o.extend_from_slice(b); } o
}

/// Load a one-blob save (v4 with inline bands, or v3).
pub fn load(b: &[u8]) -> R<World> { load_with(b, &mut |_| None) }

/// Load from a manifest; bands not inline are fetched by key (`band_key`). A missing or damaged band
/// in the window fails the load; one off-window is dropped (the generator makes it again).
pub fn load_with(b: &[u8], fetch: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> R<World> {
    let mut r = Rd { b, p: 0 };
    if r.take(8).map_err(|_| "not a Molehill save".to_string())? != MAGIC { return Err("not a Molehill save".into()); }
    let ver = r.u32()?; if ver > VERSION { return Err(format!("save version {ver} is newer than this build ({VERSION})")); }
    if ver < 4 { return load_v3(b); }
    let eng = r.str()?;
    if eng != ENGINE && eng.starts_with("bevy-v3") { return Err("save is from an older, coarser Deepholt - start a new holt (N)".into()); }
    if eng != ENGINE { return Err(format!("save is from engine '{eng}', expected '{ENGINE}'")); }
    let (gw, gh) = (r.len(1 << 13)?, r.len(1 << 13)?); if gw % 64 != 0 || gh % BH != 0 || gw == 0 || gh == 0 || gw * gh > 1 << 23 { return Err("bad world dimensions".into()); }
    let seed = r.u32()?; let tick = r.u64()?; let gtick = r.u64()?;
    let nm = r.len(256)?; let mut remap = vec![0u8; nm];
    for k in 0..nm { let name = r.str()?; remap[k] = PROPS.iter().position(|p| p.name == name).ok_or(format!("unknown material '{name}'"))? as u8; }
    let same_mats = nm == NMAT && remap.iter().enumerate().all(|(k, &v)| k == v as usize);
    let ni = r.len(4096)?; let mut inv = [0u32; NITEM]; for k in 0..ni { let v = r.u32()?; if k < NITEM { inv[k] = v; } }
    struct M { name: String, x: usize, y: usize, hp: f32, breath: f32, hunger: f32, sleep: f32, alive: bool, facing: i8 }
    let mut moles = vec![]; for _ in 0..r.len(1000)? { moles.push(M { name: r.str()?, x: r.u32()? as usize, y: r.u32()? as usize, hp: r.f32()?, breath: r.f32()?, hunger: r.f32()?, sleep: r.f32()?, alive: r.u8()? != 0, facing: r.u8()? as i8 - 1 }); }
    let (quest, chests_opened, pillars, max_depth, crumbled) = (r.u32()? as usize, r.u32()?, r.u32()?, r.f32()?, r.u64()?);
    let sp: Vec<usize> = (0..4).map(|_| r.u32().map(|v| v as usize)).collect::<R<_>>()?;
    let craft_queue: Vec<usize> = (0..r.len(64)?).map(|_| r.u32().map(|v| v as usize)).collect::<R<_>>()?;
    let mut discovered = [false; NMAT]; for k in 0..nm { let v = r.u8()? != 0; discovered[remap[k] as usize] |= v; }
    let (drng, dleft, den) = (r.u64()? | 1, r.u32()?, r.u8()? != 0);
    let mut log = vec![]; for _ in 0..r.len(1000)? { let t = r.u64()?; let s = r.str()?; log.push((t, s)); }
    let oy = r.u64()? as i64; let ox = if ver >= 12 { r.u64()? as i64 } else { 0 }; // (before v12 the window never left the home columns)
    let xcol = |r: &mut Rd| -> R<i64> { if ver >= 12 { Ok(r.u64()? as i64) } else { Ok(r.u32()? as i64) } };
    let mut opened = std::collections::HashSet::new(); for _ in 0..r.len(1 << 20)? { let x = xcol(&mut r)?; let wy = r.u64()? as i64; opened.insert((x, wy)); }
    let (harvested, deaths) = (r.u32()?, r.u32()?); let (spx, spy) = (xcol(&mut r)?, r.u64()? as i64);
    let (mode, lamp_oil, no_hunger) = if ver >= 5 { (crate::survival::Mode::from_u8(r.u8()?), r.f32()?, r.u8()? != 0) } else { (crate::survival::Mode::Plain, 0., false) };
    // v6: items ever picked up; before that, what is in the pack counts as known
    let known = if ver >= 6 { let n = r.len(4096)?; let mut k = [false; NITEM]; for j in 0..n { let v = r.u8()? != 0; if j < NITEM { k[j] = v; } } Some(k) } else { None };
    // v7: the journal (fragment ids, in reading order) and the clues found
    let (journal, found_clues) = if ver >= 7 {
        let j: Vec<u16> = (0..r.len(4096)?).map(|_| r.u32().map(|v| v as u16)).collect::<R<_>>()?;
        let mut f = std::collections::HashSet::new(); for _ in 0..r.len(1 << 20)? { let x = xcol(&mut r)?; let wy = r.u64()? as i64; f.insert((x, wy)); }
        (j.into_iter().filter(|&k| (k as usize) < crate::lore::FRAGMENTS.len()).collect(), f)
    } else { (vec![], Default::default()) };
    // v8: the relics found and the ruins visited
    let (relics, ruins_seen) = if ver >= 8 { let rl = r.u8()?; let mut s = std::collections::HashSet::new(); for _ in 0..r.len(1 << 20)? { s.insert(r.u32()?); } (rl, s) } else { (0, Default::default()) };
    // v9: transport
    let transport = if ver >= 9 { Some(r.bytes()?.to_vec()) } else { None };
    // v10: sites visited (walls travel with the bands)
    let sites_seen = if ver >= 10 { let mut s = std::collections::HashSet::new(); for _ in 0..r.len(1 << 20)? { s.insert(r.u32()?); } s } else { Default::default() };
    // v11: grottos visited
    let grottos_seen = if ver >= 11 { let mut s = std::collections::HashSet::new(); for _ in 0..r.len(1 << 20)? { s.insert(r.u32()?); } Some(s) } else { None };
    // the blob list: tiles (column, row) since v12, full-width bands (row) before
    let list: Vec<(Option<i64>, i64, u64, usize)> = (0..r.len(1 << 20)?).map(|_| Ok(if ver >= 12 { (Some(r.u64()? as i64), r.u64()? as i64, r.u64()?, r.len(1 << 28)?) } else { (None, r.u64()? as i64, r.u64()?, r.len(1 << 28)?) })).collect::<R<_>>()?;
    let inline = !r.done() && r.u8()? == 1;
    // the world: generated from the seed, then everything mutable comes from the bands
    let mut w = World::blank(gw, gh, seed);
    w.ox = ox; w.oy = oy; w.drops.clear(); w.torches.clear(); w.buildings.clear(); w.farms.clear(); w.bands.clear();
    w.logic = crate::logic::Logic::new(gw, gh); w.air = crate::air::Air::new(gw, gh);
    let tw = band::tile_w(gw); if ox.rem_euclid(tw as i64) != 0 && tw != gw { return Err("bad window column".into()); }
    let mut window = vec![false; (gw / tw) * (gh / BH)]; let mut lost = 0;
    for &(x0, wy0, h, n) in &list {
        let key = match x0 { Some(x0) => tile_key(x0, wy0, h), None => band_key(wy0, h) };
        let blob = if inline { r.take(n)?.to_vec() } else { match fetch(&key) { Some(b) => b, None => { lost += 1; continue; } } };
        if blob.len() != n || band::hash(&blob) != h { if inline { return Err("save is damaged (band checksum)".into()); } lost += 1; continue; }
        // a band from before v12 is the home columns' full width: cut it into tiles
        let tiles: Vec<(i64, Vec<u8>)> = match x0 { Some(x0) => vec![(x0, blob)], None => band::split_legacy(&blob, gw, tw)?.into_iter().enumerate().map(|(k, b)| ((k * tw) as i64, b)).collect() };
        for (x0, blob) in tiles {
            let (x, y) = (x0 - ox, wy0 - oy);
            if (wy0 - oy).rem_euclid(BH as i64) != 0 || (x0 - ox).rem_euclid(tw as i64) != 0 { continue; }
            if y >= 0 && y < gh as i64 && x >= 0 && x < gw as i64 {
                band::decode(&mut w, x as usize, y as usize, &blob, (!same_mats).then_some(&remap[..]))?; window[(y as usize / BH) * (gw / tw) + x as usize / tw] = true;
            } else {
                let blob = if same_mats { blob } else { band::remap_blob(&blob, tw, &remap)? }; w.bands.insert((x0, wy0), band::Stored::new(blob));
            }
        }
    }
    if window.iter().any(|&k| !k) { return Err("save is missing part of the world around the mole".into()); }
    w.tick = tick; w.g.tick = gtick; w.inv = inv;
    w.mode = mode; w.rules = mode.rules(); w.rules.no_hunger = no_hunger; w.lamp_oil = if lamp_oil.is_finite() { lamp_oil.clamp(0., 1e5) } else { 0. };
    let tmpl = w.moles[0].clone(); w.moles.clear();
    for m in moles { if m.x < 2 || m.y < 4 || m.x >= gw - 2 || m.y >= gh - 2 { continue; }
        let name: &'static str = Box::leak(m.name.into_boxed_str());
        w.moles.push(Mole { name, x: m.x, y: m.y, hp: m.hp.clamp(0., 100.), breath: m.breath.clamp(0., 100.), hunger: m.hunger.clamp(0., 100.), sleep: m.sleep.clamp(0., 100.), alive: m.alive, facing: m.facing.signum().max(-1), path: VecDeque::new(), job: None, work: 0, work_cell: None, carry: None, ..tmpl.clone() }); }
    w.moles.truncate(1); if w.moles.is_empty() { w.moles.push(tmpl); }
    w.quest = quest.min(crate::craft::QUESTS.len()); w.chests_opened = chests_opened; w.pillars = pillars; w.max_depth = max_depth; w.crumbled = crumbled;
    w.stockpile = (sp[0].min(gw - 1), sp[1].min(gh - 1), sp[2].min(gw - 1), sp[3].min(gh - 1));
    w.craft_queue = craft_queue.into_iter().filter(|&k| k < RECIPES.len()).collect(); w.discovered = discovered;
    w.disasters.rng = drng; w.disasters.ticks_left = dleft; w.disasters.enabled = den; w.log = log; w.opened = opened;
    w.known = known.unwrap_or_else(|| std::array::from_fn(|k| w.inv[k] > 0));
    w.harvested = harvested; w.pl.deaths = deaths; w.pl.spawn = (spx, spy);
    w.journal = journal; w.found_clues = found_clues; w.relics = relics; w.ruins_seen = ruins_seen;
    // ruins visited before their carvings went into the journal: add the carvings now (by ruin id order)
    let mut seen: Vec<u32> = w.ruins_seen.iter().copied().collect(); seen.sort();
    for id in seen { if let Some(j) = crate::lore::ruin_entry(crate::ruins::fragment_of(id)) { if !w.journal.contains(&j) { w.journal.push(j); } } }
    if let Some(t) = transport { w.decode_transport(&t)?; }
    w.sites_seen = sites_seen;
    w.refresh_chests();
    if ver < 10 { w.infer_back(); }
    // grottos visited: saved since v11; before that, a grotto whose hollow has been seen counts as visited
    match grottos_seen { Some(s) => w.grottos_seen = s, None => w.mark_seen_grottos() }
    w.g.life.iter_mut().for_each(|v| *v = 0); w.g.wake_all(); w.st = crate::stress::Stress::new(gw * gh);
    let f = w.st.solve(&w.g); w.st.crack(&w.g, &f); w.g.clear_struct();
    w.say(format!("Loaded save (tick {}).", tick));
    if lost > 0 { w.say(format!("{lost} far parts of the save were missing; the Deepholt there is as it was before you came.")); }
    Ok(w)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub fn to_b64(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 4 / 3 + 4);
    for c in b.chunks(3) { let v = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 { if k <= c.len() { s.push(B64[(v >> (18 - 6 * k) & 63) as usize] as char); } else { s.push('='); } } }
    s
}
pub fn from_b64(s: &str) -> R<Vec<u8>> {
    let mut out = vec![]; let mut acc = 0u32; let mut n = 0;
    for ch in s.bytes() { if ch == b'=' { break; } let v = B64.iter().position(|&c| c == ch).ok_or("bad base64")? as u32; acc = acc << 6 | v; n += 6; if n >= 8 { n -= 8; out.push((acc >> n) as u8); acc &= (1 << n) - 1; } }
    Ok(out)
}
