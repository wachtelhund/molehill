//! Procedural art: parallax background layers, sprites for moles, items, chests, stations, torches.
use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use molehill_sim::gen::{Biome, Noise};

pub fn image(w: u32, h: u32, data: Vec<u8>, nearest: bool) -> Image {
    let mut im = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    if nearest { im.sampler = ImageSampler::nearest(); }
    im
}
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] { [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t] }
pub fn biome_tint(b: Biome) -> [f32; 3] { match b { Biome::Loam => [1.0, 0.92, 0.82], Biome::Karst => [0.88, 0.95, 1.0], Biome::Crags => [0.9, 0.9, 0.98], Biome::Ashlands => [1.05, 0.8, 0.72] } }
fn smooth(e0: f32, e1: f32, x: f32) -> f32 { let t = ((x - e0) / (e1 - e0)).clamp(0., 1.); t * t * (3. - 2. * t) }

/// Background layer kinds. Each image covers `cells_w × cells_h` world cells at `res` cells per texel.
pub enum Layer { Strata, Caves, Veins }

/// `surface[x]`, `biome[x]` per cell column; layer image spans [-pad, w+pad] × [-pad, h+pad] cells.
/// `oy` = absolute row of window row 0, so the deep-earth bands line up with the endless world.
pub fn layer(kind: Layer, oy: i64, biome: &[Biome], w: usize, h: usize, pad: usize, res: usize, seed: u32) -> (Vec<u8>, u32, u32) {
    let (tw, th) = (((w + 2 * pad) / res) as u32, (h / res) as u32);
    let mut d = vec![0u8; (tw * th * 4) as usize];
    let n = Noise::new(seed ^ 0xBEEF); let n2 = Noise::new(seed ^ 0xF00D);
    for ty in 0..th { for tx in 0..tw {
        let cx = (tx as i64 * res as i64 - pad as i64).clamp(0, w as i64 - 1) as usize;
        let cxf = tx as f32 * res as f32 - pad as f32; let cyf = (ty as i64 * res as i64 + oy) as f32;
        // noise runs in legacy 0.25 m tiles so the rock patterns keep their size in metres at any CPM
        let (cxf, cyf) = (cxf / molehill_sim::mat::SF, cyf / molehill_sim::mat::SF);
        let depth = cyf * 0.25; let tint = biome_tint(biome[cx]);
        let (mut c, mut a): ([f32; 3], f32) = ([0.; 3], 0.);
        match kind {
            Layer::Strata => {
                // deep earth: slanted rock bands, warm loam above the starting layer, cold basalt far below
                let band = ((cyf + cxf * 0.12 + (n.fbm(cxf, 0., 140., 3) - 0.5) * 70.) / 22.).floor();
                let v = (molehill_sim::grid::hash3(band as i32 as u32, 3, seed) & 255) as f32 / 255.;
                let deep = smooth(-60., 120., depth);
                let top = [92., 70., 54.]; let mid = [78., 74., 72.]; let bot = [52., 42., 52.];
                let base = if deep < 0.5 { mix(top, mid, deep * 2.) } else { mix(mid, bot, deep * 2. - 1.) };
                let grain = n2.fbm(cxf * 1.7, cyf * 3.5, 18., 2);
                let k = 0.74 + v * 0.26 + (grain - 0.5) * 0.14;
                c = [base[0] * k * tint[0], base[1] * k * tint[1], base[2] * k * tint[2]]; a = 1.;
            }
            Layer::Caves => {
                let v = n.fbm(cxf + 700., cyf * 1.4, 60., 4);
                if v > 0.56 { let e = smooth(0.56, 0.62, v); c = [20. * tint[0], 16. * tint[1], 22. * tint[2]]; a = e * 0.85; }
                else if v > 0.52 { c = [118. * tint[0], 102. * tint[1], 92. * tint[2]]; a = 0.2; } // rim highlight
            }
            Layer::Veins => {
                // thin mineral veins: rusty roots of ore near the start, pale quartz and glowing ember-veins deeper
                let r = (n2.fbm(cxf * 0.6, cyf * 0.25, 30., 3) - 0.5).abs();
                let r2 = (n.fbm(cxf * 0.3, cyf * 0.6 + 50., 24., 3) - 0.5).abs();
                let t = smooth(0., 80., depth);
                if r < 0.02 { c = mix([120., 84., 56.], [190., 190., 200.], t); a = 0.55; }
                else if r2 < 0.012 { c = mix([150., 110., 70.], [230., 110., 50.], t); a = 0.45 + 0.3 * t; }
            }
        }
        let i = ((ty * tw + tx) * 4) as usize;
        d[i] = c[0].clamp(0., 255.) as u8; d[i + 1] = c[1].clamp(0., 255.) as u8; d[i + 2] = c[2].clamp(0., 255.) as u8; d[i + 3] = (a.clamp(0., 1.) * 255.) as u8;
    } }
    (d, tw, th)
}
/// Mushroom plot sprite; `stage` 0 = sprouting, 1 = growing, 2 = ripe.
pub fn mushroom(stage: u8) -> Image {
    let mut c = Canvas::new(24, 24);
    c.ellipse(12., 22., 9., 1.8, [0., 0., 0., 0.3]);
    let caps: &[(f32, f32, f32)] = match stage { 0 => &[(12., 18., 2.5)], 1 => &[(9., 15., 4.), (16., 17., 3.)], _ => &[(8., 12., 5.5), (16., 14., 4.5), (12., 17., 3.)] };
    for &(x, y, r) in caps {
        c.rect(x - r * 0.3, y, x + r * 0.3, 22., r * 0.2, rgb([226, 214, 190], 1.));
        c.ellipse(x, y, r, r * 0.65, rgb([168, 92, 62], 1.)); c.ellipse(x - r * 0.3, y - r * 0.25, r * 0.35, r * 0.2, [255., 230., 210., 0.45]);
        if stage == 2 { c.ellipse(x + r * 0.35, y - r * 0.1, r * 0.15, r * 0.12, [250., 240., 220., 0.9]); }
    }
    c.pixel()
}

