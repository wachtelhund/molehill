//! Furnishings of the found places: the headframe over a mine shaft, a pump wheel, sluice gates, a
//! broken bridge in a fissure, a cargo halt's railcar and bench, a holt's hearth, beds and stores.
//! They're drawn only (no cells, no physics), placed from each site's box and checked against the cells
//! actually there, so they stand on a floor or hang from a roof and never sit inside rock. The layouts
//! themselves belong to `sites.rs`; this only reads them.
use crate::art::Px;
use crate::devart::line;
use bevy::image::Image;
use molehill_sim::mat::CPM;
use molehill_sim::sites::{Kind, Site};
use molehill_sim::world::World;

const LINE: [u8; 3] = [30, 22, 22];
const WOOD: [u8; 3] = [140, 98, 58];
const WOOD_L: [u8; 3] = [180, 132, 84];
const WOOD_D: [u8; 3] = [98, 66, 38];
const IRON: [u8; 3] = [104, 106, 118];
const IRON_D: [u8; 3] = [70, 72, 84];
const BRASS: [u8; 3] = [190, 150, 70];
const BRASS_D: [u8; 3] = [136, 108, 50];
const COPPER: [u8; 3] = [176, 104, 58];
const VERD: [u8; 3] = [96, 170, 130];
const ROPE: [u8; 3] = [170, 140, 92];
const ROPE_D: [u8; 3] = [124, 100, 64];
const STONE: [u8; 3] = [132, 126, 118];
const CLOTH: [u8; 3] = [170, 150, 116];
const GLASS_COLD: [u8; 3] = [120, 120, 96];
fn sh(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k).min(255.) as u8, (c[1] as f32 * k).min(255.) as u8, (c[2] as f32 * k).min(255.) as u8] }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Prop {
    Headframe, Winch, OreChute, DigTools, Crates, MineSign, HangLamp,
    PumpWheel, Pipes, SluiceGate,
    Bridge, Rope, Scaffold,
    CrystalFloor, CrystalHang,
    Railcar, Bench, StationSign,
    Hearth, Bedroll, Pantry, Barrels,
}
pub const PROPS: [Prop; 22] = [Prop::Headframe, Prop::Winch, Prop::OreChute, Prop::DigTools, Prop::Crates, Prop::MineSign, Prop::HangLamp,
    Prop::PumpWheel, Prop::Pipes, Prop::SluiceGate, Prop::Bridge, Prop::Rope, Prop::Scaffold, Prop::CrystalFloor, Prop::CrystalHang,
    Prop::Railcar, Prop::Bench, Prop::StationSign, Prop::Hearth, Prop::Bedroll, Prop::Pantry, Prop::Barrels];

/// How a prop is held up: standing on the floor (both feet on something solid), or hanging from the roof.
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Anchor { Floor, Hang }
impl Prop {
    /// Sprite size in pixels (16 px = 1 m, so a cell is 2 px).
    pub fn size(self) -> (i32, i32) {
        use Prop::*;
        match self {
            Headframe => (56, 60), Winch => (28, 18), OreChute => (20, 36), DigTools => (22, 18), Crates => (30, 24), MineSign => (24, 16), HangLamp => (8, 18),
            PumpWheel => (44, 44), Pipes => (72, 12), SluiceGate => (24, 64), Bridge => (40, 44), Rope => (6, 88), Scaffold => (30, 52),
            CrystalFloor => (20, 18), CrystalHang => (18, 16), Railcar => (60, 34), Bench => (34, 14), StationSign => (36, 16),
            Hearth => (32, 16), Bedroll => (20, 8), Pantry => (20, 28), Barrels => (24, 20),
        }
    }
    pub fn anchor(self) -> Anchor { use Prop::*; match self { OreChute | MineSign | HangLamp | Pantry | Pipes | SluiceGate | Bridge | Rope | CrystalHang | StationSign => Anchor::Hang, _ => Anchor::Floor } }
    pub fn name(self) -> &'static str {
        use Prop::*;
        match self {
            Headframe => "headframe", Winch => "winch", OreChute => "ore chute", DigTools => "dig tools", Crates => "crates", MineSign => "mine sign", HangLamp => "hanging lamp (cold)",
            PumpWheel => "pump wheel", Pipes => "pipes", SluiceGate => "sluice gate", Bridge => "broken bridge", Rope => "hanging rope", Scaffold => "scaffolding",
            CrystalFloor => "crystal cluster", CrystalHang => "hanging crystals", Railcar => "railcar", Bench => "station bench", StationSign => "station sign",
            Hearth => "kitchen stove and pots", Bedroll => "bedroll", Pantry => "pantry rack", Barrels => "barrels",
        }
    }
    /// Footprint in cells.
    pub fn cells(self) -> (usize, usize) { let (w, h) = self.size(); (w as usize * CPM / 16, h as usize * CPM / 16) }
}

