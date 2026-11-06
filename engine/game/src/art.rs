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
pub fn tool_icon(tier: u8) -> Image { tool_px(tier).finish(ICON_LINE).image() }
/// The dig tool's icon pixels before outlining.
pub fn tool_px(tier: u8) -> Px {
    let mut p = Px::new(16, 16); let wood = [150, 105, 60];
    match tier {
        1 => { for k in 0..10 { p.set(4 + k, 14 - k, wood); p.set(5 + k, 14 - k, shade(wood, 0.8)); } // haft
               for x in 2..14 { let y = 3 + ((x as f32 - 8.).powi(2) / 9.) as i32; p.set(x, y, [150, 150, 162]); p.set(x, y + 1, [110, 110, 122]); } } // curved stone head
        2 => { p.rect(1, 6, 6, 10, [184, 115, 70]); p.rect(2, 6, 4, 6, [220, 150, 100]); p.rect(3, 11, 4, 14, wood); // copper body, crank
               for x in 7..16 { let hw = ((15 - x) as f32 * 0.45) as i32; p.rect(x, 8 - hw, x, 8 + hw, if x % 2 == 0 { [170, 176, 190] } else { [130, 136, 150] }); } } // fluted steel cone
        _ => { p.rect(0, 4, 6, 12, [84, 86, 98]); p.rect(2, 6, 4, 10, [255, 120, 40]); p.rect(3, 7, 3, 9, [255, 220, 140]); // housing, heat coil
               for x in 7..16 { let hw = ((15 - x) as f32 * 0.6) as i32; p.rect(x, 8 - hw, x, 8 + hw, if x % 2 == 0 { [120, 124, 140] } else { [96, 100, 116] }); } }
    }
    p
}
/// A mine cart in the world (20×10, 1 px = 1 world unit): a plank tub with a bronze rim and iron bands,
/// on two spoked iron wheels.
pub fn cart() -> Image {
    let mut p = Px::new(20, 10); let wood = [132, 94, 58];
    p.rect(1, 1, 18, 7, wood); p.rect(0, 0, 19, 1, [205, 150, 70]); for y in [4] { p.rect(1, y, 18, y, shade(wood, 0.78)); }
    for x in [5, 14] { p.rect(x, 2, x, 7, [92, 94, 106]); } p.set(2, 2, shade(wood, 1.25)); p.set(10, 6, shade(wood, 0.7));
    for cx in [4.5f32, 15.5] { p.oval(cx, 7.5, 2.6, 2.6, [64, 62, 66]); p.set(cx as i32, 7, [170, 160, 140]); }
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
/// The Delver, 26×20 pixels facing right (1 pixel = 1 world unit; 4 pixels = 1 tile). Climb frames are
/// turned nose-up (20×26). Velvet plum fur, pale spade claws, pink star of a nose, a tiny eye, and a
/// brass delving hat with a fungus-oil lamp on its brow. `lamp` = the lamp is lit (a bright lens) or out.
/// The lamp's lens sits at pixel (24, 6) of an upright frame: `World::headlamp` puts the beam there.
pub fn mole_sheet(lamp: bool) -> Vec<Image> {
    const FUR: [u8; 3] = [96, 80, 92]; const BELLY: [u8; 3] = [122, 104, 112]; const CLAW: [u8; 3] = [238, 204, 188]; const NAIL: [u8; 3] = [255, 240, 228];
    const HAT: [u8; 3] = [190, 150, 70]; const BRIM: [u8; 3] = [132, 96, 44]; const LAMP_CASE: [u8; 3] = [88, 74, 60];
    const LENS_LIT: [u8; 3] = [255, 244, 176]; const LENS_OUT: [u8; 3] = [70, 64, 52];
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
        // delving hat: a brass dome over the crown with a darker brim, the lamp on its brow
        let hy = (by - 3.).floor(); let hyi = hy as i32;
        for y in 0..p.h { for x in 0..p.w { let (dx, dy) = ((x as f32 + 0.5 - 18.) / 5.2, (y as f32 + 0.5 - hy) / 3.9); if dx * dx + dy * dy <= 1. && (y as f32) < hy { p.set(x, y, HAT); } } }
        p.rect(12, hyi, 23, hyi, BRIM);
        p.rect(16, hyi - 3, 17, hyi - 3, shade(HAT, 1.35)); p.set(15, hyi - 2, shade(HAT, 1.2)); // shine
        p.rect(21, hyi - 3, 23, hyi - 1, LAMP_CASE); let lens = if lamp { LENS_LIT } else { LENS_OUT }; p.rect(24, hyi - 3, 24, hyi - 1, lens); p.set(23, hyi - 2, lens);
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
    let (cx, cy) = (x, y); // cell coordinates, for patterns finer than a tile (grout, board seams)
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
        // decorative blocks
        M::Planks => { // 2-tile boards with a dark seam under each and staggered butt joints, grain along the board
            let board = cy / (2 * k); let seam = cy % (2 * k) == 2 * k - 1; let joint = x.wrapping_add(board.wrapping_mul(5)) % 10 == 0;
            if seam { 0.72 } else if joint { 0.8 } else { 0.92 + h(board, cx / 3, 61) * 0.06 + (h(0, cy, 62) - 0.5) * 0.1 } }
        M::Polished => { // smooth slabs with fine joints every metre and a faint diagonal sheen
            let joint = (cx % (4 * k) == 0) || (cy % (4 * k) == 0); if joint { 0.84 } else if (x + y) % 9 == 0 { 1.1 } else { 1.03 + (v - 0.5) * 0.03 } }
        M::Tiles => { // 2-tile squares with pale grout, each square glazed a touch differently
            let (gx, gy) = (cx % (2 * k), cy % (2 * k)); if gx == 2 * k - 1 || gy == 2 * k - 1 { 1.25 } else { 0.94 + h(cx / (2 * k), cy / (2 * k), 63) * 0.12 + if gx == 0 && gy == 0 { 0.1 } else { 0. } } }
        M::Carved => { // fluted column: grooves and ridges down its length, a band every 3 m
            if y % 12 == 0 { 1.15 } else { match x % 3 { 0 => 1.12, 1 => 0.8, _ => 0.98 } } }
        _ => 1.,
    }
}

