//! Bands: the endless Deepholt stored off-window in horizontal strips of `BH` rows (8 m).
//!
//! The simulated window is `h` rows of the endless world (`World::oy` = world row of window row 0).
//! When the window slides, the rows that leave it are packed into band blobs, and rows that come
//! back are unpacked from them; rows never visited come from the generator. A band blob carries
//! *everything* in its rows: cells and their flags, copperline and devices (with fuse state), air,
//! and the drops, torches, stations and mushroom plots anchored in it. So a door built 40 m up is
//! still there when the mole comes back.
//!
//! The same blobs are the unit of saving (`save::save_parts`): a save is a small manifest that names
//! its bands by content hash, plus the blobs. Bands that did not change keep their key, so an
//! autosave only writes what changed, however far the colony has dug.
use crate::craft::{Station, ITEMS, NITEM, STATIONS};
use crate::world::{Building, Drop, Farm, World};
use crate::{air::AB, logic::B};

/// Stations are stored as `STATION_CODE + index`; smaller codes are the four stations of older saves
/// (workbench, kiln, furnace, anvil), which become the workbench, forge, forge and workshop.
pub const STATION_CODE: u8 = 16;
pub fn station_from_code(k: u8) -> Option<Station> {
    if k >= STATION_CODE { STATIONS.get((k - STATION_CODE) as usize).copied() } else { [Station::Workbench, Station::Forge, Station::Forge, Station::Workshop].get(k as usize).copied() }
}
/// Rows per band: 8 m. The window height and every slide are multiples of it.
pub const BH: usize = 64;
const _: () = assert!(BH % B == 0 && BH % AB == 0);

pub(crate) struct Wr(pub Vec<u8>);
impl Wr {
    pub fn u8(&mut self, v: u8) { self.0.push(v); }
    pub fn u32(&mut self, v: u32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    pub fn u64(&mut self, v: u64) { self.0.extend_from_slice(&v.to_le_bytes()); }
    pub fn f32(&mut self, v: f32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    pub fn str(&mut self, s: &str) { self.u32(s.len() as u32); self.0.extend_from_slice(s.as_bytes()); }
    pub fn bytes(&mut self, b: &[u8]) { self.u32(b.len() as u32); self.0.extend_from_slice(b); }
    pub fn rle(&mut self, b: &[u8]) { let mut out = vec![]; let mut i = 0; while i < b.len() { let v = b[i]; let mut n = 1; while i + n < b.len() && b[i + n] == v && n < 255 { n += 1; } out.push(n as u8); out.push(v); i += n; } self.bytes(&out); }
    pub fn rle16(&mut self, v: &[i16]) { let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect(); self.rle(&b); }
}
pub(crate) struct Rd<'a> { pub b: &'a [u8], pub p: usize }
pub type R<T> = Result<T, String>;
impl<'a> Rd<'a> {
    pub fn take(&mut self, n: usize) -> R<&'a [u8]> { if n > self.b.len() - self.p { return Err("save is truncated".into()); } let s = &self.b[self.p..self.p + n]; self.p += n; Ok(s) }
    pub fn u8(&mut self) -> R<u8> { Ok(self.take(1)?[0]) }
    pub fn u32(&mut self) -> R<u32> { let t = self.take(4)?; Ok(u32::from_le_bytes([t[0], t[1], t[2], t[3]])) }
    pub fn u64(&mut self) -> R<u64> { let t = self.take(8)?; let mut a = [0u8; 8]; a.copy_from_slice(t); Ok(u64::from_le_bytes(a)) }
    pub fn f32(&mut self) -> R<f32> { let v = f32::from_bits(self.u32()?); Ok(if v.is_finite() { v } else { 0. }) }
    pub fn len(&mut self, max: usize) -> R<usize> { let n = self.u32()? as usize; if n > max { return Err(format!("bad length {n}")); } Ok(n) }
    pub fn str(&mut self) -> R<String> { let n = self.len(4096)?; String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "bad string".into()) }
    pub fn bytes(&mut self) -> R<&'a [u8]> { let n = self.len(1 << 28)?; self.take(n) }
    pub fn rle(&mut self, expect: usize) -> R<Vec<u8>> {
        let b = self.bytes()?; if b.len() % 2 != 0 { return Err("bad RLE".into()); }
        let mut out = Vec::with_capacity(expect); for c in b.chunks(2) { out.extend(std::iter::repeat(c[1]).take(c[0] as usize)); if out.len() > expect { return Err("RLE layer too long".into()); } }
        if out.len() != expect { return Err(format!("layer length {} != {}", out.len(), expect)); } Ok(out)
    }
    pub fn rle16(&mut self, expect: usize) -> R<Vec<i16>> { Ok(self.rle(expect * 2)?.chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect()) }
    pub fn done(&self) -> bool { self.p >= self.b.len() }
}