/// Where each family puts its furnishings: (prop, x, y, slide) in metres from the site's top-left. For a
/// floor prop (x, y) is the middle of its foot on the floor's top; for a hanging one the middle of its
/// top under the roof. `slide` is how far it may move to find room. `mirror`: x counts from the right
/// edge when the site is flipped (a halt whose tube runs west).
pub fn plan(kind: Kind) -> &'static [(Prop, f32, f32, f32)] {
    use Prop::*;
    match kind {
        Kind::Mine => &[(Headframe, 49.75, 9., 2.), (Winch, 54., 9., 3.), (OreChute, 31.5, 10., 3.), (Scaffold, 7., 21., 3.), (DigTools, 12.5, 21., 4.), (Crates, 58., 21., 6.),
            (MineSign, 45.5, 10., 4.), (HangLamp, 21.5, 0.5, 3.), (HangLamp, 41.5, 0.5, 3.), (HangLamp, 13.5, 10., 3.), (HangLamp, 61.5, 10., 3.)],
        // the holt is full of furniture already: the stove and pots stand on its table, a bedroll on the bedding
        Kind::Holt => &[(Hearth, 12., 12., 2.), (Bedroll, 6.8, 12., 2.), (Pantry, 10.5, 2.5, 3.),
            (HangLamp, 5., 3., 3.), (HangLamp, 15., 1.5, 3.), (HangLamp, 23., 3., 3.)],
        Kind::Pumpworks => &[(PumpWheel, 44.5, 19., 2.), (Pipes, 24.5, 8.5, 2.), (Pipes, 31., 8.5, 2.), (SluiceGate, 13., 8.5, 2.), (Barrels, 26., 19., 3.),
            (HangLamp, 38., 2., 3.), (HangLamp, 26., 2., 3.)],
        Kind::Fissure => &[(Bridge, 6.5, 8.5, 2.5), (Bridge, 10., 16.5, 2.5), (Bridge, 7., 24.5, 2.5), (Rope, 7., 0., 3.), (Rope, 11., 0., 3.)],
        Kind::Fossil => &[(Scaffold, 6., 15., 3.), (DigTools, 12., 15., 4.), (Crates, 24., 15., 5.), (HangLamp, 9., 6.5, 3.), (HangLamp, 18., 4.5, 4.)],
        Kind::Crystal => &[(CrystalFloor, 5., 15., 3.), (CrystalFloor, 14., 15., 3.), (CrystalFloor, 23., 15., 3.), (CrystalHang, 14., 4.5, 4.), (CrystalHang, 7., 7., 4.), (CrystalHang, 21., 7., 4.)],
        Kind::Halt => &[(Railcar, 33., 17., 2.), (Bench, 52., 17., 2.), (StationSign, 45., 2., 2.), (HangLamp, 39., 2., 2.), (HangLamp, 27.5, 2., 2.),
            (Barrels, 4., 17., 3.), (Crates, 23., 17., 3.)],
    }
}
/// Halts are laid out mirrored when their tube runs west (`Site::left`).
fn mirrored(s: &Site) -> bool { s.kind == Kind::Halt && s.left }

