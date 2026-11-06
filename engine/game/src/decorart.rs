//! Pixel art for furnishing: furniture (a base sprite and, for dyeable pieces, a pale cloth layer the
//! pigment tints), and pack icons for decorative blocks, pigments and furniture. 1 pixel = 1 world unit,
//! so a 0.25 m tile is 4 pixels and furniture lines up with the mole and the blocks.
use crate::art::Px;
use bevy::image::Image;
use molehill_sim::craft::Item;

const LINE: [u8; 3] = [32, 22, 20];
const WOOD: [u8; 3] = [158, 112, 66];
const WOOD_L: [u8; 3] = [196, 146, 92];
const WOOD_D: [u8; 3] = [112, 76, 44];
const IRON: [u8; 3] = [100, 100, 112];
fn sh(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k).min(255.) as u8, (c[1] as f32 * k).min(255.) as u8, (c[2] as f32 * k).min(255.) as u8] }
/// Undyed cloth: the colour the pale cloth layer is tinted with when no pigment has touched it.
pub fn undyed(it: Item) -> [u8; 3] { match it { Item::Nest => [150, 168, 108], Item::Rug => [176, 150, 112], _ => [206, 196, 172] } }

/// (base, cloth): the cloth layer is grey-scale, tinted at draw time. Sizes follow `Item::furn` (tiles × 4 px).
pub fn furniture(it: Item) -> (Px, Option<Px>) {
    let f = it.furn().expect("furniture"); let (w, h) = (f.w as i32 * 4, f.h as i32 * 4);
    let mut b = Px::new(w, h); let mut c = Px::new(w, h); let cl = |v: u8| [v, v, v];
    match it {
        Item::Table => { b.rect(0, 2, w - 1, 4, WOOD); b.rect(0, 2, w - 1, 2, WOOD_L); b.rect(1, 5, w - 2, 5, WOOD_D); // top
            for x in [2, w - 5] { b.rect(x, 5, x + 2, h - 1, WOOD); b.rect(x + 2, 5, x + 2, h - 1, WOOD_D); } b.rect(4, 11, w - 6, 11, WOOD_D); // legs, stretcher
            b.rect(20, 0, 22, 1, [190, 186, 176]); b.set(23, 1, [150, 146, 136]); // a cup
            b.oval(10., 1.2, 2.6, 1.2, [150, 96, 70]); b.set(10, 0, [230, 140, 100]); } // a bowl with a mushroom
        Item::Stool => { b.rect(0, 2, w - 1, 4, WOOD); b.rect(0, 2, w - 1, 2, WOOD_L); for x in [1, w - 3] { b.rect(x, 5, x + 1, h - 1, WOOD_D); } b.rect(2, 8, w - 3, 8, WOOD_D); }
        Item::Nest => { // a woven basket bed with a blanket heaped in it
            for y in 6..h { for x in 0..w { let edge = (x as f32 - (w as f32 - 1.) / 2.).abs() / (w as f32 / 2.); if edge < 1. - (y - 6) as f32 * 0.02 { b.set(x, y, if (x + y) % 3 == 0 { WOOD_D } else if (x * 2 + y) % 5 == 0 { WOOD_L } else { WOOD }); } } }
            b.rect(1, 6, w - 2, 6, WOOD_L);
            c.oval(w as f32 / 2., 5., w as f32 / 2. - 2., 3.6, cl(220)); c.oval(w as f32 * 0.3, 3.5, 4., 2.5, cl(240)); // blanket and a pillow
            for x in (3..w - 3).step_by(5) { c.set(x, 5, cl(180)); c.set(x + 1, 6, cl(180)); } }
        Item::Shelf => { // two uprights, four boards with pots, jars and a crystal
            b.rect(0, 0, 1, h - 1, WOOD); b.rect(w - 2, 0, w - 1, h - 1, WOOD); b.rect(0, 0, 0, h - 1, WOOD_L);
            for y in [1, 11, 21, h - 2] { b.rect(0, y, w - 1, y + 1, WOOD); b.rect(0, y, w - 1, y, WOOD_L); }
            b.rect(4, 6, 7, 10, [170, 98, 66]); b.rect(5, 5, 6, 5, [150, 84, 56]); // pot
            b.rect(10, 7, 12, 10, [180, 220, 230]); b.set(11, 6, IRON); b.set(10, 8, [230, 250, 255]); // jar
            b.rect(15, 17, 18, 20, [110, 200, 140]); b.rect(15, 17, 18, 17, [170, 240, 190]); // fungus jar
            b.rect(4, 15, 9, 20, [120, 84, 60]); for x in [5, 7] { b.set(x, 16, [150, 110, 80]); } // a box
            b.oval(19., 28.5, 1.6, 2.6, [120, 200, 255]); b.set(19, 27, [220, 245, 255]); // crystal
            b.rect(5, 26, 11, 29, [140, 136, 128]); b.rect(5, 26, 11, 26, [176, 172, 164]); } // a stone tablet
        Item::Rug => { for x in 0..w { let s = if x % 8 < 2 { 160 } else if x % 8 == 4 { 255 } else { 215 }; c.rect(x, 1, x, h - 1, cl(s)); }
            for x in (0..w).step_by(2) { b.set(x, 0, WOOD_L); } b.set(0, 3, WOOD_L); b.set(w - 1, 3, WOOD_L); } // fringe
        Item::Banner => { b.rect(0, 0, w - 1, 1, WOOD); b.rect(0, 0, w - 1, 0, WOOD_L); b.set(0, 2, WOOD_D); b.set(w - 1, 2, WOOD_D); // rod
            for y in 2..h { for x in 1..w - 1 { let tail = y > h - 5 && (x - w / 2).abs() < (y - (h - 5)); if !tail { c.set(x, y, cl(if x == 1 || x == w - 2 { 190 } else { 230 })); } } }
            for (x, y) in [(5, 7), (6, 7), (4, 8), (7, 8), (5, 9), (6, 9), (5, 10), (6, 10), (4, 12), (7, 12), (5, 11), (6, 11)] { c.set(x, y, cl(150)); } } // a mole's mark
        Item::Planter => { b.rect(0, 6, w - 1, h - 1, WOOD); b.rect(0, 6, w - 1, 6, WOOD_L); for x in [0, w / 2, w - 1] { b.rect(x, 7, x, h - 1, WOOD_D); }
            b.rect(1, 5, w - 2, 5, [88, 64, 46]); // soil
            for (k, x) in [3, 8, 13, 17].into_iter().enumerate() { let t = 1 + (k % 2) as i32; b.rect(x, t + 2, x, 5, [190, 230, 200]); b.oval(x as f32 + 0.5, t as f32 + 1.5, 2.2, 1.4, [96, 220, 140]); b.set(x, t + 1, [200, 255, 220]); } }
        _ => {}
    }
    let cloth = if c.c.iter().any(|p| p[3] != 0) { Some(c) } else { None };
    (b, cloth)
}
pub fn finish(p: Px) -> Image { p.finish(LINE).image() }
/// The cloth layer: outlined like the rest, kept grey so a tint colours it.
pub fn cloth(p: Px) -> Image { p.finish([60, 60, 60]).image() }

