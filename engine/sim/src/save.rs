//! Save / load (port of the v2/saveload design, binary instead of JSON).
//! Envelope: magic "MOLEHILL", format version, engine id, dimensions, seed; then length-prefixed
//! sections. Cell layers are byte-RLE. Materials are stored by *name* (a name table travels with
//! the save) so the material enum can grow/reorder. Loading regenerates the world from the seed
//! (biome/surface info, background) and then overwrites all mutable state. Loading never panics:
//! every read is bounds-checked and returns a readable error; values are sanitised.
use crate::craft::{Item, Station, NITEM, RECIPES, STATIONS};
use crate::mat::{Mat, NMAT, PROPS};
use crate::world::{Building, Chest, Drop, Mole, World};
use std::collections::VecDeque;

pub const VERSION: u32 = 1;
const MAGIC: &[u8; 8] = b"MOLEHILL";
const ENGINE: &str = "bevy-v3";

struct Wr(Vec<u8>);
impl Wr {
    fn u8(&mut self, v: u8) { self.0.push(v); }
    fn u32(&mut self, v: u32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn u64(&mut self, v: u64) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn f32(&mut self, v: f32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn str(&mut self, s: &str) { self.u32(s.len() as u32); self.0.extend_from_slice(s.as_bytes()); }
    fn bytes(&mut self, b: &[u8]) { self.u32(b.len() as u32); self.0.extend_from_slice(b); }
    fn rle(&mut self, b: &[u8]) { let mut out = vec![]; let mut i = 0; while i < b.len() { let v = b[i]; let mut n = 1; while i + n < b.len() && b[i + n] == v && n < 255 { n += 1; } out.push(n as u8); out.push(v); i += n; } self.bytes(&out); }
}
struct Rd<'a> { b: &'a [u8], p: usize }
type R<T> = Result<T, String>;
impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> R<&'a [u8]> { if self.p + n > self.b.len() { return Err("save is truncated".into()); } let s = &self.b[self.p..self.p + n]; self.p += n; Ok(s) }
    fn u8(&mut self) -> R<u8> { Ok(self.take(1)?[0]) }
    fn u32(&mut self) -> R<u32> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn u64(&mut self) -> R<u64> { Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap())) }
    fn f32(&mut self) -> R<f32> { let v = f32::from_le_bytes(self.take(4)?.try_into().unwrap()); Ok(if v.is_finite() { v } else { 0. }) }
    fn len(&mut self, max: usize) -> R<usize> { let n = self.u32()? as usize; if n > max { return Err(format!("bad length {n}")); } Ok(n) }
    fn str(&mut self) -> R<String> { let n = self.len(4096)?; String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "bad string".into()) }
    fn bytes(&mut self) -> R<&'a [u8]> { let n = self.len(1 << 28)?; self.take(n) }
    fn rle(&mut self, expect: usize) -> R<Vec<u8>> {
        let b = self.bytes()?; if b.len() % 2 != 0 { return Err("bad RLE".into()); }
        let mut out = Vec::with_capacity(expect); for c in b.chunks(2) { out.extend(std::iter::repeat(c[1]).take(c[0] as usize)); if out.len() > expect { return Err("RLE layer too long".into()); } }
        if out.len() != expect { return Err(format!("layer length {} != {}", out.len(), expect)); } Ok(out)
    }
}

pub fn save(w: &World) -> Vec<u8> {
    let mut o = Wr(vec![]); o.0.extend_from_slice(MAGIC); o.u32(VERSION); o.str(ENGINE);
    o.u32(w.g.w as u32); o.u32(w.g.h as u32); o.u32(w.info.seed); o.u64(w.tick); o.u64(w.g.tick);
    o.u32(NMAT as u32); for p in PROPS.iter() { o.str(p.name); }
    o.rle(&w.g.mat); o.rle(&w.g.flags); o.rle(&w.desig);
    o.rle(&w.logic.wire); o.rle(&w.logic.dev); o.rle(&w.logic.dev_s);
    let dp: Vec<u8> = w.logic.dev_p.iter().flat_map(|v| v.to_le_bytes()).collect(); o.rle(&dp);
    o.u32(NITEM as u32); for v in w.inv { o.u32(v); }
    o.u32(w.moles.len() as u32);
    for m in &w.moles { o.str(m.name); o.u32(m.x as u32); o.u32(m.y as u32); o.f32(m.hp); o.f32(m.breath); o.f32(m.hunger); o.f32(m.sleep); o.u8(m.alive as u8); o.u8((m.facing + 1) as u8); }
    o.u32(w.drops.len() as u32); for d in &w.drops { o.u32(d.x as u32); o.u32(d.y as u32); o.u8(d.item as u8); o.u32(d.n); }
    o.u32(w.chests.len() as u32); for c in &w.chests { o.u32(c.x as u32); o.u32(c.y as u32); o.u8(c.opened as u8); }
    o.u32(w.buildings.len() as u32); for b in &w.buildings { o.u8(b.kind as u8); o.u32(b.x as u32); o.u32(b.y as u32); o.u8(b.built as u8); }
    o.u32(w.torches.len() as u32); for &(x, y) in &w.torches { o.u32(x as u32); o.u32(y as u32); }
    o.u32(w.quest as u32); o.u32(w.chests_opened); o.u32(w.pillars); o.f32(w.max_depth); o.u64(w.crumbled);
    let sp = w.stockpile; for v in [sp.0, sp.1, sp.2, sp.3] { o.u32(v as u32); }
    o.u32(w.craft_queue.len() as u32); for &r in &w.craft_queue { o.u32(r as u32); }
    for d in w.discovered { o.u8(d as u8); }
    o.u64(w.disasters.rng); o.u32(w.disasters.ticks_left.max(60)); o.u8(w.disasters.enabled as u8);
    o.u32(w.log.len() as u32); for (t, s) in &w.log { o.u64(*t); o.str(s); }
    o.0
}

