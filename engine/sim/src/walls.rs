//! Decorative background walls: a Terraria-style layer behind the player and the falling sand.
//!
//! Walls are *not* materials. They never enter the CA, the stress solver or collision. A Delver walks
//! in front of them; they hold up nothing. They exist so a dug room reads as carved rock or a timbered
//! holt instead of a hole in the far parallax. The same light that falls on the open cell lights the
//! wall (darker, lower contrast than a real block so it is never mistaken for support).
//!
//! Storage is one byte per cell, packed with the bands (optional trailing RLE). Old saves load with
//! no wall bytes; `World::infer_back` then lines open cave the way a fresh holt would.
use crate::craft::Item;
use crate::gen::Gen;
use crate::grid::{hash3, Grid};
use crate::mat::{Kind, Mat, S, SF};
use crate::world::{disc, Event, World, CELLS_PER_BLOCK};

/// Cells of lining one pack item covers (same as a structural block: 4 legacy tiles).
pub const CELLS_PER_WALL: u16 = CELLS_PER_BLOCK;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Wall {
    None = 0,
    /// Carved host rock: the default lining of a dug room or a natural cave.
    Rock,
    /// Timber panelling: holts, mines, sleeping niches.
    Timber,
    /// Brick / dressed stone on the back.
    Masonry,
    /// Decorative beams (they hold no weight).
    Beams,
    /// Decorative copper pipes (they carry nothing).
    Pipes,
    /// Carved marks and fluting on the back.
    Carving,
    /// Damp stain, seep and moisture.
    Damp,
    /// Cracks in the lining.
    Crack,
    /// Roots through the back.
    Root,
    /// A missing panel: the far rock peeks through.
    Gap,
    /// Salt crust.
    Salt,
    /// Crystal lining (the wall does not glow; crystal *blocks* still do).
    Crystal,
    /// Fossil imprint left in the rock.
    Fossil,
    /// Soot-blackened timber: an abandoned mine, not a holt's warm panelling.
    Soot,
    /// Damp green-gray dressed stone: pumpworks that have stood in seep water.
    WetStone,
    /// Dark split volcanic rock: the lining of a fissure.
    Basalt,
    /// Cool blue-gray dressed blocks: a cargo halt's platform hall.
    Ashlar,
    /// Dark iron plates on the back. They hold no weight.
    Ironclad,
}
pub const NWALL: usize = 19;

