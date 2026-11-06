//! Pixel art for the wiring layer: copperline tiles, devices, and the chunky schematic glyphs of
//! wiring mode. Everything is 16×16 (one 1 m block; 1 pixel = 1 world unit) unless noted.
use crate::art::Px;
use bevy::image::Image;
use molehill_sim::logic::Dev;

const LINE: [u8; 3] = [28, 18, 22];
const IRON: [u8; 3] = [92, 94, 106];
const IRON_D: [u8; 3] = [64, 66, 78];
const WOOD: [u8; 3] = [140, 98, 58];
const WOOD_D: [u8; 3] = [104, 70, 40];
const BRASS: [u8; 3] = [196, 156, 72];
const BRASS_D: [u8; 3] = [140, 104, 44];
const COPPER: [u8; 3] = [176, 104, 58];
const STONE: [u8; 3] = [120, 116, 112];
fn sh(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k).min(255.) as u8, (c[1] as f32 * k).min(255.) as u8, (c[2] as f32 * k).min(255.) as u8] }
/// A line of pixels from (x0,y0) to (x1,y1), `w` thick.
pub fn line(p: &mut Px, x0: f32, y0: f32, x1: f32, y1: f32, w: i32, c: [u8; 3]) {
    let n = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.).ceil().max(1.) as i32;
    for k in 0..=n { let t = k as f32 / n as f32; let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
        for dy in 0..w { for dx in 0..w { p.set(x.floor() as i32 + dx - w / 2, y.floor() as i32 + dy - w / 2, c); } } }
}

/// Neighbour mask bits for a copperline tile.
pub const N: u8 = 1; pub const E: u8 = 2; pub const S: u8 = 4; pub const W: u8 = 8;

/// A copperline tile: a 3-pixel copper wire from the middle to each connected side, with a lit top
/// edge (sheen) and a shaded underside; a brass stud where it ends, a solder blob where it branches.
/// `lit` = carrying current (bright, warm); `f` (0..4) moves a glint outward along a lit wire.
pub fn wire(mask: u8, lit: bool, f: u8) -> Image {
    let mut p = Px::new(16, 16);
    let (core, hi, lo) = if lit { ([255, 186, 92], [255, 236, 176], [214, 128, 58]) } else { (COPPER, [222, 150, 98], [118, 64, 36]) };
    let arms = [(N, 0i32, -1i32), (E, 1, 0), (S, 0, 1), (W, -1, 0)];
    let horiz = |p: &mut Px, x0: i32, x1: i32| { p.rect(x0, 6, x1, 6, hi); p.rect(x0, 7, x1, 8, core); p.rect(x0, 9, x1, 9, lo); };
    let vert = |p: &mut Px, y0: i32, y1: i32| { p.rect(6, y0, 6, y1, hi); p.rect(7, y0, 8, y1, core); p.rect(9, y0, 9, y1, lo); };
    for &(bit, dx, dy) in &arms { if mask & bit == 0 { continue; }
        match (dx, dy) { (1, _) => horiz(&mut p, 7, 15), (-1, _) => horiz(&mut p, 0, 8), (_, 1) => vert(&mut p, 7, 15), _ => vert(&mut p, 0, 8) } }
    let n = mask.count_ones();
    // a bend gets a rounded corner pixel so it reads as bent wire, not two sticks
    if n == 2 && mask != (N | S) && mask != (E | W) { p.rect(6, 6, 9, 9, core); p.set(6, 6, if mask & (N | W) == (N | W) { lo } else { hi }); }
    if n != 2 || mask == 0 { // end stud / junction blob / lone coil
        let c = if n >= 3 { core } else { BRASS }; p.oval(8., 8., 2.8, 2.8, c); p.set(7, 6, if lit { [255, 250, 220] } else { sh(c, 1.35) }); }
    if lit { // glint travelling outward along each arm
        let d = 2 + f as i32 * 2;
        for &(bit, dx, dy) in &arms { if mask & bit != 0 { p.set(7 + dx * d, 7 + dy * d, [255, 255, 236]); } } }
    p.finish(LINE).image()
}

