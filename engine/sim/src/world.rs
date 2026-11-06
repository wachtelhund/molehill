//! World = grid + structure + light + colony (moles, jobs, items, stations, chests, quests).
use crate::craft::*;
use crate::gen::{self, Gen, GenInfo};
use crate::grid::{hash3, Grid, F_SEEN, F_WET};
use crate::light::{self, Light};
use crate::mat::{Kind, Mat, CELL_M, CPM, NMAT, S, SF, BUILT, GRUBS, NOTABLE, OPEN, SOIL};
use crate::stress::Stress;
use crate::logic::{Dev, Logic};
use crate::disasters::Disasters;
use crate::air::{self, Air};
use std::collections::VecDeque;
/// Of every 10 cells of ground that crumble, how many leave an extra cell of rubble (bulking).
pub const BULK_IN_10: u32 = 3;
/// Half-size of the active region around the mole where the falling sand runs (64 m × 48 m each way).
pub const ACTIVE_HX: usize = 64 * CPM; pub const ACTIVE_HY: usize = 48 * CPM;
/// Root-trees: a regrowth sweep every TREE_EVERY ticks; each cut trunk column grows a cell in 1 of
/// TREE_GROW_1_IN sweeps (a 3 m trunk grows back in about three minutes), while its seep is wet.
pub const TREE_EVERY: u64 = 120; pub const TREE_GROW_1_IN: u32 = 3;
/// Glowmites: how many drift in a grotto, and how long (ticks) a caught one takes to be replaced.
pub const GLOWMITES_MIN: u32 = 4; pub const GLOWMITE_BACK: u64 = 5 * 60 * 60;
/// Rows the window slides by when the colony nears its edge (32 m).
pub const SLIDE: usize = 32 * CPM;
/// Cells of a placed block one pack item makes (a legacy tile is S×S cells, and an item made 4 of them).
pub const CELLS_PER_BLOCK: u16 = 4 * (S * S) as u16;
/// Lava cooling sweep period (ticks), and the chance per sweep (1 in N) that a thin bit of lava freezes.
pub const LAVA_COOL_EVERY: u64 = 30; pub const LAVA_COOL_1_IN: u32 = 24;
use std::sync::atomic::Ordering::Relaxed;

pub const D_DIG: u8 = 1; pub const D_WOOD: u8 = 2; pub const D_BRICK: u8 = 3; pub const D_IRON: u8 = 4;
/// Decorative blocks: planks, polished stone, tiles, carved pillar.
pub const D_PLANK: u8 = 5; pub const D_POLISH: u8 = 6; pub const D_TILE: u8 = 7; pub const D_CARVED: u8 = 8;
/// Transport blocks: rail, pressure tube.
pub const D_RAIL: u8 = 9; pub const D_TUBE: u8 = 10;
/// The build code for a block item (structural or decorative).
pub fn block_code(it: Item) -> Option<u8> { Some(match it { Item::WoodPillar => D_WOOD, Item::Brick => D_BRICK, Item::IronBeam => D_IRON, Item::Planks => D_PLANK, Item::PolishedStone => D_POLISH, Item::Tiles => D_TILE, Item::CarvedPillar => D_CARVED, Item::Rail => D_RAIL, Item::Tube => D_TUBE, _ => return None }) }
/// The fewest cells crumbling in one tick that sound as a collapse (`Event::Crumble`): two tiles' worth.
pub const CRUMBLE_LOUD: usize = 2 * S * S;
pub const MOLE_H: usize = 3 * S;
const REACH: i32 = 3 * S as i32;

#[derive(Clone, Debug, PartialEq)]
pub enum Job { Dig, Build, Haul(usize), Deliver, Craft(usize), Station(usize), Flee, Eat, Sleep }

#[derive(Clone)]
pub struct Mole {
    pub name: &'static str, pub x: usize, pub y: usize, pub hp: f32, pub breath: f32,
    pub path: VecDeque<(usize, usize)>, pub job: Option<Job>, pub work: u32, pub work_cell: Option<usize>,
    pub carry: Option<(Item, u32)>, pub hunger: f32, pub sleep: f32, pub facing: i8, pub fall: u32, pub idle: u32, pub anim: u32, pub alive: bool,
}
/// What the player mole has on: the fungus-oil headlamp (on/off), what's in its paws, and where it's
/// looking (cell coords of the cursor, if any). The game sets this every frame; it survives pauses.
#[derive(Clone, Default, Debug)]
pub struct Gear { pub lamp: bool, pub held: Option<Item>, pub aim: Option<(f32, f32)> }
/// What the player is pressing this tick (the game fills it in; the sim never reads the keyboard).
#[derive(Clone, Default, Debug)]
pub struct Input { pub left: bool, pub right: bool, pub jump: bool, pub up: bool, pub down: bool, /// held: run faster (`TUNE.sprint`), costs a little food
    pub sprint: bool, pub mine: Option<(usize, usize)>, /// dig tool in hand: 0 claws (pack tools), 1 pickaxe, 2 drill, 3 bore
    pub tool: u8,
    /// brush radius (cells) the player picked for the item in hand; 0 = that item's default. Always clamped by `brush_range`.
    pub brush: f32 }
/// The player mole's platformer state (cells; velocities in cells/s).
#[derive(Clone, Default, Debug)]
pub struct Player { pub px: f32, pub py: f32, pub ppx: f32, pub ppy: f32, pub sync: Option<(usize, usize)>, pub coyote: f32, pub buffer: f32, pub lock: f32, pub cling: i8, pub jump_was: bool, pub vx: f32, pub vy: f32, pub ax: f32, pub ay: f32, pub grounded: bool, pub fall_from: usize, pub mine: Option<(usize, u32, u32)>, pub spawn: (usize, i64), pub dead_t: u32, pub deaths: u32,
    /// hit flash (s left) after taking a knock; `hp_was` = hp after the last tick, to spot the knock
    pub flash: f32, pub hp_was: f32,
    /// the material a dig was refused on during this press (too hard for the tools owned): clinks once per
    /// press and material, not per cell swept over; cleared on release or when a bite lands
    pub deny: Option<Mat>,
    /// the last dig refusal said aloud and its tick: the same words wait `DENY_REPEAT` ticks before repeating
    pub deny_said: Option<(String, u64)>,
    /// this press was already told the spot is out of reach (once per press)
    pub far: bool,
    /// the last "can't place" line said and its tick: the same words wait `DENY_REPEAT` ticks (see `refuse_place`)
    pub place_said: Option<(String, u64)>,
    /// a bite under the feet was held back this press because the floor rang hollow over a deadly drop
    /// (`fatal_drop`), and the tick that was last said aloud (the words wait `DENY_REPEAT` ticks)
    pub hollow: bool, pub hollow_said: Option<u64>,
    /// wall-jump grace (s left) after leaving a cling, and the side that wall was on
    pub wall_t: f32, pub wall_side: i8,
    /// rolling over a ledge after a climb (s left; direction = `wall_side`): carried onto the top, no gravity
    pub mantle: f32,
    /// "hungry and nothing to eat" was said: once until the next meal
    pub hungry_said: bool }
/// Said when a bite under the feet is held back over a deadly drop (`World::fatal_drop`).
/// Floor thinner than this (cells) over a deadly drop rings hollow: a bite that would leave less is held back.
pub const HOLLOW_FLOOR: usize = CPM;
pub const HOLLOW: &str = "The floor rings hollow: a deadly drop below. Dig beside it to look first.";
/// Said when a bite under the feet is held back over lava.
pub const HOT: &str = "The floor is hot under your claws: lava below. Dig beside it to look first.";
/// Ticks before the same "too hard" words are said again (the clink still sounds on each press).
pub const DENY_REPEAT: u64 = 300;
pub const REACH_CELLS: f32 = 4.5 * CPM as f32; // 4.5 m
/// Mining brush radius (cells) per dig tool: claws, pickaxe, drill, bore. A pickaxe bite is about a mole tall.
pub const BITES: [f32; 4] = [1.8 * SF, 2.5 * SF, 3.2 * SF, 4.2 * SF];
/// What a round brush is attached to: a dig tool tier (0 claws .. 3 bore) or a placeable block item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Brush { Dig(u8), Place(Item) }
/// Brush radius range in cells for an item: (min, max, default). Bigger tools allow bigger bites; a radius
/// of 0.5 is one cell, 1.0 a plus of five. Defaults: dig tools keep their `BITES`; pillars and beams place one cell, bricks a small plus.
pub fn brush_range(b: Brush) -> Option<(f32, f32, f32)> {
    Some(match b {
        Brush::Dig(t) => match t { 0 => (0.5 * SF, 1.8 * SF, BITES[0]), 1 => (0.5 * SF, 3.0 * SF, BITES[1]), 2 => (0.5 * SF, 4.0 * SF, BITES[2]), _ => (1.0 * SF, 5.5 * SF, BITES[3]) },
        Brush::Place(Item::WoodPillar) => (0.5 * SF, 3.0 * SF, 0.5 * SF),
        Brush::Place(Item::Brick) => (0.5 * SF, 3.5 * SF, 1.0 * SF),
        Brush::Place(Item::IronBeam) => (0.5 * SF, 2.0 * SF, 0.5 * SF),
        Brush::Place(Item::Planks | Item::Tiles | Item::PolishedStone) => (0.5 * SF, 3.5 * SF, 1.0 * SF),
        Brush::Place(Item::CarvedPillar) => (0.5 * SF, 2.0 * SF, 0.5 * SF),
        Brush::Place(it) if it.is_wall() => crate::walls::wall_brush_range(),
        // a pigment paints a disc of cells
        Brush::Place(it) if it.pigment().is_some() => (0.5 * SF, 4.0 * SF, 1.5 * SF),
        _ => return None,
    })
}
/// The radius actually used: the requested one (0 = default), clamped to the item's range.
pub fn brush_radius(b: Brush, want: f32) -> f32 {
    match brush_range(b) { Some((lo, hi, def)) => if want <= 0. { def } else { want.clamp(lo, hi) }, None => 0.5 * SF }
}
/// Cells (dx, dy) of a round brush of radius r, nearest the centre first.
pub fn disc(r: f32) -> Vec<(i32, i32)> {
    let k = r.ceil() as i32; let mut v = vec![];
    for dy in -k..=k { for dx in -k..=k { if (dx * dx + dy * dy) as f32 <= r * r { v.push((dx, dy)); } } }
    v.sort_by_key(|&(dx, dy)| dx * dx + dy * dy); v
}
/// Player body (cells): an AABB centred on px, feet at py.
pub const BODY_HW: f32 = 0.9 * SF; pub const BODY_H: f32 = 2.9 * SF;
/// Movement tuning (cells, seconds). One place to tweak the feel.
#[derive(Clone, Copy, Debug)]
pub struct Tune { pub run: f32, pub ground_accel: f32, pub ground_decel: f32, pub air_accel: f32, /// air drag with no direction held (keeps wall-jump and run-off momentum)
    pub air_decel: f32, pub gravity: f32, pub fall_mul: f32, pub cut_mul: f32, pub jump_v: f32, pub terminal: f32,
    pub coyote: f32, pub buffer: f32, pub climb: f32, pub slide: f32, pub wall_jump_x: f32, pub wall_jump_y: f32, pub wall_lock: f32, pub step: f32, /// sprint speed multiplier (Shift); climb acceleration (cells/s²); speed when rolling over a ledge
    pub sprint: f32, pub climb_accel: f32, pub mantle_v: f32, /// corner correction (cells): head slides round corners, feet slot into tunnel mouths
    pub nudge: f32,
    pub swim_mul: f32, pub swim_grav: f32, pub buoyancy: f32, pub swim_up: f32, pub swim_max: f32, pub lava_dps: f32, pub safe_fall: usize,
    pub lava_mul: f32, pub lava_sink: f32, pub lava_up: f32 }
/// Lengths are written in legacy 0.25 m tiles times SF, so the feel is the same in metres at any CPM.
pub const TUNE: Tune = Tune { run: 15. * SF, ground_accel: 900. * SF, ground_decel: 1400. * SF, air_accel: 480. * SF, air_decel: 60. * SF, gravity: 150. * SF, fall_mul: 1.7, cut_mul: 3.0, jump_v: 42. * SF, terminal: 60. * SF,
    coyote: 0.1, buffer: 0.1, climb: 10.5 * SF, slide: 3. * SF, wall_jump_x: 17. * SF, wall_jump_y: 36. * SF, wall_lock: 0.13, step: 2. * SF, sprint: 1.6, climb_accel: 240. * SF, mantle_v: 9. * SF, nudge: 0.6 * SF,
    swim_mul: 0.55, swim_grav: 0.35, buoyancy: 40. * SF, swim_up: 70. * SF, swim_max: 11. * SF, lava_dps: 75., safe_fall: 20 * S,
    lava_mul: 0.25, lava_sink: 3. * SF, lava_up: 22. * SF }; // lava: thick, you wade and slowly sink, and it burns fast
#[derive(Clone)]
/// A loose item: a small physics body (see `items.rs`). `x`, `y` = the cell under its centre.
pub struct Drop { pub x: usize, pub y: usize, pub item: Item, pub n: u32, pub claimed: bool,
    /// centre (cells), last tick's centre (for render interpolation), velocity (cells/s)
    pub px: f32, pub py: f32, pub ppx: f32, pub ppy: f32, pub vx: f32, pub vy: f32,
    /// ticks it has lain still (at `REST_TICKS` it sleeps); ticks before a mole may pick it up; a lit
    /// charge's fuse (ticks left, 0 = not lit)
    pub rest: u8, pub hold: u16, pub fuse: i16 }
/// A mushroom plot: the open cell where the mushroom stands, on soil. `growth` 0..1, `why` = what's
/// holding it back (bit 1 too bright, bit 2 too dry).
#[derive(Clone)]
pub struct Farm { pub x: usize, pub y: usize, pub growth: f32, pub why: u8 }
#[derive(Clone)]
pub struct Chest { pub x: usize, pub y: usize, pub opened: bool,
    /// an Ancient vault's chest (its ruin's id): relics and old materials instead of the usual loot
    pub ancient: Option<u32> }
/// A piece of furniture: its item, the footprint's left column and bottom row (cells), and the pigment
/// of its cloth (0 = undyed). It isn't solid: the mole walks past it.
#[derive(Clone, Debug, PartialEq)]
pub struct Furniture { pub item: Item, pub x: usize, pub y: usize, pub tint: u8 }
impl Furniture {
    /// Footprint in cells: (x0, y0, x1, y1) inclusive.
    pub fn rect(&self) -> (usize, usize, usize, usize) { let f = self.item.furn().unwrap_or(crate::craft::Furn { w: 1, h: 1, hang: false, cloth: false }); (self.x, self.y + 1 - f.h * S, self.x + f.w * S - 1, self.y) }
    pub fn contains(&self, x: usize, y: usize) -> bool { let (x0, y0, x1, y1) = self.rect(); x >= x0 && x <= x1 && y >= y0 && y <= y1 }
}
#[derive(Clone)]
pub struct Building { pub kind: Station, pub x: usize, pub y: usize, pub built: bool, pub claimed: bool }