/// Per-cell colour for materials that need more than a brightness pattern to read at play zoom, at
/// absolute cell (cx, cy); `above` is how many cells of the same material sit straight above (0 = the top
/// face). None keeps the material's own colour.
/// * Rail: a copper strap along the top two rows (old ones flecked with verdigris), and under it dark
///   timber sleepers every half metre with sooty ballast between, so track never reads as a plank floor.
/// * Ores: rounded half-metre nuggets of the ore's own colour (copper flecked green with verdigris, as the
///   quest line describes it; rust-red iron, pale tin, glossy coal) in a darker matrix, so a pocket stands out from the rock without glowing.
pub fn mat_color(m: molehill_sim::mat::Mat, cx: u32, cy: u32, above: u32) -> Option<[f32; 3]> {
    use molehill_sim::grid::hash3;
    use molehill_sim::mat::{Mat as M, CPM};
    let h = |a: u32, b: u32, s: u32| (hash3(a, b, s) & 255) as f32 / 255.;
    let c = CPM as u32;
    match m {
        M::Rail => Some(match above {
            0 => if h(cx, cy, 81) > 0.9 { [92., 150., 122.] } else { [222., 146., 86.] },
            1 => [150., 86., 50.],
            _ => if cx % c < c * 3 / 8 { let g = 0.9 + h(cx / c, 0, 83) * 0.2; [96. * g, 64. * g, 40. * g] } else { [40., 35., 34.] },
        }),
        M::CopperOre | M::IronOre | M::TinOre | M::Coal => {
            let b = c / 2; let row = cy / b; let off = (h(row, 0, 85) * b as f32) as u32;
            let (bx, lx, ly) = ((cx + off) / b, (cx + off) % b, cy % b);
            let corner = (lx == 0 || lx == b - 1) && (ly == 0 || ly == b - 1);
            let nug = !corner && h(bx, row, 87 + m as u32) > 0.45;
            let v = h(cx, cy, 89);
            Some(match (m, nug) {
                (M::CopperOre, true) => if v > 0.6 { [92., 172., 128.] } else if ly == 0 { [236., 150., 88.] } else { [196., 110., 60.] },
                (M::IronOre, true) => if ly == 0 { [200., 104., 72.] } else { [164., 72., 50.] },
                (M::TinOre, true) => if ly == 0 { [222., 224., 234.] } else { [178., 182., 196.] },
                (M::Coal, true) => if ly == 0 && v > 0.5 { [128., 128., 142.] } else { [22., 20., 24.] },
                (M::Coal, false) => [58., 54., 56.],
                _ => [104., 90., 80.],
            })
        }
        // chutes: a smooth timber trough, a worn pale lip on top, boards along it with a dark seam every half
        // metre and a darker underside, so a chute reads as a trough, not a plank floor
        M::Chute => { let g = 0.94 + h(cx / c, cy / (c / 2), 91) * 0.1;
            Some(match above { 0 => [214., 176., 122.], 1 => [182., 140., 90.], _ => if cy % (c / 2) == 0 { [104., 74., 46.] } else { [158. * g, 116. * g, 72. * g] } }) }
        // shaker screens: an iron grating, bars every half metre with the dark mesh between, on a timber
        // top rail, so a screen reads as see-through
        M::Screen => { let q = c / 2; let (bx, by) = (cx % q == 0, cy % q == 0);
            Some(if above == 0 { [150., 108., 62.] } else if bx && by { [176., 178., 186.] } else if bx || by { [128., 130., 140.] } else { [36., 34., 36.] }) }
        // bumpers: a stout timber buffer painted red, a brighter worn head on top, an iron band every
        // metre with a bolt in it
        M::Bumper => { let band = cy % c == c - 1; let g = 0.92 + h(cx, 0, 93) * 0.12;
            Some(if above == 0 { [216., 78., 56.] } else if above == 1 { [120., 40., 32.] } else if band { if cx % 3 == 1 { [196., 196., 204.] } else { [96., 98., 110.] } } else { [152. * g, 60. * g, 42. * g] }) }
        _ => None,
    }
}