impl Wall {
    pub fn from_u8(v: u8) -> Wall { if (v as usize) < NWALL { unsafe { std::mem::transmute(v) } } else { Wall::None } }
    pub fn name(self) -> &'static str {
        ["", "Rock wall", "Timber wall", "Masonry wall", "Beam wall", "Pipe wall", "Carved wall",
            "Damp stain", "Crack", "Root wall", "Missing panel", "Salt wall", "Crystal wall", "Fossil wall", "Sooted timber", "Wet stone wall", "Basalt wall", "Ashlar wall", "Ironclad wall"][self as usize]
    }
    /// One lore-safe line. Always says it holds no roof.
    pub fn desc(self) -> &'static str {
        ["", "Carved rock lining. Walk in front of it; it holds no roof.",
            "Timber panelling. Walk in front of it; it holds no roof.",
            "Brick lining. Walk in front of it; it holds no roof.",
            "Decorative beams on the back. They hold no weight.",
            "Old copper pipes on the back. They carry nothing.",
            "Carved marks on the back. They hold no roof.",
            "A damp stain on the lining. It holds no roof.",
            "A crack in the lining. It holds no roof.",
            "Roots through the lining. They hold no roof.",
            "A panel has come away. The far rock shows; nothing here holds a roof.",
            "Salt crust on the lining. It holds no roof.",
            "Crystal crust on the lining. It holds no roof.",
            "A bone-print in the lining. It holds no roof.",
            "Soot-blackened timber lining. Walk in front of it; it holds no roof.",
            "Damp dressed stone lining. Walk in front of it; it holds no roof.",
            "Split basalt lining. Walk in front of it; it holds no roof.",
            "Dressed blue-gray stone lining. Walk in front of it; it holds no roof.",
            "Iron plates on the back. They hold no weight."][self as usize]
    }
    /// Darker, flatter than the matching foreground block, so a wall never reads as load-bearing.
    pub fn color(self) -> [u8; 3] {
        match self {
            Wall::None => [0, 0, 0],
            Wall::Rock => [118, 108, 96],
            Wall::Timber => [158, 112, 62],
            Wall::Masonry => [132, 78, 62],
            Wall::Beams => [118, 84, 54],
            Wall::Pipes => [78, 118, 96],
            Wall::Carving => [124, 116, 106],
            Wall::Damp => [78, 92, 88],
            Wall::Crack => [96, 88, 80],
            Wall::Root => [86, 96, 56],
            Wall::Gap => [58, 50, 46],
            Wall::Salt => [196, 190, 178],
            Wall::Crystal => [64, 148, 176],
            Wall::Fossil => [154, 138, 108],
            Wall::Soot => [72, 58, 44],
            Wall::WetStone => [68, 92, 90],
            Wall::Basalt => [48, 46, 56],
            Wall::Ashlar => [88, 98, 114],
            Wall::Ironclad => [58, 60, 66],
        }
    }
    /// How much of the far parallax shows through (0 = fully covered, 1 = none of the wall).
    pub fn see_through(self) -> f32 {
        match self { Wall::None => 1., Wall::Gap => 0.55, Wall::Crack => 0.18, Wall::Damp => 0.08, _ => 0. }
    }
    /// Per-cell shade (0.75..1.2) so a wall has grain without looking like a structural face.
    pub fn shade(self, x: u32, wy: u32) -> f32 {
        let h = |a, b, s| (hash3(a, b, s) & 255) as f32 / 255.;
        let v = h(x, wy, 31);
        let k = S as u32; let (tx, ty) = (x / k, wy / k);
        match self {
            Wall::None => 1.,
            Wall::Rock => 0.88 + h(0, (ty / 3).wrapping_add(tx / 19), 41) * 0.14 + if v > 0.94 { 0.08 } else { 0. },
            Wall::Timber => {
                let board = ty / 2; let seam = wy % (2 * k) == 2 * k - 1; let joint = tx.wrapping_add(board.wrapping_mul(5)) % 8 == 0;
                if seam { 0.7 } else if joint { 0.78 } else { 0.9 + h(board, tx / 3, 61) * 0.08 }
            }
            Wall::Masonry => {
                let row = ty / 2; let off = if row % 2 == 0 { 0 } else { 2 };
                if ty % 2 == 1 || (tx + off) % 4 == 0 { 0.7 } else { 0.96 + (h((tx + off) / 4, row, 53) - 0.5) * 0.12 }
            }
            Wall::Beams => if tx % 4 == 0 || ty % 5 == 0 { 1.12 } else { 0.86 + h(0, ty, 57) * 0.1 },
            Wall::Pipes => if wy % (2 * k) < k / 2 { 1.08 } else if tx % 6 == 0 { 0.8 } else { 0.92 },
            Wall::Carving => if ty % 10 == 0 { 1.1 } else { match tx % 3 { 0 => 1.06, 1 => 0.78, _ => 0.94 } },
            Wall::Damp => 0.78 + v * 0.16,
            Wall::Crack => if (tx as i64).wrapping_sub(ty as i64).unsigned_abs() % 7 == 0 { 0.62 } else { 0.9 },
            Wall::Root => if tx.wrapping_add(ty / 2) % 3 == 0 { 0.76 } else { 1.02 },
            Wall::Gap => 0.7 + v * 0.1,
            Wall::Salt => 0.92 + if v > 0.9 { 0.16 } else { 0. } + h(tx / 2, ty / 2, 71) * 0.06,
            Wall::Crystal => 0.86 + if v > 0.82 { 0.2 } else { 0. } ,
            Wall::Fossil => if (tx / 2).wrapping_add(ty) % 5 == 0 { 1.14 } else { 0.88 },
            Wall::Soot => {
                let board = ty / 2; let seam = wy % (2 * k) == 2 * k - 1; let joint = tx.wrapping_add(board.wrapping_mul(5)) % 8 == 0;
                if seam { 0.62 } else if joint { 0.7 } else { 0.78 + h(board, tx / 3, 61) * 0.08 }
            }
            Wall::WetStone => {
                let row = ty / 2; let off = if row % 2 == 0 { 0 } else { 2 };
                if ty % 2 == 1 || (tx + off) % 4 == 0 { 0.62 } else { 0.9 + (h((tx + off) / 4, row, 53) - 0.5) * 0.1 }
            }
            Wall::Basalt => 0.8 + h(0, (ty / 2).wrapping_add(tx / 11), 43) * 0.16 + if (tx as i64).wrapping_sub(ty as i64).unsigned_abs() % 9 == 0 { -0.12 } else { 0. },
            Wall::Ashlar => {
                let row = ty / 3; let off = if row % 2 == 0 { 0 } else { 3 };
                if ty % 3 == 2 || (tx + off) % 6 == 0 { 0.68 } else { 0.96 + (h((tx + off) / 6, row, 59) - 0.5) * 0.1 }
            }
            Wall::Ironclad => if tx % 5 == 0 || ty % 4 == 0 { 1.08 } else { 0.82 + h(tx / 5, ty / 4, 67) * 0.08 },
        }
    }
    /// The pack item that places this wall, if a Delver can make it.
    pub fn item(self) -> Option<Item> {
        Some(match self {
            Wall::Rock => Item::RockWall, Wall::Timber => Item::TimberWall, Wall::Masonry => Item::MasonryWall,
            Wall::Beams => Item::BeamWall, Wall::Pipes => Item::PipeWall, Wall::Carving => Item::CarvedWall,
            _ => return None,
        })
    }
    /// What scraping this lining puts back in the pack (gen-only kinds become the nearest made wall).
    pub fn scrapes_to(self) -> Option<Item> {
        match self {
            Wall::None | Wall::Gap => None,
            Wall::Timber | Wall::Beams | Wall::Root => Some(Item::TimberWall),
            Wall::Masonry => Some(Item::MasonryWall),
            Wall::Pipes => Some(Item::PipeWall),
            Wall::Carving | Wall::Fossil => Some(Item::CarvedWall),
            Wall::Soot => Some(Item::TimberWall),
            Wall::WetStone | Wall::Ashlar => Some(Item::MasonryWall),
            Wall::Ironclad => Some(Item::PipeWall),
            _ => Some(Item::RockWall),
        }
    }
}