/// One prop in the window: its top-left cell and footprint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Placed { pub prop: Prop, pub site: u32, pub x: usize, pub y: usize, pub w: usize, pub h: usize }
impl Placed {
    fn overlaps(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> bool { self.x <= x1 && x0 < self.x + self.w && self.y <= y1 && y0 < self.y + self.h }
}

/// Every prop of every site in the window, placed against the cells there now.
pub fn dress(w: &World) -> Vec<Placed> {
    let mut out = vec![];
    for s in &w.sites { dress_site(w, s, &mut out); }
    out
}
fn dress_site(w: &World, s: &Site, out: &mut Vec<Placed>) {
    let (gw, gh) = (w.g.w, w.g.h); let sy = s.y0 - w.oy;
    if sy < 0 || sy as usize + s.h >= gh || s.x0 + s.w >= gw { return; }
    let (sx, sy) = (s.x0 as i64, sy); let m = CPM as f32;
    let solid = |x: i64, y: i64| x < 0 || y < 0 || x >= gw as i64 || y >= gh as i64 || w.g.is_solidish(y as usize * gw + x as usize);
    // things that already stand there: furniture, chests, ladders, and props placed before
    let blocked = |out: &Vec<Placed>, x0: usize, y0: usize, x1: usize, y1: usize| {
        out.iter().any(|p| p.overlaps(x0, y0, x1, y1))
            || w.furniture.iter().any(|f| { let (a, b, c, d) = f.rect(); a <= x1 && x0 <= c && b <= y1 && y0 <= d })
            || w.chests.iter().any(|c| c.x + 6 >= x0 && c.x <= x1 + 6 && c.y + 1 >= y0 && c.y <= y1 + 8)
            || w.farms.iter().any(|f| f.x + 4 >= x0 && f.x <= x1 + 4 && f.y + 1 >= y0 && f.y <= y1 + 6)
            || w.tr.ladders.iter().any(|l| l.x + 2 >= x0 && l.x <= x1 + 2 && l.top <= y1 && l.top + l.len >= y0)
    };
    for &(prop, mx, my, slide) in plan(s.kind) {
        let (pw, ph) = prop.cells(); let (pw, ph) = (pw as i64, ph as i64);
        let mx = if mirrored(s) { s.w as f32 / m - mx } else { mx };
        let (cx, cy) = (sx + (mx * m) as i64, sy + (my * m) as i64); let r = (slide * m) as i64;
        // nearest spot first: sideways in quarter-metre steps, up and down a cell at a time
        let mut offs: Vec<(i64, i64)> = vec![];
        for dx in (-r..=r).step_by((CPM / 4).max(1)) { for dy in -r..=r { offs.push((dx, dy)); } }
        offs.sort_by_key(|&(dx, dy)| dx.abs() * 2 + dy.abs());
        let found = offs.into_iter().find_map(|(dx, dy)| {
            let x0 = cx + dx - pw / 2; let (y0, y1) = match prop.anchor() { Anchor::Floor => (cy + dy - ph, cy + dy - 1), Anchor::Hang => (cy + dy, cy + dy + ph - 1) };
            let x1 = x0 + pw - 1;
            if x0 < sx || x1 >= sx + s.w as i64 || y0 < sy - 1 || y1 > sy + s.h as i64 { return None; }
            if (y0..=y1).any(|y| (x0..=x1).any(|x| solid(x, y))) { return None; }
            // standing: both feet (the outer quarters) mostly on solid; hanging: the top mostly under it
            let row_k = |y: i64, a: i64, b: i64| (a..=b).filter(|&x| solid(x, y)).count() as f32 / (b - a + 1) as f32;
            // a foot may instead lean on a post or wall standing right beside it, if the other foot is down
            let q = (pw / 4).max(1);
            let lean = |x: i64| (y0..=y1).filter(|&y| solid(x, y)).count() as i64 * 4 >= (y1 - y0 + 1) * 3;
            let held = match prop.anchor() {
                Anchor::Floor => { let (l, r) = (row_k(y1 + 1, x0, x0 + q - 1) >= 0.5, row_k(y1 + 1, x1 - q + 1, x1) >= 0.5); (l || r) && (l || lean(x0 - 1)) && (r || lean(x1 + 1)) }
                Anchor::Hang => row_k(y0 - 1, x0, x1) >= 0.5 };
            if !held || blocked(out, x0 as usize, y0 as usize, x1 as usize, y1 as usize) { return None; }
            Some(Placed { prop, site: s.id, x: x0 as usize, y: y0 as usize, w: pw as usize, h: ph as usize })
        });
        if let Some(p) = found { out.push(p); }
    }
}

/// The prop's pixels before outlining.
pub fn art_px(p: Prop) -> Px {
    use Prop::*;
    let (w, h) = p.size(); let mut c = Px::new(w, h);
    match p {
        Headframe => { // two splayed timber legs, braces, a sheave at the head and the rope down the middle
            line(&mut c, 3., h as f32 - 1., 17., 6., 3, WOOD); line(&mut c, w as f32 - 4., h as f32 - 1., w as f32 - 18., 6., 3, WOOD);
            for y in [22, 40] { let k = (h - y) as f32 / h as f32; let (a, b) = (3. + 14. * (1. - k) + 1., w as f32 - 4. - 14. * (1. - k) - 1.); line(&mut c, a, y as f32, b, y as f32, 2, WOOD_D); }
            line(&mut c, 8., 40., w as f32 - 9., 22., 1, WOOD_D); c.rect(14, 4, w - 15, 7, WOOD_L);
            c.oval(w as f32 / 2., 5., 6., 6., IRON); c.oval(w as f32 / 2., 5., 4., 4., [0, 0, 0]); for k in 0..4 { let a = k as f32 * 0.785; line(&mut c, w as f32 / 2. - a.cos() * 4., 5. - a.sin() * 4., w as f32 / 2. + a.cos() * 4., 5. + a.sin() * 4., 1, IRON_D); }
            c.oval(w as f32 / 2., 5., 1.3, 1.3, BRASS); c.rect(w / 2 + 5, 6, w / 2 + 5, h - 6, ROPE); c.rect(w / 2 + 3, h - 6, w / 2 + 7, h - 4, IRON_D); }
        Winch => { // a rope drum between two cheeks, a crank on the side
            c.rect(2, 4, 5, h - 1, WOOD); c.rect(w - 6, 4, w - 3, h - 1, WOOD); c.rect(0, h - 2, w - 1, h - 1, WOOD_D);
            c.oval(w as f32 / 2., 8., 8., 4.5, ROPE); for x in (7..w - 6).step_by(3) { c.rect(x, 4, x, 12, ROPE_D); } c.oval(w as f32 / 2., 8., 1.5, 1.5, IRON);
            line(&mut c, w as f32 - 3., 8., w as f32 - 1., 2., 1, IRON); c.rect(w - 2, 0, w - 1, 2, WOOD_L); }
        OreChute => { // a plank trough on two chains, tipped toward the rails, iron-banded
            c.rect(3, 0, 3, 10, IRON_D); c.rect(w - 4, 0, w - 4, 20, IRON_D);
            for k in 0..12 { let y0 = 10 + k * 2; let x = 1 + k / 2; c.rect(x, y0 - (k / 3), (x + 12).min(w - 1), y0 + 2 - (k / 3), if k % 4 == 0 { IRON } else { WOOD }); }
            c.rect(6, h - 4, 12, h - 1, [120, 100, 84]); c.set(8, h - 2, COPPER); c.set(10, h - 3, VERD); }
        DigTools => { // a pick and a shovel leaning crossed over a bucket
            line(&mut c, 3., h as f32 - 1., 15., 1., 1, WOOD); line(&mut c, 10., 0., 19., 4., 2, IRON);
            line(&mut c, 19., h as f32 - 1., 7., 2., 1, WOOD); c.rect(4, 0, 8, 3, IRON); c.rect(5, 3, 7, 3, IRON_D);
            c.rect(7, 10, 15, h - 1, [110, 84, 60]); c.rect(7, 10, 15, 10, BRASS); c.rect(8, 14, 14, 14, IRON_D); }
        Crates => { // two crates stacked and one beside, cross-braced
            for (x0, y0, x1, y1) in [(0, 10, 15, h - 1), (16, 12, w - 1, h - 1), (2, 0, 13, 9)] {
                c.rect(x0, y0, x1, y1, WOOD); c.rect(x0, y0, x1, y0, WOOD_L); c.rect(x0, y1, x1, y1, WOOD_D); line(&mut c, x0 as f32 + 1., y1 as f32 - 1., x1 as f32 - 1., y0 as f32 + 1., 1, WOOD_D);
                for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] { c.set(x, y, IRON); } } }
        MineSign => { // a plank board on two chains, a crossed-picks mark burned in
            c.rect(4, 0, 4, 4, IRON_D); c.rect(w - 5, 0, w - 5, 4, IRON_D); c.rect(0, 5, w - 1, h - 1, WOOD); c.rect(0, 5, w - 1, 5, WOOD_L);
            line(&mut c, 7., 13., 16., 7., 1, [60, 40, 28]); line(&mut c, 7., 7., 16., 13., 1, [60, 40, 28]); c.rect(5, 7, 8, 7, [60, 40, 28]); c.rect(15, 7, 18, 7, [60, 40, 28]); }
        HangLamp => { // a chain and a hooded lantern, cold: nobody has filled it
            c.rect(3, 0, 4, 6, IRON_D); for y in [1, 4] { c.set(2, y, IRON); c.set(5, y, IRON); } c.rect(1, 7, 6, 8, IRON); c.rect(1, 9, 6, 14, GLASS_COLD);
            c.rect(3, 10, 4, 12, sh(GLASS_COLD, 1.25)); c.rect(0, 15, 7, 16, IRON); c.rect(2, 17, 5, 17, IRON_D); }
        PumpWheel => { // a big spoked wheel with paddle buckets, on an A-stand
            let (cx, cy, r) = (w as f32 / 2., 20., 18.); line(&mut c, cx - 14., h as f32 - 1., cx, cy, 2, WOOD_D); line(&mut c, cx + 14., h as f32 - 1., cx, cy, 2, WOOD_D);
            for a in 0..64 { let t = a as f32 / 64. * std::f32::consts::TAU; for dr in [0., 1.] { c.set((cx + t.cos() * (r - dr)).round() as i32, (cy + t.sin() * (r - dr)).round() as i32, IRON); } }
            for k in 0..8 { let t = k as f32 / 8. * std::f32::consts::TAU; line(&mut c, cx, cy, cx + t.cos() * (r - 1.), cy + t.sin() * (r - 1.), 1, WOOD);
                let (bx, by) = (cx + t.cos() * r, cy + t.sin() * r); c.rect(bx as i32 - 2, by as i32 - 1, bx as i32 + 1, by as i32 + 1, WOOD_L); }
            c.oval(cx, cy, 3., 3., IRON_D); c.oval(cx, cy, 1.4, 1.4, BRASS); c.rect(0, h - 2, w - 1, h - 1, STONE); }
        Pipes => { // two copper pipes on hangers, a flanged joint and a valve
            for x in [6, w / 2, w - 7] { c.rect(x, 0, x, 3, IRON_D); }
            for (y, col) in [(3, COPPER), (8, sh(COPPER, 0.85))] { c.rect(0, y, w - 1, y + 2, col); c.rect(0, y, w - 1, y, sh(col, 1.3)); }
            for x in [18, 46] { c.rect(x, 2, x + 1, 11, BRASS); } c.oval(32., 6., 3., 2.2, IRON); c.rect(31, 0, 33, 1, IRON_D);
            for x in [10, 27, 55, 64] { c.set(x, 4, VERD); c.set(x + 1, 9, VERD); } }
        SluiceGate => { // a timber gate in an iron frame, a screw spindle up to the hand-wheel
            c.rect(0, 10, 2, h - 1, IRON); c.rect(w - 3, 10, w - 1, h - 1, IRON); c.rect(0, 10, w - 1, 12, IRON);
            c.rect(3, 26, w - 4, h - 1, WOOD); for y in (28..h).step_by(6) { c.rect(3, y, w - 4, y, WOOD_D); } for y in [32, 50] { c.rect(3, y, w - 4, y + 1, IRON_D); }
            c.rect(w / 2 - 1, 4, w / 2, 26, BRASS); for y in (6..26).step_by(3) { c.set(w / 2 + 1, y, sh(BRASS, 0.7)); }
            c.oval(w as f32 / 2., 4., 7., 2.5, IRON); c.oval(w as f32 / 2., 4., 5., 1.2, [0, 0, 0]); c.rect(w / 2 - 1, 2, w / 2, 6, BRASS_D); }
        Bridge => { // the stub of a plank bridge: rope and a few planks hanging off the lip
            line(&mut c, 1., 0., 14., 40., 1, ROPE); line(&mut c, 6., 0., 20., 38., 1, ROPE_D);
            for k in 0..6 { let t = k as f32 / 6.; let (x, y) = (1. + 13. * t, 2. + 36. * t); line(&mut c, x, y, x + 5. + (k % 2) as f32 * 2., y - 2. + (k % 3) as f32, 2, if k % 2 == 0 { WOOD } else { WOOD_L }); }
            c.rect(0, 0, 10, 2, WOOD_D); line(&mut c, 14., 40., 12., 43., 1, ROPE); line(&mut c, 20., 38., 23., 43., 1, ROPE_D);
            c.rect(30, 24, 38, 25, WOOD); line(&mut c, 32., 24., 30., 6., 1, ROPE_D); }
        Rope => { // a knotted rope down from a peg, frayed at the end
            c.rect(0, 0, 5, 2, WOOD_D); for y in 3..h - 4 { let x = 2 + ((y as f32 * 0.08).sin() * 1.2).round() as i32; c.set(x, y, ROPE); c.set(x + 1, y, if y % 3 == 0 { ROPE_D } else { ROPE }); }
            for y in [20, 44, 68] { c.rect(1, y, 4, y + 2, ROPE_D); } for (x, l) in [(1, 4), (3, 3), (4, 4)] { c.rect(x, h - 5, x, h - 5 + l - 1, ROPE); } }
        Scaffold => { // two uprights, two plank decks, diagonal braces, a ladder up one side
            c.rect(1, 0, 2, h - 1, WOOD); c.rect(w - 3, 0, w - 2, h - 1, WOOD); for y in [6, 28] { c.rect(0, y, w - 1, y + 2, WOOD_L); c.rect(0, y + 2, w - 1, y + 2, WOOD_D); }
            line(&mut c, 3., 9., w as f32 - 4., 27., 1, WOOD_D); line(&mut c, 3., 31., w as f32 - 4., h as f32 - 2., 1, WOOD_D);
            for y in (10..h).step_by(5) { c.rect(w - 9, y, w - 5, y, ROPE); } c.rect(w - 9, 9, w - 9, h - 1, ROPE_D); c.rect(5, 2, 9, 5, [120, 92, 64]); }
        CrystalFloor => { // a cluster of points from a salt crust
            for (x, top, hw, col) in [(9., 0., 3., [140, 210, 255]), (4., 6., 2.5, [120, 190, 240]), (14., 5., 2.6, [170, 225, 255]), (11., 9., 2., [110, 180, 236])] {
                for y in top as i32..h - 2 { let k = (y as f32 - top) / (h as f32 - 2. - top); let hw2 = (hw * k.min(0.4) / 0.4).max(0.5); c.rect((x - hw2) as i32, y, (x + hw2) as i32 - 1, y, if (y as f32) < top + 3. { sh(col, 1.2) } else { col }); } }
            c.rect(0, h - 2, w - 1, h - 1, [222, 220, 212]); c.set(8, 2, [250, 255, 255]); }
        CrystalHang => { // points hanging from a crusted roof
            for (x, len, col) in [(4, 12, [130, 200, 250]), (8, 15, [160, 220, 255]), (12, 10, [120, 190, 240]), (15, 7, [150, 210, 250])] {
                for y in 2..len + 1 { let hw = ((len + 1 - y) as f32 / 3.).min(2.) as i32; c.rect(x - hw, y, x + hw, y, col); } }
            c.rect(0, 0, w - 1, 1, [222, 220, 212]); c.set(8, 4, [250, 255, 255]); }
        Railcar => { // a flatbed on two bogies, low iron sides, lashed with a load under canvas
            c.rect(0, 20, w - 1, 25, WOOD); c.rect(0, 20, w - 1, 20, WOOD_L); c.rect(0, 16, 1, 20, IRON); c.rect(w - 2, 16, w - 1, 20, IRON); c.rect(0, 25, w - 1, 26, IRON_D);
            c.oval(26., 12., 22., 9., CLOTH); c.rect(4, 12, 48, 19, CLOTH); for x in [12, 26, 40] { c.rect(x, 4, x, 19, ROPE_D); } c.rect(50, 8, 57, 19, WOOD); c.rect(50, 8, 57, 8, WOOD_L);
            for cx in [8., 18., 42., 52.] { c.oval(cx, 29.5, 4., 4., IRON_D); c.oval(cx, 29.5, 1.5, 1.5, BRASS); } c.rect(2, 26, 23, 27, IRON); c.rect(37, 26, 58, 27, IRON); }
        Bench => { // a long waiting bench with a backrest, iron legs
            c.rect(0, 0, w - 1, 2, WOOD); c.rect(0, 0, w - 1, 0, WOOD_L); c.rect(2, 3, 3, 6, IRON); c.rect(w - 4, 3, w - 3, 6, IRON);
            c.rect(0, 7, w - 1, 8, WOOD_L); c.rect(0, 9, w - 1, 9, WOOD_D); for x in [3, w - 4] { c.rect(x, 10, x, h - 1, IRON); } c.rect(w / 2, 10, w / 2, h - 1, IRON_D); }
        StationSign => { // an iron board on two rods: a cart on rails and arrows each way, no words
            c.rect(6, 0, 6, 3, IRON_D); c.rect(w - 7, 0, w - 7, 3, IRON_D); c.rect(0, 4, w - 1, h - 1, IRON); c.rect(1, 5, w - 2, h - 2, [56, 60, 70]);
            let ink = [214, 200, 160]; c.rect(13, 8, 22, 11, ink); c.set(14, 12, ink); c.set(21, 12, ink); c.rect(11, 13, 24, 13, ink);
            line(&mut c, 3., 10., 8., 10., 1, ink); c.set(4, 9, ink); c.set(4, 11, ink); line(&mut c, 27., 10., 32., 10., 1, ink); c.set(31, 9, ink); c.set(31, 11, ink); }
        Hearth => { // the kitchen: a clay stove with its pot, a stack of bowls, a jar and a pestle in a mortar
            c.rect(1, 6, 13, h - 1, [158, 100, 72]); c.rect(1, 6, 13, 6, [184, 124, 90]); c.rect(4, 10, 10, 13, [44, 30, 26]); c.rect(5, 12, 9, 13, [92, 60, 48]);
            c.oval(7., 3.5, 5., 3., [62, 60, 66]); c.rect(2, 2, 12, 2, IRON); c.set(5, 0, [200, 196, 186]); c.set(9, 0, [200, 196, 186]);
            for (k, y) in [h - 2, h - 4, h - 6].into_iter().enumerate() { c.rect(16 + k as i32, y, 23 - k as i32, y + 1, if k % 2 == 0 { [176, 120, 84] } else { [150, 98, 70] }); }
            c.rect(25, 7, 29, h - 1, [180, 210, 220]); c.rect(25, 6, 29, 6, WOOD_D); c.rect(26, 10, 28, h - 2, [200, 220, 120]);
            c.oval(20., 9., 2.6, 1.4, STONE); line(&mut c, 20., 9., 23., 4., 1, WOOD_L); }
        Bedroll => { // a straw pallet, a blanket and a pillow
            c.rect(0, 4, w - 1, h - 1, [196, 168, 104]); for x in (1..w).step_by(3) { c.set(x, h - 1, [150, 124, 70]); }
            c.rect(6, 2, w - 1, 5, [120, 86, 70]); c.rect(6, 2, w - 1, 2, [150, 112, 92]); for x in (9..w).step_by(5) { c.rect(x, 3, x + 1, 4, [96, 66, 54]); } c.oval(3., 3., 3., 2., [220, 210, 186]); }
        Pantry => { // a hanging store: a board of jars on cords, sacks on hooks, dried mushrooms on a string
            for x in [1, w - 2] { c.rect(x, 0, x, 9, ROPE_D); } c.rect(0, 9, w - 1, 10, WOOD_L);
            for (x, col) in [(2, [150, 96, 70]), (7, [180, 210, 220]), (13, [170, 110, 80])] { c.rect(x, 4, x + 3, 8, col); c.rect(x, 3, x + 3, 3, sh(col, 0.75)); }
            for (x, y) in [(5, 18), (14, 20)] { c.rect(x, 11, x, y - 4, ROPE_D); c.oval(x as f32, y as f32, 3.4, 3.6, CLOTH); c.rect(x - 1, y - 4, x + 1, y - 4, ROPE_D); }
            c.rect(9, 11, 9, h - 2, ROPE); for y in (14..h - 1).step_by(4) { c.oval(9.5, y as f32, 1.6, 1.3, [190, 120, 90]); } }
        Barrels => { // two hooped barrels, one on its side in front
            c.oval(8., 8., 6., 7.5, WOOD); for y in [2, 8, 13] { c.rect(2, y, 14, y, IRON_D); } c.rect(7, 1, 9, 15, WOOD_L);
            c.oval(15., 15., 8.5, 4.5, WOOD); for x in [9, 15, 21] { c.rect(x, 11, x, 19, IRON_D); } c.oval(22., 15., 2., 4., WOOD_D); c.set(22, 15, [70, 50, 30]); }
    }
    c
}
pub fn image(p: Prop) -> Image { art_px(p).finish(LINE).image() }

