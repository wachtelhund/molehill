//! Procedural sites of the Deepholt: seven families of found places, not random loot rooms.
//!
//! Each site has a purpose, materials, layout variation, a depth/rarity band, and something a Delver
//! can *do* (salvage, drain, climb, excavate, harvest, ride). Real foreground (props, rails, water,
//! tubes) is ordinary cells and obeys the same physics as anything you build. Background lining comes
//! from `walls.rs` and holds no roof.
//!
//! Placement mirrors ruins: at most one site per SITE_W × SITE_H cell, from the seed. None near the
//! start burrow, none in a grotto, none overlapping a ruin or another site. Buried transport halts
//! sit next to an Ancient tube station when one is near, and never grow a second waygate.
use crate::craft::Item;
use crate::gen::{Gen, GROTTO_H, GROTTO_W, START_SAFE};
use crate::grid::hash3;
use crate::logic::Dev;
use crate::mat::{Mat, CELL_M, CPM, S};
use crate::ruins::{Kind as RuinKind, RUIN_H, RUIN_W};
use crate::templates::Thing;
use crate::walls::Wall;
use crate::world::{Farm, Furniture, World};

/// At most one site per 96 m × 48 m cell. Wider than a grotto cell, tighter than a ruin cell.
pub const SITE_W: usize = 96 * CPM;
pub const SITE_H: usize = 48 * CPM;
/// The first-hour find (`Gen::find_near_site`): its nearest cell lies NEAR_FIND.0 to .1 metres from the burrow,
/// and the whole site within NEAR_FIND.2 metres above or below it (inside the starting window).
pub const NEAR_FIND: (f32, f32, f32) = (35., 90., 56.);
/// Small test windows (the arena, most scenarios) stay site-free so existing holts don't change shape.
pub const SITE_MIN_W: usize = 160 * CPM;

/// Chance a site cell holds a site, by the depth (m) of its middle. None in the far wet up or the
/// lava deep; a handful around the holt layers and the mid-granite.
pub fn site_chance(d: f32) -> f32 {
    if d < -28. || d > 100. { 0. } else if d < 10. { 0.10 } else if d < 55. { 0.13 } else { 0.09 }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind { Mine, Holt, Pumpworks, Fissure, Fossil, Crystal, Halt }
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Mine => "abandoned mine", Kind::Holt => "abandoned holt", Kind::Pumpworks => "flooded pumpworks",
            Kind::Fissure => "fissure", Kind::Fossil => "fossil cave", Kind::Crystal => "salt hall",
            Kind::Halt => "buried cargo halt",
        }
    }
    /// Width and height (cells).
    pub fn size(self) -> (usize, usize) {
        match self {
            Kind::Mine => (72 * CPM, 22 * CPM),
            Kind::Holt => (28 * CPM, 14 * CPM),
            Kind::Pumpworks => (48 * CPM, 20 * CPM),
            Kind::Fissure => (18 * CPM, 40 * CPM),
            Kind::Fossil => (36 * CPM, 16 * CPM),
            Kind::Crystal => (28 * CPM, 16 * CPM),
            Kind::Halt => (80 * CPM, 18 * CPM),
        }
    }
    /// Which depths this family belongs in (metres, the cell's middle).
    pub fn at_depth(self, d: f32) -> bool {
        match self {
            Kind::Mine => d > 0. && d < 56.,
            Kind::Holt => d >= -32. && d < 20.,
            Kind::Pumpworks => d > -12. && d < 40.,
            Kind::Fissure => d > 8. && d < 90.,
            Kind::Fossil => d >= 20. && d < 88.,
            Kind::Crystal => d > 36. && d < 100.,
            Kind::Halt => d > 12. && d < 84.,
        }
    }
    pub fn from_u8(v: u8) -> Kind {
        [Kind::Mine, Kind::Holt, Kind::Pumpworks, Kind::Fissure, Kind::Fossil, Kind::Crystal, Kind::Halt][v as usize % 7]
    }
    pub fn from_name(s: &str) -> Option<Kind> {
        Some(match s {
            "mine" => Kind::Mine, "holt" => Kind::Holt, "pump" | "pumpworks" => Kind::Pumpworks,
            "fissure" => Kind::Fissure, "fossil" => Kind::Fossil, "crystal" | "salt" => Kind::Crystal,
            "halt" => Kind::Halt, _ => return None,
        })
    }
}

/// Abandoned mine track: the strap is this many cells thick (half a metre), on sleepers in ballast.
pub const MINE_RAIL: i64 = (CPM / 2) as i64;
/// Metres between the mine's timber sets (post and cap), so no stretch of its roof spans further.
pub const MINE_BAY: i64 = 4;
/// The mine's worked face (ore) at its far end, in half metres from the end wall.
pub const MINE_FACE: i64 = 5;

/// One site: its box, a variant nibble for layout, and the chest / Ancient-tube link if any.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub id: u32, pub kind: Kind, pub x0: usize, pub y0: i64, pub w: usize, pub h: usize,
    /// layout knobs packed from the seed (ore type, which bay collapsed, which way a halt faces, …)
    pub var: u32,
    /// a halt's wrecked tube runs off to the left
    pub left: bool,
    pub chest: Option<(usize, i64)>,
    /// an Ancient tube station this halt points at (the ruin's id)
    pub link: Option<u32>,
}

/// The layout knobs a site's `var` selects in the procedural reference layouts. Templates carry the
/// same choices as data (slots and roles picked from `var`, see `templates.rs`); the exporter sets each
/// knob on its own to cut the layout into a base and per-slot patches.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Knobs {
    /// mine: which bay (from 20 m) slumped
    pub slump: i64,
    /// mine: which run of track rusted away (of 4)
    pub rails: i64,
    /// mine: where the rock bands cross the worked face (of 8)
    pub band: i64,
    /// salt hall: where the hanging crystal clusters start (of 4, every 5 m)
    pub hang: i64,
    /// mine: the ore it followed
    pub ore: Mat,
}
impl Knobs {
    pub fn of(var: u32) -> Knobs {
        let v = var as i64;
        Knobs { slump: v % 6, rails: (v >> 2) % 4, band: v & 7, hang: v & 3, ore: if var & 3 != 3 { Mat::CopperOre } else { Mat::Coal } }
    }
}

/// What a Delver reads the first time they come within 8 m. No cards, no banners: a line in the log.
pub fn notice(kind: Kind, link: bool) -> &'static str {
    match kind {
        Kind::Mine => "An old mine: timber still holding, the rails gone to rust. Someone followed the seam and left.",
        Kind::Holt => "A holt, empty. Nests cold, the fungus still trying.",
        Kind::Pumpworks => "Pumpworks, half drowned. The sluices still know their job.",
        Kind::Fissure => "A fissure splits the rock. Old bridges hang in pieces; a rope still reaches.",
        Kind::Fossil => "Bones in the stone, bigger than any Delver. The rock grew around them.",
        Kind::Crystal => "A hall of salt and crystal. Water lies still and throws the lamps back.",
        Kind::Halt if link => "The old cargo halt meets a line of the Ancients. Their tube runs on from here.",
        Kind::Halt => "A cargo halt, wrecked. Carts on their sides, the tubes torn. The line once ran further.",
    }
}