/// The wall a pack item places, if it is a back-wall item.
pub fn kind_of(it: Item) -> Option<Wall> {
    Some(match it {
        Item::RockWall => Wall::Rock, Item::TimberWall => Wall::Timber, Item::MasonryWall => Wall::Masonry,
        Item::BeamWall => Wall::Beams, Item::PipeWall => Wall::Pipes, Item::CarvedWall => Wall::Carving,
        _ => return None,
    })
}

/// One byte per window cell. Never swapped by the CA: a wall stays with the room, not the grain.
pub struct Walls {
    pub cell: Vec<u8>,
    pub w: usize, pub h: usize,
    /// how much of the current wall item is still being spent (like `World::place_acc`)
    acc: u16,
    /// how much scraped lining is waiting to become a pack item
    scrap: u16,
}
impl Walls {
    pub fn new(w: usize, h: usize) -> Walls { Walls { cell: vec![0; w * h], w, h, acc: 0, scrap: 0 } }
    #[inline] pub fn get(&self, i: usize) -> Wall { Wall::from_u8(self.cell.get(i).copied().unwrap_or(0)) }
    #[inline] pub fn at(&self, x: usize, y: usize) -> Wall { if x < self.w && y < self.h { self.get(y * self.w + x) } else { Wall::None } }
    #[inline] pub fn set(&mut self, i: usize, w: Wall) { if i < self.cell.len() { self.cell[i] = w as u8; } }
    #[inline] pub fn set_at(&mut self, x: usize, y: usize, w: Wall) { if x < self.w && y < self.h { self.cell[y * self.w + x] = w as u8; } }
    pub fn count(&self, w: Wall) -> usize { self.cell.iter().filter(|&&v| v == w as u8).count() }
}