/// A device in a 1 m block. `on`: its powered / lit / pressed state; `f`: animation frame (0..3).
/// The hotbar icon for a device: its sprite, except where the in-world sprite is a sliver along the floor
/// of its block (fuse, pressure plate) and would read as nothing in a slot.
pub fn hotbar_icon(d: Dev) -> Image {
    match d {
        Dev::Fuse => { let mut p = Px::new(16, 16); // a coil of fuse cord with a free end
            for k in 0..3 { let r = 5.5 - k as f32 * 1.8; for a in 0..30 { let t = a as f32 / 30. * std::f32::consts::TAU; p.set((7. + t.cos() * r).round() as i32, (9. + t.sin() * r * 0.75).round() as i32, if k == 1 { [110, 86, 60] } else { [150, 118, 82] }); } }
            line(&mut p, 12., 8., 15., 2., 1, [150, 118, 82]); p.set(15, 1, [255, 200, 90]); p.finish(LINE).image() }
        Dev::Plate => { let mut p = Px::new(16, 16); // the plate seen from above at an angle, on its frame
            for y in 5..12 { let o = (11 - y) / 2; p.rect(1 + o, y, 14 - o, y, if y == 5 { sh(IRON, 1.3) } else { IRON }); }
            p.rect(1, 12, 14, 13, IRON_D); for (x, y) in [(4, 7), (11, 7), (3, 10), (12, 10)] { p.set(x, y, BRASS); } p.finish(LINE).image() }
        _ => device(d, false, 0),
    }
}
pub fn device(d: Dev, on: bool, f: u8) -> Image {
    let mut p = Px::new(16, 16);
    let glow = [255, 230, 120];
    match d {
        Dev::Lever => { p.rect(3, 12, 12, 14, WOOD); p.rect(3, 12, 12, 12, sh(WOOD, 1.25)); p.rect(4, 14, 11, 14, WOOD_D); // wooden base
            p.set(4, 13, IRON); p.set(11, 13, IRON); // nails
            let tip = if on { (12.5, 3.5) } else { (3.5, 3.5) }; line(&mut p, 8., 11., tip.0, tip.1, 2, IRON);
            p.oval(tip.0, tip.1, 1.8, 1.8, if on { [210, 70, 56] } else { [150, 56, 48] }); p.oval(8., 11.5, 1.6, 1.3, BRASS); }
        Dev::Plate => { let y = if on { 13 } else { 12 }; p.rect(1, y, 14, y + 1, IRON); p.rect(1, y, 14, y, sh(IRON, 1.3)); p.rect(2, 15, 13, 15, IRON_D);
            p.set(3, y, BRASS); p.set(12, y, BRASS); if !on { p.set(4, 14, IRON_D); p.set(11, 14, IRON_D); } }
        Dev::Sensor => { p.rect(6, 2, 9, 3, BRASS); p.oval(8., 8.5, 4.3, 4.6, [70, 84, 96]); p.oval(8., 8.5, 3.3, 3.6, [120, 150, 170]);
            if on { for y in 9..13 { for x in 5..11 { if p.get(x, y)[3] != 0 { p.set(x, y, [70, 130, 210]); } } } p.set(6, 9, [160, 200, 250]); }
            p.set(6, 6, [220, 236, 244]); p.rect(5, 13, 10, 14, BRASS_D); }
        Dev::Door | Dev::Floodgate => { let (c, d2) = if d == Dev::Door { (WOOD, WOOD_D) } else { (IRON, IRON_D) };
            p.rect(0, 0, 1, 15, c); p.rect(14, 0, 15, 15, c); p.rect(0, 0, 15, 1, c); p.rect(0, 1, 15, 1, d2);
            for y in [3, 12] { p.set(1, y, if d == Dev::Door { IRON } else { BRASS }); p.set(14, y, if d == Dev::Door { IRON } else { BRASS }); }
            // a floodgate is iron, braced across while shut, with the hand-wheel that winds it on top
            if d == Dev::Floodgate { if !on { p.rect(2, 7, 13, 8, IRON_D); } p.oval(8., 3.5, 2.6, 2.6, BRASS); p.oval(8., 3.5, 1.2, 1.2, [0, 0, 0]); for y in 0..16 { for x in 0..16 { if p.get(x, y) == [0, 0, 0, 255] { p.c[(y * 16 + x) as usize] = [0; 4]; } } } } }
        Dev::Pump => { p.oval(7., 9., 5.2, 5.2, IRON); p.oval(7., 9., 3.4, 3.4, IRON_D);
            let spokes = if on && f % 2 == 1 { [(1., 1.), (-1., 1.)] } else { [(1., 0.), (0., 1.)] };
            for (dx, dy) in spokes { line(&mut p, 7. - dx * 3., 9. - dy * 3., 7. + dx * 3., 9. + dy * 3., 1, BRASS); }
            p.rect(12, 7, 15, 9, COPPER); p.rect(12, 7, 15, 7, sh(COPPER, 1.3)); if on { p.set(15, 8, [110, 170, 230]); } }
        Dev::Lamp => { p.rect(7, 0, 8, 1, IRON_D); p.rect(5, 2, 10, 3, IRON); let gl = if on { glow } else { [88, 84, 66] };
            p.rect(5, 4, 10, 11, gl); if on { p.rect(7, 6, 8, 9, [255, 250, 220]); }
            for x in [5, 10] { p.rect(x, 4, x, 11, IRON); } p.rect(5, 12, 10, 13, IRON); p.rect(6, 14, 9, 14, IRON_D); }
        Dev::Tnt => { for (k, x) in [3, 6, 9].into_iter().enumerate() { p.rect(x, 7, x + 2, 14, if k == 1 { [214, 66, 52] } else { [184, 48, 40] }); p.rect(x, 7, x + 2, 7, [236, 110, 90]); }
            p.rect(3, 10, 11, 10, [96, 74, 52]); line(&mut p, 7.5, 6., 9.5, 2.5, 1, [110, 92, 70]);
            if on { p.set(10, 2, if f % 2 == 0 { [255, 240, 150] } else { [255, 140, 50] }); p.set(9, 1, [255, 220, 120]); } }
        Dev::Fuse => { for x in 0..16 { let y = 12 + ((x as f32 * 0.8).sin() * 1.2).round() as i32; p.set(x, y, [150, 118, 82]); p.set(x, y + 1, [110, 86, 60]); }
            if on { let x = (f as i32 * 4 + 2) % 16; let y = 11 + ((x as f32 * 0.8).sin() * 1.2).round() as i32; p.set(x, y, [255, 230, 120]); p.set(x, y - 1, [255, 150, 60]); } }
        Dev::And | Dev::Or | Dev::Not | Dev::Xor | Dev::Latch | Dev::Timer => {
            // a brass knot plate: inputs come in on the west and north, the output leaves east
            p.rect(2, 2, 13, 13, BRASS); p.rect(2, 2, 13, 2, sh(BRASS, 1.25)); p.rect(2, 13, 13, 13, BRASS_D);
            for (x, y) in [(3, 3), (12, 3), (3, 12), (12, 12)] { p.set(x, y, BRASS_D); }
            p.rect(0, 7, 1, 8, COPPER); if d != Dev::Not { p.rect(7, 0, 8, 1, COPPER); }
            p.rect(14, 7, 15, 8, if on { [255, 200, 96] } else { COPPER });
            glyph(&mut p, d, [74, 50, 30], f);
            p.set(12, 7, if on { [255, 236, 120] } else { [90, 66, 40] }); }
        Dev::Tx | Dev::Rep => { p.rect(3, 12, 12, 14, STONE); p.rect(3, 12, 12, 12, sh(STONE, 1.25)); // the knocking stone
            let up = !(on && f % 2 == 0); let hy = if up { 4. } else { 9. };
            line(&mut p, 13., if up { 9. } else { 11. }, 7., hy, 1, WOOD); p.rect(4, hy as i32 - 1, 8, hy as i32 + 1, IRON); p.rect(4, hy as i32 - 1, 8, hy as i32 - 1, sh(IRON, 1.3));
            if d == Dev::Rep { p.rect(11, 2, 13, 4, BRASS); p.set(14, 3, BRASS_D); }
            if on && !up { p.set(2, 10, [255, 240, 200]); p.set(13, 10, [255, 240, 200]); } }
        Dev::Rx => { // an ear-trumpet pressed to the rock
            for x in 3..13 { let r = 1 + (x - 3) / 3; p.rect(x, 8 - r, x, 8 + r, if x % 3 == 0 { BRASS_D } else { BRASS }); }
            p.rect(1, 7, 2, 9, BRASS_D); p.rect(3, 12, 12, 14, STONE);
            if on { for (x, y) in [(14, 4), (15, 8), (14, 12)] { p.set(x, y, [255, 240, 170]); } } }
        Dev::Vent => { p.rect(1, 1, 14, 14, IRON_D); p.rect(2, 2, 13, 13, [44, 46, 54]);
            let blades: &[(f32, f32)] = if on && f % 2 == 1 { &[(1., 1.), (-1., 1.)] } else { &[(1., 0.), (0., 1.)] };
            for &(dx, dy) in blades { line(&mut p, 7.5 - dx * 4.5, 7.5 - dy * 4.5, 7.5 + dx * 4.5, 7.5 + dy * 4.5, 2, IRON); }
            p.oval(7.5, 7.5, 1.4, 1.4, BRASS); }
        Dev::Heat => { p.oval(8., 9., 6.5, 5.5, [58, 50, 50]); let v = if on { [255, 150, 50] } else { [120, 60, 40] };
            line(&mut p, 3., 9., 7., 7., 1, v); line(&mut p, 7., 7., 10., 11., 1, v); line(&mut p, 10., 11., 13., 8., 1, v); if on { p.set(7, 7, [255, 230, 140]); } }
        Dev::Wheel => { p.oval(8., 8., 7., 7., WOOD_D); p.oval(8., 8., 5.2, 5.2, [0, 0, 0]); for y in 0..16 { for x in 0..16 { if p.get(x, y) == [0, 0, 0, 255] { p.c[(y * 16 + x) as usize] = [0; 4]; } } }
            let a0 = if on { f as f32 * 0.5 } else { 0. };
            for k in 0..4 { let a = a0 + k as f32 * std::f32::consts::FRAC_PI_4 * 2.; line(&mut p, 8., 8., 8. + a.cos() * 6.5, 8. + a.sin() * 6.5, 1, WOOD); }
            p.oval(8., 8., 1.5, 1.5, IRON); }
        Dev::None => {}
    }
    p.finish(LINE).image()
}