/// Tiny SDF painter for crisp vector-style sprites.
pub struct Canvas { pub w: u32, pub h: u32, pub d: Vec<f32> }
impl Canvas {
    pub fn new(w: u32, h: u32) -> Self { Canvas { w, h, d: vec![0.; (w * h * 4) as usize] } }
    fn put(&mut self, x: u32, y: u32, c: [f32; 4], cov: f32) {
        let i = ((y * self.w + x) * 4) as usize; let a = c[3] * cov; if a <= 0. { return; }
        for k in 0..3 { self.d[i + k] = self.d[i + k] * (1. - a) + c[k] * a; } self.d[i + 3] = self.d[i + 3] + a * (1. - self.d[i + 3]);
    }
    pub fn shape(&mut self, c: [f32; 4], sdf: impl Fn(f32, f32) -> f32) {
        for y in 0..self.h { for x in 0..self.w { let v = sdf(x as f32 + 0.5, y as f32 + 0.5); let cov = (0.5 - v).clamp(0., 1.); if cov > 0. { self.put(x, y, c, cov); } } }
    }
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: [f32; 4]) { self.shape(c, |x, y| { let (dx, dy) = ((x - cx) / rx, (y - cy) / ry); ((dx * dx + dy * dy).sqrt() - 1.) * rx.min(ry) }); }
    pub fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, c: [f32; 4]) {
        self.shape(c, |x, y| { let (cx, cy) = ((x0 + x1) / 2., (y0 + y1) / 2.); let (hx, hy) = ((x1 - x0) / 2. - r, (y1 - y0) / 2. - r); let qx = (x - cx).abs() - hx; let qy = (y - cy).abs() - hy; (qx.max(0.).hypot(qy.max(0.))) + qx.max(qy).min(0.) - r });
    }
    pub fn image(&self) -> Image { image(self.w, self.h, self.d.iter().enumerate().map(|(i, v)| if i % 4 == 3 { (v * 255.) as u8 } else { (v.clamp(0., 255.)) as u8 }).collect(), false) }
    /// Snap to pixel art: hard alpha (soft shadows dropped), a dark outline, nearest sampling.
    pub fn pixel(&self) -> Image {
        let mut p = Px::new(self.w as i32, self.h as i32);
        for i in 0..(self.w * self.h) as usize { let a = self.d[i * 4 + 3]; if a >= 0.5 { p.c[i] = [self.d[i * 4].clamp(0., 255.) as u8, self.d[i * 4 + 1].clamp(0., 255.) as u8, self.d[i * 4 + 2].clamp(0., 255.) as u8, 255]; } }
        let mut im = p.finish(ICON_LINE).image(); im.sampler = ImageSampler::nearest(); im
    }
}
fn rgb(c: [u8; 3], a: f32) -> [f32; 4] { [c[0] as f32, c[1] as f32, c[2] as f32, a] }
fn shade(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k).min(255.) as u8, (c[1] as f32 * k).min(255.) as u8, (c[2] as f32 * k).min(255.) as u8] }
const ICON_LINE: [u8; 3] = [28, 20, 26];
/// 16×16 outlined pixel-art item icon: 0 lump/ore, 1 ingot/brick/beam, 2 gem, 3 torch/lamp, 4 blasting charge.
pub fn icon(col: [u8; 3], kind: u8) -> Image {
    let mut p = Px::new(16, 16);
    match kind {
        0 => { p.oval(8., 9., 5.6, 4.6, shade(col, 0.8)); p.oval(7.5, 8.5, 4.4, 3.6, col); p.oval(9.5, 10.5, 1.2, 1., shade(col, 0.6)); p.rect(5, 6, 6, 6, shade(col, 1.45)); }
        1 => { p.rect(2, 7, 13, 11, shade(col, 0.85)); p.rect(3, 5, 12, 7, col); p.rect(4, 5, 8, 5, shade(col, 1.4)); }
        2 => { for y in 0..16 { for x in 0..16 { let d = (x as f32 - 7.5).abs() * 0.9 + (y as f32 - 8.).abs() * 0.7; if d < 5.4 { p.set(x, y, if x < 8 && y < 8 { shade(col, 1.3) } else if x >= 8 && y >= 8 { shade(col, 0.75) } else { col }); } } } p.set(6, 5, [255, 255, 255]); }
        4 => { for (k, x) in [3, 6, 9].into_iter().enumerate() { p.rect(x, 6, x + 2, 14, if k == 1 { shade(col, 1.15) } else { col }); p.rect(x, 6, x + 2, 6, shade(col, 1.4)); } // three bound charges
               p.rect(3, 10, 11, 10, [90, 70, 50]); for k in 0..4 { p.set(7 + k / 2, 5 - k, [200, 180, 140]); } p.set(9, 1, [255, 200, 80]); } // cord, fuse, spark
        _ => { p.rect(7, 7, 8, 14, [124, 84, 48]); p.rect(6, 9, 9, 9, [70, 70, 80]); p.oval(7.5, 5., 2.6, 3.2, col); p.oval(7.5, 5.6, 1.4, 1.8, shade(col, 1.3)); }
    }
    p.finish(ICON_LINE).image()
}
pub fn chest(open: bool) -> Image {
    let mut c = Canvas::new(32, 26);
    c.ellipse(16., 24., 14., 2., [0., 0., 0., 0.35]);
    c.rect(3., 10., 29., 24., 2., rgb([110, 70, 40], 1.)); c.rect(3., 15., 29., 17., 0., rgb([200, 160, 60], 1.));
    if open { c.rect(3., 2., 29., 8., 2., rgb([130, 84, 48], 1.)); c.ellipse(16., 11., 9., 3., rgb([255, 220, 90], 1.)); }
    else { c.rect(3., 5., 29., 12., 3., rgb([130, 84, 48], 1.)); c.rect(14., 11., 18., 17., 1., rgb([230, 200, 90], 1.)); }
    c.pixel()
}
pub fn station(kind: u8, built: bool) -> Image {
    let mut c = Canvas::new(80, 56); let a = if built { 1. } else { 0.45 };
    c.ellipse(40., 54., 38., 3., [0., 0., 0., 0.35 * a]);
    match kind {
        0 => { c.rect(4., 18., 76., 26., 2., rgb([160, 110, 64], a)); c.rect(8., 26., 14., 54., 1., rgb([120, 80, 46], a)); c.rect(66., 26., 72., 54., 1., rgb([120, 80, 46], a)); c.rect(20., 10., 34., 18., 1., rgb([150, 150, 160], a)); }
        1 => { c.rect(10., 12., 70., 54., 10., rgb([170, 100, 76], a)); c.ellipse(40., 40., 12., 10., rgb([40, 20, 16], a)); if built { c.ellipse(40., 43., 8., 6., rgb([255, 150, 60], 1.)); } }
        2 => { c.rect(14., 4., 66., 54., 4., rgb([110, 104, 112], a)); c.rect(20., 30., 60., 50., 3., rgb([30, 24, 24], a)); if built { c.rect(24., 36., 56., 48., 3., rgb([255, 120, 40], 1.)); } c.rect(28., 0., 40., 8., 1., rgb([90, 84, 92], a)); }
        _ => { c.rect(12., 20., 68., 30., 4., rgb([80, 84, 96], a)); c.rect(30., 30., 50., 46., 1., rgb([70, 74, 86], a)); c.rect(20., 46., 60., 54., 2., rgb([60, 64, 76], a)); }
    }
    if built { c.pixel() } else { c.image() } // the unbuilt ghost stays translucent
}
/// Hotbar icons for the dig tools, 16×16 pixel art: 1 pickaxe, 2 drill (copperline-driven), 3 bore (heavy heat-bore).
pub fn tool_icon(tier: u8) -> Image {
    let mut p = Px::new(16, 16); let wood = [150, 105, 60];
    match tier {
        1 => { for k in 0..10 { p.set(4 + k, 14 - k, wood); p.set(5 + k, 14 - k, shade(wood, 0.8)); } // haft
               for x in 2..14 { let y = 3 + ((x as f32 - 8.).powi(2) / 9.) as i32; p.set(x, y, [150, 150, 162]); p.set(x, y + 1, [110, 110, 122]); } } // curved stone head
        2 => { p.rect(1, 6, 6, 10, [184, 115, 70]); p.rect(2, 6, 4, 6, [220, 150, 100]); p.rect(3, 11, 4, 14, wood); // copper body, crank
               for x in 7..16 { let hw = ((15 - x) as f32 * 0.45) as i32; p.rect(x, 8 - hw, x, 8 + hw, if x % 2 == 0 { [170, 176, 190] } else { [130, 136, 150] }); } } // fluted steel cone
        _ => { p.rect(0, 4, 6, 12, [84, 86, 98]); p.rect(2, 6, 4, 10, [255, 120, 40]); p.rect(3, 7, 3, 9, [255, 220, 140]); // housing, heat coil
               for x in 7..16 { let hw = ((15 - x) as f32 * 0.6) as i32; p.rect(x, 8 - hw, x, 8 + hw, if x % 2 == 0 { [120, 124, 140] } else { [96, 100, 116] }); } }
    }
    p.finish(ICON_LINE).image()
}
/// A soft ring used to preview the mining brush.
pub fn ring() -> Image {
    let mut c = Canvas::new(64, 64);
    c.shape([255., 245., 220., 0.55], |x, y| { let r = ((x - 32.).powi(2) + (y - 32.).powi(2)).sqrt(); (r - 30.).abs() - 1.2 });
    c.shape([255., 245., 220., 0.08], |x, y| ((x - 32.).powi(2) + (y - 32.).powi(2)).sqrt() - 29.);
    let mut im = c.image(); im.sampler = bevy::image::ImageSampler::linear(); im
}