/// A furniture item's 16×16 pack icon pixels: the sprite scaled to fit, its cloth in the undyed colour.
pub fn icon_px(it: Item) -> Px {
    let mut p = Px::new(16, 16);
    let (b, c) = furniture(it); let (w, h) = (b.w, b.h); let s = (14. / w.max(h) as f32).min(1.);
    let (ow, oh) = ((w as f32 * s) as i32, (h as f32 * s) as i32); let (ox, oy) = ((16 - ow) / 2, (16 - oh) / 2 + (16 - oh) / 4);
    let u = undyed(it);
    for y in 0..oh { for x in 0..ow { let (sx, sy) = ((x as f32 / s) as i32, (y as f32 / s) as i32);
        let bc = b.get(sx, sy); if bc[3] != 0 { p.set(ox + x, oy + y, [bc[0], bc[1], bc[2]]); }
        if let Some(c) = &c { let cc = c.get(sx, sy); if cc[3] != 0 { let k = cc[0] as f32 / 255.; p.set(ox + x, oy + y, sh(u, k)); } } } }
    p
}

/// Lid clues: index 0-1 carvings, 2 a note, 3-8 odd finds (leaf, feather, blue glass, seed, pebble, bone).
/// Drawn at the mole's pixel scale; a carving is a slab set into the wall, the rest lie on the floor.
pub fn clue(k: usize) -> Image {
    let rock = [138, 132, 124]; let cut = [70, 64, 60];
    let p = match k {
        0 | 1 => { let mut p = Px::new(12, 12); p.rect(0, 0, 11, 11, rock); p.rect(0, 0, 11, 0, sh(rock, 1.2)); p.rect(0, 11, 11, 11, sh(rock, 0.8));
            if k == 0 { for a in 0..16 { let t = a as f32 / 16. * std::f32::consts::TAU; p.set((6. + t.cos() * 2.5).round() as i32, (4. + t.sin() * 2.5).round() as i32, cut); } // the round hole
                for x in [4, 6, 8] { p.rect(x, 8, x, 10, cut); } } // and what falls from it
            else { for (k2, y) in [(0, 9), (1, 6), (2, 3)] { let x = 3 + k2 * 2; p.rect(x, y, x + 1, y + 1, cut); } p.set(9, 1, cut); p.set(9, 2, cut); p.set(8, 2, cut); p.set(10, 2, cut); } // moles climbing, an arrow up
            p }
        2 => { let mut p = Px::new(9, 6); p.rect(0, 1, 8, 5, [196, 170, 128]); p.rect(1, 0, 7, 0, [196, 170, 128]); for y in [2, 4] { p.rect(1, y, 6, y, [110, 86, 60]); } p.set(7, 2, [110, 86, 60]); p.oval(7.5, 4.5, 1.4, 1.2, [120, 116, 110]); p }
        3 => { let mut p = Px::new(9, 6); p.oval(4.5, 3., 4., 2.4, [118, 140, 70]); p.rect(1, 3, 8, 3, [84, 100, 50]); p.set(3, 2, [84, 100, 50]); p.set(6, 4, [84, 100, 50]); p.set(0, 3, [100, 80, 50]); p } // leaf
        4 => { let mut p = Px::new(11, 5); p.oval(5.5, 2., 4.8, 1.6, [222, 218, 206]); p.rect(0, 2, 10, 2, [170, 160, 150]); p.set(1, 1, [240, 236, 228]); p } // feather
        5 => { let mut p = Px::new(6, 6); for y in 0..6 { for x in 0..6 { if x + y >= 2 && x + y <= 8 && (x as i32 - y as i32).abs() <= 3 { p.set(x, y, if x + y < 5 { [150, 200, 240] } else { [90, 140, 210] }); } } } p.set(2, 1, [235, 245, 255]); p } // blue glass
        6 => { let mut p = Px::new(5, 5); p.oval(2.5, 2.5, 2.2, 2., [120, 82, 50]); p.set(1, 1, [170, 128, 86]); p } // seed
        7 => { let mut p = Px::new(7, 5); p.oval(3.5, 2.5, 3.2, 2.2, [150, 150, 158]); p.set(2, 1, [210, 210, 218]); p } // pebble
        _ => { let mut p = Px::new(9, 4); p.rect(2, 1, 6, 2, [232, 228, 214]); p.oval(1., 1.5, 1.2, 1.5, [232, 228, 214]); p.oval(7.5, 1.5, 1.2, 1.5, [232, 228, 214]); p } // bone
    };
    p.finish(LINE).image()
}
/// The sprite index for a clue (see `clue`).
pub fn clue_index(kind: molehill_sim::lore::ClueKind, look: u8) -> Option<usize> {
    use molehill_sim::lore::ClueKind as K;
    match kind { K::Carving => Some((look % 2) as usize), K::Note => Some(2), K::Relic => Some(3 + (look % 6) as usize), K::Draft => None }
}
