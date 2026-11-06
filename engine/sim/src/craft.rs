//! Items, stations, recipes, tool tiers, quests.
use crate::mat::Mat;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Item {
    Dirt, Clay, Sand, Gravel, Stone, Coal, CopperOre, TinOre, IronOre, Quartz, Crystal, Wood, Fungus,
    Brick, Glass, Charcoal, CopperIngot, TinIngot, BronzeIngot, IronIngot,
    Torch, WoodPillar, IronBeam, CopperWire, Thrummer, Gate, Tnt, Lamp,
    Pickaxe, Drill, Bore, Grub, Spore, Mushroom, Vent, Glowmite,
    FungusOil, Root,
    // furnishing: decorative blocks (brush-built, structural), pigments (paint), furniture (placed props)
    Planks, PolishedStone, Tiles, CarvedPillar,
    Ochre, Rust, Chalk, Verdigris, Woad, Soot,
    Table, Stool, Nest, Shelf, Rug, Banner, Planter,
    // relics of the Ancients: found in ruins, never made (see ruins.rs)
    AncientBit, GroundLens,
    // transport (transport.rs)
    Rail, Cart, RopeLadder, Lift, Tube, TubeHopper,
    // decorative background walls (see walls.rs): they hold no roof
    RockWall, TimberWall, MasonryWall, BeamWall, PipeWall, CarvedWall,
}
pub const NITEM: usize = 69;
/// What an item can do in the world (see `Item::uses`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Uses { pub place: bool, pub throw: bool, pub pick_up: bool }
pub const ITEMS: [Item; NITEM] = [
    Item::Dirt, Item::Clay, Item::Sand, Item::Gravel, Item::Stone, Item::Coal, Item::CopperOre, Item::TinOre, Item::IronOre, Item::Quartz, Item::Crystal, Item::Wood, Item::Fungus,
    Item::Brick, Item::Glass, Item::Charcoal, Item::CopperIngot, Item::TinIngot, Item::BronzeIngot, Item::IronIngot,
    Item::Torch, Item::WoodPillar, Item::IronBeam, Item::CopperWire, Item::Thrummer, Item::Gate, Item::Tnt, Item::Lamp,
    Item::Pickaxe, Item::Drill, Item::Bore, Item::Grub, Item::Spore, Item::Mushroom, Item::Vent, Item::Glowmite,
    Item::FungusOil, Item::Root,
    Item::Planks, Item::PolishedStone, Item::Tiles, Item::CarvedPillar,
    Item::Ochre, Item::Rust, Item::Chalk, Item::Verdigris, Item::Woad, Item::Soot,
    Item::Table, Item::Stool, Item::Nest, Item::Shelf, Item::Rug, Item::Banner, Item::Planter,
    Item::AncientBit, Item::GroundLens,
    Item::Rail, Item::Cart, Item::RopeLadder, Item::Lift, Item::Tube, Item::TubeHopper,
    Item::RockWall, Item::TimberWall, Item::MasonryWall, Item::BeamWall, Item::PipeWall, Item::CarvedWall,
];
/// Paint colours by pigment index (0 = unpainted): ochre, rust, chalk, verdigris, woad, soot.
pub const PIGMENTS: [[u8; 3]; 7] = [[0, 0, 0], [206, 160, 70], [164, 70, 46], [232, 226, 210], [84, 162, 132], [74, 112, 190], [50, 46, 52]];
/// A piece of furniture's footprint in 0.25 m tiles, and how it's held up: standing on the floor, or
/// hanging from the ceiling. `cloth` = it has a dyeable part (a pigment colours it).
#[derive(Clone, Copy, Debug)] pub struct Furn { pub w: usize, pub h: usize, pub hang: bool, pub cloth: bool }
impl Item {
    /// The material a loose item is a lump of, if it is one: its density and burning come from the table.
    pub fn mat(self) -> Option<Mat> {
        use Item::*;
        Some(match self {
            Dirt => Mat::Dirt, Clay => Mat::Clay, Sand => Mat::Sand, Gravel => Mat::Gravel, Stone => Mat::Rock, Coal => Mat::Coal,
            CopperOre => Mat::CopperOre, TinOre => Mat::TinOre, IronOre => Mat::IronOre, Quartz => Mat::Quartz, Crystal => Mat::Crystal,
            Wood | WoodPillar | Torch | Table | Stool | Shelf | Planter => Mat::WoodBeam, Planks => Mat::Planks, PolishedStone => Mat::Polished, Tiles => Mat::Tiles, CarvedPillar => Mat::Carved, Fungus => Mat::Fungus, Brick => Mat::StoneBrick, Glass => Mat::Glass,
            IronBeam | IronIngot => Mat::IronBeam, Rail => Mat::Rail, Tube => Mat::Tube,
            _ => return None,
        })
    }
    /// kg/m³ of the item lying loose (decides whether it floats or sinks): its material's, else its own.
    pub fn density(self) -> f32 {
        use Item::*;
        if let Some(m) = self.mat() { return m.p().density; }
        match self {
            Charcoal => 400., CopperIngot => 8900., TinIngot => 7300., BronzeIngot => 8800., CopperWire => 3000., Thrummer | Gate | Vent => 2500.,
            Tnt => 1500., Lamp => 2000., Pickaxe => 2500., Drill => 5000., Bore => 5500., Grub => 1050., Spore => 300., Mushroom => 600., FungusOil => 900., Root => 1100., AncientBit => 7000., GroundLens => 2600., Cart => 1200., RopeLadder => 500., Lift => 3000., TubeHopper => 3000., Rug | Banner | Nest => 300., Ochre | Rust | Chalk | Verdigris | Woad | Soot => 1800.,
            _ => 2000.,
        }
    }
    /// Does heat `h` burn it up (a loose item in lava or fire)? TNT doesn't burn: it lights.
    pub fn burns(self, h: u8) -> bool {
        use Item::*;
        match self { Tnt => false, Charcoal | Grub | Spore | Mushroom | FungusOil | Root => h >= 1, _ => self.mat().map_or(false, |m| m.lit_by(h)) }
    }
    /// What the item in hand can do, from one table: set it down in the world, throw it, or neither
    /// (a crafting material). Blocks, copperline, devices, torches and tools are never thrown.
    pub fn uses(self) -> Uses {
        use Item::*;
        let place = matches!(self, WoodPillar | Brick | IronBeam | CopperWire | Torch | Tnt | Lamp | Vent | Spore) || self.block().is_some() || self.pigment().is_some() || self.furn().is_some() || self.is_wall() || matches!(self, Cart | RopeLadder | Lift | TubeHopper);
        let throw = matches!(self, Dirt | Clay | Sand | Gravel | Stone | Coal | CopperOre | TinOre | IronOre | Quartz | Crystal | Wood | Fungus
            | Charcoal | CopperIngot | TinIngot | BronzeIngot | IronIngot | Grub | Spore | Mushroom | Tnt | FungusOil | Root);
        Uses { place, throw, pick_up: matches!(self, CopperWire | Torch | Tnt | Lamp | Vent | Cart | RopeLadder | Lift | TubeHopper) || self.furn().is_some() }
    }
    /// One line naming the inputs for the item in hand (hover tooltip and the hint over the hotbar).
    pub fn controls(self) -> &'static str {
        use Item::*;
        match self {
            Tnt => "Left/right mouse: set down unlit · right mouse on it: light · Q: throw lit · E: pick up",
            WoodPillar | Brick | IronBeam | Planks | PolishedStone | Tiles | CarvedPillar => "Left mouse: build (hold to keep building)",
            RockWall | TimberWall | MasonryWall | BeamWall | PipeWall | CarvedWall => "Left mouse: panel the back (hold) · right mouse: scrape lining · never edits the rock in front",
            Rail => "Left mouse: lay track (X: snap to a straight line) · carts run along its top",
            Cart => "Left mouse: set on the track · right mouse: get in · walk into it: push · E: unload, then pick up",
            RopeLadder => "Left mouse in a drop: hang from the rock above or a ledge's lip · W/S: climb · E: take down",
            Tube => "Left mouse: lay pipe (X: snap to a straight line) · hoppers on a line blow capsules along it",
            TubeHopper => "Left mouse: set on a tube · drop items in to send them · right mouse: ride (empty paw) or pick where it sends · E: pick up",
            Lift => "Left mouse in a shaft: the head hangs under its ceiling · right mouse on head or cage: stone on (stone in paw) or off · E: take down",
            Ochre | Rust | Chalk | Verdigris | Woad | Soot => "Left mouse: paint blocks, or dye a rug, banner or nest (hold to keep painting)",
            Table | Stool | Nest | Shelf | Rug | Planter => "Left mouse: set down on the floor · E: pick up",
            Banner => "Left mouse: hang from the ceiling · E: pick up",
            CopperWire => "Left mouse: lay (drag) · E: take up (drag)",
            Torch => "Left mouse: set down · E: pick up",
            Lamp | Vent => "Left/right mouse: set down · E: pick up",
            Spore => "Left mouse: plant on soil · Q: throw",
            Thrummer | Gate => "Set down from its device slots · E: pick up",
            Glowmite => "Held, it lights the way · right mouse on a glowmite in a grotto: catch one",
            _ if self.uses().throw => "Q: throw (hold for further)",
            _ => "",
        }
    }
    /// A decorative block's material (built with the block brush).
    pub fn block(self) -> Option<Mat> { use Item::*; match self { Planks => Some(Mat::Planks), PolishedStone => Some(Mat::Polished), Tiles => Some(Mat::Tiles), CarvedPillar => Some(Mat::Carved), Rail => Some(Mat::Rail), Tube => Some(Mat::Tube), _ => None } }
    /// A pigment's paint index (1..6), see `PIGMENTS`.
    pub fn pigment(self) -> Option<u8> { use Item::*; match self { Ochre => Some(1), Rust => Some(2), Chalk => Some(3), Verdigris => Some(4), Woad => Some(5), Soot => Some(6), _ => None } }
    /// Furniture: footprint (tiles) and how it's held up.
    pub fn furn(self) -> Option<Furn> {
        use Item::*;
        let f = |w, h, hang, cloth| Some(Furn { w, h, hang, cloth });
        match self { Table => f(8, 4, false, false), Stool => f(3, 3, false, false), Nest => f(9, 3, false, true), Shelf => f(6, 8, false, false),
            Rug => f(10, 1, false, true), Banner => f(3, 6, true, true), Planter => f(5, 3, false, false), _ => None }
    }
    pub fn name(self) -> &'static str {
        ["Dirt", "Clay", "Sand", "Gravel", "Stone", "Coal", "Copper ore", "Tin ore", "Iron ore", "Quartz", "Crystal", "Wood", "Fungus",
         "Brick", "Glass", "Charcoal", "Copper ingot", "Tin ingot", "Bronze ingot", "Iron ingot",
         "Torch", "Wood pillar", "Iron beam", "Copperline", "Thrummer", "Knot", "TNT", "Lamp",
         "Pickaxe", "Drill", "Bore", "Grub", "Spore", "Mushroom", "Vent", "Glowmite", "Fungus oil", "Root",
         "Planks", "Polished stone", "Tiles", "Carved pillar",
         "Ochre", "Rust", "Chalk", "Verdigris", "Woad", "Soot",
         "Table", "Stool", "Nest", "Shelf", "Rug", "Banner", "Fungus planter", "Ancient bit", "Ground lens", "Rail", "Mine cart", "Rope ladder", "Lift", "Pressure tube", "Tube hopper",
         "Rock wall", "Timber wall", "Masonry wall", "Beam wall", "Pipe wall", "Carved wall"][self as usize]
    }
    /// One plain line for the hover tooltip: what it is and what it's for.
    pub fn desc(self) -> &'static str {
        ["Loose earth. Packs into burrow walls; weak under load.", "Stiff, wet earth. Holds a little better than dirt; fired into brick.",
         "Flows like water when nothing holds it. Melts into glass.", "Rubble. Shifts when disturbed.", "Broken rock. Builds kilns; cut into brick.",
         "Burns hot. Lights torches and feeds the furnace.", "Green-flecked rock. Smelted into copper for copperline.", "Pale ore. Alloyed with copper into bronze.",
         "Heavy red ore. Smelted into iron for beams and heatstones.", "Clear stone that lets light through.", "Glowing stone from deep down. Lights the dark.",
         "Roots and timber. Pillars, torches and workbenches.", "Soft glowing growth. Edible in a pinch.",
         "Fired stone block. Strong in a wall, heavy on a roof.", "Clear pane. Lets light through a wall.", "Wood baked black. Burns cleaner than wood.",
         "Drawn into copperline.", "Alloyed with copper into bronze.", "Tough metal: the drill's teeth are bronze.", "Strongest metal. Beams and heatstones.",
         "Lights a few metres around it. Breathes the air stale over time.", "Wooden prop. Holds up a roof or spans a short gap.",
         "Iron girder. Spans wide gaps and carries heavy roofs.", "Copper wire. Carries current between levers, knots and lamps.",
         "Knocks a pattern into the rock for a listener to hear. Stone carries it; dirt and water muffle it.",
         "Mechanical knot: joins two lines into one answer (both, either, turn, odd, catch).", "Blasting charge. Shatters rock in a wide round.",
         "Lights when current flows through it.", "Stone pick: breaks soil and soft stone (limestone, sandstone, coal, copper and tin ore).", "Bronze-toothed auger driven by copperline: cuts granite and iron ore.", "Heavy heat-bore: grinds through basalt.",
         "Fat burrowing larva. A good meal.", "Mushroom spore. Plant it on soil to grow food.", "A ripe mushroom. Fills the belly.",
         "Draws stale air out of the tunnels around it.", "A glowmite caught in a jar. Glows softly, needs no fuel.",
         "Pressed from fungus. Fuels the headlamp and soaks a torch so it burns.", "A fat, starchy root from a root tangle. A fair meal.",
         "Sawn boards for floors and panelling. A light prop.", "Stone dressed smooth. Fine walls and floors, as strong as brick.",
         "Fired clay tiles in a neat grid. Bright, but brittle under a roof.", "Fluted stone column. Holds a roof better than brick.",
         "Yellow earth pigment, ground from iron ore and clay.", "Red-brown pigment, ground from iron ore.", "Pale pigment, ground from quartz.",
         "Green pigment, scraped from weathered copper ore.", "Blue pigment, pressed from fungus.", "Black pigment, from charcoal.",
         "A sturdy board on legs. Somewhere to eat.", "A low seat.", "A soft bed of moss and cloth. Dye it.", "A tall rack of shelves.",
         "A woven floor rug. Dye it.", "A cloth banner hung from the ceiling. Dye it.", "A box of glowing fungus. A soft green light.",
         "A relic of the Ancients: a bit of a metal no forge makes now. Carried, every bite digs a quarter faster.",
         "A relic of the Ancients: a lens ground true. Carried, the headlamp burns half the oil.",
         "Track: copper straps on wooden sleepers. Lay it on a floor; mine carts run along it, downhill on their own.",
         "A copper-bound cart on four wheels. Push it, ride it, throw cargo in. Rolls downhill on its own.",
         "Fungus-fibre rope and wooden rungs. Hang it from a ledge: it unrolls up to 8 m. Slow, but needs no power.",
         "A cage on a rope with a counterweight. Load stone on to lift the cage, or wind it with current.",
         "Iron-banded glass pipe. Lay a line of it between tube hoppers to blow capsules along.",
         "A mouth and a blower on a pressure tube. Items dropped in are blown along the line to another hopper.",
         "Carved rock lining for the back of a room. Walk in front of it; it holds no roof.",
         "Timber panelling for the back of a holt. Walk in front of it; it holds no roof.",
         "Brick lining for the back of a room. Walk in front of it; it holds no roof.",
         "Decorative beams on the back. They hold no weight.",
         "Old copper pipes on the back. They carry nothing.",
         "Carved marks on the back. They hold no roof."][self as usize]
    }
    pub fn color(self) -> [u8; 3] {
        use Item::*;
        match self {
            Dirt => [122, 88, 58], Clay => [168, 104, 78], Sand => [222, 196, 132], Gravel => [140, 134, 124], Stone => [160, 156, 150],
            Coal => [50, 48, 54], CopperOre => [200, 120, 70], TinOre => [180, 190, 200], IronOre => [150, 90, 80], Quartz => [235, 232, 240],
            Crystal => [120, 200, 255], Wood => [150, 105, 60], Fungus => [96, 200, 120], Brick => [170, 80, 60], Glass => [190, 230, 240],
            Charcoal => [70, 60, 60], CopperIngot => [230, 140, 80], TinIngot => [210, 215, 225], BronzeIngot => [205, 150, 70], IronIngot => [190, 190, 200],
            Torch => [255, 190, 90], WoodPillar => [180, 130, 80], IronBeam => [120, 130, 150], CopperWire => [240, 150, 90], Thrummer => [150, 120, 255],
            Gate => [120, 220, 160], Tnt => [220, 60, 50], Lamp => [255, 240, 170], Pickaxe => [160, 156, 150], Drill => [205, 150, 70], Bore => [200, 205, 220], Grub => [235, 215, 170], Spore => [200, 170, 230], Mushroom => [214, 120, 90], Vent => [150, 200, 210], Glowmite => [190, 255, 160], FungusOil => [200, 220, 120], Root => [190, 140, 90],
            Planks => [182, 134, 84], PolishedStone => [176, 170, 160], Tiles => [190, 108, 74], CarvedPillar => [164, 158, 148],
            Ochre | Rust | Chalk | Verdigris | Woad | Soot => PIGMENTS[self.pigment().unwrap_or(0) as usize],
            Table | Stool | Shelf => [160, 112, 66], Nest => [150, 170, 110], Rug => [176, 92, 70], Banner => [200, 70, 60], Planter => [110, 200, 130], AncientBit => [150, 200, 210], GroundLens => [210, 235, 250], Rail => [150, 120, 84], Cart => [130, 96, 60], RopeLadder => [170, 140, 90], Lift => [150, 150, 160], Tube => [150, 190, 186], TubeHopper => [190, 150, 80],
            RockWall => [118, 108, 96], TimberWall => [140, 102, 66], MasonryWall => [132, 78, 62], BeamWall => [118, 84, 54], PipeWall => [148, 96, 64], CarvedWall => [124, 116, 106],
        }
    }
    /// A decorative background wall: it panels the back, never the rock in front, and holds no roof.
    pub fn is_wall(self) -> bool {
        matches!(self, Item::RockWall | Item::TimberWall | Item::MasonryWall | Item::BeamWall | Item::PipeWall | Item::CarvedWall)
    }
}
/// What digging a cell yields (one item per `CELLS_PER_ITEM` cells of that material).
pub const CELLS_PER_ITEM: u16 = 6 * (crate::mat::S * crate::mat::S) as u16; // 6 legacy tiles
pub fn drop_of(m: Mat) -> Option<Item> {
    use Mat::*;
    Some(match m {
        Dirt | Grass => Item::Dirt, Clay => Item::Clay, Sand => Item::Sand, Gravel => Item::Gravel,
        Granite | Rock | Limestone | Basalt | StoneBrick => Item::Stone, Coal => Item::Coal, CopperOre => Item::CopperOre, TinOre => Item::TinOre,
        IronOre => Item::IronOre, Quartz => Item::Quartz, Crystal => Item::Crystal, Roots | WoodBeam | Door | RootTree | TreeRoot => Item::Wood, Fungus | Mycelium => Item::Fungus, Moss => Item::Dirt,
        IronBeam => Item::IronIngot, Glass => Item::Glass, Planks => Item::Wood, Polished | Carved => Item::Stone, Tiles => Item::Clay,
        _ => return None,
    })
}

