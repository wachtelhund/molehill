//! Bands: the endless Deepholt stored off-window in tiles of `TW` columns (32 m) by `BH` rows (8 m).
//!
//! The simulated window is `w` × `h` cells of a world endless both ways (`World::ox`, `World::oy` =
//! world column and row of window cell (0, 0)). When the window slides, down or sideways, the tiles
//! that leave it are packed into blobs, and tiles that come back are unpacked from them; tiles never
//! visited come from the generator. A tile blob carries *everything* in it: cells and their flags,
//! copperline and devices (with fuse state), air, and the drops, torches, stations and mushroom plots
//! anchored in it. So a door built 40 m up, or 300 m west, is still there when the mole comes back.
//! (Saves before v12 stored full-width bands; `split_legacy` cuts them into tiles.)
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

/// Columns per tile: 32 m. A window that slides sideways is a whole number of tiles wide.
pub const TW: usize = 256;
const _: () = assert!(TW % B == 0 && TW % AB == 0 && TW % crate::grid::CHUNK == 0);
/// The tile width a window `w` columns wide stores in: `TW` where it divides the width, else the whole
/// width (a window like that never slides sideways: scenario maps).
pub fn tile_w(w: usize) -> usize { if w % TW == 0 { TW } else { w } }

/// The world row where the band holding world row `wy` starts (bands line up with the window).
pub fn band_start(oy: i64, wy: i64) -> i64 { wy - (wy - oy).rem_euclid(BH as i64) }