/// What lining a dug material leaves behind (carving the rock, not opening a hole in the parallax).
pub fn lining_of(m: Mat) -> Wall {
    match m {
        Mat::WoodBeam | Mat::Planks | Mat::RootTree | Mat::TreeRoot | Mat::Roots | Mat::Door => Wall::Timber,
        Mat::StoneBrick | Mat::Polished | Mat::Tiles | Mat::Carved => Wall::Masonry,
        Mat::IronBeam | Mat::Rail | Mat::Tube => Wall::Pipes,
        Mat::Crystal => Wall::Crystal,
        Mat::Quartz => Wall::Salt,
        _ => Wall::Rock,
    }
}

/// Natural lining of an open cave cell: mostly rock, with stains, cracks, roots and the odd missing panel.
pub fn natural_at(gen: &Gen, x: i64, wy: i64) -> Wall {
    let d = wy as f32 * crate::mat::CELL_M;
    let h = hash3(x as u32, wy as u32, gen.seed ^ 0xB4C11);
    let r = (h & 255) as f32 / 255.;
    if d < -8. && r > 0.82 { Wall::Root }
    else if d < 20. && r > 0.88 { Wall::Damp }
    else if r > 0.93 { Wall::Crack }
    else if r > 0.97 { Wall::Gap }
    else if d > 50. && r < 0.06 { Wall::Salt }
    else if d > 45. && r < 0.04 { Wall::Crystal }
    else { Wall::Rock }
}