/// Stations, not menus: you craft standing at one you have built (see PROGRESSION.md).
/// * the **workbench** for wood, stone, simple devices and furniture;
/// * the **forge**, which needs heat (lava or fire within 3 m, a heatstone fed by lava, or else it burns a coal or
///   charcoal from the pack per job), for smelting and metalwork;
/// * the **workshop**, which needs a live copperline touching it, for machines, thrummers and fine parts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Station { Workbench, Forge, Workshop }
pub const STATIONS: [Station; 3] = [Station::Workbench, Station::Forge, Station::Workshop];
impl Station {
    pub fn name(self) -> &'static str { ["Workbench", "Forge", "Workshop"][self as usize] }
    pub fn desc(self) -> &'static str { ["Saw and bench: wood, stone, simple devices and furniture.", "Smelts ore, fires brick and glass, works metal. Needs heat: lava or fire near, a heatstone, or coal.", "Fine work: knots, thrummers and the drill. Needs a live copperline run to it."][self as usize] }
    pub fn cost(self) -> &'static [(Item, u32)] {
        match self { Station::Workbench => &[(Item::Wood, 4), (Item::Stone, 4)], Station::Forge => &[(Item::Stone, 10), (Item::Clay, 4)], Station::Workshop => &[(Item::Wood, 2), (Item::Stone, 6), (Item::CopperWire, 6)] }
    }
    /// Which station art to draw (the workbench, the furnace for the forge, the anvil bench for the workshop).
    pub fn art(self) -> u8 { [0, 2, 3][self as usize] }
    /// footprint in cells
    pub const W: usize = 10 * crate::mat::S; pub const H: usize = 7 * crate::mat::S;
}