/// The ore hopper's parts (1 px = 1 world unit; the bin is `BIN_W`×`BIN_H` cells and the spout one cell
/// under it): 0 the bin's back boards, 1 its front frame with the gate shut, 2 with the gate swung open, 3 a
/// leg (stretched to the floor), 4 a pale heap of lumps the game tints with what the bin holds.
pub fn ore_bin(part: u8) -> Image {
    use molehill_sim::production::{BIN_W, BIN_H};
    let (w, h) = (BIN_W as i32 * 2, BIN_H as i32 * 2 + 2); let wood = [140, 100, 60]; let dark = shade(wood, 0.62); let iron = [92, 94, 106];
    // the bin's outline: straight sides for the top 5/8, then a taper to a spout a quarter of its width
    let half = |y: i32| -> f32 { let top = h * 5 / 8 - 1; if y <= top { w as f32 / 2. } else { let f = (y - top) as f32 / (h - 2 - top) as f32; w as f32 / 2. * (1. - f) + w as f32 / 8. * f } };
    let inside = |x: i32, y: i32| y < h - 2 && ((x as f32 + 0.5) - w as f32 / 2.).abs() < half(y);
    let mut p = Px::new(w, h);
    match part {
        0 => { for y in 0..h { for x in 0..w { if inside(x, y) { p.set(x, y, if y % 4 == 3 { shade(dark, 0.7) } else { shade(dark, 0.9 + (x % 7 == 0) as u8 as f32 * 0.1) }); } } } p.image() }
        1 | 2 => {
            let top = h * 5 / 8; let mid = w / 2;
            for y in 0..h { for x in 0..w { if !inside(x, y) { continue; }
                let edge = !inside(x - 2, y) || !inside(x + 2, y);
                if y <= 1 { p.set(x, y, [200, 150, 82]); } // rim
                else if edge { p.set(x, y, shade(wood, 0.85)); } // corner posts
                else if y >= top { p.set(x, y, if (y - top) % 3 == 2 { shade(wood, 0.7) } else { wood }); } // boarded taper
                else if y == top / 2 + 1 { p.set(x, y, shade(wood, 0.95)); } // cross brace
            } }
            for y in [top + 1, h - 4] { for x in [3, w - 4] { if inside(x, y) { p.set(x, y, iron); } } } // bolts
            p.rect(mid - 2, h - 2, mid + 1, h - 2, iron); // spout collar
            if part == 1 { p.rect(mid - 3, h - 1, mid + 2, h - 1, shade(iron, 0.8)); } else { p.rect(mid + 2, h - 2, mid + 2, h - 1, shade(iron, 0.8)); }
            // outline the bin's silhouette only: the open window onto the heap stays open
            let open: Vec<bool> = (0..w * h).map(|i| p.c[i as usize][3] == 0 && inside(i % w, i / w)).collect();
            let mut p = p.finish(ICON_LINE); for (i, o) in open.into_iter().enumerate() { if o { p.c[i] = [0; 4]; } }
            p.image() }
        3 => { let mut p = Px::new(2, 12); p.rect(0, 0, 1, 11, shade(wood, 0.8)); for y in [3, 8] { p.set(0, y, shade(wood, 0.6)); } p.image() }
        _ => { // a heap: a body of lumps with a humped top, pale so the tint gives its colour
            let mut p = Px::new(w - 4, 12);
            for y in 0..12 { for x in 0..w - 4 { let hump = 3. - 3. * (1. - ((x as f32 + 0.5) / (w - 4) as f32 * 2. - 1.).powi(2)); if (y as f32) < hump { continue; }
                let v = (molehill_sim::grid::hash3(x as u32 / 2, y as u32 / 2, 95) & 255) as f32 / 255.; let k = if y as f32 <= hump + 1. { 1.1 } else { 0.86 + v * 0.24 };
                p.set(x, y, [(230. * k).min(255.) as u8, (226. * k).min(255.) as u8, (220. * k).min(255.) as u8]); } }
            p.image() }
    }
}