impl World {
    /// Line open cells in window rows `[y0, y1)` from the generator: caves, grottos, ruins and sites.
    /// Solid rock stays unlined (nothing to see). Does not touch cells the player already panelled.
    pub fn line_back(&mut self, y0: usize, y1: usize) { self.line_back_rect(0, self.g.w, y0, y1) }
    /// `line_back` over window columns `[x0, x1)` only (a tile that slid in).
    pub fn line_back_rect(&mut self, x0: usize, x1: usize, y0: usize, y1: usize) {
        let (w, h) = (self.g.w, self.g.h); let (x1, y1) = (x1.min(w), y1.min(h));
        for y in y0..y1 { for xw in x0..x1 {
            let i = y * w + xw; if self.walls.cell[i] != 0 { continue; }
            let (x, wy) = (self.ox + xw as i64, self.oy + y as i64);
            if let Some(wl) = self.gen.site_wall(x, wy) { self.walls.cell[i] = wl as u8; continue; }
            if !shows_back(&self.g, i) { continue; }
            if self.gen.grotto_of(x, wy).is_some_and(|g| g.is_void(x, wy)) {
                let r = hash3(x as u32, wy as u32, self.info.seed ^ 0x71) & 7;
                self.walls.cell[i] = (if r < 3 { Wall::Root } else if r < 5 { Wall::Damp } else { Wall::Rock }) as u8;
            } else if self.gen.in_ruin_void(x, wy) {
                self.walls.cell[i] = Wall::Masonry as u8;
            } else {
                self.walls.cell[i] = natural_at(&self.gen, x, wy) as u8;
            }
        } }
    }
    /// Timber the start burrow so it reads as a holt, not a hole.
    pub fn line_burrow(&mut self) {
        let (w, h) = (self.g.w, self.g.h); let sx = (self.gen.w0 / 2) as i64 - self.ox;
        if sx < 0 || sx >= w as i64 || -self.oy < 0 { return; } let sx = sx as usize; // the burrow is off-window
        let sy = ((-self.oy) as usize).min(h.saturating_sub(4 * S));
        let (x0, x1) = (sx.saturating_sub(16 * S), (sx + 16 * S).min(w));
        let (y0, y1) = (sy.saturating_sub(14 * S), (sy + 4 * S).min(h));
        for y in y0..y1 { for x in x0..x1 {
            let i = y * w + x; if !shows_back(&self.g, i) { continue; }
            if self.walls.cell[i] == 0 || self.walls.get(i) == Wall::Rock {
                self.walls.cell[i] = Wall::Timber as u8;
            }
        } }
    }
    /// Old save (no wall bytes): line open cave the way a fresh holt would, then timber the burrow.
    pub fn infer_back(&mut self) {
        self.walls.cell.fill(0);
        self.line_back(0, self.g.h);
        self.line_burrow();
    }
    /// Leave a lining when a solid cell is dug, so a bite carves rock rather than opening the parallax.
    pub(crate) fn leave_lining(&mut self, i: usize, was: Mat) {
        if self.walls.cell[i] == 0 { self.walls.cell[i] = lining_of(was) as u8; }
    }
    /// Place a round brush of back walls. Never edits the foreground. Open or solid cells in reach take
    /// the lining (hidden behind rock until dug). Returns how many cells changed.
    pub fn place_walls(&mut self, x: usize, y: usize, it: Item, want: f32) -> Result<usize, &'static str> {
        let Some(wl) = kind_of(it) else { return Err("not a wall") };
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let r = crate::world::brush_radius(crate::world::Brush::Place(it), want);
        let (k, rt) = (S as i32, r / SF); let (tx, ty) = (x as i32 / k, y as i32 / k);
        let mut n = 0;
        for (dx, dy) in disc(rt) {
            for j in 0..k * k {
                let (cx, cy) = ((tx + dx) * k + j % k, (ty + dy) * k + j / k);
                if !self.wall_ok(cx, cy) { continue; }
                let i = cy as usize * self.g.w + cx as usize;
                if self.walls.get(i) == wl { continue; }
                if !self.pay_wall(it) { if n == 0 { return Err("none left"); } return Ok(n); }
                self.walls.cell[i] = wl as u8; n += 1;
            }
        }
        if n == 0 { return Err("nothing to panel"); }
        self.marks.push((1, x as f32 + 0.5, y as f32 + 0.5, r)); self.events.push(Event::Place); Ok(n)
    }
    /// Grid-snapped wall line, same squares as `place_line`, but only the back.
    pub fn place_wall_line(&mut self, a: (usize, usize), b: (usize, usize), n: usize, it: Item) -> Result<usize, &'static str> {
        let Some(wl) = kind_of(it) else { return Err("not a wall") };
        let n = n.clamp(1, 64) as i64; let (ax, ay, bx, by) = (a.0 as i64 / n, a.1 as i64 / n, b.0 as i64 / n, b.1 as i64 / n);
        let (ex, ey) = if (bx - ax).abs() >= (by - ay).abs() { (bx, ay) } else { (ax, by) };
        let mut k = 0;
        for gy in ay.min(ey)..=ay.max(ey) { for gx in ax.min(ex)..=ax.max(ex) {
            for j in 0..n * n {
                let (cx, cy) = (gx * n + j % n, gy * n + j / n);
                if !self.wall_ok(cx as i32, cy as i32) { continue; }
                let i = cy as usize * self.g.w + cx as usize;
                if self.walls.get(i) == wl { continue; }
                if !self.pay_wall(it) { if k == 0 { return Err("none left"); } return Ok(k); }
                self.walls.cell[i] = wl as u8; k += 1;
            }
        } }
        if k == 0 { return Err(if self.in_reach(b.0, b.1) { "nothing to panel" } else { "out of reach" }); }
        self.marks.push((1, b.0 as f32 + 0.5, b.1 as f32 + 0.5, n as f32 * 0.5)); self.events.push(Event::Place); Ok(k)
    }
    /// Scrape back walls with the same brush as placing. Never edits the foreground. Scraped lining
    /// returns to the pack (one item per `CELLS_PER_WALL` cells).
    pub fn scrape_walls(&mut self, x: usize, y: usize, want: f32) -> Result<usize, &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let r = crate::world::brush_radius(crate::world::Brush::Place(Item::RockWall), want);
        let (k, rt) = (S as i32, r / SF); let (tx, ty) = (x as i32 / k, y as i32 / k);
        let mut n = 0;
        for (dx, dy) in disc(rt) {
            for j in 0..k * k {
                let (cx, cy) = ((tx + dx) * k + j % k, (ty + dy) * k + j / k);
                if cx < 1 || cy < 1 || cx as usize + 1 >= self.g.w || cy as usize + 1 >= self.g.h { continue; }
                if !self.in_reach(cx as usize, cy as usize) { continue; }
                let i = cy as usize * self.g.w + cx as usize;
                let wl = self.walls.get(i); if wl == Wall::None { continue; }
                self.walls.cell[i] = 0;
                if let Some(it) = wl.scrapes_to() { self.take_scrape(it); }
                n += 1;
            }
        }
        if n == 0 { return Err("no lining here"); }
        self.marks.push((0, x as f32 + 0.5, y as f32 + 0.5, r)); self.events.push(Event::Dig); Ok(n)
    }
    /// Scripted: set one cell's wall with no reach or pack check (scenarios, tests).
    pub fn set_wall(&mut self, x: usize, y: usize, wl: Wall) {
        if x < self.g.w && y < self.g.h { self.walls.cell[y * self.g.w + x] = wl as u8; }
    }
    fn wall_ok(&self, nx: i32, ny: i32) -> bool {
        if nx < 1 || ny < 1 || nx as usize + 1 >= self.g.w || ny as usize + 1 >= self.g.h { return false; }
        let (cx, cy) = (nx as usize, ny as usize);
        self.in_reach(cx, cy) && self.g.get(cx, cy) != Mat::Bedrock
    }
    fn pay_wall(&mut self, it: Item) -> bool {
        if self.walls.acc == 0 {
            if self.inv[it as usize] == 0 { return false; }
            self.inv[it as usize] -= 1; self.walls.acc = CELLS_PER_WALL;
        }
        self.walls.acc -= 1; true
    }
    fn take_scrape(&mut self, it: Item) {
        self.walls.scrap += 1;
        if self.walls.scrap >= CELLS_PER_WALL { self.walls.scrap = 0; self.inv[it as usize] += 1; }
    }
}

