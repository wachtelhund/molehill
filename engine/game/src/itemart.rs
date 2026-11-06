//! One hand-drawn 16×16 pack icon per item: its own silhouette and its own pixels, never another item's
//! shape with a tint. Furniture icons are its sprite scaled to fit (`decorart::icon`); devices in the
//! hotbar use their in-world sprite (`devart::device`). `tests` checks that no two icons match.
use crate::art::Px;
use crate::devart::line;
use bevy::image::Image;
use molehill_sim::craft::Item;

const LINE: [u8; 3] = [28, 20, 26];
const WOOD: [u8; 3] = [150, 105, 60];
const WOOD_D: [u8; 3] = [104, 70, 40];
const WOOD_L: [u8; 3] = [196, 146, 92];
const IRON: [u8; 3] = [118, 120, 132];
const IRON_D: [u8; 3] = [78, 80, 92];
const BRASS: [u8; 3] = [196, 156, 72];
const BRASS_D: [u8; 3] = [140, 104, 44];
const COPPER: [u8; 3] = [196, 118, 66];
const ROPE: [u8; 3] = [176, 146, 96];
const STONE: [u8; 3] = [150, 146, 140];
const FLAME: [u8; 3] = [255, 196, 90];
const FLAME_L: [u8; 3] = [255, 244, 200];
fn sh(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k).min(255.) as u8, (c[1] as f32 * k).min(255.) as u8, (c[2] as f32 * k).min(255.) as u8] }
/// Fill the polygon `pts` (pixel centres inside it).
fn poly(p: &mut Px, pts: &[(f32, f32)], c: [u8; 3]) {
    for y in 0..p.h { for x in 0..p.w { let (px, py) = (x as f32 + 0.5, y as f32 + 0.5); let mut inside = false;
        for i in 0..pts.len() { let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            if (a.1 > py) != (b.1 > py) && px < (b.0 - a.0) * (py - a.1) / (b.1 - a.1) + a.0 { inside = !inside; } }
        if inside { p.set(x, y, c); } } }
}
/// Speckle the opaque pixels of `p` inside the box with `c` where the hash says so (1 in `n`).
fn fleck(p: &mut Px, x0: i32, y0: i32, x1: i32, y1: i32, n: u32, seed: u32, c: [u8; 3]) {
    for y in y0..=y1 { for x in x0..=x1 { if p.get(x, y)[3] == 0 { continue; } let h = molehill_sim::grid::hash3(x as u32, y as u32, seed); if h % n == 0 { p.set(x, y, c); } } }
}