/// What the mole holds, drawn at the mole's own pixel scale (1 pixel = 1 world unit), pointing right
/// with the grip at the left edge's middle, so the game can turn it toward the aim around the paw.
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Held { Pick, Drill, Bore, Charge, Block([u8; 3]) }
pub fn held_sprite(h: Held) -> Image {
    let wood = [150, 105, 60]; let line = [30, 20, 30];
    let p = match h {
        Held::Pick => { let mut p = Px::new(12, 11); p.rect(0, 5, 9, 5, wood); p.rect(0, 6, 9, 6, shade(wood, 0.78)); // haft
            for y in 0..11 { let x = 9 + ((y as f32 - 5.).abs() * 0.35) as i32; p.set(x, y, [150, 150, 162]); p.set(x + 1, y, [116, 116, 128]); } // curved head
            p.set(9, 0, [196, 196, 208]); p.set(9, 10, [196, 196, 208]); p }
        Held::Drill => { let mut p = Px::new(14, 7); p.rect(0, 1, 4, 5, [184, 115, 70]); p.rect(1, 1, 3, 1, [220, 150, 100]);
            for x in 5..14 { let hw = ((13 - x) as f32 * 0.32) as i32; p.rect(x, 3 - hw, x, 3 + hw, if x % 2 == 0 { [170, 176, 190] } else { [130, 136, 150] }); } p }
        Held::Bore => { let mut p = Px::new(16, 9); p.rect(0, 1, 5, 7, [84, 86, 98]); p.rect(2, 3, 3, 5, [255, 120, 40]); p.set(2, 4, [255, 220, 140]);
            for x in 6..16 { let hw = ((15 - x) as f32 * 0.42) as i32; p.rect(x, 4 - hw, x, 4 + hw, if x % 2 == 0 { [120, 124, 140] } else { [96, 100, 116] }); } p }
        Held::Charge => { let mut p = Px::new(7, 8); for (k, x) in [0, 2, 4].into_iter().enumerate() { p.rect(x, 2, x + 1, 7, if k == 1 { [230, 70, 56] } else { [200, 50, 40] }); }
            p.rect(0, 4, 5, 4, [90, 70, 50]); p.set(3, 1, [120, 100, 80]); p.set(4, 0, [255, 200, 90]); p }
        Held::Block(c) => { let mut p = Px::new(6, 6); p.rect(0, 0, 5, 5, c); p.rect(0, 0, 5, 0, shade(c, 1.25)); p.rect(0, 5, 5, 5, shade(c, 0.75)); p }
    };
    p.finish(line).image()
}