impl Gen {
    /// The site in site cell (gx, gy), if it has one.
    ///
    /// `fill_rows` asks this once per cell; the answer depends only on the site cell, so the last
    /// `(seed, gx, gy)` is memoised and a 2 M-cell window does the expensive overlap work a handful of times.
    pub fn site_at(&self, gx: i64, gy: i64) -> Option<Site> {
        if let Some((nx, ny, s)) = self.near_site { if (nx, ny) == (gx, gy) { return Some(s); } }
        thread_local! { static LAST: std::cell::RefCell<Option<(u32, usize, i64, i64, Option<Site>)>> = std::cell::RefCell::new(None); }
        LAST.with(|slot| {
            if let Some((seed, w, x, y, s)) = *slot.borrow() {
                if seed == self.seed && w == self.biome.len() && x == gx && y == gy { return s; }
            }
            let s = self.site_at_fresh(gx, gy);
            *slot.borrow_mut() = Some((self.seed, self.biome.len(), gx, gy, s));
            s
        })
    }
    fn site_at_fresh(&self, gx: i64, gy: i64) -> Option<Site> {
        let w = self.biome.len(); if w < SITE_MIN_W { return None; }
        if gx < 0 || (gx as usize + 1) * SITE_W > w { return None; }
        let h = hash3(gx as u32, gy as u32, self.seed ^ 0x51FE);
        let sh = SITE_H as i64; let mid = gy * sh + sh / 2;
        let d = mid as f32 * CELL_M;
        if (h & 0xFFFF) as f32 / 65536. >= site_chance(d) { return None; }
        let mut ks = [Kind::Mine; 7]; let mut n = 0usize;
        for k in 0..7 { let kd = Kind::from_u8(k); if kd.at_depth(d) { ks[n] = kd; n += 1; } }
        if n == 0 { return None; }
        let kind = ks[(h as usize >> 16) % n];
        self.site_build(gx, gy, h, kind)
    }
    /// How far (m) a Delver at the burrow has to go to reach this site: from the burrow (the window's middle
    /// column, depth 0) to the nearest cell of its box.
    pub fn site_trek(&self, s: &Site) -> f32 {
        let (bx, c) = ((self.biome.len() / 2) as i64, CPM as f32);
        let dx = (s.x0 as i64 - bx).max(bx - (s.x0 + s.w) as i64).max(0) as f32 / c;
        let dy = s.y0.max(-(s.y0 + s.h as i64)).max(0) as f32 / c;
        (dx * dx + dy * dy).sqrt()
    }
    /// The first-hour find (see NEAR_FIND): a found place with a chest whose nearest cell is a real trek from
    /// the burrow but not a half-day march, inside the starting window's rows. If the seed already put one
    /// there it is left alone (None); otherwise an abandoned mine (else an abandoned holt) goes into an empty
    /// site cell by the same placement rules as any site: clear of the start, of grottos and of ruins.
    pub(crate) fn find_near_site(&self) -> Option<(i64, i64, Site)> {
        let w = self.biome.len(); if w < SITE_MIN_W { return None; }
        let ncx = (w / SITE_W) as i64; let c = CPM as i64;
        let fits = |s: &Site| { let t = self.site_trek(s); s.chest.is_some() && t >= NEAR_FIND.0 && t <= NEAR_FIND.1
            && s.y0 >= -NEAR_FIND.2 as i64 * c && s.y0 + s.h as i64 <= NEAR_FIND.2 as i64 * c };
        for gy in -2..=2 { for gx in 0..ncx { if self.site_at_fresh(gx, gy).is_some_and(|s| fits(&s)) { return None; } } }
        for kind in [Kind::Mine, Kind::Holt] {
            for k in 0..96u32 {
                let h = hash3(k, 0xF17D, self.seed ^ 0x51FE);
                let (gx, gy) = ((h % ncx as u32) as i64, ((h >> 8) % 3) as i64 - 1);
                if self.site_at_fresh(gx, gy).is_some() { continue; } // never replace a site the seed placed
                let Some(s) = self.site_build(gx, gy, h, kind) else { continue };
                let d = s.centre().1 as f32 / CPM as f32;
                if kind.at_depth(d) && fits(&s) { return Some((gx, gy, s)); }
            }
        }
        None
    }
    /// Place a site of `kind` in site cell (gx, gy) from hash `h`: None where it would come near the start
    /// burrow or overlap a grotto or a ruin.
    fn site_build(&self, gx: i64, gy: i64, h: u32, kind: Kind) -> Option<Site> {
        let w = self.biome.len();
        let (sw, sh) = (SITE_W as i64, SITE_H as i64);
        let c = CPM as i64; let r = |k: u32, n: i64| (hash3(h, k, self.seed) as i64).rem_euclid(n.max(1));
        let (bw, bh) = (kind.size().0 as i64, kind.size().1 as i64);
        let m = 5 * c;
        let x0 = gx * sw + m + r(2, (sw - bw - 2 * m).max(1));
        let y0 = gy * sh + m + r(3, (sh - bh - 2 * m).max(1));
        let safe = START_SAFE as i64 * c;
        let mid = (w / 2) as i64;
        if x0 < mid + safe && x0 + bw > mid - safe && y0 < safe && y0 + bh > -safe { return None; }
        let (gw, gh) = (GROTTO_W as i64, GROTTO_H as i64);
        for gy2 in (y0 - 6 * c).div_euclid(gh)..=(y0 + bh + 6 * c).div_euclid(gh) {
            for gx2 in ((x0 - 6 * c).max(0)).div_euclid(gw)..=(x0 + bw + 6 * c).div_euclid(gw) {
                if let Some(g) = self.grotto_at(gx2, gy2) {
                    let (gx0, gx1, gy0, gy1) = (g.cx as i64 - g.rx as i64 - 3 * c, g.cx as i64 + g.rx as i64 + 3 * c, g.cy - g.ry as i64 - 3 * c, g.cy + g.ry as i64 + 3 * c);
                    if gx1 >= x0 && gx0 <= x0 + bw && gy1 >= y0 && gy0 <= y0 + bh { return None; }
                }
            }
        }
        let (rw, rh) = (RUIN_W as i64, RUIN_H as i64);
        for rgy in (y0 - 6 * c).div_euclid(rh)..=(y0 + bh + 6 * c).div_euclid(rh) {
            for rgx in ((x0 - 6 * c).max(0)).div_euclid(rw)..=(x0 + bw + 6 * c).div_euclid(rw) {
                if let Some(ru) = self.ruin_at(rgx, rgy) {
                    let (rx0, rx1, ry0, ry1) = (ru.x0 as i64, ru.x0 as i64 + ru.w as i64, ru.y0, ru.y0 + ru.h as i64);
                    if rx1 > x0 && rx0 < x0 + bw && ry1 > y0 && ry0 < y0 + bh { return None; }
                }
            }
        }
        let mut link = None; let mut left = r(4, 2) == 0;
        if kind == Kind::Halt {
            for rgy in (y0 - 50 * c).div_euclid(rh)..=(y0 + bh + 50 * c).div_euclid(rh) {
                for rgx in ((x0 - 50 * c).max(0)).div_euclid(rw)..=(x0 + bw + 50 * c).div_euclid(rw) {
                    if let Some(ru) = self.ruin_at(rgx, rgy) {
                        if ru.kind != RuinKind::TubeStation { continue; }
                        let (rx0, rx1, ry0, ry1) = (ru.x0 as i64, ru.x0 as i64 + ru.w as i64, ru.y0, ru.y0 + ru.h as i64);
                        let (cx, cy) = ((x0 + bw / 2 - (rx0 + rx1) / 2).abs(), (y0 + bh / 2 - (ry0 + ry1) / 2).abs());
                        if cx < 50 * c && cy < 40 * c { link = Some(ru.id); left = (rx0 + rx1) / 2 < x0 + bw / 2; }
                    }
                }
            }
        }
        Some(Site::new(h, kind, x0 as usize, y0, r(5, 1 << 20) as u32, left, link))
    }
    pub fn site_of(&self, x: usize, wy: i64) -> Option<Site> { self.site_at((x / SITE_W) as i64, wy.div_euclid(SITE_H as i64)) }
    pub fn sites_in_rows(&self, wy0: i64, wy1: i64) -> Vec<Site> {
        let sh = SITE_H as i64; let mut v = vec![];
        if self.biome.len() < SITE_MIN_W { return v; }
        for gy in wy0.div_euclid(sh)..=(wy1 - 1).div_euclid(sh) {
            for gx in 0..(self.biome.len() / SITE_W) as i64 {
                if let Some(s) = self.site_at(gx, gy) { if s.y0 < wy1 && s.y0 + s.h as i64 > wy0 { v.push(s); } }
            }
        }
        v
    }
    /// What a site puts at (x, wy): Some((material, flags)) inside its carved cells, None to leave the host.
    pub fn site_cell(&self, x: usize, wy: i64) -> Option<(u8, u8)> {
        self.site_of(x, wy).and_then(|s| s.cell(x, wy)).map(|m| (m as u8, 0))
    }
    pub fn site_wall(&self, x: usize, wy: i64) -> Option<Wall> { self.site_of(x, wy).and_then(|s| s.wall(x, wy)) }
    /// Open air or water a site carved (stabilisation must not fill it).
    pub fn in_site_void(&self, x: usize, wy: i64) -> bool {
        self.site_of(x, wy).and_then(|s| s.cell(x, wy)).is_some_and(|m| matches!(m, Mat::Empty | Mat::Water | Mat::Rail | Mat::Tube))
    }
}