/// The knot's mark, drawn in a 6×6 area at the plate's centre.
fn glyph(p: &mut Px, d: Dev, c: [u8; 3], f: u8) {
    match d {
        Dev::And => { for cx in [6.3, 9.7] { for a in 0..16 { let t = a as f32 / 16. * std::f32::consts::TAU; p.set((cx + t.cos() * 2.2).round() as i32, (8. + t.sin() * 2.2).round() as i32, c); } } }
        Dev::Or => { line(p, 5., 5., 8., 8., 1, c); line(p, 11., 5., 8., 8., 1, c); line(p, 8., 8., 8., 11., 1, c); }
        Dev::Not => { for a in 0..14 { let t = a as f32 / 16. * std::f32::consts::TAU; p.set((8. + t.cos() * 3.).round() as i32, (8. + t.sin() * 3.).round() as i32, c); } p.set(10, 5, c); p.set(11, 6, c); }
        Dev::Xor => { line(p, 5., 5., 11., 11., 1, c); line(p, 11., 5., 5., 11., 1, c); }
        Dev::Latch => { line(p, 9., 4., 9., 10., 1, c); for a in 0..9 { let t = a as f32 / 16. * std::f32::consts::TAU; p.set((7. + t.cos() * 2.).round() as i32, (10. + t.sin() * 2.).round() as i32, c); } }
        Dev::Timer => { line(p, 8., 4., 8. + if f % 2 == 0 { -2. } else { 2. }, 10., 1, c); p.oval(8. + if f % 2 == 0 { -2. } else { 2. }, 10.5, 1.4, 1.4, c); p.rect(6, 4, 10, 4, c); }
        _ => {}
    }
}