/// A recipe: at most two steps from the ground to the thing (ore → ingot → tool), no parts lists.
/// It appears once you have picked up any of its inputs (`World::recipe_known`).
pub struct Recipe { pub name: &'static str, pub station: Station, pub inputs: &'static [(Item, u32)], pub out: (Item, u32), pub ticks: u32 }
pub static RECIPES: &[Recipe] = &[
    // workbench: tier 1, wood and stone
    Recipe { name: "Pickaxe", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Stone, 4)], out: (Item::Pickaxe, 1), ticks: 120 },
    Recipe { name: "Wood pillar x4", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::WoodPillar, 4), ticks: 60 },
    Recipe { name: "Torch x4", station: Station::Workbench, inputs: &[(Item::Wood, 1), (Item::Coal, 1)], out: (Item::Torch, 4), ticks: 60 },
    Recipe { name: "Torch x4 (charcoal)", station: Station::Workbench, inputs: &[(Item::Wood, 1), (Item::Charcoal, 1)], out: (Item::Torch, 4), ticks: 60 },
    Recipe { name: "Vent", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Stone, 2)], out: (Item::Vent, 1), ticks: 90 },
    // furnishing: decorative blocks
    Recipe { name: "Planks x8", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::Planks, 8), ticks: 60 },
    Recipe { name: "Polished stone x4", station: Station::Workbench, inputs: &[(Item::Stone, 4), (Item::Sand, 1)], out: (Item::PolishedStone, 4), ticks: 120 },
    Recipe { name: "Tiles x6", station: Station::Forge, inputs: &[(Item::Clay, 2), (Item::Coal, 1)], out: (Item::Tiles, 6), ticks: 120 },
    Recipe { name: "Carved pillar x2", station: Station::Workbench, inputs: &[(Item::Stone, 4)], out: (Item::CarvedPillar, 2), ticks: 150 },
    // pigments: ground at the workbench from what the rock gives
    Recipe { name: "Ochre x4", station: Station::Workbench, inputs: &[(Item::IronOre, 1), (Item::Clay, 1)], out: (Item::Ochre, 4), ticks: 60 },
    Recipe { name: "Rust x4", station: Station::Workbench, inputs: &[(Item::IronOre, 1)], out: (Item::Rust, 4), ticks: 60 },
    Recipe { name: "Chalk x4", station: Station::Workbench, inputs: &[(Item::Quartz, 1)], out: (Item::Chalk, 4), ticks: 60 },
    Recipe { name: "Verdigris x4", station: Station::Workbench, inputs: &[(Item::CopperOre, 1)], out: (Item::Verdigris, 4), ticks: 60 },
    Recipe { name: "Woad x4", station: Station::Workbench, inputs: &[(Item::Fungus, 2)], out: (Item::Woad, 4), ticks: 60 },
    Recipe { name: "Soot x4", station: Station::Workbench, inputs: &[(Item::Charcoal, 1)], out: (Item::Soot, 4), ticks: 60 },
    // furniture
    Recipe { name: "Table", station: Station::Workbench, inputs: &[(Item::Wood, 4)], out: (Item::Table, 1), ticks: 120 },
    Recipe { name: "Stool", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::Stool, 1), ticks: 60 },
    Recipe { name: "Nest", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Fungus, 3)], out: (Item::Nest, 1), ticks: 120 },
    Recipe { name: "Shelf", station: Station::Workbench, inputs: &[(Item::Wood, 4)], out: (Item::Shelf, 1), ticks: 120 },
    Recipe { name: "Rug", station: Station::Workbench, inputs: &[(Item::Fungus, 4)], out: (Item::Rug, 1), ticks: 90 },
    Recipe { name: "Banner", station: Station::Workbench, inputs: &[(Item::Fungus, 2), (Item::Wood, 1)], out: (Item::Banner, 1), ticks: 90 },
    Recipe { name: "Fungus planter", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Fungus, 2), (Item::Dirt, 2)], out: (Item::Planter, 1), ticks: 90 },
    Recipe { name: "Rock wall x8", station: Station::Workbench, inputs: &[(Item::Stone, 2)], out: (Item::RockWall, 8), ticks: 60 },
    Recipe { name: "Timber wall x8", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::TimberWall, 8), ticks: 60 },
    Recipe { name: "Carved wall x4", station: Station::Workbench, inputs: &[(Item::Stone, 2)], out: (Item::CarvedWall, 4), ticks: 90 },
    Recipe { name: "Beam wall x4", station: Station::Workbench, inputs: &[(Item::WoodPillar, 1)], out: (Item::BeamWall, 4), ticks: 60 },
    Recipe { name: "Masonry wall x8", station: Station::Forge, inputs: &[(Item::Brick, 2)], out: (Item::MasonryWall, 8), ticks: 90 },
    Recipe { name: "Pipe wall x4", station: Station::Forge, inputs: &[(Item::CopperIngot, 1)], out: (Item::PipeWall, 4), ticks: 90 },
    Recipe { name: "Rope ladder", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Fungus, 2)], out: (Item::RopeLadder, 1), ticks: 90 },
    Recipe { name: "TNT", station: Station::Workbench, inputs: &[(Item::Coal, 2), (Item::Sand, 2)], out: (Item::Tnt, 1), ticks: 90 },
    // forge: heat. Smelting, firing, metalwork (tier 2 copper and bronze, tier 3 iron, and the Bore)
    Recipe { name: "Brick x4", station: Station::Forge, inputs: &[(Item::Clay, 2)], out: (Item::Brick, 4), ticks: 120 },
    Recipe { name: "Glass x2", station: Station::Forge, inputs: &[(Item::Sand, 2)], out: (Item::Glass, 2), ticks: 120 },
    Recipe { name: "Charcoal x2", station: Station::Forge, inputs: &[(Item::Wood, 2)], out: (Item::Charcoal, 2), ticks: 90 },
    Recipe { name: "Copper ingot", station: Station::Forge, inputs: &[(Item::CopperOre, 2)], out: (Item::CopperIngot, 1), ticks: 150 },
    Recipe { name: "Tin ingot", station: Station::Forge, inputs: &[(Item::TinOre, 2)], out: (Item::TinIngot, 1), ticks: 150 },
    Recipe { name: "Iron ingot", station: Station::Forge, inputs: &[(Item::IronOre, 2), (Item::Coal, 1)], out: (Item::IronIngot, 1), ticks: 200 },
    Recipe { name: "Bronze x4", station: Station::Forge, inputs: &[(Item::CopperIngot, 3), (Item::TinIngot, 1)], out: (Item::BronzeIngot, 4), ticks: 150 },
    // co-smelted straight from the ores: bronze stays one step from the ground, so the bronze drill is two
    Recipe { name: "Bronze x2", station: Station::Forge, inputs: &[(Item::CopperOre, 3), (Item::TinOre, 1)], out: (Item::BronzeIngot, 2), ticks: 150 },
    Recipe { name: "Copperline x8", station: Station::Forge, inputs: &[(Item::CopperIngot, 1)], out: (Item::CopperWire, 8), ticks: 80 },
    Recipe { name: "Iron beam x4", station: Station::Forge, inputs: &[(Item::IronIngot, 1)], out: (Item::IronBeam, 4), ticks: 120 },
    Recipe { name: "Lamp", station: Station::Forge, inputs: &[(Item::Glass, 1), (Item::Fungus, 2)], out: (Item::Lamp, 1), ticks: 90 },
    // transport: copper-strapped track and a cart to run on it (tier 2: copper)
    Recipe { name: "Rail x8", station: Station::Forge, inputs: &[(Item::CopperIngot, 1), (Item::Wood, 1)], out: (Item::Rail, 8), ticks: 120 },
    Recipe { name: "Mine cart", station: Station::Forge, inputs: &[(Item::CopperIngot, 2), (Item::Wood, 4)], out: (Item::Cart, 1), ticks: 200 },
    Recipe { name: "Pressure tube x8", station: Station::Forge, inputs: &[(Item::IronIngot, 1), (Item::Glass, 2)], out: (Item::Tube, 8), ticks: 150 },
    Recipe { name: "Bore", station: Station::Forge, inputs: &[(Item::IronIngot, 6), (Item::CopperIngot, 2), (Item::Crystal, 2)], out: (Item::Bore, 1), ticks: 300 },
    // workshop: copperline and power. Machines and fine parts
    // vertical travel: a rope ladder (tier 1: wood and fungus fibre), a counterweighted lift (tier 3: iron)
    // pressure tubes (the top of the ladder for now): glass pipe banded with iron, and hoppers with blowers
    Recipe { name: "Tube hopper", station: Station::Workshop, inputs: &[(Item::IronIngot, 2), (Item::CopperIngot, 2), (Item::Glass, 1)], out: (Item::TubeHopper, 1), ticks: 240 },
    Recipe { name: "Lift", station: Station::Workshop, inputs: &[(Item::IronIngot, 2), (Item::Wood, 4), (Item::CopperIngot, 1)], out: (Item::Lift, 1), ticks: 240 },
    Recipe { name: "Drill", station: Station::Workshop, inputs: &[(Item::BronzeIngot, 4)], out: (Item::Drill, 1), ticks: 240 },
    Recipe { name: "Knot", station: Station::Workshop, inputs: &[(Item::CopperIngot, 1), (Item::Quartz, 1)], out: (Item::Gate, 1), ticks: 100 },
    Recipe { name: "Thrummer", station: Station::Workshop, inputs: &[(Item::Crystal, 2), (Item::CopperIngot, 2), (Item::Glass, 1)], out: (Item::Thrummer, 1), ticks: 200 },
];
/// The recipe with this name (scenarios, UI).
pub fn recipe_by_name(n: &str) -> Option<usize> { RECIPES.iter().position(|r| r.name.eq_ignore_ascii_case(n)) }

