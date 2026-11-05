//! Material table. One cell = 0.25 m × 0.25 m (× 1 m depth into the screen).
//! Densities in kg/m³, strengths in kPa. Strengths are *rock-mass* values: intact lab values
//! scaled down (×≈0.02–0.05) for jointing/flaws, as is standard in rock mechanics (GSI/Hoek-Brown).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind { Empty, Solid, Powder, Liquid, Gas }

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum Mat {
    Empty = 0, Bedrock, Granite, Rock, Limestone, Basalt, Dirt, Clay, Sand, Gravel,
    Coal, CopperOre, TinOre, IronOre, Quartz, Crystal, Roots, Fungus, Grass,
    WoodBeam, StoneBrick, IronBeam, Door, Water, Lava, Steam, Fire, Glass,
}
pub const NMAT: usize = 28;

pub struct Props {
    pub name: &'static str,
    pub kind: Kind,
    pub density: f32,      // kg/m³
    pub tensile: f32,      // kPa (bottom-fibre bending capacity)
    pub compressive: f32,  // kPa
    pub dig: u16,          // ticks per cell with bare claws
    pub tier: u8,          // tool tier needed: 0 claws, 1 stone, 2 bronze, 3 iron
    pub blast: u8,         // explosive power*10 needed
    pub emit: [u8; 3],     // light emission
    pub opacity: u8,       // light lost entering cell (0..255)
    pub atten: f32,        // (legacy, unused: thrumming uses mat::wave)
    pub color: [u8; 3],
    pub crumbles_to: Mat,  // what a failed cell becomes (as loose powder); built beams break into their own loose debris
    pub repose: f32,       // 0 = flows freely, 1 = never slides (angle of repose)
}
const fn p(name: &'static str, kind: Kind, density: f32, tensile: f32, compressive: f32, dig: u16, tier: u8, blast: u8,
           emit: [u8; 3], opacity: u8, atten: f32, color: [u8; 3], crumbles_to: Mat, repose: f32) -> Props {
    Props { name, kind, density, tensile, compressive, dig, tier, blast, emit, opacity, atten, color, crumbles_to, repose }
}
use Kind::*;
use Mat as X;
pub static PROPS: [Props; NMAT] = [
    p("Empty", Empty, 1.2, 0., 0., 0, 0, 0, [0, 0, 0], 4, 0., [20, 17, 15], X::Empty, 0.),
    p("Bedrock", Solid, 3000., 1e9, 1e9, 0, 9, 99, [0, 0, 0], 255, 99., [30, 32, 40], X::Bedrock, 1.),
    p("Granite", Solid, 2700., 800., 20000., 14, 2, 8, [0, 0, 0], 120, 1.0, [96, 98, 112], X::Gravel, 0.),
    p("Sandstone", Solid, 2300., 300., 8000., 8, 1, 5, [0, 0, 0], 120, 0.6, [170, 132, 92], X::Sand, 0.),
    p("Limestone", Solid, 2500., 400., 9000., 9, 1, 5, [0, 0, 0], 120, 0.6, [178, 170, 150], X::Gravel, 0.),
    p("Basalt", Solid, 2900., 700., 18000., 12, 2, 7, [0, 0, 0], 120, 0.8, [62, 60, 70], X::Gravel, 0.),
    p("Dirt", Solid, 1600., 15., 250., 2, 0, 2, [0, 0, 0], 110, 0.3, [122, 88, 58], X::Dirt, 0.15),
    p("Clay", Solid, 1800., 40., 400., 3, 0, 3, [0, 0, 0], 110, 0.45, [168, 104, 78], X::Clay, 0.45),
    p("Sand", Powder, 1600., 0., 150., 1, 0, 1, [0, 0, 0], 110, 0.3, [222, 196, 132], X::Sand, 0.),
    p("Gravel", Solid, 1900., 3., 300., 2, 0, 2, [0, 0, 0], 110, 0.3, [140, 134, 124], X::Gravel, 0.4),
    p("Coal", Solid, 1350., 60., 2000., 5, 0, 4, [0, 0, 0], 120, 0.5, [40, 38, 42], X::Gravel, 0.),
    p("Copper ore", Solid, 3200., 120., 4000., 8, 1, 5, [0, 0, 0], 120, 0.5, [150, 110, 80], X::Gravel, 0.),
    p("Tin ore", Solid, 3400., 120., 4000., 8, 1, 5, [0, 0, 0], 120, 0.5, [140, 140, 150], X::Gravel, 0.),
    p("Iron ore", Solid, 3800., 150., 5000., 11, 2, 6, [0, 0, 0], 120, 0.7, [130, 86, 70], X::Gravel, 0.),
    p("Quartz", Solid, 2650., 250., 9000., 12, 2, 7, [0, 0, 0], 50, 0.4, [228, 224, 236], X::Gravel, 0.),
    p("Crystal", Solid, 2600., 300., 20000., 10, 2, 6, [90, 170, 255], 40, 0.2, [120, 200, 255], X::Gravel, 0.),
    p("Roots", Solid, 700., 60., 300., 2, 0, 2, [0, 0, 0], 100, 0.2, [118, 84, 50], X::Dirt, 0.2),
    p("Fungus", Powder, 400., 4., 50., 1, 0, 1, [40, 110, 60], 70, 0.2, [96, 168, 110], X::Dirt, 1.0),
    p("Grass", Solid, 1500., 20., 250., 2, 0, 2, [0, 0, 0], 110, 0.3, [92, 140, 60], X::Dirt, 0.15),
    p("Wood beam", Solid, 600., 3000., 8000., 4, 0, 3, [0, 0, 0], 150, 0.2, [196, 150, 90], X::WoodBeam, 0.3),
    p("Stone brick", Solid, 2400., 600., 15000., 8, 1, 6, [0, 0, 0], 160, 0.5, [150, 150, 160], X::Gravel, 0.),
    p("Iron beam", Solid, 7800., 40000., 120000., 20, 2, 9, [0, 0, 0], 200, 3.0, [170, 176, 190], X::IronBeam, 0.),
    p("Door", Solid, 700., 2000., 5000., 4, 0, 3, [0, 0, 0], 150, 0.4, [140, 104, 64], X::WoodBeam, 0.3),
    p("Water", Liquid, 1000., 0., 0., 0, 0, 0, [0, 0, 0], 16, 0.6, [52, 120, 200], X::Water, 0.),
    p("Lava", Liquid, 2700., 0., 0., 0, 0, 0, [255, 110, 30], 30, 2.5, [255, 110, 32], X::Lava, 0.),
    p("Steam", Gas, 0.6, 0., 0., 0, 0, 0, [0, 0, 0], 8, 0., [210, 216, 222], X::Steam, 0.),
    p("Fire", Gas, 0.3, 0., 0., 0, 0, 0, [255, 150, 50], 4, 0., [255, 160, 60], X::Empty, 0.),
    p("Glass", Solid, 2500., 100., 2000., 3, 0, 2, [0, 0, 0], 20, 0.2, [190, 220, 230], X::Sand, 0.),
];
impl Mat {
    #[inline] pub fn from_u8(v: u8) -> Mat { if (v as usize) < NMAT { unsafe { std::mem::transmute(v) } } else { Mat::Empty } }
    #[inline] pub fn p(self) -> &'static Props { &PROPS[self as usize] }
    #[inline] pub fn kind(self) -> Kind { PROPS[self as usize].kind }
    pub fn is_structural(self) -> bool { self.kind() == Kind::Solid }
    /// One plain line for the hover tooltip.
    pub fn desc(self) -> &'static str {
        ["Open tunnel.", "The Deepholt's bones. Nothing digs it.", "Hard grey stone. Needs a drill.", "Soft stone. A pickaxe cuts it.",
         "Pale, even stone. Strong in a roof.", "Dark, hard stone from cooled lava. Needs a drill.", "Loose earth. Quick to dig, weak under load.",
         "Stiff earth. Holds water back.", "Flows when nothing holds it.", "Rubble that shifts when disturbed.", "Black seam. Dig it for coal.",
         "Seam of copper ore.", "Seam of tin ore.", "Seam of iron ore. Needs a drill.", "Clear stone. Light passes through.", "Glowing stone. Lights the dark.",
         "Tangle of roots. Gives wood.", "Soft glowing growth.", "Mossy earth.", "Wooden prop. Holds up what rests on it.", "Laid brick.",
         "Iron girder. Spans wide gaps.", "Opens and shuts with current.", "Fills every hollow it can reach.", "Molten rock. Burns, and lights the dark.",
         "Hot vapour from water meeting lava.", "Burning. Spreads to wood and roots.", "Clear pane."][self as usize]
    }
}
/// Cells per metre: the one resolution knob. Everything sized in cells (movement, brushes, devices,
/// worldgen features, the window, rendering) derives from it, so the world behaves the same in metres.
pub const CPM: usize = 4;
pub const CELL_M: f32 = 1.0 / CPM as f32; // metres per cell
/// Cells per legacy tile (the original 0.25 m cell): constants first tuned in those tiles scale by this.
pub const S: usize = CPM / 4;
pub const SF: f32 = S as f32;