/// One tile's contents, unpacked: cells, block layers and the things anchored in it, with positions
/// relative to the tile's top-left cell. `tw` columns by `BH` rows.
pub struct Tile {
    pub tw: usize, pub mat: Vec<u8>, pub flags: Vec<u8>,
    wire: Vec<u8>, dev: Vec<u8>, dev_s: Vec<u8>, dev_p: Vec<i16>, burn: Vec<i16>, air: Vec<u8>,
    drops: Vec<(u32, u32, u8, u32)>, torches: Vec<(u32, u32)>, blds: Vec<(u8, u32, u32, u8)>, farms: Vec<(u32, u32, f32)>, furn: Vec<(u8, u32, u32, u8)>,
    walls: Option<Vec<u8>>,
}
impl Tile {
    /// Empty block layers and still air round the given cells (older saves kept only cells off-window).
    pub fn of_cells(mat: Vec<u8>, flags: Vec<u8>, tw: usize) -> Tile {
        let (nb, na) = ((tw / B) * (BH / B), (tw / AB) * (BH / AB));
        Tile { tw, mat, flags, wire: vec![0; nb], dev: vec![0; nb], dev_s: vec![0; nb], dev_p: vec![0; nb], burn: vec![0; nb], air: vec![0; na],
            drops: vec![], torches: vec![], blds: vec![], farms: vec![], furn: vec![], walls: None }
    }
    /// Read a blob `tw` columns wide.
    pub fn read(blob: &[u8], tw: usize) -> R<Tile> {
        let (nb, na) = ((tw / B) * (BH / B), (tw / AB) * (BH / AB));
        let mut r = Rd { b: blob, p: 0 };
        let (mat, flags) = (r.rle(tw * BH)?, r.rle(tw * BH)?);
        let (wire, dev, dev_s) = (r.rle(nb)?, r.rle(nb)?, r.rle(nb)?); let (dev_p, burn) = (r.rle16(nb)?, r.rle16(nb)?);
        let air = r.rle(na)?;
        let mut t = Tile { tw, mat, flags, wire, dev, dev_s, dev_p, burn, air, drops: vec![], torches: vec![], blds: vec![], farms: vec![], furn: vec![], walls: None };
        for _ in 0..r.len(1 << 20)? { t.drops.push((r.u32()?, r.u32()?, r.u8()?, r.u32()?)); }
        for _ in 0..r.len(1 << 20)? { t.torches.push((r.u32()?, r.u32()?)); }
        for _ in 0..r.len(1 << 16)? { t.blds.push((r.u8()?, r.u32()?, r.u32()?, r.u8()?)); }
        for _ in 0..r.len(1 << 16)? { t.farms.push((r.u32()?, r.u32()?, r.f32()?)); }
        // furniture and walls: optional trailing sections (older blobs simply end before them)
        if !r.done() { for _ in 0..r.len(1 << 16)? { t.furn.push((r.u8()?, r.u32()?, r.u32()?, r.u8()?)); } }
        if !r.done() { t.walls = Some(r.rle(tw * BH)?); }
        Ok(t)
    }
    pub fn write(&self) -> Vec<u8> {
        let mut o = Wr(vec![]);
        o.rle(&self.mat); o.rle(&self.flags); o.rle(&self.wire); o.rle(&self.dev); o.rle(&self.dev_s); o.rle16(&self.dev_p); o.rle16(&self.burn); o.rle(&self.air);
        o.u32(self.drops.len() as u32); for &(x, y, it, n) in &self.drops { o.u32(x); o.u32(y); o.u8(it); o.u32(n); }
        o.u32(self.torches.len() as u32); for &(x, y) in &self.torches { o.u32(x); o.u32(y); }
        o.u32(self.blds.len() as u32); for &(k, x, y, b) in &self.blds { o.u8(k); o.u32(x); o.u32(y); o.u8(b); }
        o.u32(self.farms.len() as u32); for &(x, y, g) in &self.farms { o.u32(x); o.u32(y); o.f32(g); }
        o.u32(self.furn.len() as u32); for &(it, x, y, t) in &self.furn { o.u8(it); o.u32(x); o.u32(y); o.u8(t); }
        let blank; let walls = match &self.walls { Some(v) => v, None => { blank = vec![0u8; self.tw * BH]; &blank } };
        o.rle(walls);
        o.0
    }
    /// Columns `x0 .. x0 + tw` of this tile (both multiples of the block sizes), with what is anchored in them.
    pub fn cut(&self, x0: usize, tw: usize) -> Tile {
        let (ow, obw, oaw) = (self.tw, self.tw / B, self.tw / AB); let (bw, aw) = (tw / B, tw / AB);
        let rows = |v: &[u8], w: usize, x: usize, n: usize, h: usize| -> Vec<u8> { (0..h).flat_map(|y| v[y * w + x..y * w + x + n].iter().copied()).collect() };
        let rows16 = |v: &[i16], w: usize, x: usize, n: usize, h: usize| -> Vec<i16> { (0..h).flat_map(|y| v[y * w + x..y * w + x + n].iter().copied()).collect() };
        let (bx, ax, x0u, x1u) = (x0 / B, x0 / AB, x0 as u32, (x0 + tw) as u32);
        let inx = |x: u32| x >= x0u && x < x1u;
        Tile { tw, mat: rows(&self.mat, ow, x0, tw, BH), flags: rows(&self.flags, ow, x0, tw, BH),
            wire: rows(&self.wire, obw, bx, bw, BH / B), dev: rows(&self.dev, obw, bx, bw, BH / B), dev_s: rows(&self.dev_s, obw, bx, bw, BH / B),
            dev_p: rows16(&self.dev_p, obw, bx, bw, BH / B), burn: rows16(&self.burn, obw, bx, bw, BH / B), air: rows(&self.air, oaw, ax, aw, BH / AB),
            drops: self.drops.iter().filter(|d| inx(d.0)).map(|&(x, y, i, n)| (x - x0u, y, i, n)).collect(),
            torches: self.torches.iter().filter(|t| inx(t.0)).map(|&(x, y)| (x - x0u, y)).collect(),
            blds: self.blds.iter().filter(|b| inx(b.1)).map(|&(k, x, y, b)| (k, x - x0u, y, b)).collect(),
            farms: self.farms.iter().filter(|f| inx(f.0)).map(|&(x, y, g)| (x - x0u, y, g)).collect(),
            furn: self.furn.iter().filter(|f| inx(f.1)).map(|&(i, x, y, t)| (i, x - x0u, y, t)).collect(),
            walls: self.walls.as_ref().map(|v| rows(v, ow, x0, tw, BH)) }
    }
    /// Pack window cells `x0 .. x0 + tw`, rows `y0 .. y0 + BH` (and what is anchored in them).
    pub fn grab(w: &World, x0: usize, y0: usize, tw: usize) -> Tile {
        let gw = w.g.w; let (x1, y1) = (x0 + tw, y0 + BH);
        let cells = |v: &[u8]| -> Vec<u8> { (y0..y1).flat_map(|y| v[y * gw + x0..y * gw + x1].iter().copied()).collect() };
        let lg = &w.logic; let (bx0, bx1, by0, by1) = (x0 / B, x1 / B, y0 / B, y1 / B);
        let blk = |v: &[u8]| -> Vec<u8> { (by0..by1).flat_map(|by| v[by * lg.bw + bx0..by * lg.bw + bx1].iter().copied()).collect() };
        let blk16 = |v: &[i16]| -> Vec<i16> { (by0..by1).flat_map(|by| v[by * lg.bw + bx0..by * lg.bw + bx1].iter().copied()).collect() };
        let abw = w.air.bw; let air: Vec<u8> = (y0 / AB..y1 / AB).flat_map(|ay| (x0 / AB..x1 / AB).map(move |ax| ay * abw + ax)).map(|k| (w.air.co2[k].clamp(0., 1.5) / 1.5 * 255.) as u8).collect();
        let inb = |x: usize, y: usize| x >= x0 && x < x1 && y >= y0 && y < y1;
        let (rx, ry) = (|x: usize| (x - x0) as u32, |y: usize| (y - y0) as u32);
        Tile { tw, mat: cells(&w.g.mat), flags: cells(&w.g.flags), wire: blk(&lg.wire), dev: blk(&lg.dev), dev_s: blk(&lg.dev_s), dev_p: blk16(&lg.dev_p), burn: blk16(&lg.burn), air,
            drops: w.drops.iter().filter(|d| inb(d.x, d.y)).map(|d| (rx(d.x), ry(d.y), d.item as u8, d.n)).collect(),
            torches: w.torches.iter().filter(|t| inb(t.0, t.1)).map(|&(x, y)| (rx(x), ry(y))).collect(),
            blds: w.buildings.iter().filter(|b| inb(b.x, b.y)).map(|b| (STATION_CODE + STATIONS.iter().position(|&s| s == b.kind).unwrap_or(0) as u8, rx(b.x), ry(b.y), b.built as u8)).collect(),
            farms: w.farms.iter().filter(|f| inb(f.x, f.y)).map(|f| (rx(f.x), ry(f.y), f.growth)).collect(),
            furn: w.furniture.iter().filter(|f| inb(f.x, f.y)).map(|f| (f.item as u8, rx(f.x), ry(f.y), f.tint)).collect(),
            walls: Some(cells(&w.walls.cell)) }
    }
    /// Unpack into window cells `x0 .. x0 + tw`, rows `y0 .. y0 + BH`, adding its anchored things (sanitised).
    /// `remap` translates material ids from an older material table. `Ok(true)` if it carried a wall
    /// layer; `Ok(false)` for an older blob (the caller infers lining). Fails without touching the window.
    pub fn put(mut self, w: &mut World, x0: usize, y0: usize, remap: Option<&[u8]>) -> R<bool> {
        let (gw, gh, tw) = (w.g.w, w.g.h, self.tw); if y0 + BH > gh || x0 + tw > gw || x0 % B != 0 || x0 % AB != 0 { return Err("tile outside the window".into()); }
        if let Some(rm) = remap { for m in self.mat.iter_mut() { *m = *rm.get(*m as usize).unwrap_or(&0); } }
        else if self.mat.iter().any(|&m| m as usize >= crate::mat::NMAT) { return Err("material id out of range".into()); }
        for y in 0..BH { let (a, b) = ((y0 + y) * gw + x0, y * tw); w.g.mat[a..a + tw].copy_from_slice(&self.mat[b..b + tw]); w.g.flags[a..a + tw].copy_from_slice(&self.flags[b..b + tw]); }
        let ndev = crate::logic::DEVS.len() as u8; let (bw, tbw) = (w.logic.bw, tw / B); let lg = &mut w.logic;
        for k in 0..tbw * (BH / B) {
            let b = (y0 / B + k / tbw) * bw + x0 / B + k % tbw;
            let d = if self.dev[k] <= ndev { self.dev[k] } else { 0 };
            lg.wire[b] = self.wire[k].min(1); lg.dev[b] = d; lg.dev_s[b] = self.dev_s[k]; lg.dev_p[b] = self.dev_p[k];
            lg.burn[b] = if d == 0 { 0 } else { self.burn[k].clamp(0, 600) }; lg.powered[b] = 0;
        }
        lg.invalidate();
        let (abw, taw) = (w.air.bw, tw / AB); for (k, &v) in self.air.iter().enumerate() { w.air.co2[(y0 / AB + k / taw) * abw + x0 / AB + k % taw] = v as f32 / 255. * 1.5; }
        let (tx, ty) = (|x: u32| x0 + x as usize, |y: u32| y0 + y as usize); let ok = |x: u32, y: u32| (x as usize) < tw && (y as usize) < BH;
        for &(x, y, it, n) in &self.drops { if ok(x, y) && (it as usize) < NITEM { w.drops.push(Drop::at(tx(x) as f32 + 0.5, ty(y) as f32 + 0.5, ITEMS[it as usize], n)); } }
        for &(x, y) in &self.torches { if ok(x, y) { w.torches.push((tx(x), ty(y))); } }
        for &(k, x, y, b) in &self.blds { if let Some(kind) = station_from_code(k) { if ok(x, y) && tx(x) + Station::W < gw && ty(y) >= Station::H { w.buildings.push(Building { kind, x: tx(x), y: ty(y), built: b != 0, claimed: false }); } } }
        for &(x, y, g) in &self.farms { if ok(x, y) && ty(y) + 1 < gh { w.farms.push(Farm { x: tx(x), y: ty(y), growth: g.clamp(0., 1.), why: 0 }); } }
        for &(it, x, y, t) in &self.furn { if ok(x, y) && (it as usize) < NITEM && ITEMS[it as usize].furn().is_some() { w.furniture.push(crate::world::Furniture { item: ITEMS[it as usize], x: tx(x), y: ty(y), tint: t.min(6) }); } }
        let had = self.walls.is_some();
        if let Some(wl) = self.walls { for y in 0..BH { let (a, b) = ((y0 + y) * gw + x0, y * tw); w.walls.cell[a..a + tw].copy_from_slice(&wl[b..b + tw]); } }
        Ok(had)
    }
}