fn shows_back(g: &Grid, i: usize) -> bool {
    matches!(g.m(i).kind(), Kind::Empty | Kind::Gas | Kind::Liquid)
}

/// How far a wall brush reaches (tiles of 0.25 m, same as a plank brush).
pub fn wall_brush_range() -> (f32, f32, f32) { (0.5 * SF, 3.5 * SF, 1.0 * SF) }

/// Contrast used when drawing a wall: darker than a real block under the same light.
pub const WALL_K: f32 = 0.78;
/// Floor brightness so a lined room in the dark still reads as timber or rock, never a black hole.
pub const WALL_AMIN: f32 = 0.34;


#[cfg(test)]
mod shade_tests {
    use super::Wall;
    #[test]
    fn shade_survives_wrapped_world_y() {
        let walls = [
            Wall::None, Wall::Rock, Wall::Timber, Wall::Masonry, Wall::Beams, Wall::Pipes,
            Wall::Carving, Wall::Damp, Wall::Crack, Wall::Root, Wall::Gap, Wall::Salt,
            Wall::Crystal, Wall::Fossil, Wall::Soot, Wall::WetStone, Wall::Basalt,
            Wall::Ashlar, Wall::Ironclad,
        ];
        for w in walls {
            let _ = w.shade(10, 0);
            let _ = w.shade(10, u32::MAX - 7);
            let _ = w.shade(u32::MAX - 11, u32::MAX - 3);
            assert!(w.shade(8, 16).is_finite());
        }
    }
}