/// Thrumming (knock-signal through rock) physics, derived from the material table.
/// * Stiffness: rock-mass modulus E ≈ 300 × UCS (modulus ratio); liquids use bulk modulus 2.2 GPa;
///   gases 142 kPa (adiabatic air). Wave speed c = √(E/ρ), impedance Z = ρc.
/// * Damping: quality factor Q = 5 + 4 × UCS[MPa] for coherent solids (granite ≈ 85, dirt ≈ 6);
///   loose or powdery ground Q = 4; liquid Q = 20; air Q = 2. Absorption α = π f /(Q c) Np/m at the
///   knock frequency f = 200 Hz, i.e. 8.686 α dB/m.
/// * Each boundary between materials transmits T = 4 Z₁Z₂/(Z₁+Z₂)² of the energy (−10 log₁₀ T dB);
///   rock→air is ≈ −30 dB, so open tunnels reflect the signal away.
pub const THRUM_HZ: f32 = 200.;
pub fn wave(m: Mat, loose: bool) -> (f32, f32) {
    let p = m.p(); let rho = p.density.max(1.);
    let (e, q) = match p.kind {
        Kind::Empty | Kind::Gas => (1.42e5, 2.),
        Kind::Liquid => (2.2e9, 20.),
        Kind::Powder => (300. * p.compressive * 1000., 4.),
        Kind::Solid => (300. * p.compressive * 1000., if loose { 4. } else { 5. + 4. * p.compressive / 1000. }),
    };
    let c = (e / rho).sqrt();
    let db_per_m = 8.686 * std::f32::consts::PI * THRUM_HZ / (q * c);
    (rho * c, db_per_m * CELL_M)
}
pub fn interface_db(z1: f32, z2: f32) -> f32 { if (z1 - z2).abs() < 1e-3 * z1.max(z2) { 0. } else { -10. * (4. * z1 * z2 / ((z1 + z2) * (z1 + z2))).log10() } }
pub const G: f32 = 9.81;