pub struct World {
    pub g: Grid, pub st: Stress, pub light: Light, pub info: GenInfo, pub tick: u64,
    pub inv: [u32; NITEM], dig_acc: [u16; NMAT], place_acc: [u16; 12], pub desig: Vec<u8>,
    pub moles: Vec<Mole>, pub drops: Vec<Drop>, pub stockpile: (usize, usize, usize, usize),
    pub buildings: Vec<Building>, pub torches: Vec<(usize, usize)>, pub chests: Vec<Chest>, pub furniture: Vec<Furniture>, paint_acc: u16,
    pub quest: usize, pub log: Vec<(u64, String)>, pub discovered: [bool; NMAT], pub chests_opened: u32, pub pillars: u32,
    pub max_depth: f32, pub craft_queue: Vec<usize>, pub gear: Gear,
    /// Items that have ever been in the pack: a recipe appears once any of its inputs is known.
    pub known: [bool; NITEM], pub crumbled: u64, pub light_rect: (usize, usize, usize, usize),
    pub events: Vec<Event>, pub react_pending: bool,
    /// where this tick's feedback happened, for in-world pops: (kind, x, y, radius in cells); kind 0 = a bite
    /// cleared, 1 = a block placed. Cleared every step like `events`.
    pub marks: Vec<(u8, f32, f32, f32)>,
    pub logic: Logic, pub disasters: Disasters, pub shake: f32,
    pub air: Air, pub farms: Vec<Farm>, pub harvested: u32,
    /// Endless vertical world: the grid is a window; `oy` = absolute row of window row 0.
    pub input: Input, pub pl: Player,
    pub gen: Gen, pub oy: i64,
    /// Bands that left the window, by the world row they start at (see `band.rs`): everything in them, packed.
    pub bands: std::collections::BTreeMap<i64, crate::band::Stored>, pub opened: std::collections::HashSet<(usize, i64)>,
    /// The Lid thread (`lore.rs`): clues in the window, the ones found (x, absolute row), and the journal (fragment ids, in the order read).
    pub clues: Vec<crate::lore::Clue>, pub found_clues: std::collections::HashSet<(usize, i64)>, pub journal: Vec<u16>,
    /// grottos touching the window (from the generator; refreshed with the chests)
    pub grottos: Vec<gen::Grotto>,
    /// keep the last blast's overpressure field (kPa per cell) in `blast_field` (tests and the harness)
    pub keep_blast_field: bool, pub blast_field: Vec<f32>,
    /// cells a blast threw, in flight (see `debris.rs`), and the recent blasts' waves (for drawing)
    pub debris: Vec<crate::debris::Debris>, pub waves: Vec<crate::debris::Wave>,
    /// carts and the rest of transport (see `transport.rs`)
    pub tr: crate::transport::Transport,
    /// blasts run since the world was made (diagnostics and tests: one charge, one blast)
    pub blasts_run: u64,
    /// glowmites caught: (grotto id, glowmite) → tick. A grotto's swarm refills: each comes back after GLOWMITE_BACK.
    pub caught: std::collections::HashMap<(u32, u8), u64>,
    /// Survival, Creative, or Plain (the bare rules: tests and scenarios); `rules` is what the mode switches on
    /// (see `survival.rs`). `lamp_oil` = seconds of oil left in the headlamp.
    pub mode: crate::survival::Mode, pub rules: crate::survival::Rules, pub lamp_oil: f32, pub lamp_dry: bool, /** the low-oil warning has been given for this measure */ pub lamp_low: bool,
    /// ruins of the Ancients touching the window (refreshed with the chests); the ones already visited (ids);
    /// relics found (bit k = `ruins::RELICS[k]`): each turns up once
    pub ruins: Vec<crate::ruins::Ruin>, pub ruins_seen: std::collections::HashSet<u32>, pub relics: u8,
    /// decorative background walls (see `walls.rs`): one byte per window cell, never in the CA
    pub walls: crate::walls::Walls,
    /// sites touching the window (refreshed with the chests); the ones already visited (ids)
    pub sites: Vec<crate::sites::Site>, pub sites_seen: std::collections::HashSet<u32>,
}
/// For audio / UI feedback.
/// What `World::pick_up` took back into the pack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Picked { Dev(Dev), Torch, Wire, Furniture(Item), Transport(Item) }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event { Dig, Crumble, Crack, Place, Chest, Quest, Explode, Discover, Hurt, Craft, Eat, Death, Warning, Quake, Click, Harvest, Gasp, Lore }

const NAMES: [&str; 8] = ["Mossy", "Pip", "Bramble", "Tuck", "Clover", "Dusty", "Fern", "Nib"];

impl World {
    /// A world shell for a seed: generator and info, one player mole, but no rows generated (all empty).
    /// `save::load` fills it from bands, which skips the slow generation of a window it would overwrite.
    pub fn blank(w: usize, h: usize, seed: u32) -> World {
        let oy = -(h as i64 / 2); let (gn, info) = gen::info_only(seed, w, h);
        let mut wd = World::shell(Grid::new(w, h), w, h, seed, oy, gn, info);
        wd.moles.push(Mole { name: NAMES[0], x: w / 2, y: h / 2, hp: 100., breath: 100., path: VecDeque::new(), job: None, work: 0, work_cell: None, carry: None, hunger: 80., sleep: 70., facing: 1, fall: 0, idle: 0, anim: 0, alive: true });
        wd.pl.spawn = (w / 2, 0); wd
    }
    fn shell(g: Grid, w: usize, h: usize, seed: u32, oy: i64, gn: Gen, info: gen::GenInfo) -> World {
        World {
            st: Stress::new(w * h), light: Light::new(w, h), tick: 0, inv: [0; NITEM], dig_acc: [0; NMAT], place_acc: [0; 12], desig: vec![0; w * h],
            moles: vec![], drops: vec![], stockpile: (0, 0, 0, 0), buildings: vec![], torches: vec![], chests: vec![], furniture: vec![], paint_acc: 0,
            quest: 0, log: vec![], discovered: [false; NMAT], chests_opened: 0, pillars: 0, max_depth: 0., craft_queue: vec![], gear: Gear::default(), known: [false; NITEM], mode: Default::default(), rules: Default::default(), lamp_oil: 0., lamp_dry: false, lamp_low: false, ruins: vec![], ruins_seen: Default::default(), relics: 0, crumbled: 0,
            light_rect: (0, 0, w - 1, h - 1), events: vec![], marks: vec![], react_pending: false, logic: Logic::new(w, h), disasters: Disasters::new(seed), shake: 0., air: Air::new(w, h), farms: vec![], harvested: 0, input: Input::default(), pl: Player::default(), gen: gn, oy, bands: Default::default(), opened: Default::default(), clues: vec![], found_clues: Default::default(), journal: vec![], grottos: vec![], caught: Default::default(), keep_blast_field: false, blast_field: vec![], debris: vec![], waves: vec![], tr: Default::default(), blasts_run: 0, info, g,
            walls: crate::walls::Walls::new(w, h), sites: vec![], sites_seen: Default::default(),
        }
    }
    pub fn new(w: usize, h: usize, seed: u32) -> World {
        let mut g = Grid::new(w, h);
        let oy = -(h as i64 / 2); let (gn, info) = gen::generate(&mut g, seed, oy);
        let mut wd = World::shell(g, w, h, seed, oy, gn, info);
        wd.refresh_chests();
        wd.line_back(0, h);
        wd.line_burrow();
        let (sx, sy) = (wd.info.start.0 * CPM, wd.info.start.1 * CPM);
        // floor of the start chamber: the first support below
        let mut fy = sy.min(h - 6 * S); while fy < h - 5 * S && !wd.g.is_solidish(wd.g.idx(sx, fy + 1)) { fy += 1; }
        wd.stockpile = (sx + 6 * S, fy - 2 * S, sx + 16 * S, fy);
        for k in 0..1 { wd.moles.push(Mole { name: NAMES[k], x: sx, y: fy, hp: 100., breath: 100., path: VecDeque::new(), job: None, work: 0, work_cell: None, carry: None, hunger: 80. + k as f32 * 4., sleep: 70. + k as f32 * 7., facing: 1, fall: 0, idle: 0, anim: k as u32 * 7, alive: true }); }
        wd.pl.spawn = (sx, wd.oy + fy as i64); wd.pl.fall_from = fy;
        light::reveal_box(&mut wd.g, sx - 60 * S, fy.saturating_sub(42 * S), sx + 60 * S, (fy + 52 * S).min(h - 1)); // the colony knows its neighbourhood
        { let b = wd.logic.block_of(sx, fy - 5 * S); let gg = &mut wd.g; wd.logic.place(gg, Dev::Vent, b); } // the burrow's air shaft
        wd.torches.push((sx - 12 * S, fy - 2 * S)); wd.torches.push((sx + 12 * S, fy - 2 * S));
        for (it, n) in [(Item::Wood, 8), (Item::Torch, 6), (Item::WoodPillar, 12), (Item::Grub, 12), (Item::CopperWire, 40), (Item::Gate, 6), (Item::Thrummer, 3), (Item::Spore, 6), (Item::Vent, 1)] { wd.inv[it as usize] = n; }
        for k in 0..NMAT { if wd.g.mat.iter().zip(&wd.g.flags).any(|(&m, &f)| m as usize == k && f & F_SEEN != 0) { wd.discovered[k] = true; } }
        // settle the structure once so crack timers start from the real state
        let f = wd.st.solve(&wd.g); wd.st.crack(&f); wd.g.clear_struct();
        wd.say("A lone mole wakes in the burrow. Seed ".to_string() + &seed.to_string());
        wd
    }
    pub fn say(&mut self, s: String) { self.log.push((self.tick, s)); if self.log.len() > 60 { self.log.remove(0); } }
    pub fn depth_m(&self, _x: usize, y: usize) -> f32 { (self.oy + y as i64) as f32 * CELL_M }
    /// Chests for every chest band overlapping the window (opened ones stay opened).
    pub fn refresh_chests(&mut self) {
        self.grottos = self.gen.grottos_in_rows(self.oy, self.oy + self.g.h as i64);
        self.ruins = self.gen.ruins_in_rows(self.oy, self.oy + self.g.h as i64);
        let h = self.g.h as i64; self.chests.clear(); let bh = gen::CHEST_BAND as i64; let mg = 4 * S as i64;
        for band in self.oy.div_euclid(bh)..=(self.oy + h).div_euclid(bh) {
            for (x, wy) in self.gen.chests_in_band(band, self.g.w) {
                let y = wy - self.oy; if y < mg || y >= h - mg { continue; }
                self.chests.push(Chest { x, y: y as usize, opened: self.opened.contains(&(x, wy)), ancient: None });
            }
        }
        for r in &self.ruins { if let Some((x, wy)) = r.chest { let y = wy - self.oy; if y < mg || y >= h - mg { continue; }
            self.chests.push(Chest { x, y: y as usize, opened: self.opened.contains(&(x, wy)), ancient: Some(r.id) }); } }
        self.refresh_sites();
        for s in &self.sites { if let Some((x, wy)) = s.chest { let y = wy - self.oy; if y < mg || y >= h - mg { continue; }
            if self.chests.iter().any(|c| c.x == x && c.y as i64 == y) { continue; }
            self.chests.push(Chest { x, y: y as usize, opened: self.opened.contains(&(x, wy)), ancient: None }); } }
        self.refresh_clues();
    }
    /// The clues in the window, from the seed (like chests); the ones already found stay found.
    pub fn refresh_clues(&mut self) {
        let h = self.g.h as i64; let bh = crate::lore::LORE_BAND as i64; let mg = 4 * S as i64; self.clues.clear();
        for band in self.oy.div_euclid(bh)..=(self.oy + h).div_euclid(bh) {
            for (kind, x, wy, look) in crate::lore::clues_in_band(&self.gen, self.gen.seed, band, self.g.w) {
                let y = wy - self.oy; if y < mg || y >= h - mg { continue; }
                self.clues.push(crate::lore::Clue { kind, x, y: y as usize, wy, found: self.found_clues.contains(&(x, wy)), look });
            }
        }
    }
    /// A Delver who comes within about a metre of a clue (in the open, not buried) reads it: the next
    /// fragment of its kind goes into the journal. Once all of a kind are read, more clues are just noted.
    fn check_clues(&mut self) {
        let r = CPM as i64 + S as i64;
        for k in 0..self.clues.len() {
            let c = &self.clues[k]; if c.found || self.g.is_solidish(c.y * self.g.w + c.x) { continue; }
            let near = self.moles.iter().any(|m| m.alive && (m.x as i64 - c.x as i64).abs() <= r && (m.y as i64 - c.y as i64).abs() <= r + 2 * S as i64);
            if !near { continue; }
            let (kind, x, wy) = (c.kind, c.x, c.wy); self.clues[k].found = true; self.found_clues.insert((x, wy));
            match crate::lore::next_fragment(kind, &self.journal) {
                Some(f) => { self.journal.push(f); self.say(format!("Journal: {} (J)", crate::lore::FRAGMENTS[f as usize].title)); }
                None => self.say(format!("Another {}, like the others.", kind.name().to_lowercase())),
            }
            self.events.push(Event::Lore);
        }
    }
    /// A ruin's carving, read on the first visit, joins the journal beside the clue fragments (once per
    /// carving: two ruins with the same carving add it once).
    pub(crate) fn journal_ruin(&mut self, text: &str) {
        let Some(id) = crate::lore::ruin_entry(text) else { return };
        if self.journal.contains(&id) { return; }
        self.journal.push(id); self.say(format!("Journal: {} (J)", crate::lore::entry(id).1));
    }
    /// Slide the window by `rows` (a multiple of `band::BH`; + = deeper). Rows leaving are packed into
    /// bands with everything in them (devices, torches, drops, stations, plots, air); rows entering are
    /// unpacked from their band, or generated if never visited. Everything with a y coordinate moves.
    pub fn shift(&mut self, rows: i64) {
        use crate::band::{self, BH};
        let (w, h) = (self.g.w, self.g.h); let s = rows.unsigned_abs() as usize; if s == 0 || s >= h || s % BH != 0 || h % BH != 0 { return; }
        for c in &self.chests { if c.opened { self.opened.insert((c.x, self.oy + c.y as i64)); } }
        let (lv, en) = if rows > 0 { (0, h - s) } else { (h - s, 0) }; // first window row leaving / entering (after the move)
        for y0 in (lv..lv + s).step_by(BH) { let blob = band::encode(self, y0); self.bands.insert(self.oy + y0 as i64, band::Stored::new(blob)); }
        let d = s * w;
        if rows > 0 {
            self.g.mat.copy_within(d.., 0); self.g.flags.copy_within(d.., 0); self.g.shade.copy_within(d.., 0); self.desig.copy_within(d.., 0); self.walls.cell.copy_within(d.., 0);
        } else {
            self.g.mat.copy_within(..w * h - d, d); self.g.flags.copy_within(..w * h - d, d); self.g.shade.copy_within(..w * h - d, d); self.desig.copy_within(..w * h - d, d); self.walls.cell.copy_within(..w * h - d, d);
        }
        self.oy += rows; self.st.shift_rows(rows, w);
        for y in en..en + s { for x in 0..w { self.desig[y * w + x] = 0; self.walls.cell[y * w + x] = 0; } }
        // entities and block layers move with the rock; what was in the leaving rows is in its band now
        let dy = |y: usize| -> Option<usize> { let v = y as i64 - rows; if v >= 0 && v < h as i64 { Some(v as usize) } else { None } };
        for m in self.moles.iter_mut() { if let Some(v) = dy(m.y) { m.y = v; } else { m.alive = false; } m.path.clear(); m.job = None; m.work_cell = None; }
        self.drops = std::mem::take(&mut self.drops).into_iter().filter_map(|mut d| dy(d.y).map(|v| { d.y = v; d.py -= rows as f32; d.ppy -= rows as f32; d })).collect();
        self.shift_transport(rows, h);
        self.debris = std::mem::take(&mut self.debris).into_iter().filter_map(|mut d| dy(d.py.max(0.) as usize).map(|_| { d.py -= rows as f32; d.ppy -= rows as f32; d })).collect(); self.waves.clear();
        self.torches = std::mem::take(&mut self.torches).into_iter().filter_map(|(x, y)| dy(y).map(|v| (x, v))).collect();
        self.buildings = std::mem::take(&mut self.buildings).into_iter().filter_map(|mut b| dy(b.y).filter(|&v| v >= Station::H).map(|v| { b.y = v; b.claimed = false; b })).collect();
        self.furniture = std::mem::take(&mut self.furniture).into_iter().filter_map(|mut f| dy(f.y).filter(|&v| v >= 16 * S).map(|v| { f.y = v; f })).collect();
        self.farms = std::mem::take(&mut self.farms).into_iter().filter_map(|mut f| dy(f.y).map(|v| { f.y = v; f })).collect();
        let sp = self.stockpile; if let (Some(a), Some(b)) = (dy(sp.1), dy(sp.3)) { self.stockpile = (sp.0, a, sp.2, b); }
        self.logic.shift_rows(rows / crate::logic::B as i64);
        self.air.shift_rows(rows / air::AB as i64);
        // entering rows: generate the never-visited ones, then unpack the stored bands over them
        let fresh: Vec<usize> = (en..en + s).step_by(BH).filter(|&y0| !self.bands.contains_key(&(self.oy + y0 as i64))).collect();
        if let (Some(&a), Some(&b)) = (fresh.first(), fresh.last()) { let b = b + BH; self.gen.fill_rows(&mut self.g, self.oy, a, b); let (gn, oy) = (&self.gen, self.oy); let m = 32 * CPM; // the old rows next to the new ones may now roof a fresh cave: stabilise into them too
            gen::stabilize_soft(&mut self.g, &mut self.st, a.saturating_sub(m), (b + m).min(h), &|x, y| gn.in_grotto_void(x, oy + y as i64), &|x, y| gen::in_start_zone(gn.biome.len(), x, oy + y as i64)); }
        for y0 in (en..en + s).step_by(BH) {
            if let Some(b) = self.bands.remove(&(self.oy + y0 as i64)) {
                match band::decode(self, y0, &b.blob, None) {
                    Ok(true) => {}
                    Ok(false) => self.line_back(y0, y0 + BH), // older blob: no wall bytes
                    Err(_) => { self.gen.fill_rows(&mut self.g, self.oy, y0, y0 + BH); self.line_back(y0, y0 + BH); }
                }
            } else {
                self.line_back(y0, y0 + BH);
            }
        }
        self.disasters.y = (self.disasters.y as i64 - rows).clamp(8, h as i64 - 8) as usize;
        self.refresh_chests();
        self.g.life.iter_mut().for_each(|v| *v = 0); self.g.wake_all();
        // structure: the solver's state moved with the rock, so only the entering rows (plus 16 m into the
        // old ones: built supports can cross the band edge) and the edge the window left (its anchors and
        // overburden change) are re-solved
        let m = 16 * crate::mat::CPM;
        let (ent, far) = if rows > 0 { ((0, en.saturating_sub(m), w - 1, h - 1), (0, 0, w - 1, m)) } else { ((0, 0, w - 1, (en + s + m).min(h - 1)), (0, h - 1 - m, w - 1, h - 1)) };
        let mut f = self.st.solve_rect(&self.g, Some(ent)); f.extend(self.st.solve_rect(&self.g, Some(far)));
        self.st.crack(&f); self.g.clear_struct();
        self.light = Light::new(w, h);
        self.pl.mine = None; if let Some(m) = self.moles.first() { self.pl.fall_from = m.y; }
    }
    /// Cells (materials, flags) of the stored band starting at world row `wy0`, for drawing far rows.
    pub fn stored_cells(&self, wy0: i64) -> Option<(Vec<u8>, Vec<u8>)> { self.bands.get(&wy0).and_then(|b| crate::band::cells(&b.blob, self.g.w)) }
    /// Keep the colony inside the window: slide when living moles crowd the top or bottom.
    fn follow_colony(&mut self) {
        let h = self.g.h; let ys: Vec<usize> = self.moles.iter().filter(|m| m.alive).map(|m| m.y).collect(); if ys.is_empty() { return; }
        let (lo, hi) = (*ys.iter().min().unwrap(), *ys.iter().max().unwrap());
        // margin m from either edge triggers a SLIDE-row slide; m ≤ (h-SLIDE)/2 guarantees the slide
        // lands the colony inside [m, h-m], so it can't immediately trigger the opposite slide
        // (with m = 100 a 256-row window thrashed back and forth every 30 ticks)
        let m = (100 * S).min(h.saturating_sub(SLIDE) / 2);
        if hi > h - m && lo >= m + SLIDE { self.shift(SLIDE as i64); } else if lo < m && hi + SLIDE <= h - m { self.shift(-(SLIDE as i64)); }
    }

