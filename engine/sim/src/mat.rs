//! Material table: the one place material behaviour is defined. One cell = 1/`CPM` m square (× 1 m deep).
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
    RootTree, TreeRoot, Moss, Mycelium,
    // decorative blocks (furnishing): built with the block brush, structural by their own numbers
    Planks, Polished, Tiles, Carved,
    // transport (transport.rs): track for mine carts, pipe for pressure tubes
    Rail, Tube,
    // production (production.rs): a timber chute items slide down, a shaker screen that sorts by size, a cart's bumper
    Chute, Screen, Bumper,
    // sluicing (production.rs): a riffle bar across a sluice floor
    Riffle,
    // mechanical power (power.rs): shafts, gears, water wheels, belts (one for each way they run), a weight's drum
    Shaft, Gear, Wheel, Belt, BeltL, Drum,
    // coal power (steam.rs): a steam engine (machinery) and the pipe that brings it steam
    Engine, SteamPipe, Boiler,
    // refinement (refine.rs): the works' shells, bellows, and foundry moulds (empty and setting)
    StampMill, Washer, Smelter, Bellows, Mould, MouldHot,
}
pub const NMAT: usize = 57;

/// Everything the simulation knows about a material, in one row. Rules read these fields instead of
/// naming materials, so a new material (or a retuned one) is a single table edit.
pub struct Props {
    pub name: &'static str,
    /// one plain line for the hover tooltip
    pub desc: &'static str,
    pub kind: Kind,
    pub density: f32,      // kg/m³ (also: grains sink through lighter liquids, float on denser ones; thrum impedance)
    pub tensile: f32,      // kPa (bottom-fibre bending capacity)
    pub compressive: f32,  // kPa (also sets the thrum stiffness and damping, see `wave`)
    pub dig: u16,          // ticks per cell with bare claws
    pub tier: u8,          // tool tier needed: 0 claws, 1 Pickaxe, 2 Drill, 3 Bore
    pub blast: u8,         // explosive power*10 needed
    pub emit: [u8; 3],     // light emission
    pub opacity: u8,       // light lost entering cell (0..255)
    pub color: [u8; 3],
    pub crumbles_to: Mat,  // what a failed cell becomes (as loose powder); built beams break into their own loose debris
    pub repose: f32,       // 0 = flows freely, 1 = never slides (angle of repose)
    /// heat it gives off: 0 none, 1 a flame, 2 molten. It burns a mole, feeds the air's fire load, lights
    /// fuses, and sets alight neighbours whose `ignite` it reaches.
    pub heat: u8,
    /// heat needed to set it alight (0 = never burns): 1 any flame, 2 only something molten (coal)
    pub ignite: u8,
    /// what it turns into when it loses its heat (lava → basalt; a flame touching water → steam)
    pub cools_to: Mat,
    /// ticks a short-lived cell lasts (steam, fire); 0 = forever
    pub life: u8,
    /// liquids: how far a cell runs along a surface per tick, in legacy 0.25 m tiles, and on how many
    /// ticks in 10 it moves at all (viscosity: lava is slow)
    pub spread: u8, pub flow_in_10: u8,
    /// loose items landing on it: how much of the impact speed bounces back (0..1), and the sliding
    /// friction coefficient (soft earth grabs, stone and iron let things skid and roll)
    pub bounce: f32, pub friction: f32,
    pub flags: u16,
}
/// Floor a mushroom plot can grow on.
pub const SOIL: u16 = 1;
/// Loose, airy ground the air survey counts as breathing (it lets stale air seep away).
pub const POROUS: u16 = 2;
/// Digging it now and then turns up a grub.
pub const GRUBS: u16 = 4;
/// Placed by moles (beams, brick, doors, glass): breaks into its own debris, never bulks into rubble,
/// and a quake doesn't shake it loose.
pub const BUILT: u16 = 8;
/// Worth a discovery note (and torches) the first time it's seen.
pub const NOTABLE: u16 = 16;
/// Not solid enough to stop a block being placed into it (it's displaced).
pub const OPEN: u16 = 32;
/// Track: a mine cart's wheels run along its top (see transport.rs).
pub const TRACK: u16 = 64;
/// Pressure tube: capsules are blown along it (see tubes.rs).
pub const TUBE: u16 = 128;
/// Chute: loose items lying on its top slide down its slope (see production.rs).
pub const CHUTE: u16 = 256;
/// Screen: a grating; items finer than `production::MESH_M` fall through it, lumps ride over.
pub const SCREEN: u16 = 512;
/// Bumper: a loaded cart that runs into it tips its load out over it.
pub const BUMPER: u16 = 1024;
/// Turning machinery (power.rs): it joins a drive train with the machinery next to it.
pub const MECH: u16 = 2048;
/// Joins a drive train on all four sides (a gear, wheel, belt or drum). A plain shaft only passes turning along its own line.
pub const HUB: u16 = 4096;
/// A belt: what lies on it is carried while its train turns (`Belt` to the right, `BeltL` to the left).
pub const BELT: u16 = 8192;
/// Carries a boiler's steam (a steam pipe, and the steam engine it feeds; steam.rs).
pub const STEAMLINE: u16 = 16384;
const BASE: Props = Props { name: "", desc: "", kind: Kind::Solid, density: 0., tensile: 0., compressive: 0., dig: 0, tier: 0, blast: 0,
    emit: [0, 0, 0], opacity: 0, color: [0, 0, 0], crumbles_to: Mat::Empty, repose: 0., heat: 0, ignite: 0, cools_to: Mat::Empty, life: 0,
    spread: 0, flow_in_10: 10, bounce: 0.35, friction: 0.5, flags: 0 };