/// Hard-edged pixel grid for pixel-art sprites: shapes fill whole pixels (no anti-aliasing), then
/// `finish` adds top-lit shading and a dark outline so sprites read at any zoom with nearest sampling.
pub struct Px { pub w: i32, pub h: i32, pub c: Vec<[u8; 4]> }
impl Px {
    pub fn new(w: i32, h: i32) -> Self { Px { w, h, c: vec![[0; 4]; (w * h) as usize] } }
    pub fn get(&self, x: i32, y: i32) -> [u8; 4] { if x < 0 || y < 0 || x >= self.w || y >= self.h { [0; 4] } else { self.c[(y * self.w + x) as usize] } }
    pub fn set(&mut self, x: i32, y: i32, c: [u8; 3]) { if x >= 0 && y >= 0 && x < self.w && y < self.h { self.c[(y * self.w + x) as usize] = [c[0], c[1], c[2], 255]; } }
    /// Filled ellipse centred on (cx, cy) with radii (rx, ry), sampled at pixel centres.
    pub fn oval(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: [u8; 3]) {
        for y in 0..self.h { for x in 0..self.w { let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry); if dx * dx + dy * dy <= 1. { self.set(x, y, c); } } }
    }
    pub fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 3]) { for y in y0..=y1 { for x in x0..=x1 { self.set(x, y, c); } } }
    /// Shade (lighter where the pixel above is empty, darker where the one below is), then outline.
    pub fn finish(mut self, outline: [u8; 3]) -> Self {
        let src = self.c.clone(); let at = |x: i32, y: i32| if x < 0 || y < 0 || x >= self.w || y >= self.h { 0 } else { src[(y * self.w + x) as usize][3] };
        for y in 0..self.h { for x in 0..self.w { let i = (y * self.w + x) as usize; if src[i][3] == 0 { continue; }
            let k = if at(x, y - 1) == 0 { 1.22 } else if at(x, y + 1) == 0 || at(x, y + 2) == 0 { 0.78 } else { 1. };
            for j in 0..3 { self.c[i][j] = (src[i][j] as f32 * k).min(255.) as u8; } } }
        for y in 0..self.h { for x in 0..self.w { let i = (y * self.w + x) as usize; if src[i][3] != 0 { continue; }
            if at(x - 1, y) != 0 || at(x + 1, y) != 0 || at(x, y - 1) != 0 || at(x, y + 1) != 0 { self.c[i] = [outline[0], outline[1], outline[2], 255]; } } }
        self
    }
    /// Turn a right-facing sprite so its nose points up (for climbing).
    pub fn rot_up(&self) -> Px { let mut o = Px::new(self.h, self.w); for y in 0..self.h { for x in 0..self.w { let c = self.get(x, y); o.c[((self.w - 1 - x) * o.w + y) as usize] = c; } } o }
    pub fn image(&self) -> Image { image(self.w as u32, self.h as u32, self.c.iter().flatten().copied().collect(), true) }
}

