//! Production: a mine's output moves as real loose items, under gravity, from the face to where it's wanted.
//! Nothing teleports and nothing is stored out of sight: every lump is a `Drop` in the world, in a cart, or
//! in an ore hopper's bin (drawn as its heap), and it gets from one to the next by falling, sliding and rolling.
//!
//! **Chutes** (`Mat::Chute`, flag `CHUTE`): a smooth timber trough laid with the block brush. A loose item
//! lying on a chute's top slides down its slope: gravity along the slope (g·sinθ) against the chute's low
//! friction (μ·g·cosθ), so a 1-in-2 chute runs items off fast and a level one holds them.
//!
//! **Shaker screens** (`Mat::Screen`, flags `CHUTE | SCREEN`): an iron grating. Anything finer than its mesh
//! (`MESH_M`, by `Item::size_m`: sand, dirt, clay, gravel) falls straight through; lumps (ore, coal, stone)
//! ride over it. Its iron grips more than timber, so lumps lie still on a gentle screen until current in
//! copperline laid along it (the same 1 m blocks) works its knocker: shaken, the grip drops to
//! `SHAKE_GRIP` of itself and the lumps walk down the slope and off the end.
//!
//! **Bumpers** (`Mat::Bumper`, flag `BUMPER`): a timber buffer at the end of the track. A loaded cart that
//! runs into one tips: it is held against the buffer and its load comes out over the front, one item every
//! `TIP_EVERY` ticks, thrown forward (`TIP_V`) as loose items, then it rights itself and is free to go back.
//! If there is no room for a lump to come out (the buffer is taller than the rim, or the spill has piled up)
//! the load jams and stays in the cart.
//!
//! **Ore hoppers** (`Item::OreHopper`): a 1.5 m timber bin on legs, its spout 0.75 m over the floor so a cart
//! fits under it. Loose items that fall into its mouth, and loose sand and rubble poured in (cells, one item
//! per `CELLS_PER_ITEM`), are its load, up to `BIN_LOAD`. A cart standing under the spout with room trips its
//! gate and the load comes down into the cart one item every `FEED_EVERY` ticks; right mouse holds the gate
//! open by hand (it then empties onto whatever is under it). Dig away both legs' footing and it falls,
//! spilling its load.
use crate::craft::{drop_of, Item, CELLS_PER_ITEM, NITEM};
use crate::items::ITEM_HW;
use crate::mat::{Kind, CHUTE, CPM, SCREEN, BUMPER};
use crate::transport::{Cart, CART_H, CART_LOAD, CART_W};
use crate::world::{Drop, Event, World, TUNE};

/// A shaker screen's mesh (m): finer than this falls through.
pub const MESH_M: f32 = 0.05;
/// The share of its grip a screen keeps while current shakes it.
pub const SHAKE_GRIP: f32 = 0.15;
/// Ticks between items coming out of a tipping cart, and how fast they are thrown forward and up (cells/s, 3 m/s and 1 m/s).
pub const TIP_EVERY: u16 = 3;
pub const TIP_V: f32 = 3. * CPM as f32; pub const TIP_UP: f32 = CPM as f32;
/// The ore hopper: bin width and height, its legs (the spout's clearance over the floor), what it holds,
/// and ticks between items let down through its gate.
pub const BIN_W: usize = 3 * CPM / 2;
pub const BIN_H: usize = CPM;
pub const BIN_LEGS: usize = 3 * CPM / 4;
pub const BIN_LOAD: u32 = 60;
pub const FEED_EVERY: u16 = 5;