#[cfg(test)]
mod tests {
    use super::mat_color;
    use molehill_sim::mat::{Mat, PROPS, CPM};
    fn lum(c: [f32; 3]) -> f32 { 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2] }
    fn dist(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
    fn col(m: Mat) -> [f32; 3] { PROPS[m as usize].color.map(|v| v as f32) }

    /// Track reads as track, not a plank floor: a bright copper strap on top, and under it dark sleepers
    /// and darker gaps, all far from the wood beam's colour.
    #[test] fn rails_do_not_read_as_a_wood_floor() {
        let wood = col(Mat::WoodBeam);
        let strap: Vec<[f32; 3]> = (0..40).map(|x| mat_color(Mat::Rail, x, 7, 0).unwrap()).collect();
        let copper = strap.iter().filter(|c| c[0] > 200. && c[0] > c[2] * 2.).count(); assert!(copper >= 30, "mostly copper on top");
        let under: Vec<[f32; 3]> = (0..2 * CPM as u32).map(|x| mat_color(Mat::Rail, x, 9, 2).unwrap()).collect();
        assert!(under.iter().all(|&c| lum(c) < 0.6 * lum(wood) && dist(c, wood) > 90.), "sleepers and gaps are dark, not wood-beam tan");
        let (lo, hi) = under.iter().fold((f32::MAX, 0f32), |(a, b), &c| (a.min(lum(c)), b.max(lum(c)))); assert!(hi - lo > 20., "sleepers stand out from the gaps");
    }

    /// An ore pocket is noticeable: a good share of its cells are nuggets clearly unlike plain rock, and
    /// none of it is bright enough to look lit from within.
    #[test] fn plank_texture_survives_wrapped_world_y() {
        use super::mat_tex;
        let _ = mat_tex(Mat::Planks, 10, 0, 0);
        let _ = mat_tex(Mat::Planks, 10, u32::MAX - 7, 0);
        let _ = mat_tex(Mat::Planks, u32::MAX - 11, u32::MAX - 3, 0);
        let _ = mat_tex(Mat::WoodBeam, 8, u32::MAX - 5, 0);
        let _ = mat_tex(Mat::StoneBrick, 8, u32::MAX - 5, 0);
    }

    /// The production blocks read as what they are and apart from each other and from timber floors: a
    /// chute's pale lip, a screen's dark mesh between bars, a bumper's red head.
    #[test] fn production_blocks_have_their_own_look() {
        let patch = |m: Mat| -> Vec<[f32; 3]> { (0..16u32).flat_map(|y| (0..16u32).map(move |x| (x, y))).map(|(x, y)| mat_color(m, x, y + 8, 2).unwrap()).collect() };
        let mean = |v: &[[f32; 3]]| { let n = v.len() as f32; [0, 1, 2].map(|j| v.iter().map(|c| c[j]).sum::<f32>() / n) };
        let screen = patch(Mat::Screen); let mesh = screen.iter().filter(|&&c| lum(c) < 50.).count() as f32 / screen.len() as f32;
        assert!(mesh > 0.4, "the screen's mesh is dark between the bars ({mesh:.2})");
        assert!(screen.iter().any(|&c| lum(c) > 150.), "the screen's bars are bright iron");
        let (chute, bump) = (mean(&patch(Mat::Chute)), mean(&patch(Mat::Bumper)));
        for (a, b, what) in [(chute, mean(&screen), "chute/screen"), (chute, bump, "chute/bumper"), (bump, mean(&screen), "bumper/screen")] { assert!(dist(a, b) > 30., "{what} too alike"); }
        assert!(lum(mat_color(Mat::Chute, 3, 3, 0).unwrap()) > lum(chute) * 1.2, "a chute's top is a pale worn lip");
        let head = mat_color(Mat::Bumper, 3, 3, 0).unwrap(); assert!(head[0] > 2.5 * head[1], "a bumper's head is painted red");
        let seams = (0..CPM as u32).map(|y| mat_color(Mat::Chute, 4, 16 + y, 2).unwrap()).filter(|&c| lum(c) < 0.75 * lum(chute)).count(); assert!(seams >= 1, "chute boards have seams");
    }

    /// The ore hopper is drawn, not boxes: back boards fill the bin, the front frame leaves its upper part
    /// open so the heap shows, the taper is boarded down to a spout, and the gate looks different open.
    #[test] fn ore_hopper_is_a_drawn_bin() {
        use super::ore_bin; use molehill_sim::production::{BIN_W, BIN_H};
        let (w, h) = (BIN_W * 2, BIN_H * 2 + 2);
        let px = |k: u8| ore_bin(k).data.unwrap(); let a = |d: &[u8], x: usize, y: usize| d[(y * w + x) * 4 + 3];
        let (back, shut, open) = (px(0), px(1), px(2));
        assert_eq!(back.len(), w * h * 4);
        assert!(a(&back, w / 2, h / 4) > 0 && a(&shut, w / 2, 3) == 0, "the bin's upper front is open onto the back boards");
        assert!(a(&shut, w / 2, h * 3 / 4) > 0, "the taper is boarded");
        assert!(a(&back, 0, h - 3) == 0 && a(&shut, 1, h - 3) == 0, "it tapers to a spout");
        assert_ne!(shut, open, "the gate shows open");
        let heap = px(4); assert!(heap.chunks(4).filter(|c| c[3] > 0).all(|c| c[0] > 150), "the heap is pale so its tint gives the ore colour");
    }

    #[test] fn ore_pockets_stand_out_without_glowing() {
        let rock = col(Mat::Rock);
        for m in [Mat::CopperOre, Mat::IronOre, Mat::TinOre, Mat::Coal] {
            let cells: Vec<[f32; 3]> = (0..32u32).flat_map(|y| (0..32u32).map(move |x| (x, y))).map(|(x, y)| mat_color(m, x, y, 1).unwrap()).collect();
            let far = cells.iter().filter(|&&c| dist(c, rock) > 60.).count() as f32 / cells.len() as f32;
            assert!(far > 0.25, "{m:?}: only {far:.2} of the pocket stands apart from rock");
            assert!(cells.iter().all(|&c| lum(c) < 235.), "{m:?}: nothing near-white");
            if m == Mat::CopperOre { let green = cells.iter().filter(|c| c[1] > c[0] + 40.).count(); assert!(green * 10 > cells.len(), "copper ore reads green-flecked"); }
        }
    }
}