/// FNV-1a 64: names a band blob by its content (a changed band gets a new key; a torn write is caught).
pub fn hash(b: &[u8]) -> u64 { let mut h = 0xcbf29ce484222325u64; for &x in b { h ^= x as u64; h = h.wrapping_mul(0x100000001b3); } h }

/// One band off-window: its encoded blob and the blob's hash.
#[derive(Clone, Debug)]
pub struct Stored { pub blob: Vec<u8>, pub hash: u64 }
impl Stored { pub fn new(blob: Vec<u8>) -> Stored { Stored { hash: hash(&blob), blob } } }

/// The world row where the band holding world row `wy` starts (bands line up with the window).
pub fn band_start(oy: i64, wy: i64) -> i64 { wy - (wy - oy).rem_euclid(BH as i64) }

/// Pack window rows `y0 .. y0+BH` (and what is anchored in them) into a blob. Material ids are raw.
pub fn encode(w: &World, y0: usize) -> Vec<u8> {
    let (gw, y1) = (w.g.w, y0 + BH); let cells = y0 * gw..y1 * gw;
    let lg = &w.logic; let blk = (y0 / B) * lg.bw..(y1 / B) * lg.bw;
    let ab = (y0 / AB) * w.air.bw..(y1 / AB) * w.air.bw;
    let mut o = Wr(vec![]);
    o.rle(&w.g.mat[cells.clone()]); o.rle(&w.g.flags[cells]);
    o.rle(&lg.wire[blk.clone()]); o.rle(&lg.dev[blk.clone()]); o.rle(&lg.dev_s[blk.clone()]); o.rle16(&lg.dev_p[blk.clone()]); o.rle16(&lg.burn[blk]);
    o.rle(&w.air.to_bytes()[ab]);
    let inb = |y: usize| y >= y0 && y < y1;
    let d: Vec<&Drop> = w.drops.iter().filter(|d| inb(d.y)).collect();
    o.u32(d.len() as u32); for d in d { o.u32(d.x as u32); o.u32((d.y - y0) as u32); o.u8(d.item as u8); o.u32(d.n); }
    let t: Vec<&(usize, usize)> = w.torches.iter().filter(|t| inb(t.1)).collect();
    o.u32(t.len() as u32); for &&(x, y) in &t { o.u32(x as u32); o.u32((y - y0) as u32); }
    let b: Vec<&Building> = w.buildings.iter().filter(|b| inb(b.y)).collect();
    o.u32(b.len() as u32); for b in b { o.u8(STATION_CODE + STATIONS.iter().position(|&s| s == b.kind).unwrap_or(0) as u8); o.u32(b.x as u32); o.u32((b.y - y0) as u32); o.u8(b.built as u8); }
    let f: Vec<&Farm> = w.farms.iter().filter(|f| inb(f.y)).collect();
    o.u32(f.len() as u32); for f in f { o.u32(f.x as u32); o.u32((f.y - y0) as u32); o.f32(f.growth); }
    o.0
}

/// Just the cells of a band blob (materials, flags), for drawing the far rows. `None` if damaged.
pub fn cells(blob: &[u8], gw: usize) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut r = Rd { b: blob, p: 0 }; let m = r.rle(gw * BH).ok()?; let f = r.rle(gw * BH).ok()?; Some((m, f))
}