/// Mole animation poses. Frames are indexed by `mole_frame`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pose { Idle, Run, Jump, Fall, Climb, Dig }
/// Frame index into the mole sheet for a pose and a phase counter.
pub fn mole_frame(p: Pose, ph: u32) -> usize { match p { Pose::Idle => (ph % 2) as usize, Pose::Run => 2 + (ph % 4) as usize, Pose::Jump => 6, Pose::Fall => 7, Pose::Climb => 8 + (ph % 2) as usize, Pose::Dig => 10 + (ph % 3) as usize } }
/// The Delver, 26×20 pixels facing right (1 pixel = 1 world unit; 4 pixels = 1 cell). Climb frames are
/// turned nose-up (20×26). Velvet plum fur, pale spade claws, pink star of a nose, a tiny eye.
pub fn mole_sheet() -> Vec<Image> {
    const FUR: [u8; 3] = [96, 80, 92]; const BELLY: [u8; 3] = [122, 104, 112]; const CLAW: [u8; 3] = [238, 204, 188]; const NAIL: [u8; 3] = [255, 240, 228];
    const NOSE: [u8; 3] = [240, 128, 156]; const HIND: [u8; 3] = [178, 140, 136]; const EYE: [u8; 3] = [18, 12, 18]; const LINE: [u8; 3] = [30, 20, 30];
    // body bob (px down), front claw (dx, dy), hind foot (dx, dy), body stretch (rx+, ry+)
    let pose = |bob: f32, claw: (f32, f32), hind: (f32, f32), st: (f32, f32), claw2: Option<(f32, f32)>| -> Px {
        let mut p = Px::new(26, 20); let by = 11. + bob;
        p.oval(5. + hind.0, 16.5 + hind.1, 2.2, 1.5, HIND); // hind foot (far)
        p.rect(1, by as i32 - 1, 2, by as i32, FUR); // tail stub
        p.oval(11.5, by, 8.6 + st.0, 6.2 + st.1, FUR); // body
        p.oval(12.5, by + 3., 6.5 + st.0, 2.6, BELLY); // belly
        p.oval(19., by + 0.5, 4.4, 3.9, FUR); // head
        p.oval(22.5, by + 1., 2.6, 1.9, FUR); // snout
        p.rect(24, by as i32, 25, by as i32 + 1, NOSE); // nose
        p.set(20, by as i32 - 1, EYE); // eye
        p.oval(7.5 + hind.0, 16.5 + hind.1, 2.2, 1.5, HIND); // hind foot (near)
        if let Some(c) = claw2 { p.oval(17. + c.0, 16. + c.1, 2.4, 1.8, CLAW); } // far claw
        p.oval(18.5 + claw.0, 16.5 + claw.1, 3.0, 2.0, CLAW); // big spade claw
        p.set((21. + claw.0) as i32, (16. + claw.1) as i32, NAIL); p.set((21. + claw.0) as i32, (17. + claw.1) as i32, NAIL);
        p.finish(LINE)
    };
    let mut v = vec![
        pose(0., (0., 0.), (0., 0.), (0., 0.), None),                 // idle 0
        pose(0.5, (0., 0.), (0., 0.), (0.3, -0.4), None),             // idle 1 (breath)
        pose(0., (2., -1.), (-2., 0.), (0., 0.), Some((-2., 0.))),    // run 0: reach
        pose(1., (0., 0.), (0., -1.), (0.3, -0.3), Some((1., -1.))),  // run 1: gather
        pose(0., (-2., 0.), (2., 0.), (0., 0.), Some((2., -1.))),     // run 2: push
        pose(1., (0., -1.), (0., 0.), (0.3, -0.3), Some((0., 0.))),   // run 3: gather
        pose(-1., (3., -4.), (-2., -2.), (-0.6, 0.6), None),          // jump: claws up, feet trailing
        pose(0., (2., 1.), (1., 1.), (0.4, -0.4), Some((-1., 1.))),   // fall: claws braced
    ];
    let climb0 = pose(0., (2., -4.), (-2., 0.), (0., 0.), Some((-1., 0.))).rot_up();
    let climb1 = pose(0., (-1., 0.), (1., -1.), (0., 0.), Some((2., -4.))).rot_up();
    v.push(climb0); v.push(climb1);
    v.push(pose(0., (4., -5.), (0., 0.), (0., 0.), None));            // dig 0: wind up
    v.push(pose(0.5, (5., -1.), (0., 0.), (0.3, -0.2), Some((2., -3.)))); // dig 1: strike
    v.push(pose(0., (2., 1.), (0., 0.), (0., 0.), Some((4., -2.))));  // dig 2: scoop
    v.into_iter().map(|p| p.image()).collect()
}