/// Dig tool tier owned: 0 claws, 1 Pickaxe, 2 Drill, 3 Bore. Higher tiers dig faster and open harder rock.
pub fn tool_tier(inv: &[u32; NITEM]) -> u8 {
    if inv[Item::Bore as usize] > 0 { 3 } else if inv[Item::Drill as usize] > 0 { 2 } else if inv[Item::Pickaxe as usize] > 0 { 1 } else { 0 }
}

#[derive(Clone, Copy)]
pub enum Goal { Depth(f32), Have(Item, u32), Built(Station), Chests(u32), Pillars(u32) }
pub struct Quest { pub text: &'static str, pub goal: Goal, pub reward: &'static [(Item, u32)] }
pub static QUESTS: &[Quest] = &[
    // The first hour, in order: none is done on arrival in Survival (the kit has a pickaxe and 6 wood, so the
    // first wood quest comes after the workbench spends 4 of it). Each says what it takes, so the HUD line and
    // "Next:" in the log are enough to go on: wood and stone make a bench, copper ore and a forge make copperline.
    Quest { text: "Dig down to 5 m", goal: Goal::Depth(5.), reward: &[(Item::Torch, 6)] },
    Quest { text: "Build a workbench (4 wood, 4 stone)", goal: Goal::Built(Station::Workbench), reward: &[(Item::Coal, 4)] },
    Quest { text: "Find wood: dig a root tangle (have 6)", goal: Goal::Have(Item::Wood, 6), reward: &[(Item::Wood, 4)] },
    Quest { text: "Dig 4 copper ore (green-flecked rock)", goal: Goal::Have(Item::CopperOre, 4), reward: &[(Item::Coal, 2)] },
    Quest { text: "Build a forge (10 stone, 4 clay)", goal: Goal::Built(Station::Forge), reward: &[(Item::Coal, 6)] },
    Quest { text: "Smelt 4 copper ingots", goal: Goal::Have(Item::CopperIngot, 4), reward: &[(Item::TinOre, 4)] },
    Quest { text: "Draw 8 copperline at the forge", goal: Goal::Have(Item::CopperWire, 8), reward: &[(Item::Torch, 4)] },
    Quest { text: "Farm mushrooms: gather 4", goal: Goal::Have(Item::Mushroom, 4), reward: &[(Item::Spore, 4), (Item::Vent, 1)] },
    Quest { text: "Prop a tunnel: place 8 pillar cells", goal: Goal::Pillars(8), reward: &[(Item::WoodPillar, 8)] },
    Quest { text: "Reach 15 m", goal: Goal::Depth(15.), reward: &[(Item::Clay, 6)] },
    Quest { text: "Open a buried chest", goal: Goal::Chests(1), reward: &[(Item::Crystal, 1)] },
    Quest { text: "Fire 10 bricks", goal: Goal::Have(Item::Brick, 10), reward: &[(Item::Coal, 8)] },
    Quest { text: "Alloy bronze", goal: Goal::Have(Item::BronzeIngot, 4), reward: &[(Item::Torch, 10)] },
    Quest { text: "Reach 40 m", goal: Goal::Depth(40.), reward: &[(Item::IronOre, 6)] },
    Quest { text: "Build a workshop and power it", goal: Goal::Built(Station::Workshop), reward: &[(Item::Quartz, 2)] },
    Quest { text: "Make a drill", goal: Goal::Have(Item::Drill, 1), reward: &[(Item::Crystal, 2)] },
    Quest { text: "Assemble a thrummer", goal: Goal::Have(Item::Thrummer, 1), reward: &[(Item::CopperWire, 16)] },
    Quest { text: "Reach 80 m - the deep", goal: Goal::Depth(80.), reward: &[(Item::Crystal, 4)] },
    Quest { text: "Open 4 chests", goal: Goal::Chests(4), reward: &[(Item::IronIngot, 4)] },
];