// ---------- wiring mode (schematic) ----------
// White art, tinted at draw time: thick lines, device glyphs in a ring, arrows.

/// A chunky schematic wire (6 px thick) for neighbour mask `mask`.
pub fn s_wire(mask: u8) -> Image {
    let mut p = Px::new(16, 16); let wh = [255, 255, 255];
    if mask & N != 0 { p.rect(5, 0, 10, 10, wh); } if mask & S != 0 { p.rect(5, 5, 10, 15, wh); }
    if mask & E != 0 { p.rect(5, 5, 15, 10, wh); } if mask & W != 0 { p.rect(0, 5, 10, 10, wh); }
    if mask.count_ones() != 2 || mask == 0 { p.rect(4, 4, 11, 11, wh); }
    p.image()
}
/// A schematic device: a thick rounded frame with the device's mark inside.
pub fn s_device(d: Dev) -> Image {
    let mut p = Px::new(16, 16); let wh = [255, 255, 255];
    for y in 0..16 { for x in 0..16 { let edge = x < 2 || y < 2 || x > 13 || y > 13; let corner = (x < 2 || x > 13) && (y < 2 || y > 13); if edge && !corner { p.set(x, y, wh); } } }
    match d {
        Dev::And | Dev::Or | Dev::Not | Dev::Xor | Dev::Latch | Dev::Timer => glyph(&mut p, d, wh, 0),
        Dev::Lever => { line(&mut p, 8., 11., 11., 4., 2, wh); p.rect(5, 11, 10, 12, wh); }
        Dev::Plate => p.rect(4, 10, 11, 11, wh),
        Dev::Sensor => { p.oval(8., 9., 3., 3., wh); line(&mut p, 8., 4., 8., 6., 1, wh); }
        Dev::Door | Dev::Floodgate => { p.rect(5, 4, 6, 11, wh); p.rect(9, 4, 10, 11, wh); if d == Dev::Floodgate { p.rect(5, 7, 10, 8, wh); } }
        Dev::Pump => { p.oval(8., 8., 3.5, 3.5, wh); p.oval(8., 8., 1.6, 1.6, [0, 0, 0]); }
        Dev::Lamp => { p.oval(8., 8., 2.2, 2.2, wh); for (x, y) in [(8, 4), (8, 12), (4, 8), (12, 8), (5, 5), (11, 5), (5, 11), (11, 11)] { p.set(x, y, wh); } }
        Dev::Tnt => { line(&mut p, 5., 5., 11., 11., 2, wh); line(&mut p, 11., 5., 5., 11., 2, wh); }
        Dev::Fuse => { for x in 4..12 { p.set(x, 8 + ((x as f32).sin() * 1.5).round() as i32, wh); } }
        Dev::Tx | Dev::Rep => { for r in [2., 4.] { for a in 0..10 { let t = -0.9 + a as f32 * 0.2; p.set((6. + t.cos() * r).round() as i32, (8. + t.sin() * r).round() as i32, wh); } } p.rect(4, 7, 5, 8, wh); }
        Dev::Rx => { for r in [2., 4.] { for a in 0..10 { let t = std::f32::consts::PI - 0.9 + a as f32 * 0.2; p.set((10. + t.cos() * r).round() as i32, (8. + t.sin() * r).round() as i32, wh); } } p.rect(10, 7, 11, 8, wh); }
        Dev::Vent => { for y in [5, 8, 11] { p.rect(4, y, 11, y, wh); } }
        Dev::Heat => { line(&mut p, 4., 11., 6., 5., 1, wh); line(&mut p, 6., 5., 8., 11., 1, wh); line(&mut p, 8., 11., 10., 5., 1, wh); line(&mut p, 10., 5., 12., 11., 1, wh); }
        Dev::Wheel => { p.oval(8., 8., 3.5, 3.5, wh); line(&mut p, 4., 8., 12., 8., 1, [0, 0, 0]); line(&mut p, 8., 4., 8., 12., 1, [0, 0, 0]); }
        Dev::None => {}
    }
    for v in p.c.iter_mut() { if v[0] == 0 && v[3] != 0 { *v = [0; 4]; } }
    p.image()
}
/// A small arrow pointing right (6×6), rotated at draw time.
pub fn s_arrow() -> Image { let mut p = Px::new(6, 6); for x in 0..6 { let h = (5 - x) as f32 * 0.5; for y in 0..6 { if (y as f32 - 2.5).abs() <= h { p.set(x, y, [255, 255, 255]); } } } p.image() }