/// Wall torch, 8×12 pixels (2×3 cells): a resin-soaked stick in an iron bracket. Three flame frames.
pub fn torch_frames() -> Vec<Image> {
    (0..3).map(|f| {
        let mut p = Px::new(8, 12);
        p.rect(3, 6, 4, 11, [124, 84, 48]); p.rect(3, 6, 4, 6, [90, 52, 30]); // stick, charred tip
        p.rect(2, 8, 5, 8, [70, 70, 80]); // bracket
        let lean = [0, 1, -1][f] as f32 * 0.4; let tall = [0., 0.6, -0.4][f];
        p.oval(4. + lean * 0.5, 4.2 - tall * 0.5, 2.2, 2.6 + tall, [236, 110, 40]); // outer flame
        p.oval(4. + lean, 4.6 - tall * 0.3, 1.3, 1.8 + tall * 0.6, [255, 200, 80]); // core
        p.set(4, 5, [255, 246, 200]);
        p.finish([34, 22, 18]).image()
    }).collect()
}
/// Soft round glow (linear sampled) drawn around light sources; tinted and faded by the caller.
pub fn glow() -> Image {
    let n = 64; let mut d = vec![0u8; n * n * 4];
    for y in 0..n { for x in 0..n { let (dx, dy) = ((x as f32 + 0.5) / n as f32 * 2. - 1., (y as f32 + 0.5) / n as f32 * 2. - 1.);
        let r = (dx * dx + dy * dy).sqrt(); let a = (1. - r).max(0.).powf(1.8);
        let o = (y * n + x) * 4; d[o] = 255; d[o + 1] = 255; d[o + 2] = 255; d[o + 3] = (a * 255.) as u8; } }
    let mut im = image(n as u32, n as u32, d, false); im.sampler = ImageSampler::linear(); im
}