/// Pack the window tile at cell (x0, y0) into a blob (`tile_w` of the window wide). Material ids are raw.
pub fn encode(w: &World, x0: usize, y0: usize) -> Vec<u8> { Tile::grab(w, x0, y0, tile_w(w.g.w)).write() }
/// Unpack a blob into the window tile at cell (x0, y0); see `Tile::put`.
pub fn decode(w: &mut World, x0: usize, y0: usize, blob: &[u8], remap: Option<&[u8]>) -> R<bool> { Tile::read(blob, tile_w(w.g.w))?.put(w, x0, y0, remap) }
/// Just the cells of a tile blob `tw` wide (materials, flags), for drawing far rows. `None` if damaged.
pub fn cells(blob: &[u8], tw: usize) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut r = Rd { b: blob, p: 0 }; let m = r.rle(tw * BH).ok()?; let f = r.rle(tw * BH).ok()?; Some((m, f))
}
/// Rewrite a blob's material ids through `remap` (a save from a build with another material table).
pub fn remap_blob(blob: &[u8], tw: usize, remap: &[u8]) -> R<Vec<u8>> {
    let mut r = Rd { b: blob, p: 0 }; let mut m = r.rle(tw * BH)?;
    for v in m.iter_mut() { *v = *remap.get(*v as usize).unwrap_or(&0); }
    let mut o = Wr(vec![]); o.rle(&m); o.0.extend_from_slice(&blob[r.p..]); Ok(o.0)
}
/// A full-width band blob from a save before v12 (`gw` columns, window column 0 = world column 0), cut
/// into `tw`-wide tile blobs, left to right.
pub fn split_legacy(blob: &[u8], gw: usize, tw: usize) -> R<Vec<Vec<u8>>> {
    let t = Tile::read(blob, gw)?; Ok((0..gw / tw).map(|k| t.cut(k * tw, tw).write()).collect())
}