#[derive(Clone, Debug)]
pub struct OreHopper {
    /// the spout's column, the first row under the bin (the spout's mouth), and the floor row its legs stand on
    pub x: usize, pub y: usize, pub floor: usize,
    pub contents: Vec<(Item, u32)>,
    /// gate held open by hand; the gate is open this tick (held, or tripped by a cart under it); ticks to the next item
    pub held: bool, pub open: bool, pub feed: u16,
    /// cells of loose sand and rubble poured in, per item, toward the next whole item
    pub grit: Vec<(Item, u16)>,
}
impl OreHopper {
    pub fn load(&self) -> u32 { self.contents.iter().map(|c| c.1).sum() }
    /// The bin (x0, y0, x1, y1) in cells, legs not included.
    pub fn rect(&self) -> (f32, f32, f32, f32) { let h = (BIN_W / 2) as f32; (self.x as f32 - h, (self.y - BIN_H) as f32, self.x as f32 + h, self.y as f32) }
    /// The bin and its legs, for hovering and clicking.
    pub fn contains(&self, x: f32, y: f32) -> bool { let (x0, y0, x1, _) = self.rect(); x >= x0 && x < x1 && y >= y0 && y < self.floor as f32 }
    fn add(&mut self, it: Item, n: u32) { if let Some(c) = self.contents.iter_mut().find(|c| c.0 == it) { c.1 += n; } else { self.contents.push((it, n)); } }
    fn take_one(&mut self) -> Option<Item> {
        let c = self.contents.first_mut()?; let it = c.0; c.1 -= 1; if c.1 == 0 { self.contents.remove(0); } Some(it)
    }
}