/// The icon's pixels before outlining (also used by the tests).
pub fn icon_px(it: Item) -> Px {
    use Item::*;
    let mut p = Px::new(16, 16); let col = it.color();
    match it {
        // ---- earths and stones: each lump has its own outline
        Dirt => { // a crumbly heap with clods rolling off
            poly(&mut p, &[(1.5, 14.), (5., 7.), (8., 5.5), (11., 7.), (14.5, 14.)], col); p.rect(2, 13, 13, 13, sh(col, 0.8));
            for (x, y) in [(6, 9), (9, 8), (8, 11), (11, 11), (5, 12)] { p.set(x, y, sh(col, 0.7)); } p.set(7, 7, sh(col, 1.3)); p.rect(13, 12, 14, 13, sh(col, 0.9)); }
        Clay => { // a squared wet block with a thumb dent and a cut face
            p.rect(3, 5, 12, 13, col); p.rect(3, 5, 12, 6, sh(col, 1.2)); p.oval(8., 9., 2., 1.5, sh(col, 0.8)); p.rect(12, 7, 12, 13, sh(col, 0.75)); p.set(4, 12, sh(col, 0.85)); }
        Sand => { // a low wide cone of grains
            poly(&mut p, &[(0.5, 14.), (8., 8.), (15.5, 14.)], col); fleck(&mut p, 0, 8, 15, 14, 4, 11, sh(col, 0.82)); p.set(8, 8, sh(col, 1.15)); p.set(3, 6, col); p.set(12, 5, col); }
        Gravel => { // three separate pebbles
            p.oval(4.5, 11.5, 3.2, 2.5, col); p.oval(11., 12., 3.6, 2.6, sh(col, 0.9)); p.oval(8., 6.5, 2.6, 2.2, sh(col, 1.1)); p.set(3, 10, sh(col, 1.3)); p.set(10, 11, sh(col, 1.25)); }
        Stone => { // an angular faceted block
            poly(&mut p, &[(2., 10.), (5., 3.), (12., 2.5), (14.5, 8.), (11., 14.), (4., 13.5)], col);
            poly(&mut p, &[(5., 3.), (12., 2.5), (10., 7.), (6., 7.)], sh(col, 1.18)); poly(&mut p, &[(10., 7.), (14.5, 8.), (11., 14.)], sh(col, 0.8)); }
        Coal => { // two glossy black lumps, one on the other
            p.oval(6., 11., 4.6, 3.6, col); p.oval(10.5, 6.5, 3.6, 3.2, sh(col, 1.1)); for (x, y) in [(4, 9), (9, 5), (10, 4)] { p.set(x, y, [130, 130, 146]); } p.set(7, 12, [20, 20, 24]); }
        CopperOre => { // a tall pentagon of rock with copper and green verdigris
            poly(&mut p, &[(4., 14.), (2.5, 7.), (7., 2.), (12.5, 5.), (12., 14.)], [112, 100, 90]);
            for (x, y) in [(5, 6), (6, 6), (9, 9), (10, 9), (10, 10), (5, 11), (6, 11)] { p.set(x, y, col); }
            for (x, y) in [(8, 4), (7, 9), (4, 9), (11, 7), (9, 12)] { p.set(x, y, [96, 186, 140]); } }
        TinOre => { // a wide pale chunk with dark square cassiterite grains
            poly(&mut p, &[(1., 9.), (4., 5.), (12., 4.5), (15., 9.), (12., 13.), (4., 13.)], col);
            for (x, y) in [(5, 7), (9, 6), (11, 10), (6, 10)] { p.rect(x, y, x + 1, y + 1, [60, 56, 64]); } p.set(4, 6, sh(col, 1.2)); }
        IronOre => { // a rust-red kidney lump with round bumps
            p.oval(8., 10., 6., 3.8, col); for cx in [4.5, 8., 11.5] { p.oval(cx, 7., 2.4, 2.2, sh(col, 1.08)); } p.set(4, 6, sh(col, 1.4)); p.set(8, 6, sh(col, 1.4)); p.rect(5, 12, 11, 12, sh(col, 0.7)); }
        Quartz => { // two hexagonal prisms rising from a base
            poly(&mut p, &[(4., 14.), (4., 6.), (6., 3.), (8., 6.), (8., 14.)], col); poly(&mut p, &[(8., 14.), (8., 8.), (10.5, 5.), (13., 8.), (13., 14.)], sh(col, 0.92));
            p.rect(5, 6, 5, 13, [255, 255, 255]); p.rect(11, 9, 12, 13, sh(col, 0.8)); }
        Crystal => { // one tall blue bipyramid
            poly(&mut p, &[(8., 0.5), (12., 7.), (8., 15.5), (4., 7.)], col); poly(&mut p, &[(8., 0.5), (8., 15.5), (4., 7.)], sh(col, 1.2)); p.set(6, 5, [240, 250, 255]); }
        Wood => { // a log on its side, end grain showing
            p.rect(1, 6, 11, 12, col); p.rect(1, 6, 11, 6, sh(col, 1.25)); for x in [3, 7] { p.rect(x, 8, x + 2, 8, sh(col, 0.75)); }
            p.oval(12.5, 9., 2.6, 3.6, [206, 160, 104]); p.oval(12.5, 9., 1.4, 2., [170, 122, 74]); p.set(12, 9, [140, 96, 56]); }
        Fungus => { // a cluster of three luminous shelf caps on a stalk
            p.rect(7, 6, 8, 14, [200, 210, 180]);
            for (cx, cy, r) in [(5., 7., 3.5), (11., 5., 3.2), (8., 11., 3.)] { p.oval(cx, cy, r, 1.6, col); p.rect((cx - r) as i32 + 1, cy as i32, (cx + r) as i32 - 1, cy as i32, sh(col, 0.7)); }
            p.set(4, 6, [200, 255, 220]); p.set(10, 4, [200, 255, 220]); }
        // ---- worked materials
        Brick => { // one fired brick in perspective with a frog
            poly(&mut p, &[(1., 8.), (5., 5.), (15., 5.), (11., 8.)], sh(col, 1.25)); p.rect(1, 8, 11, 12, col); poly(&mut p, &[(11., 8.), (15., 5.), (15., 9.), (11., 13.)], sh(col, 0.75));
            p.rect(6, 6, 10, 6, sh(col, 0.9)); }
        Glass => { // a pane leaning, with a sheen across it
            for y in 2..14 { for x in 4..13 { let xi = x - (13 - y) / 4; p.set(xi, y, if (x + y) % 9 == 0 || (x + y) % 9 == 1 { [250, 255, 255] } else { col }); } } p.rect(2, 14, 13, 14, sh(col, 0.6)); }
        Charcoal => { // two charred sticks crossed, cracked across their grain
            line(&mut p, 2., 12., 13., 4., 3, col); line(&mut p, 3., 4., 12., 13., 2, sh(col, 1.25));
            for (x, y) in [(5, 10), (9, 7), (11, 5), (6, 7), (9, 10)] { p.set(x, y, [220, 90, 40]); } }
        CopperIngot => { // the classic trapezoid
            poly(&mut p, &[(1., 12.), (4., 6.), (12., 6.), (15., 12.)], col); p.rect(4, 6, 11, 7, sh(col, 1.3)); p.rect(1, 12, 14, 12, sh(col, 0.7)); }
        TinIngot => { // a low pig with rounded ends
            p.oval(8., 10., 7., 2.6, col); p.rect(3, 8, 12, 9, sh(col, 1.15)); p.set(4, 8, [255, 255, 255]); p.rect(2, 12, 13, 12, sh(col, 0.75)); }
        BronzeIngot => { // a round bun cast in a dish
            p.oval(8., 9., 6.5, 4.5, col); p.oval(7., 8., 4., 2.4, sh(col, 1.2)); p.oval(8., 9., 1.6, 1.2, sh(col, 0.8)); p.rect(3, 12, 12, 12, sh(col, 0.7)); }
        IronIngot => { // two bars stacked crosswise
            p.rect(2, 10, 13, 13, col); p.rect(2, 10, 13, 10, sh(col, 1.2)); p.rect(5, 3, 9, 10, sh(col, 0.92)); p.rect(5, 3, 9, 3, sh(col, 1.25)); p.set(7, 6, sh(col, 0.7)); }
        // ---- light and structure
        Torch => { // a wrapped stick with a flame
            p.rect(7, 7, 8, 15, WOOD); p.rect(6, 7, 9, 9, [120, 92, 60]); p.oval(7.5, 4.5, 2.6, 3.4, FLAME); p.oval(7.5, 5., 1.2, 1.8, FLAME_L); p.set(8, 0, FLAME); }
        Lamp => { // a hooded lantern with a ring handle and glass
            for a in 0..12 { let t = a as f32 / 12. * std::f32::consts::PI; p.set((7.5 + t.cos() * 2.5).round() as i32, (2.5 - t.sin() * 2.).round() as i32, IRON_D); }
            p.rect(4, 3, 11, 4, IRON); p.rect(5, 5, 10, 12, col); p.rect(7, 7, 8, 10, [255, 255, 236]); for x in [4, 11] { p.rect(x, 5, x, 12, IRON); } p.rect(3, 13, 12, 14, IRON_D); }
        WoodPillar => { // a squared post with a cap and a foot
            p.rect(5, 3, 10, 13, col); p.rect(3, 1, 12, 2, sh(col, 1.2)); p.rect(4, 14, 11, 14, sh(col, 0.8)); for y in [5, 9] { p.rect(6, y, 9, y, sh(col, 0.8)); } p.rect(5, 3, 5, 13, sh(col, 1.15)); }
        IronBeam => { // an I-section seen end-on, the web running back
            p.rect(1, 2, 14, 4, col); p.rect(1, 11, 14, 13, col); p.rect(6, 4, 9, 11, sh(col, 0.85)); p.rect(1, 2, 14, 2, sh(col, 1.3)); for x in [3, 12] { p.set(x, 3, IRON_D); p.set(x, 12, IRON_D); } }
        CopperWire => { // a coil of copperline with a loose end
            for k in 0..3 { let r = 5. - k as f32 * 1.6; for a in 0..28 { let t = a as f32 / 28. * std::f32::consts::TAU; p.set((7.5 + t.cos() * r).round() as i32, (8.5 + t.sin() * r * 0.8).round() as i32, if k == 1 { sh(col, 0.8) } else { col }); } }
            line(&mut p, 12., 8., 15., 2., 1, col); p.set(15, 1, BRASS); }
        Thrummer => { // a stone anvil with the knocking hammer raised
            p.rect(2, 11, 13, 14, STONE); p.rect(2, 11, 13, 11, sh(STONE, 1.2)); line(&mut p, 12., 10., 6., 4., 1, WOOD); p.rect(3, 2, 7, 5, IRON); p.rect(3, 2, 7, 2, sh(IRON, 1.3));
            for (x, y) in [(1, 8), (14, 7), (15, 9)] { p.set(x, y, col); } }
        Gate => { // a knot: a loop of copper cord tied through a brass ring
            p.oval(8., 8., 5., 5., BRASS); p.oval(8., 8., 3., 3., [0, 0, 0]); line(&mut p, 1., 12., 7., 6., 2, col); line(&mut p, 9., 6., 15., 12., 2, sh(col, 0.85)); p.set(8, 3, sh(BRASS, 1.3)); }
        Tnt => { // three bound charges with a lit fuse
            for (k, x) in [3, 6, 9].into_iter().enumerate() { p.rect(x, 6, x + 2, 14, if k == 1 { sh(col, 1.15) } else { col }); p.rect(x, 6, x + 2, 6, sh(col, 1.4)); }
            p.rect(3, 10, 11, 10, [90, 70, 50]); for k in 0..4 { p.set(7 + k / 2, 5 - k, [200, 180, 140]); } p.set(9, 1, [255, 200, 80]); }
        Vent => { // a round louvred grille with a crank
            p.oval(8., 8., 6.5, 6.5, IRON_D); for y in [4, 7, 10] { p.rect(3, y, 12, y + 1, col); p.rect(3, y, 12, y, sh(col, 1.25)); } p.rect(13, 12, 15, 13, BRASS); }
        // ---- dig tools (the hotbar draws them from `art::tool_icon`; these are the pack and drop icons)
        Pickaxe => return crate::art::tool_px(1),
        Drill => return crate::art::tool_px(2),
        Bore => return crate::art::tool_px(3),
        // ---- living things
        Grub => { // a curled, segmented larva
            for a in 0..24 { let t = 0.6 + a as f32 / 24. * 4.6; let (x, y) = (8. + t.cos() * 4.2, 8.5 + t.sin() * 4.); p.oval(x, y, 1.9, 1.9, if a % 4 == 0 { sh(col, 0.8) } else { col }); }
            p.set(12, 7, [40, 30, 30]); p.set(11, 6, [255, 250, 230]); }
        Spore => { // a puffball shedding a drift of spores
            p.oval(6., 10., 4.5, 4., col); p.oval(6., 8.5, 2., 1.2, sh(col, 1.25)); p.set(6, 6, [150, 120, 180]);
            for (x, y) in [(11, 5), (13, 3), (12, 8), (14, 6), (10, 2)] { p.set(x, y, sh(col, 1.15)); } }
        Mushroom => { // a red cap on a pale stem, with spots
            p.rect(6, 8, 9, 14, [236, 226, 206]); p.oval(7.5, 6.5, 6.5, 4., col); p.rect(2, 7, 13, 8, sh(col, 0.8)); for (x, y) in [(5, 4), (9, 3), (11, 6)] { p.set(x, y, [255, 240, 220]); } }
        Glowmite => { // a pale-green glowing beetle with its legs out
            p.oval(8., 9., 4., 5., col); p.rect(8, 5, 8, 13, sh(col, 0.7)); p.oval(8., 4., 2., 1.6, sh(col, 0.6));
            for (y, dx) in [(7, 5), (9, 6), (11, 5)] { p.set(8 - dx, y, [60, 70, 50]); p.set(8 + dx, y, [60, 70, 50]); } p.set(6, 7, [240, 255, 230]); }
        FungusOil => { // a stoppered flask of greenish oil
            p.oval(8., 10.5, 5., 4.2, [180, 210, 220]); p.oval(8., 11.5, 4., 3., col); p.rect(7, 3, 8, 6, [180, 210, 220]); p.rect(6, 1, 9, 2, WOOD); p.set(6, 9, [255, 255, 255]); }
        Root => { // a forked, hairy root
            line(&mut p, 8., 1., 8., 7., 2, col); line(&mut p, 8., 7., 3., 14., 2, sh(col, 0.9)); line(&mut p, 8., 7., 12., 13., 1, sh(col, 0.9)); line(&mut p, 9., 9., 14., 10., 1, sh(col, 0.8));
            for (x, y) in [(4, 11), (11, 11), (6, 3), (13, 9)] { p.set(x, y, sh(col, 1.25)); } }
        // ---- decorative blocks: each a stack or piece of what it is, not a tinted square
        Planks => { // three boards stacked, ends staggered
            for (k, (x0, x1)) in [(1, 12), (3, 14), (0, 11)].into_iter().enumerate() { let y = 4 + k as i32 * 3; p.rect(x0, y, x1, y + 2, if k == 1 { sh(col, 0.92) } else { col }); p.rect(x0, y, x1, y, sh(col, 1.2)); p.set(x1, y + 1, sh(col, 0.7)); } }
        PolishedStone => { // a thin dressed slab with a long gleam
            poly(&mut p, &[(0.5, 11.), (4., 6.), (15.5, 6.), (12., 11.)], col); p.rect(1, 11, 12, 12, sh(col, 0.75)); line(&mut p, 4., 9.5, 12., 7., 1, [236, 232, 224]); }
        Tiles => { // four fired tiles laid with gaps
            for (x, y) in [(1, 1), (8, 1), (1, 8), (8, 8)] { p.rect(x, y, x + 5, y + 5, if (x + y) % 2 == 0 { col } else { sh(col, 0.88) }); p.rect(x, y, x + 5, y, sh(col, 1.2)); } }
        CarvedPillar => { // a fluted column with a capital and a base
            p.rect(2, 1, 13, 3, sh(col, 1.1)); p.rect(4, 4, 11, 12, col); for x in [5, 7, 9] { p.rect(x, 4, x, 12, sh(col, 0.75)); } p.rect(2, 13, 13, 14, sh(col, 0.9)); p.set(3, 2, [210, 206, 196]); }
        // ---- pigments: each kept in its own way
        Ochre => { // a narrow-necked clay jar, yellow earth spilling from its lip
            p.oval(6., 10., 4., 4.5, [150, 98, 70]); p.rect(5, 3, 7, 6, [150, 98, 70]); p.rect(4, 2, 8, 2, [178, 120, 86]); p.oval(7., 9., 1.4, 2., [178, 120, 86]);
            p.oval(12., 13.5, 3.5, 1.6, col); p.rect(9, 3, 10, 4, col); p.set(10, 6, col); p.set(11, 9, col); p.set(12, 12, sh(col, 1.2)); }
        Rust => { // red flakes heaped in a shallow dish
            p.rect(2, 11, 13, 13, [150, 98, 70]); p.rect(1, 11, 14, 11, [178, 120, 86]); poly(&mut p, &[(3., 11.), (6., 5.), (9., 7.), (11., 4.), (13., 11.)], col); for (x, y) in [(6, 7), (10, 6), (8, 9)] { p.set(x, y, sh(col, 1.3)); } }
        Chalk => { // two sticks of chalk, one snapped
            p.rect(2, 4, 5, 13, col); p.rect(2, 4, 5, 4, [255, 255, 250]); p.rect(8, 8, 13, 11, sh(col, 0.95)); p.rect(8, 8, 8, 11, [255, 255, 250]); p.set(14, 13, col); p.set(12, 14, col); }
        Verdigris => { // a scallop shell with green crust scraped into it
            for k in 0..5 { let a = 3.3 + k as f32 * 0.7; line(&mut p, 8., 13., 8. + a.cos() * 7., 13. + a.sin() * 9., 2, [210, 196, 170]); } p.oval(8., 8., 4., 3., col); p.set(7, 7, sh(col, 1.35)); p.rect(7, 13, 9, 14, [180, 166, 140]); }
        Woad => { // a dried woad ball, leaves pressed round
            p.oval(8., 8.5, 5.5, 5.5, col); for (x0, y0, x1, y1) in [(4., 6., 11., 5.), (3., 10., 12., 11.), (6., 13., 10., 3.)] { line(&mut p, x0, y0, x1, y1, 1, sh(col, 0.75)); } p.set(6, 5, sh(col, 1.4)); p.rect(7, 1, 8, 3, [70, 110, 60]); }
        Soot => { // a drawstring pouch, black dust at its mouth
            p.oval(8., 10.5, 5.5, 4.5, [120, 96, 72]); p.rect(6, 4, 9, 6, [120, 96, 72]); p.rect(5, 6, 10, 6, [86, 68, 50]); p.oval(8., 3.5, 2.5, 1.2, col); for (x, y) in [(4, 2), (11, 1), (12, 3)] { p.set(x, y, col); } }
        // ---- furniture: the sprite, scaled (see decorart::icon)
        Rug => { // a rug laid flat in perspective: a diamond worked in, fringe at both ends
            poly(&mut p, &[(0.5, 13.), (4., 6.), (15.5, 6.), (12., 13.)], col); poly(&mut p, &[(8., 7.), (10.5, 9.5), (8., 12.), (5.5, 9.5)], sh(col, 1.3)); p.set(8, 9, sh(col, 0.6));
            for k in 0..4 { p.set(1 + k * 3, 14, WOOD_L); p.set(5 + k * 3, 5, WOOD_L); } }
        Nest => { // a woven basket bed heaped with a blanket and a pillow (the 9×3-tile sprite scaled was a sliver)
            p.oval(8., 11., 7., 3.6, WOOD); for x in (2..14).step_by(3) { p.rect(x, 9, x, 14, WOOD_D); } p.rect(1, 9, 14, 9, WOOD_L);
            p.oval(9., 7.5, 5.5, 3., col); p.rect(5, 7, 13, 7, sh(col, 0.8)); p.oval(4., 6.5, 2.6, 1.8, [222, 212, 188]); }
        Table | Stool | Shelf | Banner | Planter => return crate::decorart::icon_px(it),
        // ---- relics of the Ancients
        AncientBit => { // a toothed ring of an unknown metal, too true to be hand-cut
            for k in 0..10 { let t = k as f32 / 10. * std::f32::consts::TAU; p.rect((7.5 + t.cos() * 6.).round() as i32, (7.5 + t.sin() * 6.).round() as i32, (8. + t.cos() * 6.).round() as i32, (8. + t.sin() * 6.).round() as i32, sh(col, 0.85)); }
            p.oval(8., 8., 5., 5., col); p.oval(8., 8., 2., 2., [0, 0, 0]); p.set(5, 5, [230, 250, 255]); }
        GroundLens => { // a thick round lens in a dull ring, two glints
            p.oval(8., 8., 6.5, 6.5, [110, 116, 120]); p.oval(8., 8., 5., 5., col); p.oval(6., 6., 1.6, 1.0, [255, 255, 255]); p.set(10, 11, [240, 250, 255]); }
        // ---- transport
        Rail => { // a length of copper strap on two sleepers, side on
            p.rect(0, 6, 15, 7, [222, 146, 86]); p.rect(0, 8, 15, 8, [150, 86, 50]); for x in [2, 10] { p.rect(x, 9, x + 3, 12, [96, 64, 40]); } p.set(5, 6, [96, 186, 140]); p.set(12, 7, [96, 186, 140]); }
        Cart => { // a plank tub with a bronze rim on two wheels
            poly(&mut p, &[(1., 4.), (15., 4.), (13., 11.), (3., 11.)], col); p.rect(0, 3, 15, 4, [205, 150, 70]); for y in [7, 9] { p.rect(3, y, 12, y, sh(col, 0.75)); }
            for cx in [4.5, 11.5] { p.oval(cx, 12.5, 2.4, 2.4, [60, 56, 54]); p.set(cx as i32, 12, [150, 146, 140]); } }
        RopeLadder => { // ropes and rungs, rolled at the bottom
            for x in [3, 12] { p.rect(x, 0, x, 11, ROPE); } for y in [2, 5, 8] { p.rect(4, y, 11, y, WOOD); }
            p.oval(8., 13., 6., 2.4, sh(ROPE, 0.85)); p.oval(8., 13., 3.5, 1.2, WOOD_D); }
        RopeGun => { // a root-wood bow-spring launcher, side on: the stock running down to a grip, the spring bent
            // across the fore end with its cord drawn back to the catch, a brass crank, a stone hook in the muzzle
            let root = [122, 84, 52]; line(&mut p, 1., 12., 9., 7., 2, root); p.rect(1, 12, 3, 15, sh(root, 0.8)); p.set(4, 10, sh(root, 1.25));
            for y in 1..14 { let x = 10 + ((y as f32 - 7.).abs() * 0.4) as i32; p.set(x, y, root); p.set(x + 1, y, sh(root, 0.8)); }
            line(&mut p, 11., 1.5, 6., 8.5, 1, [206, 180, 128]); line(&mut p, 11., 13.5, 6., 8.5, 1, [206, 180, 128]);
            p.oval(6.5, 11., 1.4, 1.4, BRASS); p.set(6, 13, BRASS_D);
            p.rect(11, 6, 13, 7, STONE); p.set(14, 5, sh(STONE, 1.25)); p.set(15, 4, sh(STONE, 1.25)); p.set(14, 8, sh(STONE, 0.8)); p.set(15, 9, sh(STONE, 0.8)); }
        HookRope => { // a coil of twisted fungus-fibre rope, its loose end running out to a barbed stone hook lashed on
            for r in [5.5f32, 4.0] { for k in 0..48 { let t = k as f32 / 48. * std::f32::consts::TAU; let (x, y) = (7. + r * t.cos(), 10. + r * 0.7 * t.sin());
                p.set(x as i32, y as i32, if k % 4 < 2 { ROPE } else { sh(ROPE, 0.68) }); } }
            line(&mut p, 12., 8., 13., 4., 1, ROPE); p.rect(12, 3, 13, 4, [120, 96, 64]);
            p.rect(13, 1, 13, 2, STONE); p.set(14, 0, sh(STONE, 1.25)); p.set(11, 0, sh(STONE, 1.1)); p.set(12, 1, sh(STONE, 1.1)); p.set(15, 1, sh(STONE, 0.85)); }
        Lift => { // a cage hung from a sheave
            p.oval(8., 2.5, 2.4, 2.4, IRON); p.set(8, 2, BRASS); p.rect(8, 5, 8, 6, ROPE); p.rect(3, 6, 12, 7, WOOD); p.rect(3, 13, 12, 14, WOOD);
            for x in [3, 6, 9, 12] { p.rect(x, 7, x, 13, col); } }
        Tube => { // a pressure-tube length with flanged ends
            p.rect(2, 5, 13, 11, col); p.rect(2, 5, 13, 6, sh(col, 1.25)); for x in [0, 1, 14, 15] { p.rect(x, 3, x, 13, BRASS); } p.rect(6, 7, 9, 9, sh(col, 0.8)); }
        TubeHopper => { // a brass funnel mouth on a stub of tube
            poly(&mut p, &[(1., 2.), (15., 2.), (10., 9.), (6., 9.)], col); p.rect(1, 2, 14, 3, sh(col, 1.3)); p.rect(5, 9, 10, 15, [150, 190, 186]); p.rect(4, 11, 11, 11, BRASS_D); p.oval(8., 4.5, 4., 1., [70, 50, 30]); }
        // ---- production (production.rs)
        Chute => { // a timber trough running down to the right, its lip on top, a lump sliding down it
            poly(&mut p, &[(0., 3.), (3., 3.), (16., 11.), (16., 15.), (13., 15.), (0., 7.)], col); line(&mut p, 1., 3.5, 15., 11.5, 1, WOOD_L);
            line(&mut p, 1., 6.5, 13., 14.5, 1, WOOD_D); for (x, y) in [(4, 6), (9, 9)] { p.set(x, y, WOOD_D); } p.oval(6.5, 3.5, 1.8, 1.5, STONE); p.set(6, 3, sh(STONE, 1.3)); }
        Screen => { // an iron grating in a timber frame, the mesh open between the bars
            p.rect(1, 3, 14, 12, IRON); p.rect(0, 2, 15, 3, WOOD); p.rect(0, 12, 15, 13, WOOD_D);
            for y in [5, 8, 11] { for x in [2, 5, 8, 11] { p.rect(x, y - 1, x + 1, y - 1, [0, 0, 0]); } }
            for x in [4, 7, 10, 13] { p.rect(x, 4, x, 11, IRON_D); } p.c.iter_mut().for_each(|c| if c[0] == 0 && c[1] == 0 && c[2] == 0 { c[3] = 0; }); p.set(1, 2, WOOD_L); }
        Bumper => { // a buffer stop: a squat timber post with a red pad on the front, standing on a rail end
            p.rect(0, 13, 15, 14, [222, 146, 86]); p.rect(0, 15, 15, 15, [150, 86, 50]); p.rect(6, 3, 12, 12, WOOD); p.rect(6, 3, 12, 4, WOOD_L);
            line(&mut p, 12., 4., 15., 12., 2, WOOD_D); p.rect(2, 5, 5, 9, col); p.rect(2, 5, 5, 5, sh(col, 1.3)); p.set(9, 7, IRON); p.set(9, 10, IRON); }
        OreHopper => { // a timber bin heaped with ore, tapering to a spout, on two splayed legs
            poly(&mut p, &[(1., 4.), (15., 4.), (11., 10.), (5., 10.)], col); p.rect(1, 4, 14, 4, WOOD_L); for y in [6, 8] { p.rect(4, y, 11, y, WOOD_D); }
            p.oval(8., 3.5, 5.5, 2.2, [130, 86, 70]); p.set(6, 2, [206, 116, 52]); p.set(10, 3, [206, 116, 52]); p.rect(7, 10, 8, 12, IRON_D);
            line(&mut p, 3., 8., 1., 15., 1, WOOD_D); line(&mut p, 13., 8., 15., 15., 1, WOOD_D); }
        Riffle => { // three low bars across a sloping sluice floor, water running over them
            poly(&mut p, &[(0., 6.), (16., 12.), (16., 15.), (0., 9.)], WOOD_D); for (k, x) in [2., 7., 12.].into_iter().enumerate() { let y = 6.5 + x * 0.375;
                p.rect(x as i32, y as i32 - 3, x as i32 + 1, y as i32, if k % 2 == 0 { col } else { sh(col, 1.15) }); }
            line(&mut p, 0., 4., 16., 10., 1, [92, 150, 210]); p.set(5, 5, [180, 210, 240]); p.set(10, 7, [180, 210, 240]); }
        Shaft => { // an iron rod in two bearing blocks
            p.rect(0, 7, 15, 8, col); p.rect(0, 7, 15, 7, sh(col, 1.3)); for x in [3, 11] { p.rect(x - 1, 5, x + 1, 10, WOOD_D); p.set(x, 7, IRON_D); } p.set(15, 8, IRON_D); }
        Gear => { // a toothed wheel with a hub and spokes
            p.oval(8., 8., 6., 6., col); for k in 0..8 { let a = k as f32 * std::f32::consts::PI / 4.; p.rect((8. + 6.6 * a.cos()) as i32, (8. + 6.6 * a.sin()) as i32, (8. + 6.6 * a.cos()) as i32 + 1, (8. + 6.6 * a.sin()) as i32 + 1, sh(col, 0.8)); }
            p.oval(8., 8., 3.5, 3.5, sh(col, 1.15)); p.oval(8., 8., 1.4, 1.4, IRON_D); line(&mut p, 8., 3., 8., 13., 1, sh(col, 0.85)); }
        WaterWheel => { // a paddle wheel dipping into blue water
            p.rect(0, 13, 15, 15, [72, 128, 200]); p.oval(8., 7., 6., 6., WOOD_D); p.oval(8., 7., 4.5, 4.5, [0, 0, 0]);
            p.c.iter_mut().for_each(|c| if c[0] == 0 && c[1] == 0 && c[2] == 0 { c[3] = 0; });
            for (x0, y0, x1, y1) in [(8., 1., 8., 13.), (2., 7., 14., 7.), (4., 3., 12., 11.), (12., 3., 4., 11.)] { line(&mut p, x0, y0, x1, y1, 1, col); } p.oval(8., 7., 1.5, 1.5, IRON); p.set(5, 13, [180, 210, 240]); }
        Belt => { // a fibre belt over two rollers, an arrow of stitching along it
            p.rect(2, 5, 13, 6, col); p.rect(2, 10, 13, 11, sh(col, 0.8)); for x in [2.5, 13.5] { p.oval(x, 8., 2.6, 2.6, WOOD); p.set(x as i32, 8, IRON_D); }
            for x in [6, 8, 10] { p.set(x, 5, ROPE); } p.set(11, 4, ROPE); p.set(11, 6, ROPE); }
        Counterweight => { // a rope drum and a hanging iron weight
            p.rect(3, 0, 12, 3, WOOD); p.oval(7.5, 1.5, 2., 1.5, IRON_D); p.rect(7, 4, 8, 7, ROPE); p.rect(4, 8, 11, 15, col); p.rect(4, 8, 11, 9, sh(col, 1.25)); p.set(6, 12, IRON_D); p.set(9, 12, IRON_D); }
        Boiler => { // a brick firebox with a glowing door under a riveted iron drum
            p.rect(1, 8, 14, 15, [150, 74, 56]); for (x, y) in [(3, 10), (8, 12), (12, 10), (5, 14)] { p.rect(x, y, x + 2, y, [118, 56, 44]); }
            p.rect(5, 11, 9, 14, [40, 30, 28]); p.rect(6, 12, 8, 13, FLAME); p.oval(8., 5., 7., 3.5, IRON); p.rect(2, 4, 13, 4, sh(IRON, 1.25)); for x in [3, 8, 13] { p.set(x, 6, IRON_D); } p.rect(11, 0, 12, 2, IRON_D); }
        Engine => { // a steam engine side on: an upright iron cylinder banded in brass on a timber bed, its piston rod
            // up to a crosshead, the connecting rod down to a big spoked flywheel's crank pin, a valve letting off a puff
            p.rect(0, 13, 15, 15, WOOD_D); p.rect(0, 13, 15, 13, WOOD);
            p.rect(1, 5, 5, 12, col); p.rect(1, 5, 1, 12, sh(col, 1.35)); for y in [6, 10] { p.rect(1, y, 5, y, BRASS); } p.rect(0, 4, 6, 4, IRON_D);
            p.rect(3, 1, 3, 3, IRON); p.rect(2, 1, 4, 1, BRASS_D); // piston rod and crosshead
            p.oval(11., 8.5, 4.6, 4.6, IRON_D); p.oval(11., 8.5, 3.4, 3.4, [0, 0, 0]); p.c.iter_mut().for_each(|c| if c[0] == 0 && c[1] == 0 && c[2] == 0 { c[3] = 0; });
            line(&mut p, 11., 4.5, 11., 12.5, 1, IRON); line(&mut p, 7., 8.5, 15., 8.5, 1, IRON); p.oval(11., 8.5, 1.2, 1.2, BRASS);
            line(&mut p, 4., 1.5, 13., 6.5, 1, BRASS); p.set(13, 6, [240, 210, 120]); // connecting rod to the crank pin
            p.set(6, 2, [226, 230, 236]); p.set(7, 1, [210, 214, 222]); p.set(8, 0, [196, 200, 210]); } // a puff of steam
        SteamPipe => { // a copper pipe with a flanged joint and a wisp of steam
            p.rect(0, 7, 15, 10, col); p.rect(0, 7, 15, 7, sh(col, 1.3)); p.rect(7, 5, 8, 12, BRASS_D); p.set(3, 4, [220, 224, 230]); p.set(4, 3, [220, 224, 230]); p.set(12, 4, [200, 206, 214]); }
        Ash => { // a heap of fine pale ash spilling from a shovel's worth, soft-edged, a few cinders and a last ember in it
            p.oval(8., 12.5, 7., 3., sh(col, 0.9)); p.oval(8., 11., 5., 3.2, col); p.oval(7.5, 9.5, 2.8, 2., sh(col, 1.18)); p.set(7, 8, sh(col, 1.3));
            for (x, y) in [(3, 13), (12, 12), (9, 14)] { p.set(x, y, [70, 66, 66]); } p.set(10, 10, [255, 140, 50]); p.set(5, 12, [210, 90, 40]);
            for (x, y) in [(1, 11), (14, 10), (2, 9)] { p.set(x, y, sh(col, 1.1)); } } // drifting flakes
        Slag => { // a frothy glassy clinker, rust-streaked, its broken face shining blue and violet, a drip run off it
            poly(&mut p, &[(1., 11.), (3., 5.), (7., 3.), (12., 4.), (15., 8.), (13., 12.), (8., 13.)], col);
            for (x, y) in [(4, 8), (8, 6), (11, 9), (6, 11), (12, 6)] { p.set(x, y, [20, 18, 24]); } // bubbles
            line(&mut p, 9., 4., 13., 9., 1, [96, 120, 190]); p.set(10, 5, [170, 150, 230]); p.set(12, 7, [140, 190, 230]); // the glassy face
            line(&mut p, 2., 9., 6., 12., 1, [140, 70, 40]); // a rust streak
            p.rect(8, 13, 8, 15, sh(col, 0.85)); p.set(8, 15, [60, 54, 66]); } // a drip that ran and set
        StampMill => { // a timber frame with two iron stamps over a mortar, a cam shaft across the top
            p.rect(1, 1, 2, 15, WOOD_D); p.rect(13, 1, 14, 15, WOOD_D); p.rect(1, 2, 14, 3, WOOD); p.oval(8., 2.5, 1.6, 1.6, IRON_D);
            for x in [5, 9] { p.rect(x, 4, x + 1, 9, IRON); p.rect(x - 1, 9, x + 2, 11, IRON_D); } p.rect(2, 12, 13, 15, col); p.rect(3, 13, 12, 13, sh(col, 0.7)); }
        Washer => { // a rocking tub with water in it and two spouts
            poly(&mut p, &[(1., 5.), (15., 5.), (13., 13.), (3., 13.)], WOOD); p.rect(3, 6, 12, 8, [72, 128, 200]); p.rect(3, 6, 12, 6, [150, 196, 236]);
            p.rect(0, 11, 2, 12, IRON_D); p.rect(13, 11, 15, 12, IRON_D); p.oval(8., 14., 4., 1.2, WOOD_D); p.set(6, 7, [196, 150, 96]); }
        Smelter => { // a tapering brick stack with a glowing hearth and a tap hole
            poly(&mut p, &[(3., 15.), (4., 2.), (12., 2.), (13., 15.)], col); for (x, y) in [(6, 4), (9, 7), (5, 10), (10, 12)] { p.rect(x, y, x + 2, y, sh(col, 0.75)); }
            p.rect(6, 11, 10, 14, [40, 24, 20]); p.rect(7, 12, 9, 13, FLAME); p.rect(13, 13, 15, 13, FLAME_L); p.rect(5, 0, 11, 1, sh(col, 0.6)); }
        Bellows => { // two leaf-shaped boards with a pleated side and a nozzle
            poly(&mut p, &[(1., 4.), (10., 2.), (14., 8.), (10., 14.), (1., 12.)], col); p.rect(2, 7, 11, 9, sh(col, 0.7)); for x in [3, 6, 9] { p.set(x, 8, sh(col, 1.25)); }
            p.rect(13, 7, 15, 9, BRASS_D); p.rect(0, 6, 1, 10, WOOD_D); }
        Mould => { // a sand-and-clay block with an ingot-shaped hollow
            p.rect(1, 4, 14, 13, col); p.rect(1, 4, 14, 5, sh(col, 1.2)); p.rect(4, 7, 11, 10, sh(col, 0.6)); p.rect(5, 8, 10, 9, sh(col, 0.5)); fleck(&mut p, 1, 4, 14, 13, 5, 31, sh(col, 0.9)); }
        CopperFines | TinFines | IronFines => { // washed concentrate heaped in what it was washed in, glinting with its metal, each
            // in its own vessel: copper in a shallow wooden pan (brown grains, green flecks, copper glints), tin in a deep
            // clay bowl (near-black grains, silver glints), iron in a wooden scoop with a handle (red-black grains, grey shine)
            let (grain, speck, glint) = match it { CopperFines => ([150, 92, 56], [92, 168, 124], [240, 160, 96]), TinFines => ([52, 50, 58], [96, 96, 108], [226, 230, 240]), _ => ([96, 44, 36], [60, 40, 40], [176, 176, 188]) };
            let top = match it {
                CopperFines => { poly(&mut p, &[(0., 10.), (16., 10.), (13., 14.), (3., 14.)], WOOD); p.rect(3, 13, 12, 13, WOOD_D); p.oval(8., 10., 6., 3., grain); p.rect(0, 10, 15, 10, sh(WOOD, 1.25)); 7 }
                TinFines => { p.oval(8., 10.5, 5.5, 5., [168, 110, 76]); p.rect(2, 4, 13, 9, [0, 0, 0]); p.c.iter_mut().for_each(|c| if c[0] == 0 && c[1] == 0 && c[2] == 0 { c[3] = 0; });
                    p.oval(8., 9., 4.5, 3., grain); p.rect(2, 9, 13, 9, sh([168, 110, 76], 1.2)); p.rect(6, 15, 9, 15, sh([168, 110, 76], 0.7)); 6 }
                _ => { line(&mut p, 0., 6., 5., 9., 2, WOOD_D); poly(&mut p, &[(5., 8.), (15., 8.), (15., 13.), (6., 13.)], WOOD); p.rect(6, 12, 15, 12, WOOD_D); p.oval(10.5, 8., 4.5, 2.6, grain); p.rect(5, 8, 15, 8, sh(WOOD, 1.25)); 6 }
            };
            fleck(&mut p, 4, top, 13, 9, 4, 47, speck); fleck(&mut p, 4, top, 13, 9, 3, 53, glint); }
        Rod => { // a bell crank: a triangular iron plate on a brass pivot, one rod coming in from the side to its upper arm
            // and one running down from its lower arm, each on a clevis pin: how a pull goes round a corner
            poly(&mut p, &[(7., 3.), (13., 9.), (7., 10.)], IRON_D); p.oval(8.5, 8.5, 1.5, 1.5, BRASS); p.set(8, 8, BRASS_D);
            p.rect(0, 3, 7, 4, col); p.rect(0, 3, 7, 3, sh(col, 1.3)); p.oval(7., 3.5, 1.2, 1.2, BRASS_D);
            p.rect(12, 9, 13, 15, col); p.rect(12, 9, 12, 15, sh(col, 1.3)); p.oval(12.5, 9., 1.2, 1.2, BRASS_D);
            p.rect(0, 2, 0, 5, WOOD_D); } // a guide the side rod runs through
        TreadPlate => { // a chequered iron plate on two springs
            p.rect(1, 6, 14, 9, col); for x in (2..14).step_by(3) { p.set(x, 7, sh(col, 1.3)); p.set(x + 1, 8, sh(col, 0.7)); }
            for x in [3, 11] { line(&mut p, x as f32, 10., x as f32 + 1., 14., 1, IRON_D); } p.rect(1, 14, 14, 15, STONE); }
        Float => { // a fat sealed barrel float on its rod, in a line of water
            p.rect(7, 0, 8, 6, IRON); p.oval(7.5, 9., 5.5, 3.5, col); for x in [4, 11] { p.rect(x, 6, x, 12, WOOD_D); } p.rect(0, 12, 15, 15, [72, 128, 200]); p.rect(0, 12, 15, 12, [150, 196, 236]); }
        SluiceGate => { // a planked gate leaf in a frame with a lifting rod and wheel
            p.rect(1, 2, 2, 15, WOOD_D); p.rect(13, 2, 14, 15, WOOD_D); p.rect(3, 6, 12, 15, col); for y in [8, 11, 14] { p.rect(3, y, 12, y, sh(col, 0.75)); }
            p.rect(7, 0, 8, 6, IRON); p.oval(7.5, 1.5, 3., 1.2, IRON_D); }
        Scale => { // a beam on a post with a pan of ore one side and stones the other
            p.rect(7, 4, 8, 14, WOOD_D); p.rect(4, 14, 11, 15, WOOD_D); line(&mut p, 1., 5., 14., 3., 1, BRASS); p.oval(2.5, 9., 2.5, 1., BRASS_D); p.oval(13., 7., 2.5, 1., BRASS_D);
            p.rect(1, 6, 1, 8, ROPE); p.rect(13, 4, 13, 6, ROPE); p.oval(2.5, 8., 1.2, 1., [200, 120, 70]); p.oval(13., 6., 1.5, 1., STONE); }
        Clutch => { // two toothed iron halves on a shaft, a fork lever on one
            p.rect(0, 7, 15, 8, IRON); p.rect(3, 3, 6, 12, col); p.rect(9, 3, 12, 12, sh(col, 0.85)); for y in [4, 7, 10] { p.set(7, y, col); p.set(8, y + 1, sh(col, 0.85)); }
            line(&mut p, 11., 3., 14., 0., 1, WOOD_D); }
        SteamBorer => { // the borer side on, as it stands in the world: a brick firebox at the rear with its door glowing, a riveted
            // iron barrel, the stack letting off a puff, a toothed steel cone of a bore head at the front, two spoked wheels
            p.rect(0, 4, 3, 11, [128, 70, 54]); p.rect(0, 4, 3, 4, [160, 92, 70]); p.rect(1, 7, 2, 9, FLAME); // firebox and door
            p.oval(7.5, 8., 5., 3.4, IRON); p.rect(3, 5, 11, 5, sh(IRON, 1.3)); for x in [5, 8, 11] { p.set(x, 10, IRON_D); } // barrel and rivets
            p.rect(6, 1, 7, 4, IRON_D); p.set(7, 0, [214, 214, 220]); p.set(8, 0, [196, 196, 204]); // stack and a puff
            for x in 12..16 { let hw = 16 - x; for y in 8 - hw..8 + hw { p.set(x, y, if (x + y) % 2 == 0 { [196, 198, 210] } else { [110, 112, 124] }); } } // the bore head
            p.rect(0, 12, 12, 12, IRON_D);
            for cx in [3.5f32, 9.5] { p.oval(cx, 13., 2.6, 2.6, [56, 56, 64]); p.rect(cx as i32, 11, cx as i32, 15, [96, 96, 106]); p.rect(cx as i32 - 2, 13, cx as i32 + 2, 13, [96, 96, 106]); p.set(cx as i32, 13, BRASS); } }
        StoreWagon | WorkshopWagon | BunkWagon | WaterWagon | CoalWagon => { // the wagon as it stands in the world (`art::wagon_px`): each
            // kind its own body, on two spoked iron wheels; the store and coal wagons show a load heaped above the rim
            let k = match it { StoreWagon => 0, WorkshopWagon => 1, BunkWagon => 2, WaterWagon => 3, _ => 4 };
            let body = crate::art::wagon_px(k); for y in 0..12 { for x in 0..16 { let c = body.get(x, y); if c[3] > 0 { p.set(x, y + 1, [c[0], c[1], c[2]]); } } }
            match k { 0 => { p.rect(3, 2, 5, 4, [150, 108, 64]); p.rect(9, 3, 12, 4, [196, 180, 140]); } 4 => { p.oval(8., 4., 4.5, 1.6, [28, 26, 30]); p.set(6, 3, [70, 68, 76]); } 3 => { p.rect(12, 7, 13, 9, [80, 140, 210]); } _ => {} }
            for cx in [4., 11.] { p.oval(cx, 13.5, 2.2, 2.2, [56, 56, 64]); p.rect(cx as i32, 12, cx as i32, 15, [96, 96, 106]); p.set(cx as i32, 13, BRASS); } }
        // ---- background linings: a piece of each, darker, as it sits behind
        RockWall => { // a rough slab of scraped host rock with tool marks
            poly(&mut p, &[(1., 3.), (6., 1.), (14., 2.), (15., 9.), (13., 15.), (3., 14.), (1., 9.)], sh(col, 0.85)); for y in [5, 9, 12] { line(&mut p, 3., y as f32, 11., y as f32 - 2., 1, sh(col, 0.6)); } }
        TimberWall => { // boards standing on end, uneven tops, gaps between
            for (k, x) in [1, 5, 9, 13].into_iter().enumerate() { let top = [2, 1, 3, 2][k]; p.rect(x, top, x + 2, 15, if k % 2 == 0 { col } else { sh(col, 0.9) }); p.set(x + 1, top + 4, WOOD_D); } }
        MasonryWall => { // laid courses with the joints staggered, a stepped edge
            for r in 0..4 { let y = 2 + r * 3; let off = if r % 2 == 0 { 0 } else { 2 }; for k in 0..3 { let x = off + k * 5; p.rect(x, y, x + 3, y + 1, if (r + k) % 2 == 0 { col } else { sh(col, 0.88) }); } } }
        BeamWall => { // a timber frame with a cross brace
            p.rect(1, 1, 2, 14, col); p.rect(13, 1, 14, 14, col); p.rect(1, 1, 14, 2, sh(col, 1.15)); p.rect(1, 13, 14, 14, col); line(&mut p, 3., 3., 12., 12., 2, sh(col, 0.85)); p.set(7, 7, IRON); }
        PipeWall => { // two pipes on a bracket, a joint on each
            for y in [4, 10] { p.rect(0, y, 15, y + 2, col); p.rect(0, y, 15, y, sh(col, 1.3)); p.rect(6, y - 1, 7, y + 3, BRASS); } p.rect(12, 6, 13, 10, IRON_D); }
        CarvedWall => { // an arched panel with a carved mole's mark
            p.rect(2, 7, 13, 15, col); p.oval(7.5, 7.5, 5.8, 6., col); p.oval(7.5, 9.5, 2., 2., sh(col, 0.65)); p.rect(5, 12, 10, 13, sh(col, 0.7)); p.set(7, 3, sh(col, 1.25)); }
    }
    p
}
pub fn icon(it: Item) -> Image { icon_px(it).finish(LINE).image() }