#[cfg(test)]
mod tests {
    use super::*;
    use molehill_sim::mat::S;
    use molehill_sim::survival::Mode;

    #[test] fn every_prop_has_its_own_sprite() {
        let px: Vec<Px> = PROPS.iter().map(|&p| art_px(p).finish(LINE)).collect();
        for (k, p) in px.iter().enumerate() {
            let n = p.c.iter().filter(|c| c[3] != 0).count(); assert!(n * 5 > (p.w * p.h) as usize / 4, "{:?} is mostly empty ({n} px)", PROPS[k]);
            let mut cols: Vec<[u8; 4]> = p.c.iter().filter(|c| c[3] != 0).copied().collect(); cols.sort(); cols.dedup(); assert!(cols.len() >= 4, "{:?} is flat", PROPS[k]);
        }
        for a in 0..PROPS.len() { for b in a + 1..PROPS.len() { assert!(PROPS[a].size() != PROPS[b].size() || px[a].c != px[b].c, "{:?} = {:?}", PROPS[a], PROPS[b]); } }
    }

    /// Each found place, stamped into rock, gets its furnishings, all standing in open space inside it.
    #[test] fn found_places_are_furnished() {
        let want: [(Kind, &[Prop]); 7] = [
            (Kind::Mine, &[Prop::Headframe, Prop::Winch, Prop::OreChute, Prop::Scaffold, Prop::DigTools, Prop::Crates, Prop::MineSign, Prop::HangLamp]),
            (Kind::Holt, &[Prop::Hearth, Prop::Bedroll, Prop::Pantry, Prop::HangLamp]),
            (Kind::Pumpworks, &[Prop::PumpWheel, Prop::Pipes, Prop::SluiceGate, Prop::HangLamp]),
            (Kind::Fissure, &[Prop::Bridge, Prop::Rope]),
            (Kind::Fossil, &[Prop::DigTools, Prop::Crates, Prop::HangLamp]),
            (Kind::Crystal, &[Prop::CrystalFloor, Prop::CrystalHang]),
            (Kind::Halt, &[Prop::Railcar, Prop::Bench, Prop::StationSign, Prop::HangLamp]),
        ];
        let mut seen = std::collections::HashSet::new();
        let mut w = World::new(512 * S, 256 * S, 7); w.set_mode(Mode::Plain);
        for (kind, need) in want {
            // a clean block of host rock each time, nothing left from the last stamp
            w.sites.clear(); w.furniture.clear(); w.chests.clear(); w.tr.ladders.clear(); w.tr.carts.clear(); w.farms.clear(); w.torches.clear();
            for y in 0..400 { for x in 0..800 { let i = y * w.g.w + x; w.g.set(i, molehill_sim::mat::Mat::Rock); } }
            w.stamp_site(kind, 40, 40);
            let got = dress(&w); let s = *w.sites.last().unwrap();
            let names: Vec<&str> = got.iter().map(|p| p.prop.name()).collect();
            for &p in need { assert!(got.iter().any(|g| g.prop == p), "{kind:?} has no {} (got {names:?})", p.name()); }
            assert!(got.len() + 1 >= plan(kind).len(), "{kind:?}: only {} of {} placed ({names:?})", got.len(), plan(kind).len());
            for p in &got {
                seen.insert(p.prop);
                assert!(p.x >= s.x0 && p.x + p.w <= s.x0 + s.w, "{:?} leaves the {kind:?}", p.prop);
                for y in p.y..p.y + p.h { for x in p.x..p.x + p.w { assert!(!w.g.is_solidish(y * w.g.w + x), "{:?} in {kind:?} sits in rock at {x},{y}", p.prop); } }
            }
        }
        for p in PROPS { assert!(seen.contains(&p), "{} is never placed", p.name()); }
    }
}