impl World {
    /// Can a loose `it` pass through the cell (open, liquid, gas, or a screen it is finer than)?
    #[inline] pub(crate) fn item_passes(&self, x: usize, y: usize, it: Item) -> bool {
        self.passable(x, y) || self.g.get(x, y).has(SCREEN) && it.size_m() < MESH_M
    }
    /// Is the cell a screen with current in its block (its knocker shaking it)?
    pub fn shaking(&self, x: usize, y: usize) -> bool { self.g.get(x, y).has(SCREEN) && (self.logic.net_lit(self.logic.block_of(x, y)) || self.mech_shakes(x, y)) }
    /// The chute cell an item lying at (px, py) rests on, if any (the cells under its box, middle first).
    pub(crate) fn chute_under(&self, px: f32, py: f32, it: Item) -> Option<(usize, usize)> {
        let r = (py + ITEM_HW + 0.05).floor() as i32; let (x0, x1) = ((px - ITEM_HW).floor() as i32, (px + ITEM_HW - 1e-3).floor() as i32);
        let mid = px.floor() as i32;
        std::iter::once(mid).chain(x0..=x1).find(|&c| self.g.in_bounds(c, r) && { let m = self.g.get(c as usize, r as usize); m.has(CHUTE) && !self.item_passes(c as usize, r as usize, it) })
            .map(|c| (c as usize, r as usize))
    }
    /// The top of a chute in column x within `reach` rows of row y (a chute cell with no chute on it).
    fn chute_top(&self, x: i32, y: i32, reach: i32) -> Option<f32> {
        let chute = |r: i32| self.g.in_bounds(x, r) && self.g.get(x as usize, r as usize).has(CHUTE);
        (0..=reach).flat_map(|d| [y + d, y - d]).find(|&r| chute(r) && !chute(r - 1)).map(|r| r as f32)
    }
    /// The slope under an item at column x on the chute cell in `row` (rows down per cell to the right; + = it
    /// falls away to the right), like a cart reads its track: from the chute's top up to a metre either
    /// side (1 m: a chute laid in steps reads as its run), the nearest column toward the item that is chute (so
    /// the rock a chute is laid on, or the drop past its end, never counts).
    pub fn chute_slope(&self, x: f32, row: usize) -> f32 {
        let l = CPM as i32; let (r, xc) = (row as i32, x.floor() as i32);
        let side = |dir: i32| (1..=l).rev().map(|d| xc + dir * d).find_map(|c| self.chute_top(c, r, l).map(|t| (c as f32, t)));
        let mid = self.chute_top(xc, r, l).unwrap_or(r as f32);
        let (a, b) = (side(-1).unwrap_or((xc as f32, mid)), side(1).unwrap_or((xc as f32, mid)));
        if b.0 > a.0 { (b.1 - a.1) / (b.0 - a.0) } else { 0. }
    }
    /// An item lying on a chute: gravity along its slope against its friction (less while a screen shakes).
    pub(crate) fn chute_slide(&self, d: &mut Drop, cx: usize, cy: usize, dt: f32) {
        let s = self.chute_slope(d.px, cy);
        let (sin, cos) = (s / (1. + s * s).sqrt(), 1. / (1. + s * s).sqrt());
        let g = TUNE.gravity; let mu = self.g.get(cx, cy).p().friction * if self.shaking(cx, cy) { SHAKE_GRIP } else { 1. };
        let (a, f) = (g * sin, mu * g * cos);
        if d.vx.abs() > f * dt { d.vx -= d.vx.signum() * f * dt; d.vx += a * dt; } else if a.abs() > f { d.vx += (a - a.signum() * f) * dt; } else { d.vx = 0.; }
    }

    /// Is there a bumper right in front of the cart (the way it runs), from its rim down to the track?
    pub(crate) fn bumper_ahead(&self, c: &Cart, dir: i8) -> bool {
        let front = c.x + dir as f32 * CART_W / 2.;
        let (a, b) = if dir > 0 { (front.floor() as i32, (front + 2.).floor() as i32) } else { ((front - 2.).floor() as i32, (front - 1e-3).floor() as i32) };
        let (y0, y1) = ((c.y - CART_H - 2.).floor() as i32, c.y as i32);
        (y0..=y1).any(|r| (a..=b).any(|x| self.g.in_bounds(x, r) && self.g.get(x as usize, r as usize).has(BUMPER)))
    }
    /// One tick of a tipping cart: held at the buffer, a lump over the front every `TIP_EVERY` ticks; when it's
    /// empty it rights itself (and a powered line drives it back the other way). True while still tipping.
    pub(crate) fn step_tip(&mut self, c: &mut Cart) -> bool {
        c.v = 0.; c.tip = c.tip.saturating_add(1);
        if c.load() == 0 { c.tip = 0; c.dir = -c.dir; return false; }
        if c.tip % TIP_EVERY != 0 { return true; }
        let dir = c.dir as f32;
        // out over the front rim (clear of the cart, so it isn't caught again), as low as there is room, up to 1 m above it
        let sx = c.x + dir * (CART_W / 2. + ITEM_HW + 0.1);
        let it = c.cargo[0].0;
        let Some(sy) = (0..CPM as i32).map(|k| c.y - CART_H - ITEM_HW - 0.5 - k as f32).find(|&sy| !self.item_box_hits(sx, sy, it)) else {
            // nowhere for it to go: the load is jammed in the cart
            if c.tip == TIP_EVERY { self.say("The cart's load is jammed against the buffer.".into()); }
            c.tip = 0; c.dir = -c.dir; return false;
        };
        let c0 = &mut c.cargo[0]; c0.1 -= 1; if c0.1 == 0 { c.cargo.remove(0); }
        let mut d = Drop::at(sx, sy, it, 1); d.vx = dir * TIP_V; d.vy = -TIP_UP; d.hold = 20;
        self.drops.push(d);
        if c.tip == TIP_EVERY { self.events.push(Event::Click); }
        true
    }
    pub(crate) fn item_box_hits_pub(&self, px: f32, py: f32, it: Item) -> bool { self.item_box_hits(px, py, it) }
    fn item_box_hits(&self, px: f32, py: f32, it: Item) -> bool {
        let (x0, x1) = ((px - ITEM_HW).floor() as i32, (px + ITEM_HW - 1e-3).floor() as i32);
        let (y0, y1) = ((py - ITEM_HW).floor() as i32, (py + ITEM_HW - 1e-3).floor() as i32);
        (y0..=y1).any(|y| (x0..=x1).any(|x| !self.g.in_bounds(x, y) || !self.item_passes(x as usize, y as usize, it)))
    }

    /// Set an ore hopper down over (x, y): its legs on the floor under the cursor, its bin above a cart's height.
    pub(crate) fn place_bin(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let half = BIN_W / 2;
        if x < half + 2 || x + half + 2 >= self.g.w { return Err("too near the edge"); }
        let floor = (y..(y + 4 * CPM).min(self.g.h - 1)).find(|&r| !self.passable(x - half, r) || !self.passable(x + half - 1, r)).ok_or("needs a floor under it")?;
        if floor < BIN_LEGS + BIN_H + 2 { return Err("too near the edge"); }
        let by = floor - BIN_LEGS;
        if self.passable(x - half, floor) || self.passable(x + half - 1, floor) { return Err("its legs need a floor under both"); }
        if !(by - BIN_H..floor).all(|r| (x - half..x + half).all(|c| self.passable(c, r))) { return Err("needs 1.5 m by 1.75 m of open space"); }
        if self.tr.bins.iter().any(|b| b.x.abs_diff(x) < BIN_W && b.y.abs_diff(by) < BIN_H + BIN_LEGS) { return Err("there's an ore hopper there"); }
        self.tr.bins.push(OreHopper { x, y: by, floor, contents: vec![], held: false, open: false, feed: 0, grit: vec![] });
        self.events.push(Event::Place); Ok(())
    }
    pub fn bin_at(&self, x: usize, y: usize) -> Option<usize> { let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5); self.tr.bins.iter().position(|b| b.contains(xf, yf)) }
    /// Right mouse on an ore hopper: hold its gate open, or let it shut.
    pub(crate) fn use_bin(&mut self, x: usize, y: usize) -> bool {
        let Some(k) = self.bin_at(x, y) else { return false };
        let b = &mut self.tr.bins[k]; b.held = !b.held;
        let s = if b.held { "Gate held open: the hopper empties onto what's under it." } else { "Gate let go: it opens only for a cart under it." };
        self.say(s.into()); self.events.push(Event::Click); true
    }
    /// `E` on an ore hopper: its load into the pack; an empty one comes up itself.
    pub(crate) fn pick_up_bin(&mut self, x: usize, y: usize) -> Option<Item> {
        let k = self.bin_at(x, y)?;
        if self.tr.bins[k].load() > 0 { let mut got = vec![];
            for (it, n) in std::mem::take(&mut self.tr.bins[k].contents) { self.inv[it as usize] += n; got.push(format!("{n} {}", it.name().to_lowercase())); }
            self.say(format!("Emptied the ore hopper: {}.", got.join(", "))); return Some(Item::OreHopper); }
        self.tr.bins.remove(k); self.inv[Item::OreHopper as usize] += 1; Some(Item::OreHopper)
    }

    /// One tick of every ore hopper: does it still stand, what fell in, and the gate.
    pub(crate) fn step_bins(&mut self) {
        let mut k = 0;
        while k < self.tr.bins.len() {
            let b = self.tr.bins[k].clone(); let half = BIN_W / 2;
            // footing: both legs' floor gone and it falls, its load spilling out
            if self.passable(b.x - half, b.floor) && self.passable(b.x + half - 1, b.floor) {
                let (x0, y0, x1, y1) = b.rect(); let mut j = 0u32;
                for (it, n) in b.contents { for _ in 0..n { let f = (j % 6) as f32 / 6.; self.drops.push(Drop::at(x0 + 1. + f * (x1 - x0 - 2.), y0 + (y1 - y0) * 0.5, it, 1)); j += 1; } }
                self.drops.push(Drop::at(b.x as f32, b.y as f32 - 1., Item::OreHopper, 1));
                self.tr.bins.remove(k); self.say("An ore hopper lost its footing and fell.".into()); self.events.push(Event::Place); continue;
            }
            let mut b = b; let (x0, y0, x1, y1) = b.rect();
            // loose items in its mouth are its load (a lit charge too: it can go off in there)
            let mut j = 0;
            while j < self.drops.len() {
                let d = &self.drops[j];
                if d.fuse == 0 && d.px >= x0 && d.px < x1 && d.py >= y0 - 1. && d.py < y1 && b.load() + d.n <= BIN_LOAD { let d = self.drops.swap_remove(j); b.add(d.item, d.n); } else { j += 1; }
            }
            // sand and rubble poured in as cells: counted into whole items, the cells taken in
            for r in y0 as usize..y1 as usize { for c in x0 as usize..x1 as usize {
                let i = self.g.idx(c, r); let m = self.g.m(i);
                if !(m.kind() == Kind::Powder || m.kind() == Kind::Solid && self.g.loose(i)) || b.load() >= BIN_LOAD { continue; }
                let Some(it) = drop_of(m) else { continue };
                self.g.set(i, crate::mat::Mat::Empty);
                let e = match b.grit.iter_mut().position(|g| g.0 == it) { Some(p) => p, None => { b.grit.push((it, 0)); b.grit.len() - 1 } };
                b.grit[e].1 += 1; if b.grit[e].1 >= CELLS_PER_ITEM { b.grit[e].1 = 0; b.add(it, 1); }
            } }
            // the gate: tripped by a cart standing under the spout with room left (counting what's still falling)
            let xs = b.x as f32;
            let tripped = self.tr.carts.iter().any(|c| (c.x - xs).abs() < CART_W / 2. - ITEM_HW && c.y - CART_H >= y1 - 1. && c.y - CART_H <= y1 + CPM as f32 && c.tip == 0
                && c.load() + self.drops.iter().filter(|d| (d.px - xs).abs() < CART_W / 2. && d.py > y1 && d.py < c.y).map(|d| d.n).sum::<u32>() < CART_LOAD);
            b.open = b.held || tripped;
            if b.open && b.load() > 0 {
                b.feed += 1;
                let (sx, sy) = (xs, y1 + ITEM_HW + 0.05);
                let it = b.contents[0].0;
                if b.feed >= FEED_EVERY && !self.item_box_hits(sx, sy, it) { b.feed = 0; b.take_one(); let mut d = Drop::at(sx, sy, it, 1); d.vy = 0.5 * CPM as f32; d.hold = 20; self.drops.push(d); }
            } else { b.feed = 0; }
            self.tr.bins[k] = b; k += 1;
        }
    }
    pub(crate) fn shift_bins(&mut self, rows: i64, h: usize) {
        self.tr.bins.retain_mut(|b| { let (y, f) = (b.y as i64 - rows, b.floor as i64 - rows);
            if y < (BIN_H + 2) as i64 || f >= h as i64 - 1 { return false; } b.y = y as usize; b.floor = f as usize; true });
    }
    pub(crate) fn encode_bins(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.bins.len() as u32);
        for b in &self.tr.bins { o.u32(b.x as u32); o.u32(b.y as u32); o.u32(b.floor as u32); o.u8(b.held as u8);
            o.u32(b.contents.len() as u32); for &(it, n) in &b.contents { o.u8(it as u8); o.u32(n); } }
    }
    pub(crate) fn decode_bins(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (w, h) = (self.g.w, self.g.h);
        for _ in 0..r.len(4096)? {
            let (x, y, floor, held) = (r.u32()? as usize, r.u32()? as usize, r.u32()? as usize, r.u8()? != 0);
            let mut contents = vec![]; for _ in 0..r.len(NITEM)? { let (it, n) = (r.u8()?, r.u32()?); if (it as usize) < NITEM { contents.push((crate::craft::ITEMS[it as usize], n)); } }
            if x > BIN_W && x + BIN_W < w && y > BIN_H + 1 && floor < h - 1 && floor > y { self.tr.bins.push(OreHopper { x, y, floor, contents, held, open: false, feed: 0, grit: vec![] }); }
        }
        Ok(())
    }
}