    #[inline] pub(crate) fn passable(&self, x: usize, y: usize) -> bool { let m = self.g.get(x, y); m.passable() && !self.is_building_cell(x, y, false) }
    fn is_building_cell(&self, _x: usize, _y: usize, _solid: bool) -> bool { false }
    pub fn standable(&self, x: usize, y: usize) -> bool {
        let g = &self.g; if x < 1 || x + 1 >= g.w || y < MOLE_H || y + 1 >= g.h { return false; }
        for k in 0..MOLE_H { if !self.passable(x, y - k) { return false; } }
        if g.is_solidish(g.idx(x, y + 1)) { return true; }
        for k in 0..MOLE_H { if g.is_static(g.idx(x - 1, y - k)) || g.is_static(g.idx(x + 1, y - k)) { return true; } }
        g.is_static(g.idx(x, y - MOLE_H))
    }
    #[allow(dead_code)] fn bfs(&self, from: (usize, usize), max: usize, goal: impl Fn(usize, usize) -> bool) -> Option<VecDeque<(usize, usize)>> {
        let w = self.g.w; let mut prev: std::collections::HashMap<usize, usize> = Default::default();
        let s = from.1 * w + from.0; prev.insert(s, s); let mut q = VecDeque::from([s]);
        while let Some(c) = q.pop_front() {
            let (x, y) = (c % w, c / w);
            if goal(x, y) { let mut p = VecDeque::new(); let mut k = c; while k != s { p.push_front((k % w, k / w)); k = prev[&k]; } return Some(p); }
            if prev.len() > max { break; }
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, -1), (0, 1), (1, -1), (-1, -1), (1, 1), (-1, 1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy); if nx < 1 || ny < 1 || nx as usize >= w - 1 || ny as usize >= self.g.h - 1 { continue; }
                let n = ny as usize * w + nx as usize; if prev.contains_key(&n) { continue; }
                if !self.standable(nx as usize, ny as usize) { continue; }
                let step_up = dy < 0 && self.g.is_solidish(self.g.idx(nx as usize, y)) && y >= MOLE_H && self.passable(x, y - MOLE_H); // hop onto a one-cell ledge
                if dx != 0 && dy != 0 && !step_up && !(self.standable(nx as usize, y) || self.standable(x, ny as usize)) { continue; }
                prev.insert(n, c); q.push_back(n);
            }
        }
        None
    }
    /// Digging power: the tool in hand, else the best pack tools.
    pub fn dig_power(&self) -> u8 { if self.input.tool > 0 { self.tool() } else { tool_tier(&self.inv) } }
    /// The dig tool in hand (0 claws .. 3 bore); in Survival you can only use tools you own.
    pub fn tool(&self) -> u8 { let t = self.input.tool.min(3); if self.rules.tools { t.min(tool_tier(&self.inv)) } else { t } }
    fn tier_ok(&self, m: Mat) -> bool { m != Mat::Bedrock && m.p().tier <= self.dig_power().max(1).min(3) + if m.p().tier <= 1 { 1 } else { 0 } }
    fn diggable(&self, i: usize) -> bool { self.can_dig(self.g.m(i)) }
    /// Can the tools owned (or the pick in hand) dig this material at all?
    pub fn can_dig(&self, m: Mat) -> bool { matches!(m.kind(), Kind::Solid | Kind::Powder) && m != Mat::Bedrock && self.tier_ok(m) }
    /// Why a solid can't be dug with what the mole has, in a word for the log and the hover tip (None if it can).
    pub fn dig_refusal(&self, m: Mat) -> Option<String> {
        if m.kind() != Kind::Solid || self.can_dig(m) { return None; }
        if m == Mat::Bedrock { return Some("Bedrock. Nothing digs it.".into()); }
        Some(match self.come_back_with(m) { Some(t) => format!("{} is too hard for your {}: {t}.", m.p().name, self.tool_name()), None => format!("{} is too hard for your {}.", m.p().name, self.tool_name()) })
    }
    /// The dig tool the mole is using, in a word (claws, pickaxe, drill, bore).
    pub fn tool_name(&self) -> &'static str { ["claws", "pickaxe", "drill", "bore"][self.dig_power().min(3) as usize] }
    /// For rock the tools owned can't cut: what to come back with, and how it is made (None if it can be dug,
    /// or nothing ever digs it). Said by the refusal, the hover tip and the discovery line.
    pub fn come_back_with(&self, m: Mat) -> Option<&'static str> {
        if m.kind() != Kind::Solid || self.can_dig(m) || m == Mat::Bedrock { return None; }
        match m.p().tier { 2 => Some("come back with a drill (bronze, made at a powered workshop)"), 3 => Some("come back with a bore (iron, copper and crystal, made at the forge)"), _ => None }
    }
    #[allow(dead_code)] fn dig_target_near(&self, x: usize, y: usize) -> Option<usize> {
        let w = self.g.w; let mut best = None; let mut bd = i32::MAX;
        for dy in -REACH - 1..=REACH - 1 { for dx in -REACH..=REACH {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(nx, ny) { continue; }
            let i = ny as usize * w + nx as usize;
            if self.desig[i] == D_DIG && self.diggable(i) && self.g.n4(nx as usize, ny as usize).any(|j| !self.g.is_solidish(j)) {
                let d = dx * dx + (dy + 1) * (dy + 1); if d < bd { bd = d; best = Some(i); }
            }
        } }
        best
    }
    #[allow(dead_code)] fn build_target_near(&self, x: usize, y: usize) -> Option<usize> {
        let w = self.g.w;
        for dy in -REACH - 1..=REACH - 1 { for dx in -REACH..=REACH {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(nx, ny) { continue; }
            let (ux, uy) = (nx as usize, ny as usize); let i = uy * w + ux; let d = self.desig[i];
            if d >= D_WOOD && self.g.mat[i] == 0 && self.has_build_item(d) && (ux as i32 - x as i32).abs() + (uy as i32 - y as i32).abs() > 0
                && self.g.n4(ux, uy).any(|j| self.g.is_solidish(j)) && !self.moles.iter().any(|m| m.x == ux && m.y >= uy && m.y < uy + MOLE_H) { return Some(i); }
        } }
        None
    }
    #[allow(dead_code)] fn has_build_item(&self, d: u8) -> bool { let it = build_item(d); self.inv[it as usize] > 0 || self.place_acc[d as usize] > 0 }

    pub fn designate(&mut self, cx: usize, cy: usize, r: usize, code: u8) {
        let r2 = (r * r) as i32;
        for dy in -(r as i32)..=r as i32 { for dx in -(r as i32)..=r as i32 {
            if dx * dx + dy * dy > r2 { continue; }
            let (x, y) = (cx as i32 + dx, cy as i32 + dy); if !self.g.in_bounds(x, y) { continue; }
            let i = y as usize * self.g.w + x as usize;
            if code == 0 { self.desig[i] = 0; }
            else if code == D_DIG { if self.g.is_solidish(i) && self.g.m(i) != Mat::Bedrock { self.desig[i] = D_DIG; } }
            else if self.g.mat[i] == 0 || self.g.m(i).kind() != Kind::Solid { self.desig[i] = code; }
        } }
    }
    pub fn place_torch(&mut self, x: usize, y: usize) -> bool { self.try_place_torch(x, y).is_ok() }
    /// Set a torch down at (x, y), or say why not (none left, no room, no fungus oil to light it in Survival).
    pub fn try_place_torch(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if self.inv[Item::Torch as usize] == 0 { return Err("none left"); }
        if x >= self.g.w || y >= self.g.h || self.g.is_solidish(self.g.idx(x, y)) { return Err("something is in the way"); }
        if self.rules.fuel && !self.take_oil() { return Err("it needs fungus oil to burn"); }
        self.inv[Item::Torch as usize] -= 1; self.torches.push((x, y)); self.events.push(Event::Place); self.marks.push((1, x as f32 + 0.5, y as f32 + 0.5, 0.6 * SF)); Ok(())
    }
    /// A place or build was refused: say why in one short line ("Can't place brick: needs something to rest
    /// on."). The same words aren't said again for `DENY_REPEAT` ticks, so clicking at the same spot, or
    /// holding a brush over it, never stacks messages; a different reason speaks at once.
    /// A dig was refused: say why, unless the same words were said in the last `DENY_REPEAT` ticks.
    fn say_dig_no(&mut self, s: String) {
        if self.pl.deny_said.as_ref().is_some_and(|(t, k)| *t == s && self.tick < k + DENY_REPEAT) { return; }
        self.pl.deny_said = Some((s.clone(), self.tick)); self.say(s);
    }
    pub fn refuse_place(&mut self, what: &str, why: &str) {
        let what = if what.chars().nth(1).is_some_and(char::is_uppercase) { what.to_string() } else { what.to_lowercase() }; // TNT stays TNT
        let s = format!("Can't place {what}: {why}.");
        if self.pl.place_said.as_ref().is_some_and(|(t, k)| *t == s && self.tick < k + DENY_REPEAT) { return; }
        self.pl.place_said = Some((s.clone(), self.tick)); self.say(s);
    }
    pub fn place_building(&mut self, kind: Station, x: usize, y: usize) -> Result<(), &'static str> {
        if self.buildings.iter().any(|b| b.kind == kind) { return Err("already have one"); }
        for &(it, n) in kind.cost() { if self.inv[it as usize] < n { return Err("missing materials"); } }
        for yy in y + 1 - Station::H..=y { for xx in x..x + Station::W { if xx >= self.g.w || self.g.is_solidish(self.g.idx(xx, yy)) { return Err("needs open space"); } } }
        if !(x..x + Station::W).all(|xx| self.g.is_solidish(self.g.idx(xx, y + 1))) { return Err("needs a flat floor"); }
        for &(it, n) in kind.cost() { self.inv[it as usize] -= n; }
        self.buildings.push(Building { kind, x, y, built: true, claimed: false }); self.events.push(Event::Place); self.say(format!("Built a {}.", kind.name())); Ok(())
    }
    pub fn queue_craft(&mut self, r: usize) { if self.craft_queue.len() < 12 { self.craft_queue.push(r); } }
    /// A TNT charge going off at (x, y): a pressure wave (see `blast.rs`), the one place its size is set.
    pub fn blast_tnt(&mut self, x: usize, y: usize) { self.blast(x, y, crate::blast::TNT_KG); }

    pub fn step(&mut self) {
        self.tick += 1; self.events.clear(); self.marks.clear();
        self.learn();
        // active region: the falling sand runs only around the mole (the view at the widest zoom, with room
        // to spare); the rest of the window waits, frozen mid-flow, and catches up when the mole comes near
        self.g.active = self.moles.first().map(|m| { let c = crate::grid::CHUNK; let (hx, hy) = (ACTIVE_HX / c + 1, ACTIVE_HY / c + 1);
            let (mx, my) = (m.x / c, m.y / c); (mx.saturating_sub(hx), my.saturating_sub(hy), (mx + hx).min(self.g.cx - 1), (my + hy).min(self.g.cy - 1)) });
        self.g.step_ca();
        // structural changes are flagged per 8 m chunk (by `Grid::set` and by CA moves/reactions), so the
        // re-solve covers only the regions around them; reactions are batched every 24 ticks
        if self.g.reactions.swap(0, Relaxed) != 0 { self.react_pending = true; }
        self.g.undermined.swap(0, Relaxed);
        if (self.tick % 6 == 0 && self.g.struct_dirty.is_some() || self.tick % 6 == 0 && self.g.sdirty.iter().any(|f| f.load(Relaxed) & 1 != 0)) || (self.tick % 24 == 0 && self.react_pending) {
            let bits = if self.tick % 24 == 0 && self.react_pending { 3 } else { 1 };
            self.react_pending = false; self.g.struct_dirty = None;
            let boxes = self.g.take_struct(bits);
            let f = self.solve_regions(boxes);            let before = self.st.cracking.len(); self.st.crack(&f); if self.st.cracking.len() > before { self.events.push(Event::Crack); }
        }
        let crumble = self.st.tick(&self.g);
        // a collapse is heard: one of at least CRUMBLE_LOUD cells in a tick. The odd cell or two trickling off a
        // cracked face (as round a fresh crater, for seconds after the blast) falls without a boom.
        if crumble.len() >= CRUMBLE_LOUD { self.events.push(Event::Crumble); }
        let mut bulk = vec![];
        for &i in &crumble { let i = i as usize; let was = self.g.m(i); let c = was.p().crumbles_to; self.g.set(i, c); if c.kind() == Kind::Solid { self.g.set_loose(i); } self.crumbled += 1;
            if !was.has(BUILT) && hash3(i as u32, 77, self.tick as u32) % 10 < BULK_IN_10 { bulk.push((i, c)); } }
        // broken ground bulks: rubble takes ~1.3× the room the solid did, so a cave-in fills its own
        // void and chokes off instead of climbing as a chimney through the whole layer. The extra
        // rubble spills to the nearest open cell reachable from the break through loose ground,
        // level with it or lower (never through rock, so it can't appear in a sealed room).
        let (w, h) = (self.g.w, self.g.h);
        for (i, c) in bulk {
            let y0 = i / w; let mut q = VecDeque::from([i]); let mut seen = vec![i];
            while let Some(k) = q.pop_front() {
                if k != i && matches!(self.g.m(k).kind(), Kind::Empty | Kind::Gas) { self.g.set(k, c); if c.kind() == Kind::Solid { self.g.set_loose(k); } break; }
                if seen.len() > 48 * S { break; }
                let (x, y) = (k % w, k / w);
                for (nx, ny) in [(x, y + 1), (x.wrapping_sub(1), y), (x + 1, y), (x.wrapping_sub(1), y + 1), (x + 1, y + 1)] {
                    if nx >= w || ny >= h || ny < y0 { continue; }
                    let j = ny * w + nx; if seen.contains(&j) || self.g.is_static(j) || self.g.m(j).kind() == Kind::Liquid { continue; }
                    seen.push(j); q.push_back(j);
                }
            } }
        self.step_player();
        self.step_transport();
        self.step_items();
        self.step_debris(); let now = self.tick; self.waves.retain(|v| v.live(now));
        if self.tick % 2 == 0 { self.step_moles(); }
        // a knock (fall, lava, being buried, drowning) flashes the mole and jolts the view in proportion
        self.step_survival();
        if let Some(m) = self.moles.first() { let dmg = self.pl.hp_was - m.hp;
            if m.alive && dmg > 0.5 && self.pl.hp_was > 0. { self.pl.flash = 0.22; self.shake = self.shake.max((0.6 + dmg * 0.08).min(2.5)); }
            self.pl.hp_was = m.hp; }
        // wiring / logic / Thrumming
        let mp: Vec<(usize, usize)> = self.moles.iter().filter(|m| m.alive).map(|m| (m.x, m.y)).collect();
        self.logic.step(&mut self.g, &mp, self.tick);
        for s in std::mem::take(&mut self.logic.msgs) { self.say(s); }
        let booms = std::mem::take(&mut self.logic.booms);
        for b in booms { if self.logic.dev[b] == Dev::Tnt as u8 { self.logic.remove(&mut self.g, b); let (x, y) = self.logic.centre(b); self.blast_tnt(x, y); } }
        if self.tick % LAVA_COOL_EVERY == 0 { self.cool_lava(); }
        if self.tick % TREE_EVERY == 0 { self.grow_trees(); }
        // disasters
        let mut dz = std::mem::replace(&mut self.disasters, Disasters::new(0));
        dz.step(self); self.disasters = dz;
        if self.tick % air::UPDATE_TICKS == 0 { self.step_air(); }
        if self.tick % 30 == 0 { self.step_farms(); self.check_furniture(); self.step_ruins(); self.step_sites(); }
        if self.tick % 30 == 0 { self.follow_colony(); }
        self.shake *= 0.93;
        if self.tick % 30 == 0 { self.check_quests(); self.check_chests(); }
        if self.tick % 10 == 0 { self.check_clues(); }
    }
    /// Stress re-solve over clusters of dirty chunk boxes (boxes within 16 m of each other merge).
    fn solve_regions(&mut self, mut boxes: Vec<(usize, usize, usize, usize)>) -> Vec<u32> {
        if boxes.is_empty() { return vec![]; }
        boxes = crate::stress::merge_boxes(boxes, 16 * CPM);
        // budget: about an eighth of the window's cells per re-solve, nearest the mole first; the rest stay
        // flagged for the next pass (6 ticks on), so a world that's settling everywhere never stalls a frame
        let (w, h) = (self.g.w, self.g.h);
        let (mx, my) = self.moles.first().map_or((w / 2, h / 2), |m| (m.x, m.y));
        let dist = |b: &(usize, usize, usize, usize)| mx.abs_diff((b.0 + b.2) / 2) + my.abs_diff((b.1 + b.3) / 2);
        boxes.sort_by_key(|b| dist(b));
        let work = |b: &(usize, usize, usize, usize)| ((b.2 - b.0 + 1 + 2 * crate::stress::SIDE + 8 * CPM).min(w)) * ((b.3 - b.1 + 1 + 28 * CPM).min(h));
        let (mut f, mut spent) = (vec![], 0usize);
        for b in boxes {
            if spent > 0 && spent + work(&b) > w * h / 8 { // over budget: leave it flagged
                for y in (b.1..=b.3).step_by(crate::grid::CHUNK) { for x in (b.0..=b.2).step_by(crate::grid::CHUNK) { self.g.note_struct(y * w + x, 1); } }
                continue; }
            spent += work(&b); f.extend(self.st.solve_rect(&self.g, Some(b)));
        }
        f
    }
    pub fn vents(&self) -> Vec<(usize, usize)> { self.logic.dev_list.iter().filter(|&&b| self.logic.dev[b] == Dev::Vent as u8).map(|&b| self.logic.centre(b)).collect() }
    fn step_air(&mut self) {
        for m in &self.moles { if m.alive { self.air.add(m.x, m.y - 1, if m.job == Some(Job::Sleep) { air::MOLE_CO2 * 0.5 } else { air::MOLE_CO2 }); } }
        for &(x, y) in &self.torches { self.air.add(x, y, air::TORCH_CO2); }
        for f in &self.farms { self.air.add(f.x, f.y, air::FARM_CO2); }
        let v = self.vents(); self.air.update(&self.g, &v);
    }
    /// Plant a spore on the open cell (x, y) standing on soil.
    pub fn plant(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if self.inv[Item::Spore as usize] == 0 { return Err("no spores (dig fungus to find some)"); }
        if x < 1 || y + 1 >= self.g.h || x + 1 >= self.g.w { return Err("out of bounds"); }
        // snap to the floor: climb out of rock or drop through air (up to 1.5 m)
        let mut y = y; for _ in 0..6 * S { if y > 1 && self.g.mat[self.g.idx(x, y)] != 0 { y -= 1; } else if y + 2 < self.g.h && self.g.mat[self.g.idx(x, y + 1)] == 0 { y += 1; } else { break; } }
        if self.g.mat[self.g.idx(x, y)] != 0 { return Err("needs an open cell"); }
        if !self.g.get(x, y + 1).has(SOIL) || self.g.loose(self.g.idx(x, y + 1)) { return Err("needs a soil floor (dirt, clay, roots)"); }
        if self.farms.iter().any(|f| f.x.abs_diff(x) < 3 * S && f.y.abs_diff(y) < 3 * S) { return Err("too close to another mushroom"); }
        self.inv[Item::Spore as usize] -= 1; self.farms.push(Farm { x, y, growth: 0., why: 0 }); self.events.push(Event::Place); Ok(())
    }
    /// Growth check for one plot: (bright, dry).
    pub fn farm_conditions(&self, x: usize, y: usize) -> (bool, bool) {
        let near = |px: usize, py: usize, r: usize| px.abs_diff(x) <= r * S && py.abs_diff(y) <= r * S;
        let bright = self.torches.iter().any(|&(tx, ty)| near(tx, ty, 10)) || self.logic.dev_list.iter().any(|&b| self.logic.dev[b] == Dev::Lamp as u8 && self.logic.dev_s[b] != 0 && { let (cx, cy) = self.logic.centre(b); near(cx, cy, 10) })
            || self.buildings.iter().any(|b| b.built && b.kind == Station::Forge && near(b.x + Station::W / 2, b.y, 12));
        let mut wet = 0;
        let r = 4 * S as i32; for dy in -r..=r { for dx in -r..=r { let (nx, ny) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(nx, ny) { continue; } let i = self.g.idx(nx as usize, ny as usize);
            if self.g.mat[i] == Mat::Water as u8 { wet += 3; } else if self.g.flags[i] & F_WET != 0 || self.g.mat[i] == Mat::Clay as u8 { wet += 1; } } }
        (bright, wet < 4 * S * S)
    }
    fn step_farms(&mut self) {
        let mut k = 0;
        while k < self.farms.len() {
            let (x, y) = (self.farms[k].x, self.farms[k].y);
            let soil = self.g.get(x, y + 1);
            if self.g.mat[self.g.idx(x, y)] != 0 || !soil.has(SOIL) { self.farms.swap_remove(k); continue; }
            let (bright, dry) = self.farm_conditions(x, y);
            self.farms[k].why = bright as u8 | (dry as u8) << 1;
            // dark + moist: ~60 s to fruit; one condition missing: 4× slower; neither: stalls
            let rate = match (bright, dry) { (false, false) => 1. / 120., (true, true) => 0., _ => 1. / 480. };
            self.farms[k].growth += rate;
            if self.farms[k].growth >= 1. {
                self.farms[k].growth = 0.;
                self.drop_item(x, y, Item::Mushroom, 2);
                if hash3(x as u32, y as u32, self.tick as u32) % 3 == 0 { self.drop_item(x, y, Item::Spore, 1); }
                self.harvested += 2; self.events.push(Event::Harvest);
            }
            k += 1;
        }
    }
    /// The player's head (where the headlamp sits) and the beam's direction: toward the aim, or
    /// ahead and a little down when nothing is aimed at.
    pub fn headlamp(&self) -> ((f32, f32), (f32, f32)) {
        let f = self.moles.first().map_or(1, |m| m.facing) as f32;
        // the lamp's lens on the sprite (art::mole_sheet): 2.75 tiles ahead of the body centre and 2.5 up
        // from the feet; climbing, the sprite is turned nose-up against the wall
        let c = self.pl.cling as f32;
        let head = if c != 0. { (self.pl.px - c * 2.5 * SF, self.pl.py - 5.0 * SF) } else { (self.pl.px + f * 2.75 * SF, self.pl.py - 2.5 * SF) };
        let rest = if c != 0. { (-c * 0.3, -1.) } else { (f, 0.25) };
        let dir = match self.gear.aim { Some((ax, ay)) if (ax - head.0).abs() + (ay - head.1).abs() > 1.5 * SF => (ax - head.0, ay - head.1), _ => rest };
        (head, dir)
    }
    /// Every light source that isn't a glowing material.
    pub fn light_sources(&self) -> Vec<light::Source> {
        use light::{kinds::*, Source};
        let c = |x: usize, y: usize| (x as f32 + 0.5, y as f32 + 0.5);
        let mut v: Vec<Source> = self.torches.iter().map(|&(x, y)| { let (x, y) = c(x, y); Source::point(x, y, TORCH.0, TORCH.1) }).collect();
        for f in &self.furniture { if f.item == Item::Planter { let (x0, y0, x1, _) = f.rect(); v.push(Source::point((x0 + x1) as f32 / 2. + 0.5, y0 as f32, PLANTER.0, PLANTER.1)); } }
        for b in &self.buildings { if b.built && b.kind == Station::Forge { let (x, y) = c(b.x + Station::W / 2, b.y - S); v.push(Source::point(x, y, KILN.0, KILN.1)); } }
        for &b in &self.logic.dev_list { if self.logic.dev[b] == Dev::Lamp as u8 && self.logic.dev_s[b] != 0 { let (x, y) = self.logic.centre(b); let (x, y) = c(x, y); v.push(Source::point(x, y, LAMP.0, LAMP.1)); } }
        for (x, y, _, _) in self.glowmites() { v.push(Source::point(x, y, GLOWMITE.0, GLOWMITE.1)); }
        if self.gear.held == Some(Item::Glowmite) { v.push(Source::point(self.pl.px + self.moles.first().map_or(0., |m| m.facing as f32) * 1.6 * SF, self.pl.py - 1.4 * SF, GLOWMITE_JAR.0, GLOWMITE_JAR.1)); }
        for (k, m) in self.moles.iter().enumerate() { if !m.alive { continue; }
            if k == 0 {
                let (head, dir) = self.headlamp();
                if self.lamp_lit() { v.push(Source::beam(head.0, head.1, HEADLAMP.0, HEADLAMP.1, dir.0, dir.1, HEADLAMP_HALF)); v.push(Source::point(head.0, head.1, LAMP_GLOW.0, LAMP_GLOW.1)); }
                else { v.push(Source::point(self.pl.px, self.pl.py - 1.5 * SF, MOLE_GLOW.0, MOLE_GLOW.1)); }
                // a torch in the paw lights like a (slightly smaller) torch
                if self.gear.held == Some(Item::Torch) { let f = m.facing as f32; v.push(Source::point(self.pl.px + f * 1.6 * SF, self.pl.py - 1.4 * SF, HELD_TORCH.0, HELD_TORCH.1)); }
            } else { let (x, y) = c(m.x, m.y - S); v.push(Source::point(x, y, MOLE_GLOW.0, MOLE_GLOW.1)); }
        }
        v
    }
    pub fn compute_light(&mut self, rect: (usize, usize, usize, usize)) {
        let mut src = self.light_sources(); self.light_rect = rect;
        src.extend(light::surface_sources(&self.g, rect, 5 * crate::mat::CPM)); // lava, crystal, fungus, fire: rays too
        self.light.compute(&self.g, rect, &src);
    }

    fn step_moles(&mut self) {
        let n = self.moles.len();
        for k in 0..n {
            if !self.moles[k].alive { continue; }
            self.moles[k].anim += 1;
            self.mole_physics(k);
            if !self.moles[k].alive { continue; }
            if (self.tick / 2 + k as u64) % 4 == 0 { let (x, y) = (self.moles[k].x, self.moles[k].y - S); let before = self.g.flags.clone(); light::reveal(&mut self.g, x, y, 28 * S);
                // the headlamp shows the way further ahead, inside its beam
                if k == 0 && self.lamp_lit() { let (h, d) = self.headlamp(); let l = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-6); light::reveal_cone(&mut self.g, h.0, h.1, light::Cone { dx: d.0 / l, dy: d.1 / l, half: light::kinds::HEADLAMP_HALF }, (light::kinds::HEADLAMP.1 * crate::mat::CPM as f32) as usize); }
                self.note_discoveries(&before, x, y); }
            let d = self.depth_m(self.moles[k].x, self.moles[k].y); if d > self.max_depth { self.max_depth = d; }
        }
    }
    fn note_discoveries(&mut self, before: &[u8], x: usize, y: usize) {
        let w = self.g.w; let (x0, y0, x1, y1) = (x.saturating_sub(29 * S), y.saturating_sub(29 * S), (x + 29 * S).min(w - 1), (y + 29 * S).min(self.g.h - 1));
        for yy in y0..=y1 { for xx in x0..=x1 {
            let i = yy * w + xx; if before[i] & F_SEEN == 0 && self.g.flags[i] & F_SEEN != 0 {
                let m = self.g.m(i);
                if !self.discovered[m as usize] && m.has(NOTABLE) {
                    self.discovered[m as usize] = true; let d = self.depth_m(xx, yy);
                    self.inv[Item::Torch as usize] += 2; self.events.push(Event::Discover);
                    // a find the tools owned can't open yet says so, and what opens it: a reason to come back
                    let gate = self.come_back_with(m).map_or(String::new(), |t| format!(". Your {} can't cut it: {t}", self.tool_name()));
                    self.say(format!("Discovery: {} at {:.0} m (+2 torches){gate}", m.p().name, d.max(0.)));
                }
            }
        } }
    }
    fn mole_physics(&mut self, k: usize) {
        let (x, y) = (self.moles[k].x, self.moles[k].y);
        let head = self.g.idx(x, y - S); let hm = self.g.m(head);
        let in_lava = (0..MOLE_H).any(|j| self.g.get(x, y - j).p().heat > 0 && !(k == 0 && self.g.get(x, y - j) == Mat::Lava));
        let no_hunger = self.rules.no_hunger;
        let m = &mut self.moles[k];
        if in_lava { m.hp -= 4.; }
        // needs (rates per second at 60 tps; step_moles runs every 2nd tick → dt = 1/30 s)
        let dt = 1. / 30.; let busy = matches!(m.job, Some(Job::Dig) | Some(Job::Build) | Some(Job::Craft(_)) | Some(Job::Station(_)) | Some(Job::Haul(_)) | Some(Job::Deliver));
        let mult = if busy { 1.5 } else { 1. };
        // (hunger off, a Survival option: the belly stays full)
        m.hunger = if no_hunger { 100. } else { (m.hunger - 0.33 * mult * dt).max(0.) };
        if m.job == Some(Job::Sleep) { m.sleep = (m.sleep + 2.5 * dt).min(100.); } else { m.sleep = 100.; /* one mole, no bedtime for now */ }
        if m.hunger <= 0. { m.hp -= 1. * dt; }
        if m.hunger > 60. && m.sleep > 50. && m.breath > 80. { m.hp = (m.hp + 0.5 * dt).min(100.); }
        let co2 = self.air.at(x, y - S);
        if hm == Mat::Water { m.breath = (m.breath - 0.25).max(0.); if m.breath <= 0. { m.hp -= 0.6; } }
        else if co2 > 0.45 { m.breath = (m.breath - (co2 - 0.45) * 1.2).max(0.); if m.breath <= 0. { m.hp -= 0.4; } }
        else { m.breath = (m.breath + 2.).min(100.); }
        // (a mole sealed in a pressure-tube capsule is inside the pipe, not buried)
        let buried = !(k == 0 && self.tr.in_tube.is_some()) && (0..MOLE_H).any(|j| self.g.is_solidish(self.g.idx(x, y - j)));
        if buried {
            self.moles[k].hp -= 1.5; self.events.push(Event::Hurt);
            // try to wriggle up or sideways
            for (dx, dy) in [(0i32, -1i32), (-1, 0), (1, 0), (0, -2), (-2, 0), (2, 0), (0, -3)] { let (dx, dy) = (dx * S as i32, dy * S as i32); let (nx, ny) = ((x as i32 + dx) as usize, (y as i32 + dy) as usize); if self.fits(nx, ny) { self.moles[k].x = nx; self.moles[k].y = ny; break; } }
            self.moles[k].path.clear();
        }
        // a hungry mole snacks from its pack
        // (the player's mole says what it ate and what's left, or once that there is nothing to eat)
        if self.moles[k].hunger < 40. && !self.rules.no_hunger {
            let food = FOODS.iter().map(|&f| (f, food_value(f))).filter(|&(_, v)| v > 0.).find(|&(it, _)| self.inv[it as usize] > 0);
            if let Some((it, v)) = food {
                self.inv[it as usize] -= 1; self.moles[k].hunger = (self.moles[k].hunger + v).min(100.); self.events.push(Event::Eat);
                if k == 0 { self.pl.hungry_said = false; let left = self.inv[it as usize]; self.say(format!("Ate a {} ({left} left).", it.name().to_lowercase()));
                    // the pack's last food: said while there's still a full belly's time to find more or turn back
                    if FOODS.iter().all(|&f| self.inv[f as usize] == 0) { self.events.push(Event::Warning); self.say("That was the last food in the pack. Dig soil for grubs or root tangles for roots, or turn back.".into()); } }
            } else if k == 0 && !self.pl.hungry_said {
                self.pl.hungry_said = true; self.events.push(Event::Warning);
                self.say("Hungry, and nothing to eat. Dig soil for grubs or root tangles for roots.".into());
            }
        }
        if self.rules.harmless { self.moles[k].hp = self.moles[k].hp.max(1.); }
        if self.moles[k].hp <= 0. { if k == 0 && self.rules.drop_pack { self.drop_pack(); } self.moles[k].alive = false; self.pl.dead_t = 0; self.pl.deaths += 1; let nm = self.moles[k].name; self.events.push(Event::Death); self.say(format!("{} has died. Back to the burrow...", nm)); }
    }
    fn drop_item(&mut self, x: usize, y: usize, item: Item, n: u32) {
        if let Some(d) = self.drops.iter_mut().find(|d| d.item == item && d.asleep() && d.fuse == 0 && d.x.abs_diff(x) <= 3 * S && d.y.abs_diff(y) <= 3 * S && d.n < 20) { d.n += n; return; }
        // it pops out of the bite a little (a seeded nudge, so runs stay deterministic)
        let h = hash3(x as u32, y as u32, self.tick as u32); let mut d = Drop::at(x as f32 + 0.5, y as f32 + 0.5, item, n);
        d.vx = ((h % 9) as f32 - 4.) * SF; d.vy = -8. * SF; self.drops.push(d);
    }
    fn dig_cell(&mut self, i: usize, mx: usize, my: usize) {
        let m = self.g.m(i); let wet = self.g.flags[i] & F_WET != 0;
        self.leave_lining(i, m);
        self.g.set(i, if wet { Mat::Water } else { Mat::Empty }); self.desig[i] = 0; self.events.push(Event::Dig);
        if m.has(GRUBS) && hash3(i as u32, self.tick as u32, 0x6b) % (48 * S * S) as u32 == 0 { self.drop_item(mx, my, Item::Grub, 1); }
        if m == Mat::Roots && hash3(i as u32, self.tick as u32, 0x52) % (8 * S * S) as u32 == 0 { self.drop_item(mx, my, Item::Root, 1); }
        if m == Mat::Fungus && hash3(i as u32, self.tick as u32, 0x5b) % (2 * S * S) as u32 == 0 { self.drop_item(mx, my, Item::Spore, 1); }
        if let Some(it) = drop_of(m) {
            self.dig_acc[m as usize] += 1;
            let per = if matches!(m, Mat::Crystal | Mat::Quartz) { 3 * (S * S) as u16 } else { CELLS_PER_ITEM };
            if self.dig_acc[m as usize] >= per { self.dig_acc[m as usize] = 0; self.drop_item(mx, my, it, 1); }
        }
        let (x, y) = (i % self.g.w, i / self.g.w); light::reveal(&mut self.g, x, y, 3 * S);
    }
    fn place_cell(&mut self, i: usize, d: u8) {
        let it = build_item(d);
        if self.place_acc[d as usize] == 0 { if self.inv[it as usize] == 0 { return; } self.inv[it as usize] -= 1; self.place_acc[d as usize] = CELLS_PER_BLOCK; }
        self.place_acc[d as usize] -= 1;
        self.g.set(i, build_mat(d)); self.desig[i] = 0; self.pillars += 1; self.events.push(Event::Place);
    }
    /// Lava loses heat where there's little of it: a cell with fewer than 10 tiles' worth of lava in
    /// the 5×5-tile (1.25 m) neighbourhood (a splash, a thin sheet, a trickle down a wall) now and then freezes into solid
    /// basalt where it rests on solid ground. Deep pools keep their heat (only their thin edges
    /// crust), so lava lakes stay molten.
    fn cool_lava(&mut self) {
        let (w, h) = (self.g.w, self.g.h); let lava = Mat::Lava as u8;
        let mut freeze = vec![];
        // each sweep visits a quarter of the rows at 4× the chance: the same cooling rate for a quarter of the scan
        let (sweep, rows) = ((self.tick / LAVA_COOL_EVERY) as u32, 4usize);
        for y in (sweep as usize % rows..h).step_by(rows) { for x in 0..w { let i = y * w + x; if self.g.mat[i] != lava { continue; }
            if hash3(x as u32, y as u32, sweep ^ 0x1A7A) % (LAVA_COOL_1_IN / rows as u32) != 0 { continue; }
            let mut n = 0;
            let r = 2 * S; for yy in y.saturating_sub(r)..=(y + r).min(h - 1) { for xx in x.saturating_sub(r)..=(x + r).min(w - 1) { if self.g.mat[yy * w + xx] == lava { n += 1; } } }
            // it sets from the bottom up, on solid ground: a crust on top of molten lava would just break up again
            if n - 1 < 10 * S * S && y + 1 < h && self.g.is_static(i + w) { freeze.push(i); } } }
        for i in freeze { self.g.set(i, Mat::Lava.p().cools_to); }
    }
    fn check_chests(&mut self) {
        for c in 0..self.chests.len() {
            if self.chests[c].opened { continue; }
            let (cx, cy) = (self.chests[c].x, self.chests[c].y);
            let near = self.moles.iter().any(|m| m.alive && m.x.abs_diff(cx) <= 4 * S && m.y.abs_diff(cy) <= 4 * S);
            if !near || self.g.is_solidish(self.g.idx(cx, cy)) { continue; }
            self.chests[c].opened = true; self.chests_opened += 1; self.opened.insert((cx, self.oy + cy as i64));
            let h = hash3(cx as u32, cy as u32, self.info.seed);
            let d = self.depth_m(cx, cy);
            let loot: Vec<(Item, u32)> = if self.chests[c].ancient.is_some() { self.vault_loot(h) }
                else if let Some(k) = self.site_at_chest(cx, cy) { crate::sites::site_loot(k, h) }
                else if d > 50. { vec![(Item::Crystal, 2 + h % 3), (Item::IronIngot, 2), (Item::Tnt, 1 + h % 2)] }
                else if d > 25. { vec![(Item::CopperIngot, 2 + h % 3), (Item::TinIngot, 1), (Item::Torch, 6)] }
                else { vec![(Item::Coal, 4 + h % 4), (Item::Torch, 4), (Item::WoodPillar, 8)] };
            let mut s = String::new(); for &(it, n) in &loot { self.inv[it as usize] += n; s += &format!(" {}x{}", it.name(), n); }
            let who = if self.chests[c].ancient.is_some() { "An Ancient vault, opened" } else if self.site_at_chest(cx, cy).is_some() { "An old site chest" } else { "Treasure chest" };
            self.events.push(Event::Chest); self.say(format!("{} at {:.0} m:{}", who, d.max(0.), s));
        }
    }
    /// The mole's 1×3 body fits at (x, y) (y = feet).
    pub fn fits(&self, x: usize, y: usize) -> bool {
        x >= 1 && x + 1 < self.g.w && y >= MOLE_H && y + 1 < self.g.h && (0..MOLE_H).all(|j| self.passable(x, y - j))
    }
    /// Cell centre within the mole's reach (measured from its chest).
    pub fn in_reach(&self, x: usize, y: usize) -> bool {
        let Some(m) = self.moles.first() else { return false }; let (dx, dy) = (x as f32 - m.x as f32, y as f32 - (m.y as f32 - SF));
        dx * dx + dy * dy <= REACH_CELLS * REACH_CELLS
    }
    /// Ticks to mine cell i with the current tools (None = can't: wrong kind or tool tier too low).
    pub fn mine_time(&self, i: usize) -> Option<u32> {
        if !self.diggable(i) { return None; }
        let m = self.g.m(i); let t = (m.p().dig as u32 * 4 / (1 + self.dig_power() as u32) + 1).max(4);
        Some(if self.inv[Item::AncientBit as usize] > 0 { (t * 3 / 4).max(3) } else { t }) // the Ancients' bit bites a quarter faster
    }
    /// Cells in the mining bite around (x, y): a disc of the tool's brush radius (default BITES[tool]).
    pub fn bite_cells(&self, x: usize, y: usize) -> Vec<usize> {
        let b = brush_radius(Brush::Dig(self.tool()), self.input.brush);
        disc(b).into_iter().filter_map(|(dx, dy)| { let (nx, ny) = (x as i32 + dx, y as i32 + dy); self.g.in_bounds(nx, ny).then(|| self.g.idx(nx as usize, ny as usize)) }).collect()
    }
    /// Dig assist for tight spaces: when the aim is level with the body (sideways) or within its columns
    /// (up/down), the bite also clears a body-sized passage from the body to the aim (at most 6 cells), so a
    /// tunnel dug by pointing ahead always fits the 1.8x2.9 mole and never leaves a lip at head or toe height.
    pub fn passage_cells(&self, mx: usize, my: usize) -> Vec<usize> {
        let (px, py) = (self.pl.px, self.pl.py); let (mx, my) = (mx as i32, my as i32);
        let (c0, c1) = ((px - BODY_HW).floor() as i32, (px + BODY_HW - 1e-3).floor() as i32);
        let (r0, r1) = ((py - BODY_H).floor() as i32, (py - 1e-3).floor() as i32);
        let (xs, ys) = if (r0 - 1..=r1).contains(&my) && (mx > c1 || mx < c0) {
            (if mx > c1 { (c1 + 1, mx.min(c1 + 6 * S as i32)) } else { (mx.max(c0 - 6 * S as i32), c0 - 1) }, (r0, r1))
        } else if (c0 - 1..=c1 + 1).contains(&mx) && (my > r1 || my < r0) {
            ((c0, c1), if my > r1 { (r1 + 1, my.min(r1 + 6 * S as i32)) } else { (my.max(r0 - 6 * S as i32), r0 - 1) })
        } else { return vec![] };
        let mut v = vec![];
        for y in ys.0..=ys.1 { for x in xs.0..=xs.1 { if self.g.in_bounds(x, y) { v.push(self.g.idx(x as usize, y as usize)); } } }
        v
    }
    /// Would taking these bite cells leave the mole over a deadly drop, or lava, with too little floor to
    /// trust? Some(the line to say) only if the bite cuts into the floor under the body and afterwards any
    /// column under the body has less than `HOLLOW_FLOOR` of floor left above lava (`HOT`) or above a fall,
    /// to rock rather than water, that does more damage than the hp left (`HOLLOW`): a notch in a slab over a
    /// void is where it snaps (the stress solver cracks it), so a thin spot under the feet is as deadly as a
    /// hole. Reads the cells as they are, dark or not: the mole feels a hollow or hot floor through its claws.
    pub fn fatal_drop(&self, cells: &[usize]) -> Option<&'static str> {
        let (px, py) = (self.pl.px, self.pl.py);
        let (c0, c1) = ((px - BODY_HW).floor() as i32, (px + BODY_HW - 1e-3).floor() as i32);
        let fy = (py + 0.05 - 1e-3).floor() as i32; // the row the feet stand on (as `box_hits` reads ground)
        let bit: std::collections::HashSet<usize> = cells.iter().copied().filter(|&i| self.mine_time(i).is_some()).collect();
        if !bit.iter().any(|&i| { let (x, y) = ((i % self.g.w) as i32, (i / self.g.w) as i32); x >= c0 && x <= c1 && y >= fy }) { return None; }
        let open = |x: i32, y: i32| self.g.in_bounds(x, y) && (self.passable(x as usize, y as usize) || bit.contains(&self.g.idx(x as usize, y as usize)));
        let at = |x: i32, y: i32| if self.g.in_bounds(x, y) { self.g.get(x as usize, y as usize) } else { Mat::Bedrock };
        let liquid = |x: i32, y: i32| at(x, y).kind() == Kind::Liquid;
        let hp = self.moles.first().map_or(0., |m| m.hp);
        let deadly = |fall: i32| fall > TUNE.safe_fall as i32 && (fall - TUNE.safe_fall as i32) as f32 * 3. / SF >= hp;
        let (thick, deep) = (HOLLOW_FLOOR as i32, 64 * CPM as i32);
        let mut why = None;
        for x in c0..=c1 {
            // the hole this bite leaves (the mole drops into it), the floor left under that, then the open
            // drop below it to a landing: rock (deadly if far), water (safe) or lava (always deadly)
            let mut y = fy; while y < fy + deep && open(x, y) && !liquid(x, y) { y += 1; }
            if at(x, y) == Mat::Lava { return Some(HOT); }
            if liquid(x, y) || !self.g.in_bounds(x, y) { continue; }
            let f0 = y; while y < f0 + thick && self.g.in_bounds(x, y) && !open(x, y) { y += 1; }
            if y >= f0 + thick || !self.g.in_bounds(x, y) { continue; } // solid enough here
            while y < fy + deep && open(x, y) && !liquid(x, y) { y += 1; }
            if at(x, y) == Mat::Lava { return Some(HOT); }
            if !liquid(x, y) && deadly(y - fy) { why = Some(HOLLOW); }
        }
        why
    }
    /// Does the body AABB (centre px, feet py) overlap any solid cell?
    pub fn box_hits(&self, px: f32, py: f32) -> bool {
        let (x0, x1) = ((px - BODY_HW).floor() as i32, (px + BODY_HW - 1e-3).floor() as i32);
        let (y0, y1) = ((py - BODY_H).floor() as i32, (py - 1e-3).floor() as i32);
        for y in y0..=y1 { for x in x0..=x1 { if !self.g.in_bounds(x, y) || !self.passable(x as usize, y as usize) { return true; } } }
        !self.tr.lifts.is_empty() && self.platform_hits(px, py) // a lift's cage floor
    }
    fn body_fluid_at(&self, px: f32, py: f32) -> (bool, bool) {
        let (x, ym) = (px as usize, (py - BODY_H * 0.5) as usize); if x >= self.g.w || ym >= self.g.h { return (false, false); }
        let m = self.g.get(x, ym);
        let lava = [py - 0.5 * SF, py - BODY_H * 0.5, py - BODY_H + 0.2 * SF].iter().any(|&yy| yy >= 0. && (yy as usize) < self.g.h && self.g.get(x, yy as usize) == Mat::Lava);
        (m == Mat::Water, lava)
    }
    fn body_fluid(&self) -> (bool, bool) { self.body_fluid_at(self.pl.px, self.pl.py) }
    /// Walking into loose sand/gravel shoves the leading cells up over the mole's head. True if it made room.
    fn push_powder(&mut self, nx: f32, py: f32, dir: i32) -> bool {
        let col = if dir > 0 { (nx + BODY_HW - 1e-3).floor() as i32 } else { (nx - BODY_HW).floor() as i32 };
        let (y0, y1) = ((py - BODY_H).floor() as i32, (py - 1e-3).floor() as i32);
        let mut cells = vec![];
        for y in y0..=y1 { if !self.g.in_bounds(col, y) { return false; } let i = self.g.idx(col as usize, y as usize); let m = self.g.m(i);
            if self.passable(col as usize, y as usize) { continue; } if m.kind() != Kind::Powder { return false; } cells.push(i); }
        for i in cells { let (cx, mut cy) = (i % self.g.w, i / self.g.w); let m = self.g.m(i);
            while cy > 0 && !self.passable(cx, cy) { cy -= 1; } if cy == 0 { return false; } let j = self.g.idx(cx, cy); self.g.set(j, m); self.g.set(i, Mat::Empty); }
        true
    }
    /// Loose grains (sand, rubble) overlapping the body move out: just behind the body (away from where the
    /// mole is heading), else up the column above the head, else just ahead. Only when every overlapping cell is
    /// loose; solid rock in the body is a real burial and stays.
    fn shove_grains(&mut self, dir: i32) {
        let (x0, x1) = ((self.pl.px - BODY_HW).floor() as i32, (self.pl.px + BODY_HW - 1e-3).floor() as i32);
        let (y0, y1) = ((self.pl.py - BODY_H).floor() as i32, (self.pl.py - 1e-3).floor() as i32);
        let mut cells = vec![];
        for y in y0..=y1 { for x in x0..=x1 { if !self.g.in_bounds(x, y) { return; } let (ux, uy) = (x as usize, y as usize); if self.passable(ux, uy) { continue; }
            let i = self.g.idx(ux, uy); if !(self.g.m(i).kind() == Kind::Powder || self.g.loose(i)) { return; } cells.push((x, y)); } }
        if cells.len() > 2 * S * S { return; } // more than a few grains: that's a burial, the wriggle/breath rules apply
        let side = if dir != 0 { dir } else { 1 };
        for (x, y) in cells {
            let free = |w: &World, a: i32, b: i32| w.g.in_bounds(a, b) && w.g.mat[w.g.idx(a as usize, b as usize)] == 0;
            let up = (1..=4 * S as i32).map(|k| y0 - k).take_while(|&b| self.g.in_bounds(x, b) && !self.g.is_static(self.g.idx(x as usize, b as usize))).find(|&b| free(self, x, b)).map(|b| (x, b));
            let beside = |s: i32| (1..=2 * S as i32).map(|k| if s > 0 { x1 + k } else { x0 - k }).find(|&a| free(self, a, y)).map(|a| (a, y));
            if let Some((a, b)) = beside(-side).or(up).or_else(|| beside(side)) {
                let (i, j) = (self.g.idx(x as usize, y as usize), self.g.idx(a as usize, b as usize));
                let (m, f) = (self.g.mat[i], self.g.flags[i]); self.g.set(j, Mat::from_u8(m)); self.g.flags[j] = f; self.g.set(i, Mat::Empty);
            }
        }
    }
    /// Keys already held when control returns (a menu closing on Space, a load) must not act: a held
    /// jump only counts again after it is released and pressed anew.
    /// Rising into a corner: the x within `max` cells of the body that would let it pass (left or right edge
    /// aligned to the cell boundary), nearest first.
    fn corner_nudge(&self, ny: f32, max: f32) -> Option<f32> {
        let px = self.pl.px; let r = (px - BODY_HW).ceil() + BODY_HW; let l = (px + BODY_HW).floor() - BODY_HW;
        let mut c: Vec<f32> = (0..=2 * S).flat_map(|k| [r + k as f32, l - k as f32]).collect(); c.sort_by(|a, b| (a - px).abs().total_cmp(&(b - px).abs()));
        c.into_iter().find(|&x| (x - px).abs() <= max && (x - px).abs() > 1e-4 && !self.box_hits(x, ny) && !self.box_hits(x, self.pl.py))
    }
    pub fn swallow_held(&mut self) { self.pl.jump_was = true; }
    /// A vertical wall right beside the body on `side` (1 = right): solid in the column just past that edge,
    /// over the rows the body spans (inset a little, so a ceiling or floor the body only touches doesn't count:
    /// you can cling to and climb side walls, never ceilings).
    fn side_wall(&self, side: i32) -> bool { self.side_wall_at(self.pl.px, self.pl.py, side, 0.08 * SF) }
    /// `side_wall` for a body at (px, py), looking `reach` cells past its edge.
    fn side_wall_at(&self, px: f32, py: f32, side: i32, reach: f32) -> bool {
        let (c0, c1) = if side > 0 { ((px + BODY_HW - 1e-3).floor() as i32 + 1, (px + BODY_HW + reach - 1e-3).floor() as i32) }
            else { ((px - BODY_HW - reach).floor() as i32, (px - BODY_HW).floor() as i32 - 1) };
        let (y0, y1) = ((py - BODY_H + 0.1 * SF).floor() as i32, (py - 0.1 * SF).floor() as i32);
        (c0..=c1).any(|col| (y0..=y1).any(|y| !self.g.in_bounds(col, y) || !self.passable(col as usize, y as usize)))
    }
    /// Climbing on `side`: the top of the wall beside the body, if it ends below the head and there's room to
    /// stand on it (feet row as f32). A wall that only steps back a cell (a recess) isn't a ledge: you climb on.
    fn ledge(&self, side: i8) -> Option<f32> {
        let (px, py) = (self.pl.px, self.pl.py);
        let col = if side > 0 { (px + BODY_HW + 0.08 * SF - 1e-3).floor() as i32 } else { (px - BODY_HW - 0.08 * SF).floor() as i32 };
        let (y0, y1) = ((py - BODY_H + 0.1 * SF).floor() as i32, (py - 0.1 * SF).floor() as i32);
        let r = (y0..=y1).find(|&y| !self.g.in_bounds(col, y) || !self.passable(col as usize, y as usize))?;
        if r == y0 { return None; } // the wall goes on past the head
        let (r, sx) = (r as f32, px + side as f32 * SF);
        (!self.box_hits(px, r) && !self.box_hits(sx, r) && !self.side_wall_at(sx, r, side as i32, 0.08 * SF)).then_some(r)
    }
    /// Kick off the wall on `side` (1 = wall to the right): away and up, steering locked briefly.
    fn wall_jump(&mut self, side: i8) {
        let t = TUNE;
        self.pl.vx = -(side as f32) * t.wall_jump_x; self.pl.vy = -t.wall_jump_y; self.pl.lock = t.wall_lock;
        self.pl.buffer = 0.; self.pl.cling = 0; self.pl.coyote = 0.; self.pl.wall_t = 0.; self.events.push(Event::Place);
    }
    fn step_player(&mut self) {
        if self.moles.is_empty() { return; }
        let dt = 1. / 60.;
        if !self.moles[0].alive {
            self.pl.dead_t += 1; if self.pl.dead_t < 150 { return; }
            // respawn at the burrow (slide the window back to it if needed)
            let (sx, swy) = self.pl.spawn; let h = self.g.h as i64;
            let (mg, sl) = (100 * S as i64, SLIDE as i64);
            for _ in 0..64 { let y = swy - self.oy; if y < mg { self.shift(-sl); } else if y > h - mg { self.shift(sl); } else { break; } }
            let mut y = (swy - self.oy).clamp(4 * S as i64, h - 3 * S as i64) as usize; while y > 4 * S && !self.fits(sx, y) { y -= 1; }
            let m = &mut self.moles[0]; m.x = sx; m.y = y; m.hp = 100.; m.breath = 100.; m.hunger = m.hunger.max(50.); m.alive = true; m.fall = 0;
            self.pl = Player { spawn: self.pl.spawn, deaths: self.pl.deaths, fall_from: y, ..Default::default() };
            self.say("You crawl back into the burrow.".into()); return;
        }
        if self.ride_step() { return; }
        let inp = self.input.clone();
        let t = TUNE;
        self.pl.flash = (self.pl.flash - dt).max(0.);
        // keep the float body in sync when something else moved the mole (respawn, window shift, load)
        { let m = &self.moles[0]; if self.pl.sync != Some((m.x, m.y)) { self.pl.jump_was = inp.jump; self.pl.px = m.x as f32 + 0.5; self.pl.py = m.y as f32 + 1.0; self.pl.vx = 0.; self.pl.vy = 0.; self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py; self.pl.fall_from = m.y; self.pl.grounded = true; } }
        self.pl.ppx = self.pl.px; self.pl.ppy = self.pl.py;
        let (wet, lava) = self.body_fluid();
        if lava { self.moles[0].hp -= t.lava_dps * dt; if self.tick % 20 == 0 { self.events.push(Event::Hurt); } }
        let dir = inp.right as i32 - inp.left as i32;
        let jump_edge = inp.jump && !self.pl.jump_was; self.pl.jump_was = inp.jump;
        if jump_edge { self.pl.buffer = t.buffer; } else { self.pl.buffer = (self.pl.buffer - dt).max(0.); }
        let ground = self.box_hits(self.pl.px, self.pl.py + 0.05);
        let (wall_l, wall_r) = (self.side_wall(-1), self.side_wall(1));
        if ground { self.pl.coyote = t.coyote; } else { self.pl.coyote = (self.pl.coyote - dt).max(0.); }
        self.pl.lock = (self.pl.lock - dt).max(0.);
        // wall cling: airborne, touching a wall and pressing into it (or already clinging and still touching)
        // (an existing cling holds across a gap or recess of up to a cell, and is pulled back to the wall)
        let c = self.pl.cling as i32;
        let side = if wall_r && (dir > 0 || c == 1) { 1 } else if wall_l && (dir < 0 || c == -1) { -1 }
            else if c != 0 && self.side_wall_at(self.pl.px, self.pl.py, c, 1.1 * SF) { c } else { 0 };
        // grab as soon as the rise has slowed to climbing speed (no float up to the apex first)
        self.pl.cling = if !ground && !wet && !lava && self.pl.mantle <= 0. && side != 0 && (self.pl.vy >= -t.climb || c != 0) && !(dir == -side) { side as i8 } else { 0 };
        // on a rope ladder (transport, lifts.rs): no cling, and a jump off it counts as from the ground
        let climbing = self.ladder_climb(ground, jump_edge); if climbing { self.pl.cling = 0; self.pl.coyote = t.coyote; }
        if self.pl.cling != 0 { self.pl.wall_t = t.coyote; self.pl.wall_side = self.pl.cling; } else if ground { self.pl.wall_t = 0.; } else { self.pl.wall_t = (self.pl.wall_t - dt).max(0.); }
        if dir != 0 { self.moles[0].facing = dir as i8; }
        let swim = if lava { t.lava_mul } else if wet { t.swim_mul } else { 1. };
        // horizontal: near-instant on the ground, slightly softer in the air, locked briefly after a wall jump
        let sprint = inp.sprint && dir != 0 && !wet && !lava;
        if sprint && ground { self.moles[0].hunger = (self.moles[0].hunger - 0.5 * dt).max(0.); } // a little food
        let target = if self.pl.lock > 0. { self.pl.vx } else { dir as f32 * t.run * swim * if sprint { t.sprint } else { 1. } };
        let a = if ground { if dir == 0 || (target * self.pl.vx) < 0. { t.ground_decel } else { t.ground_accel } } else if dir == 0 { t.air_decel } else { t.air_accel } * dt;
        self.pl.vx += (target - self.pl.vx).clamp(-a, a);
        // vertical
        if climbing {
            // hands on the rungs: W/S climb at the ladder's pace, and it hangs still otherwise (no slide)
            self.pl.vy = if inp.up { -crate::lifts::LADDER_UP } else if inp.down { crate::lifts::LADDER_DOWN } else { 0. };
            self.pl.vx = self.pl.vx.clamp(-4. * SF, 4. * SF);
        } else if self.pl.mantle > 0. {
            // rolling over the ledge: carried sideways onto the top at the ledge's height
            self.pl.mantle -= dt; self.pl.vy = 0.; self.pl.vx = self.pl.wall_side as f32 * t.mantle_v;
            if self.pl.buffer > 0. { self.pl.mantle = 0.; self.pl.vy = -t.jump_v; self.pl.buffer = 0.; self.pl.coyote = 0.; }
        } else if self.pl.cling != 0 {
            let c = self.pl.cling; self.moles[0].facing = c;
            // climb speed eases in quickly; a grab while falling catches at once
            let want = if inp.up { -t.climb } else if inp.down { t.climb * 1.3 } else { t.slide };
            let a = t.climb_accel * dt; self.pl.vy = if self.pl.vy > want { want } else { self.pl.vy + (want - self.pl.vy).clamp(-a, a) };
            // pressed against the wall; pulled back to it across a gap or into a recess
            // (only into a wall that steps back for good: a shallow notch is climbed straight past)
            let opens = !self.side_wall(c as i32) && !self.side_wall_at(self.pl.px, self.pl.py - 2. * SF, c as i32, 0.08 * SF);
            self.pl.vx = c as f32 * if self.side_wall(c as i32) { 2. * SF } else if opens { 8. * SF } else { 0. };
            // a small outcrop overhead: ease out round it in good time instead of bumping into it
            if self.pl.vy < 0. && self.box_hits(self.pl.px, self.pl.py - 1.5 * SF) && !self.box_hits(self.pl.px - c as f32 * 1.05 * SF, self.pl.py - 1.5 * SF) && !self.box_hits(self.pl.px - c as f32 * 1.05 * SF, self.pl.py) { self.pl.vx = -(c as f32) * 8. * SF; }
            if self.pl.buffer > 0. { self.wall_jump(c); }
            else if inp.up || dir == c as i32 { if let Some(r) = self.ledge(c) {
                // the wall ends below the head: rise to its top at climbing speed, then roll over it
                if self.pl.py - r <= t.climb * dt + 1e-3 { self.pl.py = r; self.pl.vy = 0.; self.pl.mantle = 0.12; self.pl.wall_side = c; self.pl.cling = 0; self.pl.vx = c as f32 * t.mantle_v; }
                else { self.pl.vy = self.pl.vy.min(-t.climb); }
            } }
        } else if lava {
            // molten rock is thick: you sink slowly through it and can only claw upward a little
            self.pl.vy += t.gravity * 0.2 * dt;
            if inp.jump || inp.up { self.pl.vy -= t.lava_up * dt; }
            self.pl.vy = self.pl.vy.clamp(-t.lava_sink, t.lava_sink);
            if self.pl.buffer > 0. && ground { self.pl.vy = -t.jump_v * 0.35; self.pl.buffer = 0.; } // a weak push off the bottom
        } else if wet {
            self.pl.vy += (t.gravity * t.swim_grav - t.buoyancy) * dt;
            if inp.jump || inp.up { self.pl.vy -= t.swim_up * dt; }
            self.pl.vy = self.pl.vy.clamp(-t.swim_max, t.swim_max);
            if self.pl.buffer > 0. && !self.box_hits(self.pl.px, self.pl.py - 1.5 * SF) && !self.body_fluid_at(self.pl.px, self.pl.py - 1.6 * SF).0 { self.pl.vy = -t.jump_v * 0.8; self.pl.buffer = 0.; } // hop out at the surface
        } else {
            if self.pl.buffer > 0. && self.pl.coyote > 0. { self.pl.vy = -t.jump_v; self.pl.buffer = 0.; self.pl.coyote = 0.; }
            // just let go of a wall (pressing away from it, say) and jump: still a wall jump
            else if self.pl.buffer > 0. && self.pl.wall_t > 0. { self.wall_jump(self.pl.wall_side); }
            let g = if self.pl.vy > 0. { t.gravity * t.fall_mul } else if !inp.jump { t.gravity * t.cut_mul } else { t.gravity };
            self.pl.vy = (self.pl.vy + g * dt).min(t.terminal);
        }
        // swept, axis-separated movement in small substeps
        let (dx, dy) = (self.pl.vx * dt, self.pl.vy * dt);
        let n = ((dx.abs().max(dy.abs()) / 0.25).ceil() as i32).max(1);
        let mut airfall = !ground;
        for _ in 0..n {
            let sx = dx / n as f32; let nx = self.pl.px + sx;
            if sx != 0. {
                if !self.box_hits(nx, self.pl.py) { self.pl.px = nx; }
                else {
                    // step up 1-2 cells (on the ground) or mantle a ledge (clinging) without losing speed
                    let lift = if ground || self.pl.coyote > 0. { t.step } else { 0. }; // ledges while climbing: see `ledge`
                    let mut k = 1.; let mut ok = false;
                    while k <= lift { let ny = (self.pl.py - k).ceil(); if !self.box_hits(nx, ny) && !self.box_hits(self.pl.px, ny) { self.pl.py = ny; self.pl.px = nx; ok = true; break; } k += 1.; }
                    // airborne: slot into a tunnel mouth when the feet are within a little of its floor row
                    if !ok && !ground && self.pl.cling == 0 && self.pl.mantle <= 0. { let ny = self.pl.py.round(); if (ny - self.pl.py).abs() <= t.nudge && !self.box_hits(nx, ny) && !self.box_hits(self.pl.px, ny) { self.pl.py = ny; self.pl.px = nx; ok = true; } }
                    if !ok && !self.push_powder(nx, self.pl.py, sx.signum() as i32) { self.pl.vx = 0.; }
                    else if !ok { self.pl.vx *= 0.85; }
                }
            }
            let sy = dy / n as f32; let ny = self.pl.py + sy;
            if sy != 0. {
                if !self.box_hits(self.pl.px, ny) { self.pl.py = ny; }
                else if let Some(cx) = (sy < 0.).then(|| self.corner_nudge(ny, if self.pl.cling != 0 { 1.1 * SF } else { t.nudge })).flatten() { self.pl.px = cx; self.pl.py = ny; } // head clipped a corner: slide around it
                else {
                    // snap against the floor/ceiling, but never search more than a cell past where we
                    // were: when already wedged in (buried), an unbounded search pops you out through rock
                    let was = self.pl.py;
                    if sy > 0. { self.pl.py = ny.floor(); while self.box_hits(self.pl.px, self.pl.py) && self.pl.py > was - 1. { self.pl.py -= 0.01; } airfall = false; }
                    else { self.pl.py = (self.pl.py - BODY_H).floor() + BODY_H + 0.0; while self.box_hits(self.pl.px, self.pl.py) && self.pl.py < was + 1. { self.pl.py += 0.01; } }
                    if self.box_hits(self.pl.px, self.pl.py) { self.pl.py = was; }
                    self.pl.vy = 0.; }
            }
        }
        // buried by falling rock/sand: wriggle up if there's room, else stay put (breath runs out)
        // (only through loose stuff: a solid rock ceiling stops you, you don't pop out above it)
        if self.box_hits(self.pl.px, self.pl.py) { for k in 1..=3 * S {
            let (x0, x1) = ((self.pl.px - BODY_HW).floor() as i32, (self.pl.px + BODY_HW - 1e-3).floor() as i32);
            let (y0, y1) = ((self.pl.py - k as f32 - BODY_H).floor() as i32, (self.pl.py - 1e-3).floor() as i32);
            let rock = (y0..=y1).any(|y| (x0..=x1).any(|x| !self.g.in_bounds(x, y) || self.g.is_static(self.g.idx(x as usize, y as usize))));
            if rock { break; }
            if !self.box_hits(self.pl.px, self.pl.py - k as f32) { self.pl.py -= k as f32; break; } } }
        // a few grains that trickled into the body (the sand doesn't know the mole is there) and can't
        // be wriggled out of get shoved aside, so a stray grain never wedges the mole in place
        if self.box_hits(self.pl.px, self.pl.py) { self.shove_grains(dir); }
        self.pl.px = self.pl.px.clamp(2. * SF, self.g.w as f32 - 2. * SF); self.pl.py = self.pl.py.clamp(BODY_H + 1., self.g.h as f32 - 2.);
        let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize);
        let landed = self.box_hits(self.pl.px, self.pl.py + 0.05);
        if landed && !self.pl.grounded && !wet && !lava { let f = y.saturating_sub(self.pl.fall_from); if f >= 8 * S { self.shake = self.shake.max(0.25 + f as f32 * 0.02 / SF); } else if f >= 2 * S { self.shake = self.shake.max(0.1); /* even a hop down lands with a bump */ } if f > t.safe_fall { self.moles[0].hp -= (f - t.safe_fall) as f32 * 3. / SF; self.events.push(Event::Hurt); } }
        if landed || wet || lava || self.pl.cling != 0 || self.pl.vy < 0. { self.pl.fall_from = y; }
        let _ = airfall;
        self.pl.grounded = landed;
        if self.pl.vx.abs() > 0.5 * SF && landed || self.pl.cling != 0 && self.pl.vy.abs() > SF { self.moles[0].anim += 1; }
        self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y));
        // mining: hold on a spot; each bite clears a mole-sized disc (radius BITES[tool]) around it. Progress
        // carries over while the cursor drags within 2 cells, so sweeping carves a tunnel smoothly.
        match inp.mine {
            Some((mx, my)) if mx < self.g.w && my < self.g.h && self.in_reach(mx, my) => {
                let mut cells = self.bite_cells(mx, my);
                for c in self.passage_cells(mx, my) { if !cells.contains(&c) { cells.push(c); } }
                let need = cells.iter().filter_map(|&i| self.mine_time(i)).max();
                if let Some(why) = need.and_then(|_| self.fatal_drop(&cells)) {
                    // the bite would open the floor under the whole body onto a drop that kills: hold it back and
                    // say so at once (a mole feels the hollow through its claws). Digging beside the drop still
                    // works, so the player can open it up, look, and climb or jump down by choice.
                    self.pl.mine = None;
                    if !self.pl.hollow {
                        self.pl.hollow = true; self.events.push(Event::Warning); self.shake = self.shake.max(0.08);
                        if self.pl.hollow_said.map_or(true, |t| self.tick >= t + DENY_REPEAT) { self.pl.hollow_said = Some(self.tick); self.say(why.into()); }
                    }
                } else if let Some(need) = need {
                    let i = self.g.idx(mx, my);
                    let work = match self.pl.mine { Some((j, w, _)) if (j % self.g.w).abs_diff(mx) <= 2 * S && (j / self.g.w).abs_diff(my) <= 2 * S => w + 1, _ => 1 };
                    self.moles[0].facing = if mx >= x { 1 } else { -1 };
                    if work % 10 == 1 { self.events.push(Event::Dig); self.shake = self.shake.max(0.1 + 0.03 * self.tool() as f32); } // each strike is heard and felt: on press, then every 1/6 s
                    if work >= need { self.pl.mine = None; self.pl.deny = None; for c in cells { if self.mine_time(c).is_some() { self.dig_cell(c, x, y); } }
                        let b = brush_radius(Brush::Dig(self.tool()), self.input.brush); self.marks.push((0, mx as f32 + 0.5, my as f32 + 0.5, b)); self.shake = self.shake.max(0.3 + 0.1 * self.tool() as f32); }
                    else { self.pl.mine = Some((i, work, need)); }
                } else {
                    self.pl.mine = None;
                    // nothing here gives to your tools: say so at once (a clink and a word), not silence. Once per
                    // press and material, so sweeping along a granite wall clinks once, and the words don't repeat
                    // for DENY_REPEAT ticks however often you click.
                    let hard = cells.iter().map(|&c| self.g.m(c)).find(|&m| m.kind() == Kind::Solid && !self.can_dig(m));
                    if let Some(m) = hard { if self.pl.deny != Some(m) {
                        self.pl.deny = Some(m); self.events.push(Event::Crack); self.shake = self.shake.max(0.08);
                        if let Some(s) = self.dig_refusal(m) { self.say_dig_no(s); }
                    } }
                    // water or lava under the cursor and nothing solid: nothing to dig, said once per press
                    else if let Some(m) = cells.iter().map(|&c| self.g.m(c)).find(|&m| m.kind() == Kind::Liquid) { if self.pl.deny != Some(m) {
                        self.pl.deny = Some(m); self.say_dig_no(format!("{} can't be dug: it flows.", m.p().name));
                    } }
                }
            }
            // held but out of reach: said once per press (the press goes on, so no repeat while held)
            Some(_) => { self.pl.mine = None; if !self.pl.far { self.pl.far = true; self.say_dig_no("Too far to dig: step closer.".into()); } }
            None => { self.pl.mine = None; self.pl.deny = None; self.pl.far = false; self.pl.hollow = false; }
        }
        // pick up anything lying within 3 cells
        let mut k = 0;
        while k < self.drops.len() {
            let d = &self.drops[k];
            if d.hold == 0 && d.fuse == 0 && d.x.abs_diff(x) <= 3 * S && d.y.abs_diff(y) <= 4 * S { let d = self.drops.swap_remove(k); self.inv[d.item as usize] += d.n; } else { k += 1; }
        }
    }
    /// Place a block item (pillar, brick, beam) into an open cell within reach, touching something solid.
    pub fn place_block(&mut self, x: usize, y: usize, it: Item) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        self.place_block_at(x, y, it)
    }
    /// `place_block` without the reach check (scripted building, scenarios).
    pub fn place_block_at(&mut self, x: usize, y: usize, it: Item) -> Result<(), &'static str> {
        let Some(d) = block_code(it) else { return Err("not a block") };
        if x < 1 || y < 1 || x + 1 >= self.g.w || y + 1 >= self.g.h { return Err("out of bounds"); }
        let i = self.g.idx(x, y); if self.g.mat[i] != 0 && !self.g.m(i).has(OPEN) { return Err("something is in the way"); }
        if self.moles.iter().any(|m| m.alive && m.x == x && y <= m.y && y + MOLE_H > m.y) { return Err("you are standing there"); }
        if !self.g.n4(x, y).any(|j| self.g.is_solidish(j)) { return Err("needs something to rest on"); }
        if self.place_acc[d as usize] == 0 && self.inv[it as usize] == 0 { return Err("none left"); }
        self.place_cell(i, d); self.marks.push((1, x as f32 + 0.5, y as f32 + 0.5, 0.5 * SF)); Ok(())
    }
    /// Place a device (lever, knot, TNT, ...) in the 1 m block under (x, y), paying its pack item.
    /// TNT goes down inert: light it (`use_device`), or put current on a copperline beside it. It isn't set
    /// down on or beside a copperline that is already live: it would go off on the next tick, which is easy
    /// to do by accident (cut the power first; a charge already in place still goes off when the line is powered).
    pub fn place_device(&mut self, d: Dev, x: usize, y: usize) -> Result<(), &'static str> {
        if d == Dev::None { return Err("not a device"); }
        if x >= self.g.w || y >= self.g.h { return Err("out of bounds"); }
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let b = self.logic.block_of(x, y);
        if self.logic.dev[b] != 0 { return Err("something is already there"); }
        if d == Dev::Tnt && self.logic.live_at(b) { return Err("the copperline is live (cut the power first)"); }
        if d == Dev::Door && self.logic.occupied(b, &self.moles.iter().filter(|m| m.alive).map(|m| (m.x, m.y)).collect::<Vec<_>>()) { return Err("you are standing there"); }
        let cost = d.cost(); if let Some(it) = cost { if self.inv[it as usize] == 0 { return Err("none left"); } }
        let had_wire = self.logic.wire[b] != 0;
        if !self.logic.place(&mut self.g, d, b) { return Err("needs open space"); }
        if had_wire && self.logic.wire[b] == 0 { self.inv[Item::CopperWire as usize] += 1; } // a knot replaced the copperline
        if let Some(it) = cost { self.inv[it as usize] -= 1; }
        let (cx, cy) = self.logic.centre(b); self.events.push(Event::Place); self.marks.push((1, cx as f32 + 0.5, cy as f32 + 0.5, SF)); Ok(())
    }
    /// Right mouse on a device: flip a lever, step a thrum channel or ticker, light a fuse or TNT.
    /// Root-trees grow back from their roots. A trunk column whose root cell (the top root under it) still
    /// stands, with seep water within reach of the root, grows a cell upward now and then until it is as tall
    /// as the root's tree (`gen::tree_height`) or hits rock. Cut the roots, or drain the seep, and it never
    /// grows again. Stateless: it reads the grid, so it survives slides and saves. Runs in the active region.
    fn grow_trees(&mut self) {
        let (w, h) = (self.g.w, self.g.h); let c = crate::grid::CHUNK;
        let (x0, y0, x1, y1) = self.g.active.map_or((0, 1, w - 1, h - 1), |(a, b, cx1, cy1)| (a * c, (b * c).max(1), ((cx1 + 1) * c - 1).min(w - 1), ((cy1 + 1) * c - 1).min(h - 1)));
        let (root, trunk) = (Mat::TreeRoot as u8, Mat::RootTree as u8); let mut grow = vec![];
        for y in y0..=y1 { for x in x0..=x1 { let i = y * w + x;
            if self.g.mat[i] != root || !(self.g.mat[i - w] == trunk || self.g.m(i - w).passable()) { continue; }
            if hash3(x as u32, y as u32, (self.tick / TREE_EVERY) as u32 ^ 0x7EE) % TREE_GROW_1_IN != 0 { continue; }
            // climb the trunk to its top, then grow one cell into open space above it
            let full = gen::tree_height(x, self.oy + y as i64, self.info.seed);
            let mut k = 1; while k <= full && y >= k + 1 && self.g.mat[i - k * w] == trunk { k += 1; }
            if k > full || y < k + 1 { continue; }
            let j = i - k * w; if !matches!(self.g.m(j).kind(), crate::mat::Kind::Empty | crate::mat::Kind::Gas) { continue; }
            if !self.seep_near(x, y) { continue; }
            grow.push(j);
        } }
        for j in grow { self.g.set(j, Mat::RootTree); }
    }
    /// Seep water within 6 m sideways, 4 m above and 2 m below of (x, y): a root-tree drinks from it.
    pub fn seep_near(&self, x: usize, y: usize) -> bool {
        let (w, h, c) = (self.g.w, self.g.h, CPM); let water = Mat::Water as u8;
        (y.saturating_sub(4 * c)..(y + 2 * c).min(h)).any(|yy| (x.saturating_sub(6 * c)..(x + 6 * c).min(w)).any(|xx| self.g.mat[yy * w + xx] == water))
    }
    /// The glowmites drifting in the window now: (x, y) in cells, grotto index, glowmite number. Each grotto
    /// keeps a small swarm (4-7) drifting slowly about its hollow on its own looping path, minus any
    /// caught in the last GLOWMITE_BACK ticks.
    pub fn glowmites(&self) -> Vec<(f32, f32, usize, u8)> {
        let t = self.tick as f32 / 60.; let mut v = vec![];
        for (gi, g) in self.grottos.iter().enumerate() {
            let n = GLOWMITES_MIN + g.id % 4;
            for k in 0..n as u8 {
                if self.caught.get(&(g.id, k)).map_or(false, |&t0| self.tick < t0 + GLOWMITE_BACK) { continue; }
                let h = hash3(g.id, k as u32, 0x61); let f = |b: u32| ((h >> b) & 255) as f32 / 255.;
                let (rx, ry) = (g.rx as f32, g.ry as f32);
                let x = g.cx as f32 + rx * 0.6 * (f(0) * 6.28 + t * (0.05 + 0.08 * f(8))).sin() + 0.5 * SF * (t * (1.3 + f(16))).sin();
                let yr = (g.cy - self.oy) as f32 - ry * 0.15 + ry * 0.35 * (f(24) * 6.28 + t * (0.07 + 0.06 * f(4))).sin() + 0.4 * SF * (t * 1.7 + f(12) * 6.).cos();
                if yr >= 0. && yr < self.g.h as f32 { v.push((x, yr, gi, k)); }
            }
        }
        v
    }
    /// Right mouse near a glowmite (within half a metre of the click, and in reach): catch it in a jar.
    pub fn catch_glowmite(&mut self, x: usize, y: usize) -> bool {
        if x >= self.g.w || y >= self.g.h || !self.in_reach(x, y) { return false; }
        let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5); let r = 0.5 * CPM as f32;
        let Some(&(_, _, gi, k)) = self.glowmites().iter().filter(|g| (g.0 - cx).powi(2) + (g.1 - cy).powi(2) <= r * r)
            .min_by(|a, b| ((a.0 - cx).powi(2) + (a.1 - cy).powi(2)).total_cmp(&((b.0 - cx).powi(2) + (b.1 - cy).powi(2)))) else { return false };
        self.caught.insert((self.grottos[gi].id, k), self.tick); self.inv[Item::Glowmite as usize] += 1;
        self.events.push(Event::Click); self.say("Caught a glowmite in a jar.".into()); true
    }
    pub fn use_device(&mut self, x: usize, y: usize) -> bool {
        if x >= self.g.w || y >= self.g.h || !self.in_reach(x, y) { return false; }
        if self.use_transport(x, y) { return true; }
        let b = self.logic.block_of(x, y); if self.logic.dev[b] == 0 { return false; }
        self.logic.interact(b); self.events.push(Event::Click); let (cx, cy) = self.logic.centre(b); self.marks.push((1, cx as f32 + 0.5, cy as f32 + 0.5, 0.7 * SF)); true
    }
    /// Lay one copperline on the block under (x, y). False if it can't (already wired, a knot, none left).
    pub fn lay_wire(&mut self, x: usize, y: usize) -> bool {
        if x >= self.g.w || y >= self.g.h || !self.in_reach(x, y) || self.inv[Item::CopperWire as usize] == 0 { return false; }
        let b = self.logic.block_of(x, y);
        if !self.logic.place_wire(b) { return false; }
        self.inv[Item::CopperWire as usize] -= 1; true
    }
    /// Pick up the device and copperline on the block under (x, y), back into the pack (a lit charge is
    /// defused). Returns the device, if there was one.
    /// Pick up the placed thing at (x, y) back into the pack (`E`): one thing per call, the device first, then
    /// a torch, then the copperline segment, so holding `E` and sweeping takes up a run of copperline. A lit
    /// TNT charge or burning fuse cord can't be picked up. Nothing needs digging out.
    pub fn pick_up(&mut self, x: usize, y: usize) -> Option<Picked> {
        if x >= self.g.w || y >= self.g.h || !self.in_reach(x, y) { return None; }
        if let Some(it) = self.pick_up_transport(x, y) { self.events.push(Event::Click); return Some(Picked::Transport(it)); }
        let b = self.logic.block_of(x, y); let d = Dev::from_u8(self.logic.dev[b]);
        let got = if d != Dev::None {
            if self.logic.lit(b) { return None; }
            let wire = self.logic.wire[b]; self.logic.remove(&mut self.g, b); self.logic.wire[b] = wire; // the copperline under it stays laid
            if let Some(it) = d.cost() { self.inv[it as usize] += 1; } Picked::Dev(d)
        } else if let Some(k) = self.torch_at(x, y) {
            self.torches.swap_remove(k); self.inv[Item::Torch as usize] += 1; Picked::Torch
        } else if let Some(k) = self.furniture_at(x, y) {
            let fu = self.furniture.swap_remove(k); self.inv[fu.item as usize] += 1; Picked::Furniture(fu.item)
        } else if self.logic.wire[b] != 0 {
            self.logic.wire[b] = 0; self.logic.invalidate(); self.inv[Item::CopperWire as usize] += 1; Picked::Wire
        } else { return None };
        self.events.push(Event::Click); self.marks.push((1, x as f32 + 0.5, y as f32 + 0.5, SF)); Some(got)
    }
    // ---------- furnishing ----------
    /// Paint with a pigment: a disc of radius `want` (0 = default, clamped like a brush) around (x, y).
    /// Standing solid cells in reach take the colour; a dyeable piece of furniture (rug, banner, nest) under
    /// the cursor takes it too. One pigment paints 16 tiles' worth of cells. Returns how many cells changed.
    pub fn paint(&mut self, x: usize, y: usize, it: Item, want: f32) -> Result<usize, &'static str> {
        let Some(p) = it.pigment() else { return Err("not a pigment") };
        if !self.in_reach(x, y) { return Err("out of reach"); }
        if let Some(k) = self.furniture_at(x, y) { if self.furniture[k].item.furn().is_some_and(|f| f.cloth) && self.furniture[k].tint != p {
            if !self.pay_paint(it, 4 * (S * S) as u16) { return Err("none left"); } self.furniture[k].tint = p; self.events.push(Event::Place); return Ok(1); } }
        let r = brush_radius(Brush::Place(it), want); let mut n = 0;
        for (dx, dy) in disc(r) { let (cx, cy) = (x as i32 + dx, y as i32 + dy); if !self.g.in_bounds(cx, cy) || !self.in_reach(cx as usize, cy as usize) { continue; }
            let i = cy as usize * self.g.w + cx as usize; if self.g.m(i).kind() != Kind::Solid || self.g.paint(i) == p { continue; }
            if !self.pay_paint(it, 1) { break; } self.g.set_paint(i, p); n += 1; }
        if n == 0 { return Err("nothing here takes paint"); }
        self.marks.push((1, x as f32 + 0.5, y as f32 + 0.5, r)); self.events.push(Event::Place); Ok(n)
    }
    /// Spend `cells` cells' worth of a pigment (one pigment covers 16 tiles).
    fn pay_paint(&mut self, it: Item, cells: u16) -> bool {
        let per = 16 * (S * S) as u16;
        for _ in 0..cells { if self.paint_acc == 0 { if self.inv[it as usize] == 0 { return false; } self.inv[it as usize] -= 1; self.paint_acc = per; } self.paint_acc -= 1; }
        true
    }
    /// Where a piece of furniture would go for a cursor at (x, y): centred on x, standing on the first floor
    /// at or below y (or hanging from the first ceiling at or above it). `None` if there's no floor/ceiling near.
    pub fn furniture_spot(&self, it: Item, x: usize, y: usize) -> Option<Furniture> {
        let f = it.furn()?; let (w, h) = (f.w * S, f.h * S); let x0 = x.checked_sub(w / 2)?; if x0 + w >= self.g.w { return None; }
        let solid = |cx: usize, cy: usize| cy < self.g.h && self.g.is_solidish(cy * self.g.w + cx);
        let held = |row: usize| (x0..x0 + w).filter(|&cx| solid(cx, row)).count() * 3 >= w * 2; // two thirds of its width rests on it
        let reach = 6 * S;
        if !f.hang { (y..(y + reach).min(self.g.h - 1)).find(|&r| !solid(x, r) && solid(x, r + 1)).filter(|&r| held(r + 1) && r + 1 >= h).map(|r| Furniture { item: it, x: x0, y: r, tint: 0 }) }
        else { (y.saturating_sub(reach)..=y).rev().find(|&r| r > 0 && !solid(x, r) && solid(x, r - 1)).filter(|&r| held(r - 1) && r + h < self.g.h).map(|r| Furniture { item: it, x: x0, y: r + h - 1, tint: 0 }) }
    }
    /// Set down a piece of furniture from the pack (see `furniture_spot`). Its footprint must be open and in
    /// reach, and it can't overlap other furniture (a rug can lie under anything).
    pub fn place_furniture(&mut self, it: Item, x: usize, y: usize) -> Result<(), &'static str> {
        let Some(fu) = self.furniture_spot(it, x, y) else { return Err(if it.furn().is_some_and(|f| f.hang) { "needs a ceiling to hang from" } else { "needs a floor to stand on" }) };
        if self.inv[it as usize] == 0 { return Err("none left"); }
        let (x0, y0, x1, y1) = fu.rect();
        if !self.in_reach((x0 + x1) / 2, (y0 + y1) / 2) { return Err("out of reach"); }
        for cy in y0..=y1 { for cx in x0..=x1 { if self.g.is_solidish(cy * self.g.w + cx) { return Err("not enough room"); } } }
        let rug = it == Item::Rug;
        if self.furniture.iter().any(|o| (o.item == Item::Rug) == rug && { let (a0, b0, a1, b1) = o.rect(); a0 <= x1 && x0 <= a1 && b0 <= y1 && y0 <= b1 }) { return Err("something is already there"); }
        self.inv[it as usize] -= 1; self.marks.push((1, (x0 + x1) as f32 / 2. + 0.5, (y0 + y1) as f32 / 2. + 0.5, ((x1 - x0) as f32 / 2.).max(1.)));
        self.furniture.push(fu); self.events.push(Event::Place); Ok(())
    }
    /// The furniture under (x, y), topmost first (a rug last, so the table on it is picked first).
    pub fn furniture_at(&self, x: usize, y: usize) -> Option<usize> {
        let hit: Vec<usize> = (0..self.furniture.len()).filter(|&k| self.furniture[k].contains(x, y)).collect();
        hit.iter().copied().find(|&k| self.furniture[k].item != Item::Rug).or(hit.first().copied())
    }
    /// Furniture whose floor (or ceiling) was dug away falls loose as its item.
    fn check_furniture(&mut self) {
        let mut k = 0;
        while k < self.furniture.len() {
            let fu = self.furniture[k].clone(); let Some(f) = fu.item.furn() else { self.furniture.swap_remove(k); continue };
            let (x0, y0, x1, y1) = fu.rect(); let row = if f.hang { y0.checked_sub(1) } else { Some(y1 + 1).filter(|&r| r < self.g.h) };
            let held = row.is_some_and(|r| (x0..=x1).filter(|&cx| self.g.is_solidish(r * self.g.w + cx)).count() * 3 >= (x1 - x0 + 1) * 2);
            let buried = (y0..=y1).any(|cy| (x0..=x1).filter(|&cx| self.g.is_solidish(cy * self.g.w + cx)).count() * 2 > x1 - x0 + 1);
            if !held || buried { self.furniture.swap_remove(k); self.drop_item((x0 + x1) / 2, y1, fu.item, 1); continue; }
            k += 1;
        }
    }
    /// The placed torch drawn at (x, y), if any (a torch is about half a metre tall, standing on its cell).
    pub fn torch_at(&self, x: usize, y: usize) -> Option<usize> {
        let (x, y) = (x as i64, y as i64); let r = 2 * S as i64;
        self.torches.iter().position(|&(tx, ty)| (x - tx as i64).abs() <= r && y - ty as i64 <= S as i64 && ty as i64 - y <= 2 * r)
    }
    /// Place a round brush of blocks centred on (x, y) with radius `want` (0 = the item's default, clamped to
    /// its range). Only open cells in reach are filled, never the mole's body, and each new cell must touch
    /// something solid (the ground or a cell placed just before it), so a brush grows out of what it rests on.
    /// Returns how many cells were placed.
    pub fn place_brush(&mut self, x: usize, y: usize, it: Item, want: f32) -> Result<usize, &'static str> {
        let Some(d) = block_code(it) else { return Err("not a block") };
        let r = brush_radius(Brush::Place(it), want);
        // blocks go down in whole legacy tiles (S×S cells, on the global tile grid): a radius-r brush is
        // a disc of tiles, so building comes out the same at any resolution
        let (k, rt) = (S as i32, r / SF); let (tx, ty) = (x as i32 / k, y as i32 / k);
        let todo: Vec<(i32, i32)> = disc(rt).into_iter().flat_map(|(dx, dy)| (0..k * k).map(move |j| ((tx + dx) * k + j % k, (ty + dy) * k + j / k)))
            .filter(|&(nx, ny)| self.open_for_block(nx, ny)).collect();
        if todo.is_empty() { return Err(if self.in_reach(x, y) { "something is in the way" } else { "out of reach" }); }
        let n = self.fill_cells(todo, it, d)?; self.marks.push((1, x as f32 + 0.5, y as f32 + 0.5, r)); Ok(n)
    }
    /// Grid snap: fill the squares of `n` cells (aligned to the n-cell grid) along a straight line from the
    /// square holding `a` toward the one holding `b`. The line runs along whichever axis the drag mostly
    /// follows, so a drag lays a clean row or column. Same rules as the brush: open cells in reach, never the
    /// mole, each new cell resting on something. Returns how many cells were placed.
    pub fn place_line(&mut self, a: (usize, usize), b: (usize, usize), n: usize, it: Item) -> Result<usize, &'static str> {
        let Some(d) = block_code(it) else { return Err("not a block") };
        let n = n.clamp(1, 64) as i64; let (ax, ay, bx, by) = (a.0 as i64 / n, a.1 as i64 / n, b.0 as i64 / n, b.1 as i64 / n);
        let (ex, ey) = if (bx - ax).abs() >= (by - ay).abs() { (bx, ay) } else { (ax, by) };
        let mut todo = vec![];
        for gy in ay.min(ey)..=ay.max(ey) { for gx in ax.min(ex)..=ax.max(ex) {
            for j in 0..n * n { let (cx, cy) = (gx * n + j % n, gy * n + j / n); if self.open_for_block(cx as i32, cy as i32) { todo.push((cx as i32, cy as i32)); } }
        } }
        if todo.is_empty() { return Err(if self.in_reach(b.0, b.1) { "something is in the way" } else { "out of reach" }); }
        let k = self.fill_cells(todo, it, d)?; self.marks.push((1, b.0 as f32 + 0.5, b.1 as f32 + 0.5, n as f32 * 0.5)); Ok(k)
    }
    /// The squares `place_line` would fill, as (x0, y0, x1, y1) inclusive cells: for drawing the outline.
    pub fn line_extent(a: (usize, usize), b: (usize, usize), n: usize) -> (usize, usize, usize, usize) {
        let n = n.clamp(1, 64); let (ax, ay, bx, by) = (a.0 / n, a.1 / n, b.0 / n, b.1 / n);
        let (ex, ey) = if bx.abs_diff(ax) >= by.abs_diff(ay) { (bx, ay) } else { (ax, by) };
        (ax.min(ex) * n, ay.min(ey) * n, ax.max(ex) * n + n - 1, ay.max(ey) * n + n - 1)
    }
    fn open_for_block(&self, nx: i32, ny: i32) -> bool {
        if nx < 1 || ny < 1 || nx as usize + 1 >= self.g.w || ny as usize + 1 >= self.g.h { return false; }
        let (cx, cy) = (nx as usize, ny as usize); let i = self.g.idx(cx, cy);
        (self.g.mat[i] == 0 || self.g.m(i).has(OPEN)) && self.in_reach(cx, cy)
            && !self.moles.iter().any(|m| m.alive && m.x == cx && cy <= m.y && cy + MOLE_H > m.y)
    }
    /// Place the open cells in `todo`, growing out from what they rest on (a cell needs a solid neighbour,
    /// which can be one placed just before), while the pack lasts.
    fn fill_cells(&mut self, mut todo: Vec<(i32, i32)>, it: Item, d: u8) -> Result<usize, &'static str> {
        let mut n = 0;
        loop {
            let before = todo.len();
            todo.retain(|&(nx, ny)| {
                let (cx, cy) = (nx as usize, ny as usize);
                if !self.g.n4(cx, cy).any(|j| self.g.is_solidish(j)) { return true; }
                if self.place_acc[d as usize] == 0 && self.inv[it as usize] == 0 { return true; }
                let i = self.g.idx(cx, cy); self.place_cell(i, d); n += 1; false
            });
            if todo.is_empty() || todo.len() == before { break; }
        }
        if n == 0 { return Err(if self.place_acc[d as usize] == 0 && self.inv[it as usize] == 0 { "none left" } else { "needs something to rest on" }); }
        Ok(n)
    }
    /// Craft one recipe at once; needs the right station built within 16 cells of the mole.
    /// Note every item now in the pack as known, and say which recipes that opened.
    pub fn learn(&mut self) {
        let fresh: Vec<usize> = (0..NITEM).filter(|&k| self.inv[k] > 0 && !self.known[k]).collect();
        if fresh.is_empty() { return; }
        let before: Vec<bool> = (0..RECIPES.len()).map(|r| self.recipe_known(r)).collect();
        for k in fresh { self.known[k] = true; }
        if !self.rules.crafting { return; }
        let new: Vec<&str> = (0..RECIPES.len()).filter(|&r| !before[r] && self.recipe_known(r)).map(|r| RECIPES[r].name).collect();
        if !new.is_empty() { self.say(format!("New recipes: {}.", new.join(", "))); }
    }
    /// Is this recipe listed? Creative (and Plain): always. Survival: once any of its inputs has been picked up.
    pub fn recipe_known(&self, r: usize) -> bool { !self.rules.crafting || RECIPES[r].inputs.iter().any(|&(it, _)| self.known[it as usize]) }
    /// The built station of this kind the mole is standing at (within 4 m across, 2.5 m up or down).
    pub fn station_here(&self, kind: Station) -> Option<&Building> {
        let m = self.moles.first()?;
        self.buildings.iter().find(|b| b.built && b.kind == kind && (b.x + Station::W / 2).abs_diff(m.x) <= 16 * S && b.y.abs_diff(m.y) <= 10 * S)
    }
    /// Is the forge hot without burning fuel: lava or fire within 3 m, or a heatstone fed by lava within 3 m?
    pub fn forge_hot(&self, b: &Building) -> bool {
        let (x0, x1, y0, y1) = (b.x, b.x + Station::W - 1, (b.y + 1).saturating_sub(Station::H), b.y);
        let r = 3 * CPM; let (ax, ay, bx, by) = (x0.saturating_sub(r), y0.saturating_sub(r), (x1 + r).min(self.g.w - 1), (y1 + r).min(self.g.h - 1));
        (ay..=by).any(|y| (ax..=bx).any(|x| self.g.get(x, y).p().heat >= 1))
            || self.logic.dev_list.iter().any(|&k| { let (cx, cy) = self.logic.centre(k);
                self.logic.dev[k] == Dev::Heat as u8 && self.logic.dev_s[k] != 0 && cx + r >= x0 && cx <= x1 + r && cy + r >= y0 && cy <= y1 + r })
    }
    /// Is this station working? The workbench always is; the forge needs heat (lava or fire within 3 m, or a
    /// heatstone fed by lava within 3 m); the workshop needs a live copperline in or touching its footprint.
    pub fn station_ready(&self, b: &Building) -> Result<(), &'static str> {
        let (x0, x1, y0, y1) = (b.x, b.x + Station::W - 1, (b.y + 1).saturating_sub(Station::H), b.y);
        match b.kind {
            Station::Workbench => Ok(()),
            Station::Forge => {
                if self.forge_hot(b) || self.inv[Item::Coal as usize] + self.inv[Item::Charcoal as usize] > 0 { Ok(()) }
                else { Err("the forge is cold: bring coal to burn, or lava or fire within 3 m, or a heatstone") }
            }
            Station::Workshop => {
                let bb = crate::logic::B; let (ax, ay, bx, by) = (x0.saturating_sub(bb), y0.saturating_sub(bb), (x1 + bb).min(self.g.w - 1), (y1 + bb).min(self.g.h - 1));
                let live = (ay..=by).step_by(bb).any(|y| (ax..=bx).step_by(bb).any(|x| { let k = self.logic.block_of(x, y); self.logic.wire[k] != 0 && self.logic.net_lit(k) }));
                if live { Ok(()) } else { Err("the workshop has no power: run a live copperline to it") }
            }
        }
    }
    /// Craft one recipe at once. Survival: the recipe must be known and the mole must stand at a working
    /// station of its kind (`rules.crafting`). Creative: anywhere (free access), still paid from the pack.
    pub fn craft_now(&mut self, r: usize) -> Result<(), String> {
        let rec = &RECIPES[r]; if self.moles.is_empty() { return Err("no mole".into()); } let mut fuel: Option<Item> = None;
        if self.rules.crafting {
            if !self.recipe_known(r) { return Err("you haven't found what that needs yet".into()); }
            let Some(b) = self.station_here(rec.station) else { return Err(format!("stand at a {}", rec.station.name().to_lowercase())) };
            self.station_ready(b)?;
            fuel = if b.kind == Station::Forge && !self.forge_hot(b) { Some(if self.inv[Item::Coal as usize] > 0 { Item::Coal } else { Item::Charcoal }) } else { None };
        }
        if let Some(&(it, n)) = rec.inputs.iter().find(|&&(it, n)| self.inv[it as usize] < n + (fuel == Some(it)) as u32) { return Err(format!("need {} {}", n + (fuel == Some(it)) as u32, it.name().to_lowercase())); }
        if let Some(f) = fuel { if self.inv[f as usize] == 0 { return Err("the forge needs a coal to burn".into()); } self.inv[f as usize] -= 1; }
        for &(it, n) in rec.inputs { self.inv[it as usize] -= n; } self.inv[rec.out.0 as usize] += rec.out.1;
        self.events.push(Event::Craft); self.say(format!("Crafted {}.", rec.name)); self.learn(); Ok(())
    }
    pub fn quest_done(&self, q: usize) -> bool {
        match QUESTS[q].goal {
            Goal::Depth(m) => self.max_depth >= m, Goal::Have(it, n) => self.inv[it as usize] >= n,
            Goal::Built(s) => self.buildings.iter().any(|b| b.built && b.kind == s), Goal::Chests(n) => self.chests_opened >= n, Goal::Pillars(n) => self.pillars >= n * (S * S) as u32,
            Goal::Found(m) => self.discovered[m as usize],
        }
    }
    fn check_quests(&mut self) {
        while self.quest < QUESTS.len() && self.quest_done(self.quest) {
            let q = &QUESTS[self.quest]; let mut s = String::new();
            for &(it, n) in q.reward { self.inv[it as usize] += n; s += &format!(" {}x{}", it.name(), n); }
            self.say(format!("Quest complete: {} - reward{}", q.text, s)); self.events.push(Event::Quest); self.quest += 1;
            if self.quest < QUESTS.len() && !self.quest_done(self.quest) { self.say(format!("Next: {}.", QUESTS[self.quest].text)); }
        }
    }
}
/// What the Delver eats from the pack, best first, and how much hunger each restores.
pub const FOODS: [Item; 4] = [Item::Grub, Item::Mushroom, Item::Root, Item::Fungus];
pub fn food_value(it: Item) -> f32 { match it { Item::Grub => 45., Item::Mushroom => 35., Item::Root => 30., Item::Fungus => 25., _ => 0. } }
pub fn build_item(d: u8) -> Item { match d { D_BRICK => Item::Brick, D_IRON => Item::IronBeam, D_PLANK => Item::Planks, D_POLISH => Item::PolishedStone, D_TILE => Item::Tiles, D_CARVED => Item::CarvedPillar, D_RAIL => Item::Rail, D_TUBE => Item::Tube, _ => Item::WoodPillar } }
pub fn build_mat(d: u8) -> Mat { match d { D_BRICK => Mat::StoneBrick, D_IRON => Mat::IronBeam, D_PLANK => Mat::Planks, D_POLISH => Mat::Polished, D_TILE => Mat::Tiles, D_CARVED => Mat::Carved, D_RAIL => Mat::Rail, D_TUBE => Mat::Tube, _ => Mat::WoodBeam } }