/// Unpack a blob into window rows `y0 .. y0+BH`, adding its anchored things. `remap` translates
/// material ids from an older material table. Fails cleanly (and leaves the rows untouched) on a damaged blob.
pub fn decode(w: &mut World, y0: usize, blob: &[u8], remap: Option<&[u8]>) -> R<()> {
    let (gw, gh) = (w.g.w, w.g.h); if y0 + BH > gh { return Err("band outside the window".into()); }
    let (bw, abw) = (w.logic.bw, w.air.bw); let nb = bw * (BH / B);
    let mut r = Rd { b: blob, p: 0 };
    let mut mat = r.rle(gw * BH)?; let flags = r.rle(gw * BH)?;
    if let Some(rm) = remap { for m in mat.iter_mut() { *m = *rm.get(*m as usize).unwrap_or(&0); } }
    else if mat.iter().any(|&m| m as usize >= crate::mat::NMAT) { return Err("material id out of range".into()); }
    let (wire, dev, dev_s) = (r.rle(nb)?, r.rle(nb)?, r.rle(nb)?); let (dev_p, burn) = (r.rle16(nb)?, r.rle16(nb)?);
    let air = r.rle(abw * (BH / AB))?;
    let mut drops = vec![]; for _ in 0..r.len(1 << 20)? { let (x, y, it, n) = (r.u32()? as usize, r.u32()? as usize, r.u8()?, r.u32()?); if x < gw && y < BH && (it as usize) < NITEM { drops.push(Drop::at(x as f32 + 0.5, (y0 + y) as f32 + 0.5, ITEMS[it as usize], n)); } }
    let mut torches = vec![]; for _ in 0..r.len(1 << 20)? { let (x, y) = (r.u32()? as usize, r.u32()? as usize); if x < gw && y < BH { torches.push((x, y0 + y)); } }
    let mut blds = vec![]; for _ in 0..r.len(1 << 16)? { let (k, x, y, b) = (r.u8()?, r.u32()? as usize, r.u32()? as usize, r.u8()? != 0); if let Some(kind) = station_from_code(k) { if x + Station::W < gw && y < BH && y0 + y >= Station::H { blds.push(Building { kind, x, y: y0 + y, built: b, claimed: false }); } } }
    let mut farms = vec![]; for _ in 0..r.len(1 << 16)? { let (x, y, g) = (r.u32()? as usize, r.u32()? as usize, r.f32()?); if x < gw && y < BH && y0 + y + 1 < gh { farms.push(Farm { x, y: y0 + y, growth: g.clamp(0., 1.), why: 0 }); } }
    // all read: apply
    let c0 = y0 * gw; w.g.mat[c0..c0 + gw * BH].copy_from_slice(&mat); w.g.flags[c0..c0 + gw * BH].copy_from_slice(&flags);
    let ndev = crate::logic::DEVS.len() as u8; let b0 = (y0 / B) * bw; let lg = &mut w.logic;
    for k in 0..nb {
        let d = if dev[k] <= ndev { dev[k] } else { 0 };
        lg.wire[b0 + k] = wire[k].min(1); lg.dev[b0 + k] = d; lg.dev_s[b0 + k] = dev_s[k]; lg.dev_p[b0 + k] = dev_p[k];
        lg.burn[b0 + k] = if d == 0 { 0 } else { burn[k].clamp(0, 600) }; lg.powered[b0 + k] = 0;
    }
    lg.invalidate();
    let a0 = (y0 / AB) * abw; for (k, &v) in air.iter().enumerate() { w.air.co2[a0 + k] = v as f32 / 255. * 1.5; }
    w.drops.extend(drops); w.torches.extend(torches); w.buildings.extend(blds); w.farms.extend(farms);
    Ok(())
}

/// A blob with only cells (older saves cached rows without devices or things): empty block layers, still air.
pub fn encode_cells(mat: &[u8], flags: &[u8], bw: usize, abw: usize) -> Vec<u8> {
    let nb = bw * (BH / B); let mut o = Wr(vec![]);
    o.rle(mat); o.rle(flags); for _ in 0..3 { o.rle(&vec![0u8; nb]); } for _ in 0..2 { o.rle16(&vec![0i16; nb]); }
    o.rle(&vec![0u8; abw * (BH / AB)]); for _ in 0..4 { o.u32(0); }
    o.0
}
/// Rewrite a blob's material ids through `remap` (a save from a build with another material table).
pub fn remap_blob(blob: &[u8], gw: usize, remap: &[u8]) -> R<Vec<u8>> {
    let mut r = Rd { b: blob, p: 0 }; let mut m = r.rle(gw * BH)?;
    for v in m.iter_mut() { *v = *remap.get(*v as usize).unwrap_or(&0); }
    let mut o = Wr(vec![]); o.rle(&m); o.0.extend_from_slice(&blob[r.p..]); Ok(o.0)
}