use Kind::*;
use Mat as X;
pub static PROPS: [Props; NMAT] = [
    Props { name: "Empty", desc: "Open tunnel.", kind: Empty, density: 1.2, opacity: 4, color: [20, 17, 15], crumbles_to: X::Empty, ..BASE },
    Props { name: "Bedrock", desc: "The Deepholt's bones. Nothing digs it.", kind: Solid, density: 3000., tensile: 1e9, compressive: 1e9, tier: 9, blast: 99, opacity: 255, color: [30, 32, 40], crumbles_to: X::Bedrock, repose: 1., ..BASE },
    Props { name: "Granite", desc: "Hard grey stone. Needs a drill.", kind: Solid, density: 2700., tensile: 800., compressive: 20000., dig: 22, tier: 2, blast: 8, opacity: 120, color: [96, 98, 112], crumbles_to: X::Gravel, flags: NOTABLE, ..BASE },
    Props { name: "Sandstone", desc: "Soft stone. A pickaxe cuts it.", kind: Solid, density: 2300., tensile: 300., compressive: 8000., dig: 12, tier: 1, blast: 5, opacity: 120, color: [170, 132, 92], crumbles_to: X::Sand, ..BASE },
    Props { name: "Limestone", desc: "Pale, even stone. Strong in a roof.", kind: Solid, density: 2500., tensile: 400., compressive: 9000., dig: 14, tier: 1, blast: 5, opacity: 120, color: [178, 170, 150], crumbles_to: X::Gravel, flags: NOTABLE, ..BASE },
    Props { name: "Basalt", desc: "Dark, hard stone from cooled lava. Needs a bore.", kind: Solid, density: 2900., tensile: 700., compressive: 18000., dig: 19, tier: 3, blast: 10, opacity: 120, color: [62, 60, 70], crumbles_to: X::Gravel, flags: NOTABLE, ..BASE },
    Props { name: "Dirt", desc: "Loose earth. Quick to dig, weak under load.", kind: Solid, density: 1600., tensile: 15., compressive: 250., dig: 2, blast: 2, opacity: 110, color: [122, 88, 58], crumbles_to: X::Dirt, repose: 0.15, flags: SOIL | POROUS | GRUBS, bounce: 0.05, friction: 0.9, ..BASE },
    Props { name: "Clay", desc: "Stiff earth. Holds water back.", kind: Solid, density: 1800., tensile: 40., compressive: 400., dig: 4, blast: 3, opacity: 110, color: [168, 104, 78], crumbles_to: X::Clay, repose: 0.45, flags: SOIL | POROUS | GRUBS | NOTABLE, bounce: 0.08, friction: 0.95, ..BASE },
    Props { name: "Sand", desc: "Flows when nothing holds it.", kind: Powder, density: 1600., tensile: 0., compressive: 150., dig: 1, blast: 1, opacity: 110, color: [222, 196, 132], crumbles_to: X::Sand, flags: POROUS, bounce: 0.05, friction: 0.9, ..BASE },
    Props { name: "Gravel", desc: "Rubble that shifts when disturbed.", kind: Solid, density: 1900., tensile: 3., compressive: 300., dig: 2, blast: 2, opacity: 110, color: [140, 134, 124], crumbles_to: X::Gravel, repose: 0.4, flags: SOIL | POROUS | GRUBS, bounce: 0.15, friction: 0.7, ..BASE },
    Props { name: "Coal", desc: "Black seam. Dig it for coal.", kind: Solid, density: 1350., tensile: 60., compressive: 2000., dig: 7, blast: 4, opacity: 120, color: [40, 38, 42], crumbles_to: X::Coal, ignite: 2, flags: NOTABLE, ..BASE },
    Props { name: "Copper ore", desc: "Seam of copper ore.", kind: Solid, density: 3200., tensile: 120., compressive: 4000., dig: 12, tier: 1, blast: 5, opacity: 120, color: [206, 116, 52], crumbles_to: X::CopperOre, flags: NOTABLE, ..BASE },
    Props { name: "Tin ore", desc: "Seam of tin ore.", kind: Solid, density: 3400., tensile: 120., compressive: 4000., dig: 12, tier: 1, blast: 5, opacity: 120, color: [140, 140, 150], crumbles_to: X::TinOre, flags: NOTABLE, ..BASE },
    Props { name: "Iron ore", desc: "Seam of iron ore. Needs a drill.", kind: Solid, density: 3800., tensile: 150., compressive: 5000., dig: 17, tier: 2, blast: 6, opacity: 120, color: [130, 86, 70], crumbles_to: X::IronOre, flags: NOTABLE, ..BASE },
    Props { name: "Quartz", desc: "Clear stone. Light passes through.", kind: Solid, density: 2650., tensile: 250., compressive: 9000., dig: 18, tier: 2, blast: 7, opacity: 50, color: [228, 224, 236], crumbles_to: X::Gravel, flags: NOTABLE, bounce: 0.4, friction: 0.35, ..BASE },
    Props { name: "Crystal", desc: "Glowing stone. Lights the dark.", kind: Solid, density: 2600., tensile: 300., compressive: 20000., dig: 15, tier: 2, blast: 6, emit: [90, 170, 255], opacity: 40, color: [120, 200, 255], crumbles_to: X::Gravel, flags: NOTABLE, bounce: 0.4, friction: 0.35, ..BASE },
    Props { name: "Roots", desc: "Tangle of roots. Gives wood.", kind: Solid, density: 700., tensile: 60., compressive: 300., dig: 2, blast: 2, opacity: 100, color: [118, 84, 50], crumbles_to: X::Dirt, repose: 0.2, ignite: 1, flags: SOIL | POROUS | GRUBS, bounce: 0.15, friction: 0.8, ..BASE },
    Props { name: "Fungus", desc: "Soft glowing growth.", kind: Powder, density: 400., tensile: 4., compressive: 50., dig: 1, blast: 1, emit: [40, 110, 60], opacity: 70, color: [96, 168, 110], crumbles_to: X::Dirt, repose: 1.0, ignite: 1, flags: SOIL | POROUS | NOTABLE, bounce: 0.05, friction: 1.0, ..BASE },
    Props { name: "Grass", desc: "Mossy earth.", kind: Solid, density: 1500., tensile: 20., compressive: 250., dig: 2, blast: 2, opacity: 110, color: [92, 140, 60], crumbles_to: X::Dirt, repose: 0.15, flags: SOIL | GRUBS, bounce: 0.1, friction: 0.9, ..BASE },
    Props { name: "Wood beam", desc: "Wooden prop. Holds up what rests on it.", kind: Solid, density: 600., tensile: 3000., compressive: 8000., dig: 4, blast: 3, opacity: 150, color: [196, 150, 90], crumbles_to: X::WoodBeam, repose: 0.3, ignite: 1, flags: BUILT, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Stone brick", desc: "Laid brick.", kind: Solid, density: 2400., tensile: 600., compressive: 15000., dig: 12, tier: 1, blast: 6, opacity: 160, color: [150, 150, 160], crumbles_to: X::Gravel, flags: BUILT, ..BASE },
    Props { name: "Iron beam", desc: "Iron girder. Spans wide gaps.", kind: Solid, density: 7800., tensile: 40000., compressive: 120000., dig: 20, tier: 2, blast: 11, opacity: 200, color: [170, 176, 190], crumbles_to: X::IronBeam, flags: BUILT, bounce: 0.45, friction: 0.35, ..BASE },
    Props { name: "Door", desc: "Opens and shuts with current.", kind: Solid, density: 700., tensile: 2000., compressive: 5000., dig: 4, blast: 3, opacity: 150, color: [140, 104, 64], crumbles_to: X::WoodBeam, repose: 0.3, ignite: 1, flags: BUILT, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Water", desc: "Fills every hollow it can reach.", kind: Liquid, density: 1000., opacity: 16, color: [52, 120, 200], crumbles_to: X::Water, spread: 6, flow_in_10: 10, flags: NOTABLE | OPEN, ..BASE },
    Props { name: "Lava", desc: "Molten rock. Burns, and lights the dark.", kind: Liquid, density: 2700., emit: [255, 110, 30], opacity: 30, color: [255, 110, 32], crumbles_to: X::Lava, heat: 2, cools_to: X::Basalt, spread: 2, flow_in_10: 3, flags: NOTABLE, ..BASE },
    Props { name: "Steam", desc: "Hot vapour from water meeting lava.", kind: Gas, density: 0.6, opacity: 8, color: [210, 216, 222], crumbles_to: X::Steam, life: 90, flags: OPEN, ..BASE },
    Props { name: "Fire", desc: "Burning. Spreads to wood and roots.", kind: Gas, density: 0.3, emit: [255, 150, 50], opacity: 4, color: [255, 160, 60], crumbles_to: X::Empty, heat: 1, cools_to: X::Steam, life: 70, ..BASE },
    Props { name: "Glass", desc: "Clear pane.", kind: Solid, density: 2500., tensile: 100., compressive: 2000., dig: 3, blast: 2, opacity: 20, color: [190, 220, 230], crumbles_to: X::Sand, flags: BUILT, bounce: 0.4, friction: 0.3, ..BASE },
    // grottos: a root-tree's trunk (the wood source), its roots (it grows back from them), damp moss and glowing
    // mycelium (a skin grown into the stone floor: it bears what the stone under it bears)
    Props { name: "Root-tree", desc: "Cave tree. Cut it for wood; leave its roots and it grows back.", kind: Solid, density: 700., tensile: 2500., compressive: 6000., dig: 4, blast: 3, opacity: 150, color: [104, 78, 52], crumbles_to: X::RootTree, repose: 0.3, ignite: 1, flags: NOTABLE, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Root-tree roots", desc: "A root-tree's roots. While they hold and the seep runs, the tree regrows.", kind: Solid, density: 900., tensile: 80., compressive: 2500., dig: 3, blast: 2, opacity: 110, color: [132, 96, 58], crumbles_to: X::Dirt, repose: 0.2, ignite: 1, flags: SOIL | POROUS | GRUBS, bounce: 0.15, friction: 0.8, ..BASE },
    Props { name: "Moss", desc: "Damp moss on the grotto floor.", kind: Solid, density: 1300., tensile: 40., compressive: 2500., dig: 1, blast: 1, opacity: 110, color: [74, 128, 70], crumbles_to: X::Dirt, repose: 0.15, ignite: 1, flags: SOIL | POROUS, bounce: 0.05, friction: 1.0, ..BASE },
    Props { name: "Mycelium", desc: "Glowing threads through the damp ground.", kind: Solid, density: 1100., tensile: 40., compressive: 2500., dig: 1, blast: 1, emit: [60, 140, 170], opacity: 90, color: [150, 205, 215], crumbles_to: X::Dirt, repose: 0.15, ignite: 1, flags: SOIL | POROUS | NOTABLE, bounce: 0.05, friction: 1.0, ..BASE },
    Props { name: "Planks", desc: "Sawn boards. Warm underfoot; a light, fair prop.", kind: Solid, density: 550., tensile: 2500., compressive: 6000., dig: 4, blast: 3, opacity: 150, color: [182, 134, 84], crumbles_to: X::Planks, repose: 0.3, ignite: 1, flags: BUILT, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Polished stone", desc: "Dressed and rubbed smooth. As strong as brick.", kind: Solid, density: 2500., tensile: 600., compressive: 15000., dig: 12, tier: 1, blast: 6, opacity: 160, color: [176, 170, 160], crumbles_to: X::Gravel, flags: BUILT, ..BASE },
    Props { name: "Tiles", desc: "Fired clay tiles. Bright, but brittle under a roof.", kind: Solid, density: 2000., tensile: 300., compressive: 9000., dig: 5, blast: 4, opacity: 160, color: [190, 108, 74], crumbles_to: X::Gravel, flags: BUILT, ..BASE },
    Props { name: "Carved pillar", desc: "Fluted stone column. Very strong in compression.", kind: Solid, density: 2600., tensile: 800., compressive: 22000., dig: 14, tier: 1, blast: 7, opacity: 170, color: [164, 158, 148], crumbles_to: X::Gravel, flags: BUILT, ..BASE },
    Props { name: "Rail", desc: "Copper straps on wooden sleepers. Mine carts run along it.", kind: Solid, density: 1800., tensile: 3000., compressive: 9000., dig: 4, blast: 4, opacity: 150, color: [104, 84, 78], crumbles_to: X::Rail, repose: 0.3, flags: BUILT | TRACK, bounce: 0.3, friction: 0.35, ..BASE },
    Props { name: "Pressure tube", desc: "Iron-banded glass pipe. Capsules blow along it while it holds pressure.", kind: Solid, density: 2600., tensile: 1500., compressive: 6000., dig: 5, blast: 4, opacity: 60, color: [150, 190, 186], crumbles_to: X::Tube, repose: 0.3, flags: BUILT | TUBE, bounce: 0.4, friction: 0.3, ..BASE },
    Props { name: "Chute", desc: "Smooth timber trough. Loose things slide down it if it slopes.", kind: Solid, density: 600., tensile: 2500., compressive: 6000., dig: 4, blast: 3, opacity: 150, color: [168, 128, 82], crumbles_to: X::Chute, repose: 0.3, ignite: 1, flags: BUILT | CHUTE, bounce: 0.1, friction: 0.12, ..BASE },
    Props { name: "Shaker screen", desc: "Iron grating on a timber frame. Fines drop through, lumps ride over; current shakes it.", kind: Solid, density: 1500., tensile: 4000., compressive: 8000., dig: 5, blast: 4, opacity: 90, color: [120, 116, 112], crumbles_to: X::Screen, repose: 0.3, flags: BUILT | CHUTE | SCREEN, bounce: 0.15, friction: 0.45, ..BASE },
    Props { name: "Bumper", desc: "Timber buffer at the end of the track. A loaded cart that hits it tips out.", kind: Solid, density: 700., tensile: 3000., compressive: 8000., dig: 4, blast: 3, opacity: 150, color: [150, 70, 48], crumbles_to: X::Bumper, repose: 0.3, ignite: 1, flags: BUILT | BUMPER, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Riffle bar", desc: "A low timber bar across a sluice floor. Heavy lumps settle behind it.", kind: Solid, density: 700., tensile: 3000., compressive: 8000., dig: 4, blast: 3, opacity: 150, color: [122, 88, 54], crumbles_to: X::Riffle, repose: 0.3, ignite: 1, flags: BUILT, bounce: 0.2, friction: 0.6, ..BASE },
    Props { name: "Iron shaft", desc: "An iron rod on bearings. Carries turning along its line; a gear takes it round a corner.", kind: Solid, density: 7800., tensile: 30000., compressive: 90000., dig: 18, tier: 2, blast: 10, opacity: 200, color: [140, 144, 156], crumbles_to: X::Shaft, flags: BUILT | MECH, bounce: 0.45, friction: 0.35, ..BASE },
    Props { name: "Gear", desc: "A toothed wooden wheel. Joins shafts, belts and wheels on all sides.", kind: Solid, density: 700., tensile: 3000., compressive: 8000., dig: 4, blast: 3, opacity: 170, color: [160, 120, 72], crumbles_to: X::Gear, repose: 0.3, ignite: 1, flags: BUILT | MECH | HUB, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Water wheel", desc: "Timber paddles on an axle. A stream running under it turns what it's geared to.", kind: Solid, density: 650., tensile: 2500., compressive: 6000., dig: 4, blast: 3, opacity: 140, color: [128, 96, 60], crumbles_to: X::Wheel, repose: 0.3, ignite: 1, flags: BUILT | MECH | HUB, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Belt", desc: "Woven fibre belt over wooden rollers. While it turns, what lies on it is carried right.", kind: Solid, density: 900., tensile: 2000., compressive: 5000., dig: 3, blast: 3, opacity: 160, color: [92, 64, 44], crumbles_to: X::Belt, repose: 0.3, ignite: 1, flags: BUILT | MECH | HUB | BELT, bounce: 0.1, friction: 0.9, ..BASE },
    Props { name: "Belt (left)", desc: "Woven fibre belt over wooden rollers. While it turns, what lies on it is carried left.", kind: Solid, density: 900., tensile: 2000., compressive: 5000., dig: 3, blast: 3, opacity: 160, color: [92, 64, 44], crumbles_to: X::BeltL, repose: 0.3, ignite: 1, flags: BUILT | MECH | HUB | BELT, bounce: 0.1, friction: 0.9, ..BASE },
    Props { name: "Weight drum", desc: "A rope drum with a hanging weight. The weight falls to drive the line; spare power winds it up.", kind: Solid, density: 1500., tensile: 4000., compressive: 9000., dig: 6, blast: 4, opacity: 190, color: [118, 112, 108], crumbles_to: X::Drum, flags: BUILT | MECH | HUB, bounce: 0.3, friction: 0.5, ..BASE },
    Props { name: "Steam engine", desc: "Iron cylinder and crank. A boiler's steam drives it, and it turns the drive train it's part of.", kind: Solid, density: 7000., tensile: 20000., compressive: 60000., dig: 16, tier: 2, blast: 9, opacity: 200, color: [96, 92, 98], crumbles_to: X::Engine, flags: BUILT | MECH | HUB | STEAMLINE, bounce: 0.4, friction: 0.4, ..BASE },
    Props { name: "Steam pipe", desc: "Copper pipe that carries a boiler's steam to an engine.", kind: Solid, density: 4000., tensile: 6000., compressive: 12000., dig: 6, tier: 1, blast: 5, opacity: 180, color: [176, 108, 66], crumbles_to: X::SteamPipe, flags: BUILT | STEAMLINE, bounce: 0.4, friction: 0.35, ..BASE },
    Props { name: "Boiler", desc: "Brick firebox under an iron drum. Feed coal in at the top; it boils water from beside it.", kind: Solid, density: 3000., tensile: 8000., compressive: 30000., dig: 14, tier: 1, blast: 9, opacity: 220, color: [120, 84, 70], crumbles_to: X::Gravel, flags: BUILT, bounce: 0.3, friction: 0.6, ..BASE },
    Props { name: "Stamp mill", desc: "Iron stamps a cam lifts and drops on what lies in its mortar. Crushes ore.", kind: Solid, density: 4000., tensile: 9000., compressive: 40000., dig: 14, tier: 1, blast: 9, opacity: 220, color: [112, 100, 92], crumbles_to: X::Gravel, flags: BUILT | MECH | HUB, bounce: 0.3, friction: 0.6, ..BASE },
    Props { name: "Washer", desc: "A rocking tub of water. Sorts what's dropped in by weight: heavy left, light right.", kind: Solid, density: 1200., tensile: 3000., compressive: 8000., dig: 5, blast: 4, opacity: 200, color: [104, 112, 120], crumbles_to: X::Gravel, ignite: 1, flags: BUILT | MECH | HUB, bounce: 0.2, friction: 0.6, ..BASE },
    Props { name: "Smelter", desc: "A clay-lined brick stack. Fuel and ore go in at the top; bellows make it hot enough.", kind: Solid, density: 2400., tensile: 6000., compressive: 25000., dig: 12, tier: 1, blast: 8, opacity: 230, color: [136, 70, 54], crumbles_to: X::Gravel, flags: BUILT, bounce: 0.3, friction: 0.6, ..BASE },
    Props { name: "Bellows", desc: "Leaf-and-board bellows on a crank. Turned by a drive train, they blow a smelter's fire.", kind: Solid, density: 700., tensile: 2000., compressive: 5000., dig: 4, blast: 3, opacity: 170, color: [140, 108, 72], crumbles_to: X::Bellows, repose: 0.3, ignite: 1, flags: BUILT | MECH | HUB, bounce: 0.3, friction: 0.55, ..BASE },
    Props { name: "Ingot mould", desc: "A sand-and-clay mould. Built against a smelter, it's filled when an ingot's worth is molten.", kind: Solid, density: 1800., tensile: 300., compressive: 4000., dig: 3, blast: 3, opacity: 160, color: [150, 128, 96], crumbles_to: X::Sand, flags: BUILT, bounce: 0.1, friction: 0.8, ..BASE },
    Props { name: "Setting mould", desc: "A mould full of molten metal, glowing as it sets. An ingot pops out when it's cool.", kind: Solid, density: 4000., tensile: 300., compressive: 4000., dig: 3, blast: 3, emit: [200, 90, 30], opacity: 160, color: [220, 110, 50], crumbles_to: X::Sand, heat: 1, flags: BUILT, bounce: 0.1, friction: 0.8, ..BASE },
];
impl Mat {
    #[inline] pub fn from_u8(v: u8) -> Mat { if (v as usize) < NMAT { unsafe { std::mem::transmute(v) } } else { Mat::Empty } }
    #[inline] pub fn p(self) -> &'static Props { &PROPS[self as usize] }
    #[inline] pub fn kind(self) -> Kind { PROPS[self as usize].kind }
    pub fn is_structural(self) -> bool { self.kind() == Kind::Solid }
    /// One plain line for the hover tooltip.
    pub fn desc(self) -> &'static str { self.p().desc }
    #[inline] pub fn has(self, flag: u16) -> bool { self.p().flags & flag != 0 }
    /// Something a mole's body passes through (open, liquid or gas).
    #[inline] pub fn passable(self) -> bool { matches!(self.kind(), Kind::Empty | Kind::Liquid | Kind::Gas) }
    /// Would heat `h` set this alight?
    #[inline] pub fn lit_by(self, h: u8) -> bool { let n = self.p().ignite; n > 0 && h >= n }
}
/// Cells per metre: the one resolution knob. Everything sized in cells (movement, brushes, devices,
/// worldgen features, the window, rendering) derives from it, so the world behaves the same in metres.
pub const CPM: usize = 8;
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
