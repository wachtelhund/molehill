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

/// 16×16 pack icons for the furnishing items.
pub fn icon(it: Item) -> Image {
    let mut p = Px::new(16, 16); let col = it.color();
    if it.block().is_some() { // a block swatch showing its pattern
        p.rect(2, 2, 13, 13, col);
        match it {
            Item::Planks => { for y in [5, 9] { p.rect(2, y, 13, y, sh(col, 0.7)); } p.set(8, 3, sh(col, 0.75)); p.set(5, 7, sh(col, 0.75)); p.set(10, 11, sh(col, 0.75)); }
            Item::PolishedStone => { p.rect(3, 3, 6, 3, sh(col, 1.25)); p.set(10, 9, sh(col, 1.15)); p.set(11, 10, sh(col, 1.15)); }
            Item::Tiles => { for v in [7] { p.rect(2, v, 13, v, [236, 222, 200]); p.rect(v, 2, v, 13, [236, 222, 200]); } }
            _ => { for x in [4, 7, 10] { p.rect(x, 2, x, 13, sh(col, 0.75)); } p.rect(2, 2, 13, 3, sh(col, 1.2)); p.rect(2, 12, 13, 13, sh(col, 1.2)); }
        }
        return p.finish(LINE).image();
    }
    if it.pigment().is_some() { // a little clay pot heaped with the pigment
        p.rect(4, 8, 11, 13, [150, 98, 70]); p.rect(3, 8, 12, 8, [178, 120, 86]); p.rect(5, 14, 10, 14, [120, 76, 54]);
        p.oval(7.5, 7.5, 4.2, 2.6, col); p.set(6, 6, sh(col, 1.3));
        return p.finish(LINE).image();
    }
    // furniture: the sprite scaled to fit, its cloth in the undyed colour
    let (b, c) = furniture(it); let (w, h) = (b.w, b.h); let s = (14. / w.max(h) as f32).min(1.);
    let (ow, oh) = ((w as f32 * s) as i32, (h as f32 * s) as i32); let (ox, oy) = ((16 - ow) / 2, (16 - oh) / 2 + (16 - oh) / 4);
    let u = undyed(it);
    for y in 0..oh { for x in 0..ow { let (sx, sy) = ((x as f32 / s) as i32, (y as f32 / s) as i32);
        let bc = b.get(sx, sy); if bc[3] != 0 { p.set(ox + x, oy + y, [bc[0], bc[1], bc[2]]); }
        if let Some(c) = &c { let cc = c.get(sx, sy); if cc[3] != 0 { let k = cc[0] as f32 / 255.; p.set(ox + x, oy + y, sh(u, k)); } } } }
    p.finish(LINE).image()
}