/// What each item looks like out in the world, for the catalogue and the coverage test: placed blocks
/// are cells of their material, devices and furniture their own sprites, everything else lies about as
/// its icon (`sync_sprites`, kind 1).
pub fn world_form(it: Item) -> &'static str {
    use Item::*;
    if it.furn().is_some() { return "furniture sprite"; }
    if it.is_wall() { return "background lining"; }
    match it {
        Planks | PolishedStone | Tiles | CarvedPillar | Rail | Tube | Chute | Screen | Bumper | Riffle | Shaft | Gear | WaterWheel | Belt | Engine | SteamPipe | Bellows | Mould | Rod | TreadPlate | Float | Clutch | WoodPillar | IronBeam | Brick | Glass => "placed cells",
        Torch => "torch flame", Lamp | Thrummer | Gate | Tnt | Vent => "device sprite", CopperWire => "copperline tile",
        Cart => "cart sprite", RopeLadder => "rope ladder", HookRope => "rope chain and stone hook", RopeGun => "held rope gun (loaded or empty)", Lift => "lift cage", TubeHopper => "hopper mouth", OreHopper => "ore hopper bin", Counterweight => "counterweight on its drum", Boiler => "boiler shell cells", StampMill => "stamp mill shell cells", Washer => "washer shell cells", Smelter => "smelter shell cells", SluiceGate => "sluice gate cells", Scale => "scale pan cells", SteamBorer => "borer body, head and rider", StoreWagon | WorkshopWagon | BunkWagon | WaterWagon | CoalWagon => "wagon body on wheels",
        Mushroom | Spore => "mushroom plot (planted)",
        _ => "loose (icon)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use molehill_sim::craft::ITEMS;
    /// The pixels of a finished sprite as (alpha mask, luminance band 0..3 relative to its own range).
    /// A tinted copy of another sprite has the same mask and bands, so it scores 0.
    pub fn signature(p: &Px) -> Vec<u8> {
        let lum = |c: [u8; 4]| 0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32;
        let ls: Vec<f32> = p.c.iter().filter(|c| c[3] != 0).map(|&c| lum(c)).collect();
        let (lo, hi) = ls.iter().fold((f32::MAX, f32::MIN), |(a, b), &l| (a.min(l), b.max(l)));
        p.c.iter().map(|&c| if c[3] == 0 { 0 } else { 1 + (((lum(c) - lo) / (hi - lo).max(1.) * 3.99) as u8) }).collect()
    }
    pub fn distance(a: &[u8], b: &[u8]) -> usize { a.iter().zip(b).filter(|(x, y)| x != y).count() }
    pub fn masks_differ(a: &[u8], b: &[u8]) -> usize { a.iter().zip(b).filter(|(x, y)| (**x == 0) != (**y == 0)).count() }

    #[test] fn every_item_has_its_own_icon() {
        let px: Vec<Px> = ITEMS.iter().map(|&it| icon_px(it).finish(LINE)).collect();
        for (k, p) in px.iter().enumerate() {
            let opaque = p.c.iter().filter(|c| c[3] != 0).count(); assert!(opaque > 40, "{:?} is nearly empty ({opaque} px)", ITEMS[k]);
            let mut cols: Vec<[u8; 4]> = p.c.iter().filter(|c| c[3] != 0).copied().collect(); cols.sort(); cols.dedup();
            assert!(cols.len() >= 4, "{:?} is a flat box ({} colours)", ITEMS[k], cols.len());
        }
        let sig: Vec<Vec<u8>> = px.iter().map(signature).collect();
        let mut bad = vec![];
        for a in 0..sig.len() { for b in a + 1..sig.len() {
            let (d, m) = (distance(&sig[a], &sig[b]), masks_differ(&sig[a], &sig[b]));
            if d < 20 || m < 8 { bad.push(format!("{:?}/{:?} (pattern {d}, silhouette {m})", ITEMS[a], ITEMS[b])); }
        } }
        assert!(bad.is_empty(), "icons that share art: {}", bad.join(", "));
    }
    /// Every slot icon fills enough of its 16×16 to read in the hotbar (the fuse, plate and nest were slivers).
    #[test] fn every_slot_icon_fills_its_slot() {
        let cover = |im: Image| im.data.unwrap().chunks(4).filter(|c| c[3] != 0).count();
        for &it in ITEMS.iter() { let n = cover(icon(it)); assert!(n >= 80, "{:?} icon covers only {n} of 256 px", it); }
        for &d in molehill_sim::logic::DEVS.iter() { let n = cover(crate::devart::hotbar_icon(d)); assert!(n >= 80, "{:?} hotbar icon covers only {n} of 256 px", d); }
    }
    #[test] fn every_item_says_how_it_shows_in_the_world() {
        for &it in ITEMS.iter() { assert!(!world_form(it).is_empty()); }
        assert_eq!(world_form(Item::Table), "furniture sprite"); assert_eq!(world_form(Item::Gravel), "loose (icon)");
    }
}