/// Per-cell pixel-art texture for a solid material at absolute cell (x, y): a brightness multiplier.
/// Rock gets strata, granite salt-and-pepper, basalt columns, bricks a running bond, beams grain and rivets,
/// ores and crystals glints (crystals twinkle with `t`). One cell = one texel, so patterns are cell-sized.
pub fn mat_tex(m: molehill_sim::mat::Mat, x: u32, y: u32, t: u64) -> f32 {
    use molehill_sim::grid::hash3;
    use molehill_sim::mat::Mat as M;
    let h = |a: u32, b: u32, s: u32| (hash3(a, b, s) & 255) as f32 / 255.;
    let v = h(x, y, 31); // per-cell speckle: finer cells, finer grain
    // patterns with a size (bands, bricks, rivets, columns) are laid out in legacy 0.25 m tiles
    let k = molehill_sim::mat::S as u32; let (x, y) = (x / k, y / k);
    match m {
        M::Granite => if v > 0.9 { 1.22 } else if v < 0.1 { 0.8 } else { 1. },
        M::Rock => { let band = h(0, y / 3 + x / 23, 41); 0.92 + band * 0.14 + if v > 0.93 { 0.12 } else { 0. } }
        M::Limestone => { let band = h(0, y / 2 + x / 40, 43); let fossil = if v > 0.97 { 1.2 } else { 1. }; (0.9 + band * 0.16) * fossil }
        M::Basalt => { let col = x / 4 + (h(x / 4, y / 9, 47) * 2.) as u32; if (x + col) % 4 == 0 { 0.84 } else { 1. + (h(x / 4, y / 6, 49) - 0.5) * 0.1 } }
        M::Bedrock => if (x / 3 + y / 2) % 5 == 0 { 0.82 } else { 1. },
        M::Dirt | M::Clay => if v > 0.95 { 1.25 } else if v < 0.06 { 0.78 } else { 1. },
        M::Gravel => if (x + y) % 2 == 0 && v > 0.4 { 1.18 } else { 0.9 },
        M::Coal => if v > 0.86 { 1.6 } else { 1. },
        M::CopperOre | M::TinOre | M::IronOre => if v > 0.82 { 1.35 } else if v < 0.2 { 0.82 } else { 1. },
        M::Quartz | M::Crystal => { let tw = ((t as f32 * 0.05 + v * 40.).sin() * 0.5 + 0.5).powi(6); if v > 0.75 { 1.15 + tw * 0.45 } else { 0.95 } }
        M::StoneBrick => { // 4×2 running bond with dark mortar
            let row = y / 2; let off = if row % 2 == 0 { 0 } else { 2 };
            if y % 2 == 1 || (x + off) % 4 == 0 { 0.72 } else { 1. + (h((x + off) / 4, row, 53) - 0.5) * 0.16 } }
        M::WoodBeam | M::Door => { let grain = h(0, y, 57); let knot = if h(x / 2, y, 59) > 0.96 { 0.72 } else { 1. }; (0.9 + grain * 0.18) * knot }
        M::IronBeam => { let rivet = x % 3 == 1 && y % 3 == 1; let seam = x % 6 == 0; if rivet { 1.3 } else if seam { 0.8 } else { 1. } }
        M::Roots => if (x + y / 2) % 3 == 0 { 0.8 } else { 1.08 },
        M::Fungus => if v > 0.85 { 1.3 } else { 1. },
        _ => 1.,
    }
}