pub fn load(b: &[u8]) -> R<World> {
    let mut r = Rd { b, p: 0 };
    if r.take(8).map_err(|_| "not a Molehill save".to_string())? != MAGIC { return Err("not a Molehill save".into()); }
    let ver = r.u32()?; if ver > VERSION { return Err(format!("save version {ver} is newer than this build ({VERSION})")); }
    let eng = r.str()?; if eng != ENGINE { return Err(format!("save is from engine '{eng}', expected '{ENGINE}'")); }
    let (gw, gh) = (r.len(1 << 14)?, r.len(1 << 14)?); if gw % 64 != 0 || gh % 64 != 0 || gw == 0 || gh == 0 { return Err("bad world dimensions".into()); }
    let seed = r.u32()?; let tick = r.u64()?; let gtick = r.u64()?;
    let nm = r.len(256)?; let mut remap = vec![0u8; nm];
    for k in 0..nm { let name = r.str()?; remap[k] = PROPS.iter().position(|p| p.name == name).ok_or(format!("unknown material '{name}'"))? as u8; }
    let n = gw * gh;
    let mut mat = r.rle(n)?; for m in mat.iter_mut() { if *m as usize >= nm { return Err("material id out of range".into()); } *m = remap[*m as usize]; }
    let flags = r.rle(n)?; let desig = r.rle(n)?;
    let mut w = World::new(gw, gh, seed);
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
    w.drops.clear(); for _ in 0..r.len(1 << 20)? { let (x, y, it, nn) = (r.u32()? as usize, r.u32()? as usize, r.u8()?, r.u32()?); if x < gw && y < gh && (it as usize) < NITEM { w.drops.push(Drop { x, y, item: crate::craft::ITEMS[it as usize], n: nn, claimed: false }); } }
    w.chests.clear(); for _ in 0..r.len(1 << 16)? { let (x, y, o) = (r.u32()? as usize, r.u32()? as usize, r.u8()? != 0); if x < gw && y < gh { w.chests.push(Chest { x, y, opened: o }); } }
    w.buildings.clear(); for _ in 0..r.len(1 << 16)? { let (k, x, y, b) = (r.u8()?, r.u32()? as usize, r.u32()? as usize, r.u8()? != 0); if (k as usize) < STATIONS.len() && x + Station::W < gw && y < gh { w.buildings.push(Building { kind: STATIONS[k as usize], x, y, built: b, claimed: false }); } }
    w.torches.clear(); for _ in 0..r.len(1 << 20)? { let (x, y) = (r.u32()? as usize, r.u32()? as usize); if x < gw && y < gh { w.torches.push((x, y)); } }
    w.quest = (r.u32()? as usize).min(crate::craft::QUESTS.len()); w.chests_opened = r.u32()?; w.pillars = r.u32()?; w.max_depth = r.f32()?; w.crumbled = r.u64()?;
    let sp: Vec<usize> = (0..4).map(|_| r.u32().map(|v| v as usize)).collect::<R<_>>()?; w.stockpile = (sp[0].min(gw - 1), sp[1].min(gh - 1), sp[2].min(gw - 1), sp[3].min(gh - 1));
    w.craft_queue = (0..r.len(64)?).map(|_| r.u32().map(|v| v as usize)).collect::<R<Vec<_>>>()?.into_iter().filter(|&k| k < RECIPES.len()).collect();
    for k in 0..NMAT { w.discovered[k] = r.u8()? != 0; }
    w.disasters.rng = r.u64()? | 1; w.disasters.ticks_left = r.u32()?; w.disasters.enabled = r.u8()? != 0;
    w.log.clear(); for _ in 0..r.len(1000)? { let t = r.u64()?; let s = r.str()?; w.log.push((t, s)); }
    let _ = Item::Dirt; let _ = Mat::Empty;
    // derived state
    w.g.life.iter_mut().for_each(|v| *v = 0); w.g.wake_all(); w.st = crate::stress::Stress::new(n);
    let f = w.st.solve(&w.g); w.st.crack(&f); w.g.struct_dirty = None;
    w.say(format!("Loaded save (tick {}).", tick));
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