#[cfg(test)]
mod tests {
    use super::*;
    use molehill_sim::logic::DEVS;
    /// Every device has its own sprite (the hotbar shows it too): not one shape tinted another colour.
    #[test] fn every_device_has_its_own_sprite() {
        let sig = |d: Dev| { let im = device(d, false, 0); let px = im.data.unwrap(); let lum: Vec<f32> = px.chunks(4).map(|c| if c[3] == 0 { -1. } else { 0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32 }).collect();
            let (lo, hi) = lum.iter().filter(|&&l| l >= 0.).fold((f32::MAX, f32::MIN), |(a, b), &l| (a.min(l), b.max(l)));
            lum.iter().map(|&l| if l < 0. { 0u8 } else { 1 + ((l - lo) / (hi - lo).max(1.) * 3.99) as u8 }).collect::<Vec<u8>>() };
        let s: Vec<Vec<u8>> = DEVS.iter().map(|&d| sig(d)).collect();
        let mut bad = vec![];
        for a in 0..s.len() { for b in a + 1..s.len() { let d = s[a].iter().zip(&s[b]).filter(|(x, y)| x != y).count(); if d < 10 { bad.push(format!("{:?}/{:?} ({d})", DEVS[a], DEVS[b])); } } }
        assert!(bad.is_empty(), "devices that share art: {}", bad.join(", "));
    }
}