impl Site {
    pub fn new(id: u32, kind: Kind, x0: usize, y0: i64, var: u32, left: bool, link: Option<u32>) -> Site {
        let (w, h) = kind.size();
        let mut s = Site { id, kind, x0, y0, w, h, var, left, chest: None, link };
        s.chest = crate::templates::get(kind).chest(var, left).map(|(cx, cy)| (x0 + cx, y0 + cy as i64));
        s
    }
    /// The procedural reference chest (see `cell_reference`), as an offset from the box's top-left.
    #[doc(hidden)]
    pub fn chest_reference(kind: Kind, left: bool) -> Option<(usize, usize)> {
        let (w, h) = kind.size(); let (x0, y0) = (0usize, 0i64);
        let mut s = Site { id: 0, kind, x0, y0, w, h, var: 0, left, chest: None, link: None };
        let c = CPM as i64;
        // chests stand in open air just above the floor so check_chests can open them
        let cy = y0 + h as i64 - 2 * c + c / 2 - 1;
        s.chest = match kind {
            Kind::Mine => Some((x0 + w - 8 * CPM, cy)),
            Kind::Holt => Some((x0 + 10 * CPM, cy)), // hall floor between the table and the hearth
            Kind::Pumpworks => Some((x0 + 32 * CPM, cy)), // dry hall, west of the wheel
            Kind::Fossil => Some((x0 + 6 * CPM + CPM / 2, cy)), // between the shelf and the table
            Kind::Crystal => Some((x0 + w / 2 + 4 * CPM, cy)), // between the centre and east spires
            Kind::Halt => Some((x0 + if left { w - 5 * CPM } else { 5 * CPM }, cy)), // warehouse aisle
            Kind::Fissure => None,
        };
        s.chest.map(|(x, y)| (x, y as usize))
    }
    /// The ore an abandoned mine followed: copper three times in four (it's the shallow ore), else coal.
    pub fn mine_ore(&self) -> Mat { Knobs::of(self.var).ore }
    pub fn centre(&self) -> (usize, i64) { (self.x0 + self.w / 2, self.y0 + self.h as i64 / 2) }
    /// Material this site stamps at (x, wy), or None to leave the host rock: read from its template
    /// (`templates.rs`, the data under `assets/sites/`).
    pub fn cell(&self, x: usize, wy: i64) -> Option<Mat> {
        let (dx, dy) = (x as i64 - self.x0 as i64, wy - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 { return None; }
        crate::templates::get(self.kind).cell(dx as usize, dy as usize, self.var, self.left)
    }
    /// The procedural reference layout the templates were exported from (`examples/export_sites.rs`),
    /// with explicit layout knobs. The golden test `templates_match_the_procedural_sites` keeps the two equal.
    #[doc(hidden)]
    pub fn cell_reference(&self, x: usize, wy: i64, k: &Knobs) -> Option<Mat> {
        let (dx, dy) = (x as i64 - self.x0 as i64, wy - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 { return None; }
        let c = CPM as i64; let (w, h) = (self.w as i64, self.h as i64);
        match self.kind {
            Kind::Mine => self.carve_mine(k, dx, dy, w, h, c),
            Kind::Holt => self.carve_holt(k, dx, dy, w, h, c),
            Kind::Pumpworks => self.carve_pump(k, dx, dy, w, h, c),
            Kind::Fissure => self.carve_fissure(k, dx, dy, w, h, c),
            Kind::Fossil => self.carve_fossil(k, dx, dy, w, h, c),
            Kind::Crystal => self.carve_crystal(k, dx, dy, w, h, c),
            Kind::Halt => self.carve_halt(k, dx, dy, w, h, c),
        }
    }
    /// Two-level timbered mine: haulage drift, upper heading, a shaft house with a headframe and
    /// winch silhouetted against host rock, ore chutes, collapsed timbering, and two adits. Posts
    /// are short sets under the beams (every `MINE_BAY` metres) so the roof stands without a colonnade.
    /// Geometry shared by the fossil cave and the salt hall: a wide oval whose floor
    /// chord meets both mouths, so the nave is one walk from either way in.
    fn oval_nav(w: i64, h: i64) -> (f32, f32, f32, f32) {
        (w as f32 * 0.5, h as f32 * 0.66, w as f32 * 0.50, h as f32 * 0.50)
    }
    /// A 3 m doorway through the shell under a 1 m iron lintel and a 2 m limestone
    /// cap, keyed into the ring so the mouth does not sag onto a mole standing in it.
    fn oval_mouth(dx: i64, dy: i64, w: i64, h: i64, c: i64, cx: f32, cy: f32, rx: f32, ry: f32, r2: f32) -> Option<Mat> {
        use Mat::*;
        let floor = h - c;
        if dy < floor - 6 * c || dy >= floor { return None; }
        let ny_f = (floor as f32 - cy) / ry;
        let half_f = if ny_f.abs() >= 1.0 { 0.0 } else { rx * (1.0 - ny_f * ny_f).sqrt() };
        let left_in = (cx - half_f).round() as i64 + c / 2;
        let right_in = (cx + half_f).round() as i64 - c / 2;
        let in_left = dx < left_in.max(4 * c);
        let in_right = dx >= right_in.min(w - 4 * c);
        if !(in_left || in_right) { return None; }
        if dy >= floor - 3 * c { return if r2 > 1.0 { Some(Empty) } else { None }; }
        if dy >= floor - 4 * c { return Some(IronBeam); }
        Some(Limestone)
    }
    fn carve_mine(&self, k: &Knobs, dx: i64, dy: i64, w: i64, h: i64, c: i64) -> Option<Mat> {
        use Mat::*;
        let ore = k.ore;
        let bay = 20 * c + k.slump * c;
        let floor = h - c;
        let face = MINE_FACE * c / 2;
        let post = c / 2;
        let uf = 9 * c;
        let shaft0 = 48 * c;
        let shaft1 = shaft0 + 2 * c;
        let hx = (shaft0 + shaft1) / 2;
        let in_shaft = dx >= shaft0 && dx < shaft1;
        let shaft_house = dx >= shaft0 - 6 * c && dx < shaft1 + 6 * c;
        let slump = dx >= bay - c && dx < bay + 7 * c;
        let slump_k = if slump { (dx - (bay - c)).clamp(0, 6 * c) * 2 * c / (6 * c) } else { 0 };
        // open hoistway to the underside of the floor (the floor band stays rock for the track test)
        if in_shaft && dy < floor { return Some(Empty); }
        // timber A-frame around the hoist: a tall headframe peeking through the drift
        // roof into host rock, legs landing on the upper deck so nothing floats
        if shaft_house && !in_shaft {
            let dist = (dx - hx).abs();
            let cap0 = 2 * c;
            let tall = cap0 + 4 * c;
            if dy <= tall {
                let exp = dy * 5 * c / tall.max(1);
                if dist > c + c / 4 && (dist - exp).abs() < c / 2 { return Some(WoodBeam); }
            }
            // legs stop 3 m above the upper deck so the heading stays open onto the hoist
            if (dist - 5 * c).abs() < c / 2 && dy >= cap0 && dy < uf - 3 * c {
                return Some(WoodBeam);
            }
            let (wx, wy, rr) = (hx + 3 * c, cap0 + c, 3 * c / 2);
            let d2 = (dx - wx) * (dx - wx) + (dy - wy) * (dy - wy);
            if d2 <= (rr + c / 4) * (rr + c / 4) && d2 >= (rr - c / 2) * (rr - c / 2) { return Some(IronBeam); }
            if (dy - wy).abs() < c / 4 && (dx - wx).abs() < rr { return Some(IronBeam); }
        }
        let cap = 2 * c + slump_k;
        if dy < cap {
            return if dy < cap - c { None } else { Some(if slump && dy >= cap - c / 2 { Gravel } else { Limestone }) };
        }
        if dy >= floor {
            let fy = dy - floor; let gone = (dx / (3 * c) + k.rails) % 4 == 3;
            return Some(if fy < MINE_RAIL && !gone && dx >= face && !in_shaft { Rail }
                else if fy < MINE_RAIL + 3 && dx >= face && dx % c < 2 { WoodBeam }
                else if fy < MINE_RAIL + 3 && dx >= face { Gravel }
                else { Rock });
        }
        let chute = {
            let span = (floor - uf).max(1);
            let t = (dy - uf) as f32 / span as f32;
            t >= 0. && t <= 1. && (dx - (22 * c + (t * 4. * c as f32) as i64)).abs() < 3 * c / 2
        };
        if dy >= uf && dy < uf + c && !in_shaft && !chute && !slump {
            return Some(if (dx / c) % 5 == 2 { WoodBeam } else { Planks });
        }
        if dx >= w - c / 2 {
            let lower = dy >= floor - 3 * c && dy < floor;
            let upper = dy >= uf - 3 * c && dy < uf;
            return Some(if lower || upper { Empty } else { Rock });
        }
        let xf = face - (floor - dy).max(0) * c / (floor - c / 2).max(1);
        if dx < xf {
            let band = (dy + dx / 3 + k.band) % (2 * c) < 2;
            return Some(if band { Rock } else { ore });
        }
        let spill = (3 * c / 2) - (dx - face) * (3 * c / 2) / (5 * c / 2);
        if dx >= face - c && spill > 0 && dy >= floor - spill { return Some(ore); }
        if chute {
            if (dx + dy) % (3 * c) < 2 { return Some(ore); }
            return Some(Empty);
        }
        if slump && dy >= cap && dy < floor && dx >= bay && dx < bay + 2 * c {
            if dy >= floor - 2 * c { return Some(if (dx + dy) % (2 * c) < c { Gravel } else { WoodBeam }); }
        }
        if dx >= 14 * c && dx < 16 * c && dy >= floor - c && dy < floor {
            return Some(if (dx + dy) % c < c / 2 { ore } else { Rock });
        }
        if dx >= 25 * c && dx < 26 * c && dy >= floor - c && dy < floor { return Some(WoodBeam); }
        if dx >= 36 * c && dx < 39 * c && dy >= uf - 2 * c && dy < uf {
            return Some(if dy >= uf - c { Planks } else { WoodBeam });
        }
        // wrecked cart bodies sitting on the rails (wheels above the floor band the track test owns)
        if dx >= 40 * c && dx < 45 * c && dy >= floor - 5 * c / 2 && dy < floor {
            if dy >= floor - 2 * c && dy < floor && dx > 40 * c + c && dx < 45 * c - c { return Some(Empty); }
            if ((dx - 41 * c).abs() < c / 2 || (dx - 44 * c).abs() < c / 2) && dy >= floor - c { return Some(IronBeam); }
            let edge = dx < 40 * c + c / 3 || dx >= 45 * c - c / 3 || dy < floor - 5 * c / 2 + c / 3 || dy >= floor - c / 3;
            return Some(if edge { IronBeam } else { Planks });
        }
        if dx >= 54 * c && dx < 59 * c && dy >= floor - 5 * c / 2 && dy < floor {
            if dy >= floor - 2 * c && dy < floor && dx > 54 * c + c && dx < 59 * c - c { return Some(Empty); }
            if ((dx - 55 * c).abs() < c / 2 || (dx - 58 * c).abs() < c / 2) && dy >= floor - c { return Some(IronBeam); }
            let edge = dx < 54 * c + c / 3 || dx >= 59 * c - c / 3 || dy < floor - 5 * c / 2 + c / 3 || dy >= floor - c / 3;
            return Some(if edge { IronBeam } else { Planks });
        }
        let q = (dx - face - c).rem_euclid(MINE_BAY * c);
        let under_roof = dy >= cap && dy < cap + 2 * c;
        let under_deck = dy >= uf + c && dy < uf + 3 * c;
        if dx >= face && q < post && (under_roof || under_deck) { return Some(WoodBeam); }
        if dx >= face - c / 2 && dy >= cap && dy < cap + 2 && (q < post + c / 2 || q >= MINE_BAY * c - c / 2) { return Some(WoodBeam); }
        if dx >= face && dy >= uf - 2 && dy < uf && (q < post + c / 2 || q >= MINE_BAY * c - c / 2) && !in_shaft {
            return Some(WoodBeam);
        }
        Some(Empty)
    }
    /// A lived-in holt: nesting alcove, kitchen hearth and chimney, hall, a storage loft and a
    /// fungus garden. The dome leaves host rock above it, so the silhouette is a house, not a box.
    fn carve_holt(&self, _k: &Knobs, dx: i64, dy: i64, w: i64, h: i64, c: i64) -> Option<Mat> {
        use Mat::*;
        let t = (dx as f32 - w as f32 / 2.) / (w as f32 / 2.);
        let top = (2.2 * c as f32 + 3.0 * c as f32 * t * t).round() as i64;
        let floor = h - c;
        let nest_end = 8 * c;
        let hall_end = 18 * c;
        let doorway = dy >= floor - 2 * c && dy < floor;
        let chimney = (dx - 13 * c).abs() < c;
        if chimney && dy < top + 2 * c {
            if dy < top { return Some(StoneBrick); }
        }
        if dy < top { return None; }
        if dy < top + 2 * c { return Some(Limestone); }
        if dy >= floor { return Some(if dx >= hall_end { Clay } else { Planks }); }
        if dx < c / 2 { return Some(if doorway { Empty } else { Limestone }); }
        let right = w - c / 2 - (((dy / c) as f32 * 0.5).sin() * (0.5 * c as f32)) as i64;
        if dx >= right { return Some(if doorway { Empty } else { Limestone }); }
        if (dx >= nest_end && dx < nest_end + c / 2) || (dx >= hall_end && dx < hall_end + c / 2) {
            return Some(if doorway || (dx >= hall_end && dy >= 6 * c && dy < 8 * c) { Empty } else { Planks });
        }
        // kitchen hearth (between the table and the loft), tiled firebox open, chimney above
        // kitchen hearth: a tiled mantel and thin jambs, open at walking height so the
        // hall is one walk from the nest through to the garden (a solid 3 m block would seal it)
        if dx >= 11 * c && dx < 13 * c && dy >= floor - 3 * c && dy < floor {
            if dy < floor - 2 * c { return Some(Tiles); }
            if dx < 11 * c + c / 4 || dx >= 13 * c - c / 4 { return Some(StoneBrick); }
            return Some(Empty);
        }
        // storage loft over the inner hall
        if dx >= 14 * c && dx < hall_end && dy >= 6 * c && dy < 6 * c + c / 2 { return Some(Planks); }
        if dx >= 14 * c && dx < 14 * c + c / 2 && dy >= 6 * c && dy < floor - 3 * c { return Some(WoodBeam); }
        if dx >= 14 * c && dx < 16 * c && dy >= 6 * c - 2 * c && dy < 6 * c {
            return Some(if (dx / (c / 2)) % 2 == 0 { Planks } else { WoodBeam });
        }
        // bedding beside the nest sprites
        if dy >= floor - c && dy < floor && dx > c && (dx < 3 * c - 10 || (dx >= 5 * c + 10 && dx < nest_end - c / 2)) {
            return Some(if (dx / (c / 2) + dy / (c / 2)) % 3 == 0 { Fungus } else { Dirt });
        }
        // fungus garden clumps that leave farm and planter footprints open
        if dx >= hall_end + c && dx < w - c {
            let col = dx / c;
            if (col == 20 || col == 22 || col == 26) && dy >= floor - 2 * c && dy < floor {
                return Some(if col == 22 && dy >= floor - c { Clay } else { Fungus });
            }
        }
        // plate-rail and drying herbs under the roof, well above walking height
        if dy >= top + 2 * c && dy < top + 2 * c + c / 4 && dx > c && dx < w - c {
            return Some(WoodBeam);
        }
        if dy >= top + 2 * c && dy < top + 3 * c && dx > c && dx < w - c
            && !(dx >= nest_end && dx < nest_end + c / 2) && !(dx >= hall_end && dx < hall_end + c / 2)
            && (dx / (c / 2)) % 5 == 1
        {
            return Some(Roots);
        }
        Some(Empty)
    }
    /// Brick pumpworks: canal and flooded intake, a closed sluice, a dry machine hall, a big
    /// pump wheel sitting on the floor, pipe runs hung from the roof, and a service door.
    /// No full-height colonnade: the rooms read as one works, not a grid of boxes.
    fn carve_pump(&self, _k: &Knobs, dx: i64, dy: i64, w: i64, h: i64, c: i64) -> Option<Mat> {
        use Mat::*;
        let floor = h - c;
        let split = 20 * c;
        let pump_wall = 34 * c;
        let doorway = dy >= floor - 2 * c && dy < floor;
        let water_line = h - 3 * c;
        let cap = if dx >= pump_wall {
            2 * c
        } else if dx >= 6 * c && dx < 13 * c {
            3 * c + c / 2
        } else if dx < split {
            3 * c
        } else {
            2 * c
        };
        if dy < cap - c { return None; }
        if dy < cap { return Some(StoneBrick); }
        if dy >= floor { return Some(StoneBrick); }
        if dx < c / 2 { return Some(if dy >= water_line && dy < floor { Empty } else { StoneBrick }); }
        if dx >= w - c / 2 { return Some(if doorway { Empty } else { StoneBrick }); }
        if dx >= split && dx < split + c {
            return Some(if dy >= h - 3 * c && dy < h - 2 * c { Door }
                else if dy >= h - 2 * c && dy < h - c { Clay }
                else if dy >= 8 * c && dy < 10 * c { Empty } // service passage above the held sump
                else { StoneBrick });
        }
        // partition onto the wheel: a door and a viewing window, not a blank wall
        if dx >= pump_wall && dx < pump_wall + c / 2 {
            if doorway { return Some(Empty); }
            if dy >= 7 * c && dy < 12 * c { return Some(Empty); }
            return Some(StoneBrick);
        }
        // pipes hung from the brick cap so they cannot fall
        if dy >= cap && dy < cap + c / 2 && dx > c && dx < w - c && (dx < split || dx > split + c) {
            return Some(IronBeam);
        }
        if (dx - 10 * c).abs() < c / 3 && dy >= cap && dy < water_line { return Some(IronBeam); }
        if (dx - 26 * c).abs() < c / 3 && dy >= cap && dy < floor - 3 * c { return Some(IronBeam); }
        // a collapsed timber still keyed into the intake roof
        if (dx - 8 * c).abs() < c / 3 && dy >= cap && dy < cap + 6 * c { return Some(WoodBeam); }
        if dx < split {
            if dy >= water_line { return Some(Water); }
            return Some(Empty);
        }
        if dx >= split + 2 * c && dx < split + 5 * c && dy >= floor - c && dy < floor { return Some(Planks); }
        if dx >= split + 7 * c && dx < split + 10 * c && dy >= floor - 2 * c && dy < floor {
            return Some(if (dx / (c / 2)) % 2 == 0 { Planks } else { WoodBeam });
        }
        // pump wheel: 10 m iron wheel sitting on the pump-room floor, fat rim and spokes
        if dx > pump_wall + c {
            let rr = 5 * c;
            let (wx, wy) = (pump_wall + 7 * c, floor - rr);
            let ax = dx - wx; let ay = dy - wy;
            let d2 = ax * ax + ay * ay;
            let inner = rr - c; let outer = rr + c / 4;
            if d2 >= inner * inner && d2 <= outer * outer { return Some(IronBeam); }
            if d2 <= outer * outer && (ay.abs() < c / 2 || ax.abs() < c / 2 || (ax - ay).abs() < c / 2 || (ax + ay).abs() < c / 2) {
                return Some(IronBeam);
            }
            if d2 <= c * c { return Some(IronBeam); }
            if (dx - wx).abs() < c / 3 && dy >= wy && dy < floor { return Some(IronBeam); }
            if dx >= pump_wall + 2 * c && dx < pump_wall + 5 * c && dy >= floor - 4 * c && dy < floor {
                let shell = dx < pump_wall + 2 * c + c / 2 || dx >= pump_wall + 5 * c - c / 2 || dy < floor - 4 * c + c / 2;
                return Some(if shell { IronBeam } else { Empty });
            }
        }
        Some(Empty)
    }
    /// A tall split in the host rock that widens mid-drop, with broken bridges, hanging ropes,
    /// rubble-strewn ledges and a mouth on each side.
    fn carve_fissure(&self, _k: &Knobs, dx: i64, dy: i64, w: i64, h: i64, c: i64) -> Option<Mat> {
        use Mat::*;
        let wave = ((dy as f32 / c as f32) * 0.55).sin() * (1.4 * c as f32);
        let mid = 1. - ((dy as f32 / h as f32) - 0.45).abs() * 1.6;
        let extra = (mid.max(0.) * 1.8 * c as f32) as i64;
        let left = 2 * c + wave as i64 - extra;
        let right = w - 2 * c + (wave * 0.45) as i64 + extra;
        let floor = h - c;
        let west_mouth = dy >= 5 * c && dy < 8 * c;
        let east_mouth = dy >= 21 * c && dy < 24 * c;
        if west_mouth && dx < left + 3 * c { return Some(Empty); }
        if east_mouth && dx >= right - 3 * c { return Some(Empty); }
        if dx < left || dx >= right { return None; }
        let ledge = |at: i64| dy >= at - c / 2 && dy < at + c / 2;
        let span = |gap: i64| (dx - (left + right) / 2).abs() > gap;
        let west_rope = left + 2 * c;
        let east_rope = right - 2 * c;
        let mid_rope = (left + right) / 2;
        // decks stop half a metre short of the wall so a climber can pass the lip
        let inset = dx >= left + c / 2 && dx < right - c / 2;
        if inset && ledge(8 * c) { return Some(if dx < left + 5 * c { WoodBeam } else if dx >= right - 4 * c { IronBeam } else if span(c) { WoodBeam } else { Empty }); }
        if inset && ledge(16 * c) { return Some(if span(2 * c) { WoodBeam } else { Empty }); }
        if inset && ledge(24 * c) { return Some(if dx < left + 6 * c { WoodBeam } else if dx >= right - 3 * c { IronBeam } else if span(c) { IronBeam } else { Empty }); }
        if inset && ledge(32 * c) { return Some(if dx >= (left + right) / 2 { WoodBeam } else { Empty }); }
        // small rubble against the wall, leaving a walkway along the deck
        if dy >= 8 * c - c / 2 && dy < 8 * c && dx > left + c && dx < left + 2 * c { return Some(Gravel); }
        if dy >= 24 * c - c / 2 && dy < 24 * c && dx > right - 2 * c && dx < right - c { return Some(Rock); }
        // rope segments hang in the voids and stop 2 m above the next deck, so a mole
        // can walk each deck past them and drop through the designed gaps
        let rope_at = |x: i64| (dx - x).abs() < c / 3;
        let hanging = |from: i64, to: i64| dy >= from + c / 2 && dy < to - 2 * c;
        if (rope_at(west_rope) || rope_at(east_rope) || rope_at(mid_rope))
            && (hanging(0, 8 * c) || hanging(8 * c, 16 * c) || hanging(16 * c, 24 * c)
                || hanging(24 * c, 32 * c) || hanging(32 * c, floor))
        {
            return Some(if rope_at(west_rope) { WoodBeam } else { Roots });
        }
        if dy < 6 * c && (dx / c) % 5 == 2 { return Some(Roots); }
        if dy >= floor { return Some(if dx % c < c / 2 { Gravel } else { Rock }); }
        if dy >= floor - c && (dx + dy) % (3 * c) < c * 2 { return Some(Water); }
        Some(Empty)
    }
    /// Oval excavation around a readable mineralised skeleton, wooden scaffold and tools, two
    /// mouths. Limestone follows the oval as a shell; the bounding-box corners stay host rock.
    fn carve_fossil(&self, _k: &Knobs, dx: i64, dy: i64, w: i64, h: i64, c: i64) -> Option<Mat> {
        use Mat::*;
        let (cx, cy, rx, ry) = Self::oval_nav(w, h);
        let nx = (dx as f32 - cx) / rx; let ny = (dy as f32 - cy) / ry;
        let floor = h - c;
        let r2 = nx * nx + ny * ny;
        // scaffold posts stop 3 m above the floor so a mole can walk through each bay
        let post = |x: i64, y0: i64, y1: i64| dx >= x && dx < x + c / 2 && dy >= y0 && dy < y1.min(floor - 3 * c);
        let bone_gap = (dx - 15 * c).abs() < c || (dx - 25 * c).abs() < c;
        if post(3 * c, 2 * c, floor) || post(8 * c, 3 * c, floor) || post(12 * c, 4 * c, 9 * c) { return Some(WoodBeam); }
        if let Some(m) = Self::oval_mouth(dx, dy, w, h, c, cx, cy, rx, ry, r2) { return Some(m); }
        if r2 > 1. {
            if r2 < 1.72 { return Some(Limestone); }
            return None;
        }
        let walk = (dy >= 5 * c && dy < 5 * c + c / 2 && dx >= 3 * c && dx < 14 * c)
            || (dy >= 8 * c && dy < 8 * c + c / 2 && dx >= 3 * c && dx < 12 * c);
        let hatch = (dx - 3 * c).abs() < c || (dx - 8 * c).abs() < c || (dx - 12 * c).abs() < c;
        if walk && !hatch { return Some(Planks); }
        // profile skull facing left (toward the ribs), so it reads as a lying beast, not a face
        let (sx, sy) = (w - 8 * c, h - 7 * c);
        let skx = (dx - sx) as f32 / (4.2 * c as f32);
        let sky = (dy - sy) as f32 / (3.4 * c as f32);
        let in_cranium = skx * skx + sky * sky < 1. && dy < floor;
        let in_snout = dx < sx && dx > sx - 5 * c
            && dy >= sy - c && dy < sy + 2 * c
            && (sx - dx) < 5 * c - (dy - (sy - c)) * 2;
        if !bone_gap && (in_cranium || in_snout) && dy < floor - 3 * c {
            let exn = (dx - (sx - c)) as f32 / (1.3 * c as f32);
            let eyn = (dy - (sy - c / 2)) as f32 / (1.0 * c as f32);
            if exn * exn + eyn * eyn < 1. { return Some(Empty); }
            return Some(Carved);
        }
        if !bone_gap && dy < floor - 3 * c && dx >= sx - 5 * c && dx < sx + c && dy >= sy + c && dy < sy + 2 * c + c / 3 {
            let tooth = ((sx - dx) / (c / 2).max(1)) % 2 == 0;
            if tooth && dy >= sy + 2 * c - c / 4 { return Some(Empty); }
            return Some(Carved);
        }
        let spine_y = h - 4 * c - ((dx - w / 2).abs() * c) / (w / 2).max(1);
        if !bone_gap && dy < floor - 3 * c && dx > 7 * c && dx < sx - 3 * c && (dy - spine_y).abs() < c { return Some(Carved); }
        let rib = (0..8).any(|k| {
            let px = 8 * c + k * 2 * c + c / 2;
            if px >= sx - 4 * c { return false; }
            let rise = 5 * c - (k as i64 - 3).abs() * c / 2;
            let yo = spine_y - dy;
            yo > 0 && yo < rise && (dx - px - yo / 4).abs() < c / 2
        });
        if !bone_gap && rib && dy < floor - 3 * c { return Some(Carved); }
        let limb = (dx > 9 * c && dx < 16 * c && (dy - (spine_y + (dx - 9 * c) / 2)).abs() < c && dy > spine_y)
            || (dx > 17 * c && dx < 24 * c && (dy - (spine_y + (dx - 17 * c) / 3)).abs() < c && dy > spine_y);
        if !bone_gap && limb && dy < floor - 3 * c { return Some(Carved); }
        if dy < 3 * c && (dx / c) % 3 == 1 { return Some(Roots); }
        // tools away from the shelf (5 c) and table (8 c) footprints
        if dx >= 20 * c && dx < 21 * c && dy >= floor - c && dy < floor { return Some(IronBeam); }
        if dy >= floor { return Some(Rock); }
        Some(Empty)
    }
    /// Salt cathedral: an oval nave (shell, not a box) with towering crystal spires, hanging
    /// clusters, quartz columns and three glow-pools.
    fn carve_crystal(&self, k: &Knobs, dx: i64, dy: i64, w: i64, h: i64, c: i64) -> Option<Mat> {
        use Mat::*;
        let (cx, cy, rx, ry) = Self::oval_nav(w, h);
        let nx = (dx as f32 - cx) / rx; let ny = (dy as f32 - cy) / ry;
        let floor = h - c;
        let r2 = nx * nx + ny * ny;
        let col = (dx - 7 * c).abs() < c / 3 || (dx - (w - 7 * c)).abs() < c / 3
            || (dx - w / 2).abs() < c / 3 || (dx - 10 * c).abs() < c / 3;
        if let Some(m) = Self::oval_mouth(dx, dy, w, h, c, cx, cy, rx, ry, r2) { return Some(m); }
        // columns hang from the vault to 3 m above the floor: a walk, not sealed strips
        if col && dy < floor - 3 * c && r2 <= 1.05 { return Some(Quartz); }
        if r2 > 1. {
            if r2 < 1.72 { return Some(Limestone); }
            return None;
        }
        let spire = |px: i64, height: i64, base: i64, mat: Mat| {
            let yh = floor - dy;
            if yh <= 0 || yh >= height { return None; }
            let half = base * (height - yh) / height / 2 + c / 4;
            // walk under the whole base: any floor-sitting cone would seal the nave in 2d
            if yh < 3 * c { return None; }
            // neck the last 2 m down to a 1 m stem so the underside is not a wide unsupported beam
            if yh < 5 * c && (dx - px).abs() >= c / 2 { return None; }
            if (dx - px).abs() < half { Some(mat) } else { None }
        };
        if let Some(m) = spire(w / 2, 11 * c, 5 * c, Crystal) { return Some(m); }
        if let Some(m) = spire(6 * c, 8 * c, 4 * c, Quartz) { return Some(m); }
        if let Some(m) = spire(w - 6 * c, 9 * c, 4 * c, Crystal) { return Some(m); }
        if let Some(m) = spire(10 * c, 6 * c, 3 * c, Crystal) { return Some(m); }
        // shallow glow-pools between the spires, plus the floor basins
        if dy >= floor - c / 2 && dy < floor
            && ((dx - 8 * c).abs() < 2 * c || (dx - (w / 2 + 4 * c)).abs() < 2 * c || (dx - (w - 8 * c)).abs() < c)
        {
            return Some(Water);
        }
        let pool = dy >= floor && ((dx - w / 2).abs() < 4 * c || (dx - 6 * c).abs() < 2 * c || (dx - (w - 6 * c)).abs() < 2 * c);
        let hang = dy < 4 * c && dy >= c && (dx / c + k.hang).rem_euclid(5) == 0;
        if pool { Some(Water) }
        else if hang { Some(Crystal) }
        else if dy >= floor { Some(Quartz) }
        else { Some(Empty) }
    }
    /// Chunk-scale cargo halt: vaulted iron hall on rails, a derelict railcar with a cabin and
    /// stack, waiting benches and a hanging sign, a pressure tube off one side, and a lower-roofed
    /// warehouse of crates off the other. Never a waygate.
    fn carve_halt(&self, _k: &Knobs, dx: i64, dy: i64, w: i64, h: i64, c: i64) -> Option<Mat> {
        use Mat::*;
        let floor = h - c;
        let hall0 = 26 * c;
        let hall1 = 56 * c;
        let in_hall = dx >= hall0 && dx < hall1;
        let in_west = dx < hall0;
        let tube_west = self.left;
        if in_hall {
            let mid = (hall0 + hall1) / 2;
            let half = (hall1 - hall0) / 2;
            let t = (dx - mid) as f32 / half.max(1) as f32;
            let top = (1.2 * c as f32 + 2.2 * c as f32 * t * t).round() as i64;
            if dy < top { return None; }
            if dy < top + c { return Some(IronBeam); }
            if dy >= floor {
                let lx = dx - hall0;
                let fy = dy - floor;
                return Some(if fy < MINE_RAIL && lx >= 2 * c && lx < hall1 - hall0 - 2 * c { Rail } else { Polished });
            }
            if (dx - hall0).abs() < c / 4 || (dx - hall1).abs() < c / 4 {
                return Some(if dy >= 3 * c && dy < floor { Empty } else { IronBeam });
            }
            // two station columns hold the vault without turning the hall into a colonnade
            if ((dx - 33 * c).abs() < c / 4 || (dx - 49 * c).abs() < c / 4) && dy >= top + c && dy < floor - 3 * c {
                return Some(IronBeam);
            }
            // hanging station sign (the banner sprite sits on this board)
            if dx >= 32 * c && dx < 37 * c && dy >= 3 * c && dy < 5 * c { return Some(Carved); }
            if dx >= 34 * c && dx < 35 * c && dy >= 2 * c && dy < 3 * c { return Some(IronBeam); }
            // derelict railcar: cabin with a window, plank body, iron wheels and a stack
            let (c0, c1) = (37 * c, 47 * c);
            if dx >= 38 * c && dx < 39 * c && dy >= floor - 6 * c && dy < floor - 4 * c { return Some(IronBeam); }
            if dx >= c0 && dx < c1 && dy >= floor - 4 * c && dy < floor {
                if dy >= floor - c && ((dx - (c0 + 2 * c)).abs() < c * 2 / 3 || (dx - (c1 - 2 * c)).abs() < c * 2 / 3) {
                    return Some(IronBeam);
                }
                if dx < c0 + 3 * c {
                    if dx > c0 + c / 2 && dx < c0 + 3 * c - c / 2 && dy >= floor - 3 * c && dy < floor - 2 * c {
                        return Some(Empty);
                    }
                    let edge = dx < c0 + c / 3 || dx >= c0 + 3 * c - c / 3 || dy < floor - 4 * c + c / 3;
                    return Some(if edge { IronBeam } else { WoodBeam });
                }
                if dx > c0 + 4 * c && dx < c0 + 8 * c && dy >= floor - 2 * c - c / 2 && dy < floor - c { return Some(Empty); }
                let edge = dx >= c1 - c / 3 || dy < floor - 3 * c + c / 3 || dy >= floor - c / 2;
                return Some(if edge { IronBeam } else { Planks });
            }
            // waiting benches, leaving the stool sprites at 50 c and 53 c an empty footprint
            let stoolish = (dx >= 50 * c && dx < 51 * c) || (dx >= 53 * c && dx < 54 * c);
            if !stoolish && dx >= 48 * c && dx < 56 * c && dy >= floor - c && dy < floor - c / 4 { return Some(Planks); }
            return Some(Empty);
        }
        let in_tube = if tube_west { in_west } else { dx >= hall1 };
        if in_tube {
            let (t0, t1) = (6 * c, 11 * c);
            if dy < t0 - c / 2 || dy >= t1 + c / 2 {
                if (in_west && dx > hall0 - 4 * c || !in_west && dx < hall1 + 4 * c) && dy >= floor - c && dy < floor {
                    return Some(Gravel);
                }
                return None;
            }
            if dy < t0 || dy >= t1 { return Some(IronBeam); }
            if dy < t0 + c / 4 || dy >= t1 - c / 4 { return Some(IronBeam); }
            if (dx / c) % 5 == 3 { return Some(Gravel); }
            return Some(Tube);
        }
        // warehouse: lower roof so the hall vault reads against host rock
        if dy < 4 * c { return None; }
        if dy < 5 * c { return Some(IronBeam); }
        if dy >= floor { return Some(Polished); }
        let at_outer = if tube_west { dx >= w - c / 2 } else { dx < c / 2 };
        if at_outer { return Some(if dy >= floor - 3 * c && dy < floor { Empty } else { IronBeam }); }
        if ((dx - (if tube_west { 64 * c } else { 12 * c })).abs() < c / 4) && dy >= 5 * c && dy < floor - 3 * c { return Some(IronBeam); }
        let stacks = if tube_west { [60 * c, 66 * c, 72 * c] } else { [8 * c, 14 * c, 20 * c] };
        for (i, sx) in stacks.iter().enumerate() {
            let height = if i == 1 { 3 * c } else if i == 2 { 4 * c } else { 2 * c };
            if dx >= *sx && dx < *sx + 2 * c && dy >= floor - height && dy < floor {
                return Some(if (dx + dy) % (2 * c) < c { Planks } else { WoodBeam });
            }
        }
        let bin = if tube_west { 70 * c } else { 16 * c };
        if dx >= bin && dx < bin + 3 * c && dy >= floor - c && dy < floor { return Some(Gravel); }
        Some(Empty)
    }

    /// Lining this site wants at (x, wy). None = leave whatever `line_back` would put.
    pub fn wall(&self, x: usize, wy: i64) -> Option<Wall> {
        let (dx, dy) = (x as i64 - self.x0 as i64, wy - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 { return None; }
        if self.cell(x, wy).is_none() { return None; }
        crate::templates::get(self.kind).lining_with(dx as usize, dy as usize, self.var, self.left, hash3(x as u32, wy as u32, self.id))
    }
    /// The procedural reference lining (see `cell_reference`): wall at (x, wy) in the variant `k`.
    #[doc(hidden)]
    pub fn wall_reference(&self, x: usize, wy: i64, k: &Knobs) -> Option<Wall> {
        let (dx, dy) = (x as i64 - self.x0 as i64, wy - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 { return None; }
        if self.cell_reference(x, wy, k).is_none() { return None; }
        Some(self.lining_reference(dx, dy, hash3(x as u32, wy as u32, self.id)))
    }
    /// The reference lining at box offset (dx, dy) for a cell hash `h`.
    #[doc(hidden)]
    pub fn lining_reference(&self, dx: i64, dy: i64, h: u32) -> Wall {
        match self.kind {
            Kind::Mine => if (h & 7) == 0 { Wall::Gap } else if (h & 7) == 1 { Wall::Beams } else if (h & 7) == 2 { Wall::Crack } else if (h & 7) == 3 { Wall::Rock } else { Wall::Soot },
            Kind::Holt => if dy > self.h as i64 - 2 * CPM as i64 && (h & 3) == 0 { Wall::Damp }
                else if dx > 18 * CPM as i64 && (h & 3) == 0 { Wall::Root }
                else if h & 7 == 2 { Wall::Beams }
                else if h & 7 == 3 { Wall::Root }
                else if h & 15 == 4 { Wall::Carving }
                else { Wall::Timber },
            Kind::Pumpworks => if (h & 7) == 0 { Wall::Pipes } else if (h & 7) == 1 { Wall::Damp } else { Wall::WetStone },
            Kind::Fissure => if dy > self.h as i64 - 3 * CPM as i64 { Wall::Damp } else if (h & 3) == 0 { Wall::Crack } else { Wall::Basalt },
            Kind::Fossil => if (h & 3) == 0 { Wall::Fossil } else if (h & 3) == 1 { Wall::Crack } else { Wall::Rock },
            Kind::Crystal => if (h & 3) == 0 { Wall::Salt } else if (h & 3) == 1 { Wall::Crystal } else { Wall::Damp },
            Kind::Halt => if (h & 7) == 0 { Wall::Ironclad } else if (h & 7) == 1 { Wall::Gap } else if (h & 15) == 2 { Wall::Pipes } else { Wall::Ashlar },
        }
    }
    /// The procedural reference for what a site owns besides cells (see `Site::cell_reference`): its
    /// things with the box's top-left at (x0, y0), in the order they are set down.
    #[doc(hidden)]
    pub fn things_reference(s: &Site, x0: usize, y0: usize) -> Vec<Thing> {
        let c = CPM; let mut v = vec![];
        match s.kind {
            Kind::Holt => {
                v.push(Thing::Furn { item: Item::Nest, at: [x0 + 3 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Nest, at: [x0 + 5 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Rug, at: [x0 + 3 * c, y0 + s.h - 2 * c] });
                // the table and stools stand on the open floor between the nest niche and the plank loft
                // (under the loft there's only half a metre of headroom)
                v.push(Thing::Furn { item: Item::Table, at: [x0 + 8 * c + c / 2, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Stool, at: [x0 + 7 * c + c / 2, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Stool, at: [x0 + 9 * c + c / 2, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Rug, at: [x0 + 8 * c + c / 2, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Shelf, at: [x0 + 15 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Banner, at: [x0 + 13 * c, y0 + 4 * c] });
                v.push(Thing::Furn { item: Item::Banner, at: [x0 + 4 * c, y0 + 5 * c] });
                v.push(Thing::Furn { item: Item::Banner, at: [x0 + 10 * c, y0 + 4 * c] });
                v.push(Thing::Farm { at: [x0 + s.w - 7 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Farm { at: [x0 + s.w - 5 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Planter, at: [x0 + s.w - 9 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Planter, at: [x0 + s.w - 3 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Torch { at: [x0 + 4 * c, y0 + s.h - 4 * c] });
                v.push(Thing::Torch { at: [x0 + 13 * c, y0 + s.h - 4 * c] });
                v.push(Thing::Torch { at: [x0 + s.w - 6 * c, y0 + s.h - 4 * c] });
            }
            Kind::Mine => {
                v.push(Thing::Torch { at: [x0 + 8 * c, y0 + s.h - 3 * c] });
                v.push(Thing::Torch { at: [x0 + 30 * c, y0 + s.h - 3 * c] });
                v.push(Thing::Torch { at: [x0 + 60 * c, y0 + s.h - 3 * c] });
                v.push(Thing::Torch { at: [x0 + 20 * c, y0 + 8 * c] });
                v.push(Thing::Torch { at: [x0 + 50 * c, y0 + 8 * c] });
                v.push(Thing::Torch { at: [x0 + 48 * c + c, y0 + 5 * c] });
                v.push(Thing::Furn { item: Item::Shelf, at: [x0 + s.w - 10 * c, y0 + s.h - 2 * c] });
                let sx = x0 + 48 * c + c / 2;
                v.push(Thing::Ladder { at: [sx, y0 + 9 * c], anchor: [sx.saturating_sub(c), (y0 + 9 * c).saturating_sub(1)] });
                v.push(Thing::Ladder { at: [sx, y0 + 16 * c], anchor: [sx.saturating_sub(c), (y0 + 16 * c).saturating_sub(1)] });
                v.push(Thing::Cart { at: [(x0 + 40 * c) as f32 + 0.5, (y0 + s.h - c) as f32] });
                v.push(Thing::Cart { at: [(x0 + 52 * c) as f32 + 0.5, (y0 + s.h - c) as f32] });
            }
            Kind::Pumpworks => {
                let gx = x0 + 20 * c + c / 2; let gy = y0 + s.h - 3 * c + c / 2;
                v.push(Thing::Device { dev: Dev::Floodgate, at: [gx, gy] });
                let lx = x0 + 22 * c; let ly = y0 + s.h - 4 * c;
                v.push(Thing::Device { dev: Dev::Lever, at: [lx, ly] });
                v.push(Thing::Wire { at: [lx, ly] }); v.push(Thing::Wire { at: [gx, gy] }); v.push(Thing::Wire { at: [(lx + gx) / 2, ly] }); v.push(Thing::Wire { at: [(lx + gx) / 2, gy] });
                v.push(Thing::SwitchOn { at: [lx, ly] });
                v.push(Thing::Device { dev: Dev::Pump, at: [x0 + 37 * c, y0 + s.h - 3 * c] });
                v.push(Thing::Ladder { at: [x0 + 28 * c, y0 + 8 * c], anchor: [x0 + 27 * c, (y0 + 8 * c).saturating_sub(1)] });
                v.push(Thing::Torch { at: [x0 + 10 * c, y0 + s.h - 6 * c] });
                v.push(Thing::Torch { at: [x0 + 24 * c, y0 + s.h - 4 * c] });
                v.push(Thing::Torch { at: [x0 + 42 * c, y0 + s.h - 8 * c] });
                v.push(Thing::Torch { at: [x0 + s.w - 4 * c, y0 + s.h - 5 * c] });
            }
            Kind::Fissure => {
                let lx = x0 + 4 * c;
                let rx = x0 + s.w - 5 * c;
                for top_m in [8, 16, 24, 32] {
                    let top = y0 + top_m * c + c / 2;
                    v.push(Thing::Ladder { at: [lx, top], anchor: [x0 + 3 * c, top.saturating_sub(1)] });
                    v.push(Thing::Ladder { at: [rx, top], anchor: [x0 + s.w - 4 * c, top.saturating_sub(1)] });
                }
                v.push(Thing::Torch { at: [x0 + 5 * c, y0 + 8 * c] });
                v.push(Thing::Torch { at: [x0 + s.w / 2, y0 + 16 * c] });
                v.push(Thing::Torch { at: [x0 + s.w - 6 * c, y0 + 24 * c] });
            }
            Kind::Halt => {
                let cx = x0 + 40 * c; let cy = y0 + s.h - c;
                v.push(Thing::Cart { at: [cx as f32 + 0.5, cy as f32] });
                let wx = if s.left { x0 + s.w - 16 * c } else { x0 + 8 * c };
                v.push(Thing::Furn { item: Item::Shelf, at: [wx, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Shelf, at: [wx + 8 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Table, at: [wx + 4 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Torch { at: [x0 + 40 * c, y0 + s.h - 4 * c] });
                v.push(Thing::Torch { at: [wx + 2 * c, y0 + s.h - 4 * c] });
                v.push(Thing::Furn { item: Item::Banner, at: [x0 + 34 * c, y0 + 5 * c] });
                v.push(Thing::Furn { item: Item::Stool, at: [x0 + 50 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Stool, at: [x0 + 53 * c, y0 + s.h - 2 * c] });
            }
            Kind::Fossil => {
                v.push(Thing::Torch { at: [x0 + s.w / 2, y0 + s.h - 4 * c] });
                v.push(Thing::Torch { at: [x0 + 6 * c, y0 + s.h - 5 * c] });
                v.push(Thing::Torch { at: [x0 + s.w - 10 * c, y0 + s.h - 5 * c] });
                v.push(Thing::Furn { item: Item::Shelf, at: [x0 + 5 * c, y0 + s.h - 2 * c] });
                v.push(Thing::Furn { item: Item::Table, at: [x0 + 8 * c, y0 + s.h - 2 * c] });
                // through the walk hatches at 12 m so the scaffold is a climb from the floor
                v.push(Thing::Ladder { at: [x0 + 12 * c, y0 + 5 * c], anchor: [x0 + 11 * c, (y0 + 5 * c).saturating_sub(1)] });
                v.push(Thing::Ladder { at: [x0 + 12 * c, y0 + 8 * c], anchor: [x0 + 11 * c, (y0 + 8 * c).saturating_sub(1)] });
            }
            Kind::Crystal => {
                v.push(Thing::Torch { at: [x0 + s.w / 2, y0 + s.h - 4 * c] });
                v.push(Thing::Torch { at: [x0 + 6 * c, y0 + s.h - 4 * c] });
                v.push(Thing::Torch { at: [x0 + s.w - 6 * c, y0 + s.h - 4 * c] });
            }
        }
        v
    }
}

/// Loot in a site chest: old kit and what the place was for, never relics (those stay in Ancient vaults).
pub fn site_loot(kind: Kind, h: u32) -> Vec<(Item, u32)> {
    match kind {
        Kind::Mine => vec![(Item::Coal, 4 + h % 3), (Item::CopperOre, 2 + h % 2), (Item::WoodPillar, 4), (Item::Rail, 6)],
        Kind::Holt => vec![(Item::Torch, 3), (Item::Grub, 2), (Item::Wood, 3), (Item::Fungus, 2)],
        Kind::Pumpworks => vec![(Item::CopperWire, 6), (Item::IronIngot, 1 + h % 2), (Item::Clay, 4)],
        Kind::Fissure => vec![(Item::RopeLadder, 1), (Item::Wood, 2)],
        Kind::Fossil => vec![(Item::Quartz, 3 + h % 2), (Item::Stone, 6), (Item::Crystal, 1)],
        Kind::Crystal => vec![(Item::Crystal, 3 + h % 2), (Item::Quartz, 2), (Item::Chalk, 2)],
        Kind::Halt => vec![(Item::Rail, 8), (Item::CopperIngot, 1), (Item::Tube, 4), (Item::Stone, 6)],
    }
}

impl World {
    /// Refresh the sites touching the window (with the chests).
    pub fn refresh_sites(&mut self) {
        self.sites = self.gen.sites_in_rows(self.oy, self.oy + self.g.h as i64);
        self.stamp_site_things();
    }
    /// Devices, furniture, farms, carts and ladders a site owns. Idempotent: a thing already there is left.
    pub fn stamp_site_things(&mut self) {
        let sites = self.sites.clone();
        for s in &sites { self.stamp_one(s); }
    }
    /// Scripted: stamp one site of `kind` with its top-left at window (x0, y0). Used by scenarios.
    pub fn stamp_site(&mut self, kind: Kind, x0: usize, y0: usize) {
        self.stamp_site_var(kind, x0, y0, 0, false);
    }
    /// Like `stamp_site`, with a layout nibble and side. Used by the reachability test
    /// so every family is walked across seeds and both halt sides.
    pub fn stamp_site_var(&mut self, kind: Kind, x0: usize, y0: usize, var: u32, left: bool) {
        let (w, h) = kind.size();
        if x0 + w >= self.g.w || y0 + h >= self.g.h { return; }
        let s = Site::new(0x51FE ^ (x0 as u32).wrapping_mul(131) ^ (y0 as u32), kind, x0, self.oy + y0 as i64, var, left, None);
        for y in 0..s.h { for x in 0..s.w {
            if let Some(m) = s.cell(s.x0 + x, s.y0 + y as i64) {
                let i = (y0 + y) * self.g.w + s.x0 + x; self.g.set(i, m); self.g.flags[i] = 0;
            }
            if let Some(wl) = s.wall(s.x0 + x, s.y0 + y as i64) { self.walls.set_at(s.x0 + x, y0 + y, wl); }
        } }
        if let Some((cx, cwy)) = s.chest {
            let cy = (cwy - self.oy) as usize;
            if cy < self.g.h && !self.chests.iter().any(|c| c.x == cx && c.y == cy) {
                self.chests.push(crate::world::Chest { x: cx, y: cy, opened: false, ancient: None });
            }
        }
        self.stamp_one(&s);
        self.sites.push(s);
    }
    fn stamp_one(&mut self, s: &Site) {
        let y0 = (s.y0 - self.oy) as i64;
        if y0 < 4 || y0 + s.h as i64 >= self.g.h as i64 - 4 { return; }
        let (x0, y0) = (s.x0, y0 as usize);
        for t in crate::templates::get(s.kind).things(s.var, s.left) { self.set_thing(t.offset(x0, y0)); }
    }
    /// Set one site thing down at its window cell. Each is skipped if it doesn't fit or is already there.
    pub(crate) fn set_thing(&mut self, t: Thing) {
        match t {
            Thing::Torch { at } => self.try_torch(at[0], at[1]),
            Thing::Furn { item, at } => self.try_furn(item, at[0], at[1]),
            Thing::Farm { at } => self.try_farm(at[0], at[1]),
            Thing::Ladder { at, anchor } => self.try_ladder(at[0], at[1], anchor[0], anchor[1]),
            Thing::Cart { at } => self.try_cart(at[0], at[1]),
            Thing::Device { dev, at } => self.try_dev(dev, at[0], at[1]),
            Thing::Wire { at } => self.try_wire(at[0], at[1]),
            Thing::SwitchOn { at } => if let Some(b) = self.try_block(at[0], at[1]) { self.logic.dev_s[b] = 1; self.logic.invalidate(); },
        }
    }
    fn try_block(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.g.w || y >= self.g.h { return None; }
        Some(self.logic.block_of(x, y))
    }
    fn try_dev(&mut self, d: Dev, x: usize, y: usize) {
        let Some(b) = self.try_block(x, y) else { return };
        if self.logic.dev[b] != 0 { return; }
        if !self.logic.place(&mut self.g, d, b) {
            self.logic.dev[b] = d as u8; self.logic.invalidate();
        }
    }
    fn try_wire(&mut self, x: usize, y: usize) {
        let Some(b) = self.try_block(x, y) else { return };
        let _ = self.logic.place_wire(b);
    }
    fn try_furn(&mut self, it: Item, x: usize, y: usize) {
        if x >= self.g.w || y >= self.g.h { return; }
        if self.furniture.iter().any(|f| f.contains(x, y)) { return; }
        if let Some(fu) = self.furniture_spot(it, x, y) {
            let (x0, y0, x1, y1) = fu.rect();
            if (y0..=y1).any(|cy| (x0..=x1).any(|cx| self.g.is_solidish(cy * self.g.w + cx))) { return; }
            self.furniture.push(fu);
        } else if let Some(f) = it.furn() {
            let (fw, fh) = (f.w * S, f.h * S); if x + fw >= self.g.w || y < fh { return; }
            self.furniture.push(Furniture { item: it, x, y, tint: 0 });
        }
    }
    fn try_farm(&mut self, x: usize, y: usize) {
        if x >= self.g.w || y + 1 >= self.g.h { return; }
        if self.farms.iter().any(|f| f.x.abs_diff(x) < 2 * S && f.y.abs_diff(y) < 2 * S) { return; }
        if !self.g.m(y * self.g.w + x).passable() { return; }
        self.farms.push(Farm { x, y, growth: 0.4, why: 0 });
    }
    fn try_torch(&mut self, x: usize, y: usize) {
        if x >= self.g.w || y >= self.g.h { return; }
        if self.torches.iter().any(|&(tx, ty)| tx.abs_diff(x) < 2 * S && ty.abs_diff(y) < 2 * S) { return; }
        if self.g.is_solidish(y * self.g.w + x) { return; }
        self.torches.push((x, y));
    }
    fn try_ladder(&mut self, x: usize, top: usize, ax: usize, ay: usize) {
        if x >= self.g.w || top >= self.g.h { return; }
        if self.tr.ladders.iter().any(|l| l.x.abs_diff(x) < S && l.top.abs_diff(top) < S) { return; }
        let len = (top..(top + crate::lifts::LADDER_M).min(self.g.h - 1)).take_while(|&r| self.passable(x, r)).count();
        if len < 2 * S { return; }
        self.tr.ladders.push(crate::lifts::Ladder { x, top, len, ax: ax.min(self.g.w - 1), ay: ay.min(self.g.h - 1) });
    }
    fn try_cart(&mut self, x: f32, y: f32) {
        if x < 2. || y < 2. || x >= self.g.w as f32 - 2. || y >= self.g.h as f32 - 2. { return; }
        if self.tr.carts.iter().any(|c| (c.x - x).abs() < crate::transport::CART_W && (c.y - y).abs() < crate::transport::CART_H) { return; }
        self.tr.carts.push(crate::transport::Cart {
            x, y, px: x, py: y, v: 0., vy: 0., dir: 1, on_rail: self.is_rail(x as i32, y as i32),
            cargo: vec![(Item::Stone, 6 + (x as u32) % 5)], tip: 0,
        });
    }
    /// First visit within 8 m: a line in the log. No cards.
    pub(crate) fn step_sites(&mut self) {
        let Some(m) = self.moles.first().filter(|m| m.alive) else { return };
        let (mx, my) = (m.x as i64, self.oy + m.y as i64); let reach = 8 * CPM as i64;
        let near: Vec<(u32, Kind, bool)> = self.sites.iter().filter(|s| !self.sites_seen.contains(&s.id)
            && mx >= s.x0 as i64 - reach && mx < (s.x0 + s.w) as i64 + reach && my >= s.y0 - reach && my < s.y0 + s.h as i64 + reach)
            .map(|s| (s.id, s.kind, s.link.is_some())).collect();
        for (id, kind, link) in near {
            self.sites_seen.insert(id); self.events.push(crate::world::Event::Discover);
            self.say(notice(kind, link).into());
        }
    }
    /// The site whose chest sits at window (x, y), if any.
    pub fn site_at_chest(&self, x: usize, y: usize) -> Option<Kind> {
        let wy = self.oy + y as i64;
        self.sites.iter().find(|s| s.chest == Some((x, wy))).map(|s| s.kind)
    }
}

/// Overlap test used by the rarity/non-overlap tests.
pub fn boxes_overlap(a: (i64, i64, i64, i64), b: (i64, i64, i64, i64)) -> bool {
    a.2 > b.0 && a.0 < b.2 && a.3 > b.1 && a.1 < b.3
}
