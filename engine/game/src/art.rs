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
pub enum Layer { Strata, Caves, Roots, HillsFar, HillsNear }

/// `surface[x]`, `biome[x]` per cell column; layer image spans [-pad, w+pad] × [-pad, h+pad] cells.
pub fn layer(kind: Layer, surface: &[usize], biome: &[Biome], w: usize, h: usize, pad: usize, res: usize, seed: u32) -> (Vec<u8>, u32, u32) {
    let (tw, th) = (((w + 2 * pad) / res) as u32, (h / res) as u32);
    let mut d = vec![0u8; (tw * th * 4) as usize];
    let n = Noise::new(seed ^ 0xBEEF); let n2 = Noise::new(seed ^ 0xF00D);
    for ty in 0..th { for tx in 0..tw {
        let cx = (tx as i64 * res as i64 - pad as i64).clamp(0, w as i64 - 1) as usize;
        let cxf = tx as f32 * res as f32 - pad as f32; let cyf = ty as f32 * res as f32;
        let s = surface[cx] as f32; let depth = (cyf - s) * 0.25; let tint = biome_tint(biome[cx]);
        let (mut c, mut a): ([f32; 3], f32) = ([0.; 3], 0.);
        match kind {
            Layer::Strata => {
                if depth > 2. {
                    let band = ((cyf + (n.fbm(cxf, 0., 140., 3) - 0.5) * 60.) / 22.).floor();
                    let v = (molehill_sim::grid::hash3(band as i32 as u32, 3, seed) & 255) as f32 / 255.;
                    let deep = smooth(0., 110., depth);
                    let top = [96., 72., 52.]; let mid = [84., 80., 76.]; let bot = [58., 44., 50.];
                    let base = if deep < 0.5 { mix(top, mid, deep * 2.) } else { mix(mid, bot, deep * 2. - 1.) };
                    let grain = n2.fbm(cxf * 1.7, cyf * 3.5, 18., 2);
                    let k = 0.78 + v * 0.22 + (grain - 0.5) * 0.12;
                    c = [base[0] * k * tint[0], base[1] * k * tint[1], base[2] * k * tint[2]]; a = smooth(2., 5., depth);
                }
            }
            Layer::Caves => {
                if depth > 6. {
                    let v = n.fbm(cxf + 700., cyf * 1.4, 60., 4);
                    if v > 0.56 { let e = smooth(0.56, 0.62, v); c = [22. * tint[0], 18. * tint[1], 22. * tint[2]]; a = e * 0.8; }
                    else if v > 0.52 { c = [120. * tint[0], 104. * tint[1], 92. * tint[2]]; a = 0.18; } // rim highlight
                }
            }
            Layer::Roots => {
                if depth > 0. && depth < 22. {
                    let r = (n2.fbm(cxf * 0.6, cyf * 0.15, 30., 3) - 0.5).abs();
                    let r2 = (n.fbm(cxf * 0.25, cyf * 0.6 + 50., 24., 3) - 0.5).abs();
                    let fade = 1. - smooth(6., 22., depth);
                    if r < 0.025 || r2 < 0.018 { c = [70. * tint[0], 48. * tint[1], 30. * tint[2]]; a = 0.85 * fade; }
                }
            }
            Layer::HillsFar | Layer::HillsNear => {
                let near = matches!(kind, Layer::HillsNear);
                let base = surface.iter().sum::<usize>() as f32 / surface.len() as f32;
                let amp = if near { 36. } else { 70. }; let sc = if near { 120. } else { 260. };
                let top = base - (if near { 6. } else { 18. }) - n.fbm(cxf + if near { 0. } else { 999. }, 1., sc, 4) * amp;
                if cyf > top {
                    let t = smooth(top, top + 140., cyf);
                    c = if near { mix([86., 120., 84.], [58., 82., 64.], t) } else { mix([120., 146., 160.], [96., 120., 136.], t) };
                    let tree = near && cyf < top + 3. + 4. * n2.at(cxf / 5., 0.) && (molehill_sim::grid::hash3(cxf as i32 as u32 / 3, 9, seed) % 4 == 0);
                    if tree { c = [52., 84., 58.]; }
                    a = 1.;
                }
            }
        }
        let i = ((ty * tw + tx) * 4) as usize;
        d[i] = c[0].clamp(0., 255.) as u8; d[i + 1] = c[1].clamp(0., 255.) as u8; d[i + 2] = c[2].clamp(0., 255.) as u8; d[i + 3] = (a.clamp(0., 1.) * 255.) as u8;
    } }
    (d, tw, th)
}
pub fn sky(h: u32) -> Image {
    let mut d = vec![0u8; (h * 4) as usize];
    for y in 0..h { let t = y as f32 / h as f32; let c = mix([92., 150., 214.], [206., 226., 238.], t); let i = (y * 4) as usize; d[i] = c[0] as u8; d[i + 1] = c[1] as u8; d[i + 2] = c[2] as u8; d[i + 3] = 255; }
    image(1, h, d, false)
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
}
fn rgb(c: [u8; 3], a: f32) -> [f32; 4] { [c[0] as f32, c[1] as f32, c[2] as f32, a] }
pub fn mole(frame: u32) -> Image {
    let mut c = Canvas::new(40, 30); let bob = if frame % 2 == 1 { 1. } else { 0. };
    c.ellipse(20., 29., 14., 2.2, [0., 0., 0., 0.35]); // shadow
    c.ellipse(11. + bob, 25., 4., 3., rgb([70, 58, 66], 1.)); c.ellipse(27. - bob, 25., 4., 3., rgb([70, 58, 66], 1.)); // feet
    c.ellipse(19., 17., 14., 10., rgb([92, 78, 88], 1.)); // body
    c.ellipse(17., 13., 10., 5., rgb([120, 104, 114], 0.6)); // sheen
    c.ellipse(31., 18., 6., 5.5, rgb([102, 86, 96], 1.)); // head
    c.ellipse(37., 19., 2.6, 2.2, rgb([240, 140, 160], 1.)); // nose
    c.ellipse(32., 16., 1.1, 1.1, rgb([20, 16, 20], 1.)); // eye
    c.ellipse(33., 25., 3.4, 2., rgb([245, 205, 190], 1.)); // claw/hand
    c.image()
}
pub fn icon(col: [u8; 3], kind: u8) -> Image {
    let mut c = Canvas::new(24, 24);
    c.ellipse(12., 21., 9., 2., [0., 0., 0., 0.3]);
    let dark = [col[0] as f32 * 0.6, col[1] as f32 * 0.6, col[2] as f32 * 0.6, 1.];
    match kind {
        0 => { c.ellipse(12., 13., 9., 7.5, dark); c.ellipse(12., 12., 7.8, 6.4, rgb(col, 1.)); c.ellipse(9.5, 9.5, 2.5, 1.6, [255., 255., 255., 0.35]); } // nugget/lump
        1 => { c.rect(3., 7., 21., 17., 2., dark); c.rect(4., 7., 20., 15., 2., rgb(col, 1.)); c.rect(6., 8.5, 14., 10., 1., [255., 255., 255., 0.35]); } // ingot/brick
        2 => { c.shape(dark, |x, y| ((x - 12.).abs() * 0.8 + (y - 12.).abs() * 0.5 - 7.)); c.shape(rgb(col, 1.), |x, y| ((x - 12.).abs() * 0.8 + (y - 12.).abs() * 0.5 - 6.)); } // gem
        _ => { c.rect(10., 6., 14., 20., 1.5, rgb([150, 105, 60], 1.)); c.ellipse(12., 6., 3.4, 4.2, rgb(col, 1.)); } // torch-like
    }
    c.image()
}
pub fn chest(open: bool) -> Image {
    let mut c = Canvas::new(32, 26);
    c.ellipse(16., 24., 14., 2., [0., 0., 0., 0.35]);
    c.rect(3., 10., 29., 24., 2., rgb([110, 70, 40], 1.)); c.rect(3., 15., 29., 17., 0., rgb([200, 160, 60], 1.));
    if open { c.rect(3., 2., 29., 8., 2., rgb([130, 84, 48], 1.)); c.ellipse(16., 11., 9., 3., rgb([255, 220, 90], 1.)); }
    else { c.rect(3., 5., 29., 12., 3., rgb([130, 84, 48], 1.)); c.rect(14., 11., 18., 17., 1., rgb([230, 200, 90], 1.)); }
    c.image()
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
    c.image()
}
