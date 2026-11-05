//! Items, stations, recipes, tool tiers, quests.
use crate::mat::Mat;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Item {
    Dirt, Clay, Sand, Gravel, Stone, Coal, CopperOre, TinOre, IronOre, Quartz, Crystal, Wood, Fungus,
    Brick, Glass, Charcoal, CopperIngot, TinIngot, BronzeIngot, IronIngot,
    Torch, WoodPillar, IronBeam, CopperWire, Thrummer, Gate, Tnt, Lamp,
    StoneTools, BronzeTools, IronTools, Grub, Spore, Mushroom, Vent,
    FungusOil, Root,
}
pub const NITEM: usize = 37;
/// What an item can do in the world (see `Item::uses`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Uses { pub place: bool, pub throw: bool, pub pick_up: bool }
pub const ITEMS: [Item; NITEM] = [
    Item::Dirt, Item::Clay, Item::Sand, Item::Gravel, Item::Stone, Item::Coal, Item::CopperOre, Item::TinOre, Item::IronOre, Item::Quartz, Item::Crystal, Item::Wood, Item::Fungus,
    Item::Brick, Item::Glass, Item::Charcoal, Item::CopperIngot, Item::TinIngot, Item::BronzeIngot, Item::IronIngot,
    Item::Torch, Item::WoodPillar, Item::IronBeam, Item::CopperWire, Item::Thrummer, Item::Gate, Item::Tnt, Item::Lamp,
    Item::StoneTools, Item::BronzeTools, Item::IronTools, Item::Grub, Item::Spore, Item::Mushroom, Item::Vent,
    Item::FungusOil, Item::Root,
];
impl Item {
    /// The material a loose item is a lump of, if it is one: its density and burning come from the table.
    pub fn mat(self) -> Option<Mat> {
        use Item::*;
        Some(match self {
            Dirt => Mat::Dirt, Clay => Mat::Clay, Sand => Mat::Sand, Gravel => Mat::Gravel, Stone => Mat::Rock, Coal => Mat::Coal,
            CopperOre => Mat::CopperOre, TinOre => Mat::TinOre, IronOre => Mat::IronOre, Quartz => Mat::Quartz, Crystal => Mat::Crystal,
            Wood | WoodPillar | Torch => Mat::WoodBeam, Fungus => Mat::Fungus, Brick => Mat::StoneBrick, Glass => Mat::Glass,
            IronBeam | IronIngot => Mat::IronBeam,
            _ => return None,
        })
    }
    /// kg/m³ of the item lying loose (decides whether it floats or sinks): its material's, else its own.
    pub fn density(self) -> f32 {
        use Item::*;
        if let Some(m) = self.mat() { return m.p().density; }
        match self {
            Charcoal => 400., CopperIngot => 8900., TinIngot => 7300., BronzeIngot => 8800., CopperWire => 3000., Thrummer | Gate | Vent => 2500.,
            Tnt => 1500., Lamp => 2000., StoneTools => 2500., BronzeTools => 5000., IronTools => 5500., Grub => 1050., Spore => 300., Mushroom => 600., FungusOil => 900., Root => 1100.,
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
        let place = matches!(self, WoodPillar | Brick | IronBeam | CopperWire | Torch | Tnt | Lamp | Vent | Spore);
        let throw = matches!(self, Dirt | Clay | Sand | Gravel | Stone | Coal | CopperOre | TinOre | IronOre | Quartz | Crystal | Wood | Fungus
            | Charcoal | CopperIngot | TinIngot | BronzeIngot | IronIngot | Grub | Spore | Mushroom | Tnt | FungusOil | Root);
        Uses { place, throw, pick_up: matches!(self, CopperWire | Torch | Tnt | Lamp | Vent) }
    }
    /// One line naming the inputs for the item in hand (hover tooltip and the hint over the hotbar).
    pub fn controls(self) -> &'static str {
        use Item::*;
        match self {
            Tnt => "Left/right mouse: set down unlit · right mouse on it: light · Q: throw lit · E: pick up",
            WoodPillar | Brick | IronBeam => "Left mouse: build (hold to keep building)",
            CopperWire => "Left mouse: lay (drag) · E: take up (drag)",
            Torch => "Left mouse: set down · E: pick up",
            Lamp | Vent => "Left/right mouse: set down · E: pick up",
            Spore => "Left mouse: plant on soil · Q: throw",
            Thrummer | Gate => "Set down from its device slots · E: pick up",
            _ if self.uses().throw => "Q: throw (hold for further)",
            _ => "",
        }
    }
    pub fn name(self) -> &'static str {
        ["Dirt", "Clay", "Sand", "Gravel", "Stone", "Coal", "Copper ore", "Tin ore", "Iron ore", "Quartz", "Crystal", "Wood", "Fungus",
         "Brick", "Glass", "Charcoal", "Copper ingot", "Tin ingot", "Bronze ingot", "Iron ingot",
         "Torch", "Wood pillar", "Iron beam", "Copperline", "Thrummer", "Knot", "TNT", "Lamp",
         "Stone tools", "Bronze tools", "Iron tools", "Grub", "Spore", "Mushroom", "Vent", "Fungus oil", "Root"][self as usize]
    }
    /// One plain line for the hover tooltip: what it is and what it's for.
    pub fn desc(self) -> &'static str {
        ["Loose earth. Packs into burrow walls; weak under load.", "Stiff, wet earth. Holds a little better than dirt; fired into brick.",
         "Flows like water when nothing holds it. Melts into glass.", "Rubble. Shifts when disturbed.", "Broken rock. Builds kilns; cut into brick.",
         "Burns hot. Lights torches and feeds the furnace.", "Green-flecked rock. Smelted into copper for copperline.", "Pale ore. Smelted into tin for bronze.",
         "Heavy red ore. Smelted into iron for beams and heatstones.", "Clear stone that lets light through.", "Glowing stone from deep down. Lights the dark.",
         "Roots and timber. Pillars, torches and workbenches.", "Soft glowing growth. Edible in a pinch.",
         "Fired stone block. Strong in a wall, heavy on a roof.", "Clear pane. Lets light through a wall.", "Wood baked black. Burns cleaner than wood.",
         "Drawn into copperline.", "Alloyed with copper into bronze.", "Tough metal for better tools.", "Strongest metal. Beams and heatstones.",
         "Lights a few metres around it. Breathes the air stale over time.", "Wooden prop. Holds up a roof or spans a short gap.",
         "Iron girder. Spans wide gaps and carries heavy roofs.", "Copper wire. Carries current between levers, knots and lamps.",
         "Knocks a pattern into the rock for a listener to hear. Stone carries it; dirt and water muffle it.",
         "Mechanical knot: joins two lines into one answer (both, either, turn, odd, catch).", "Blasting charge. Shatters rock in a wide round.",
         "Lights when current flows through it.", "Pack tools of stone.", "Pack tools of bronze.", "Pack tools of iron.",
         "Fat burrowing larva. A good meal.", "Mushroom spore. Plant it on soil to grow food.", "A ripe mushroom. Fills the belly.",
         "Draws stale air out of the tunnels around it.",
         "Pressed from fungus. Fuels the headlamp and soaks a torch so it burns.", "A fat, starchy root from a root tangle. A fair meal."][self as usize]
    }
    pub fn color(self) -> [u8; 3] {
        use Item::*;
        match self {
            Dirt => [122, 88, 58], Clay => [168, 104, 78], Sand => [222, 196, 132], Gravel => [140, 134, 124], Stone => [160, 156, 150],
            Coal => [50, 48, 54], CopperOre => [200, 120, 70], TinOre => [180, 190, 200], IronOre => [150, 90, 80], Quartz => [235, 232, 240],
            Crystal => [120, 200, 255], Wood => [150, 105, 60], Fungus => [96, 200, 120], Brick => [170, 80, 60], Glass => [190, 230, 240],
            Charcoal => [70, 60, 60], CopperIngot => [230, 140, 80], TinIngot => [210, 215, 225], BronzeIngot => [205, 150, 70], IronIngot => [190, 190, 200],
            Torch => [255, 190, 90], WoodPillar => [180, 130, 80], IronBeam => [120, 130, 150], CopperWire => [240, 150, 90], Thrummer => [150, 120, 255],
            Gate => [120, 220, 160], Tnt => [220, 60, 50], Lamp => [255, 240, 170], StoneTools => [160, 156, 150], BronzeTools => [205, 150, 70], IronTools => [200, 205, 220], Grub => [235, 215, 170], Spore => [200, 170, 230], Mushroom => [214, 120, 90], Vent => [150, 200, 210], FungusOil => [200, 220, 120], Root => [190, 140, 90],
        }
    }
}
/// What digging a cell yields (one item per `CELLS_PER_ITEM` cells of that material).
pub const CELLS_PER_ITEM: u16 = 6 * (crate::mat::S * crate::mat::S) as u16; // 6 legacy tiles
pub fn drop_of(m: Mat) -> Option<Item> {
    use Mat::*;
    Some(match m {
        Dirt | Grass => Item::Dirt, Clay => Item::Clay, Sand => Item::Sand, Gravel => Item::Gravel,
        Granite | Rock | Limestone | Basalt | StoneBrick => Item::Stone, Coal => Item::Coal, CopperOre => Item::CopperOre, TinOre => Item::TinOre,
        IronOre => Item::IronOre, Quartz => Item::Quartz, Crystal => Item::Crystal, Roots | WoodBeam | Door => Item::Wood, Fungus => Item::Fungus,
        IronBeam => Item::IronIngot, Glass => Item::Glass,
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Station { Workbench, Kiln, Furnace, Anvil }
pub const STATIONS: [Station; 4] = [Station::Workbench, Station::Kiln, Station::Furnace, Station::Anvil];
impl Station {
    pub fn name(self) -> &'static str { ["Workbench", "Kiln", "Furnace", "Anvil"][self as usize] }
    pub fn desc(self) -> &'static str { ["Saw and bench for wooden things.", "Bakes clay into brick and sand into glass.", "Smelts ore into metal.", "Hammers metal into tools and beams."][self as usize] }
    pub fn cost(self) -> &'static [(Item, u32)] {
        match self { Station::Workbench => &[(Item::Wood, 6)], Station::Kiln => &[(Item::Stone, 8), (Item::Clay, 4)], Station::Furnace => &[(Item::Brick, 10)], Station::Anvil => &[(Item::IronIngot, 5)] }
    }
    /// footprint in cells
    pub const W: usize = 10 * crate::mat::S; pub const H: usize = 7 * crate::mat::S;
}

pub struct Recipe { pub name: &'static str, pub station: Station, pub inputs: &'static [(Item, u32)], pub out: (Item, u32), pub ticks: u32 }
pub static RECIPES: &[Recipe] = &[
    Recipe { name: "Torch x4", station: Station::Workbench, inputs: &[(Item::Wood, 1), (Item::Coal, 1)], out: (Item::Torch, 4), ticks: 60 },
    Recipe { name: "Torch x4 (charcoal)", station: Station::Workbench, inputs: &[(Item::Wood, 1), (Item::Charcoal, 1)], out: (Item::Torch, 4), ticks: 60 },
    Recipe { name: "Wood pillar x4", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::WoodPillar, 4), ticks: 60 },
    Recipe { name: "Stone tools", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Stone, 4)], out: (Item::StoneTools, 1), ticks: 120 },
    Recipe { name: "Brick x4", station: Station::Kiln, inputs: &[(Item::Clay, 2), (Item::Coal, 1)], out: (Item::Brick, 4), ticks: 120 },
    Recipe { name: "Glass x2", station: Station::Kiln, inputs: &[(Item::Sand, 2), (Item::Coal, 1)], out: (Item::Glass, 2), ticks: 120 },
    Recipe { name: "Charcoal x2", station: Station::Kiln, inputs: &[(Item::Wood, 2)], out: (Item::Charcoal, 2), ticks: 90 },
    Recipe { name: "Copper ingot", station: Station::Furnace, inputs: &[(Item::CopperOre, 2), (Item::Coal, 1)], out: (Item::CopperIngot, 1), ticks: 150 },
    Recipe { name: "Tin ingot", station: Station::Furnace, inputs: &[(Item::TinOre, 2), (Item::Coal, 1)], out: (Item::TinIngot, 1), ticks: 150 },
    Recipe { name: "Iron ingot", station: Station::Furnace, inputs: &[(Item::IronOre, 2), (Item::Coal, 2)], out: (Item::IronIngot, 1), ticks: 200 },
    Recipe { name: "Bronze x4", station: Station::Furnace, inputs: &[(Item::CopperIngot, 3), (Item::TinIngot, 1)], out: (Item::BronzeIngot, 4), ticks: 150 },
    Recipe { name: "Bronze tools", station: Station::Anvil, inputs: &[(Item::BronzeIngot, 3), (Item::Wood, 1)], out: (Item::BronzeTools, 1), ticks: 200 },
    Recipe { name: "Iron tools", station: Station::Anvil, inputs: &[(Item::IronIngot, 3), (Item::Wood, 1)], out: (Item::IronTools, 1), ticks: 240 },
    Recipe { name: "Iron beam x4", station: Station::Anvil, inputs: &[(Item::IronIngot, 1)], out: (Item::IronBeam, 4), ticks: 120 },
    Recipe { name: "Copperline x8", station: Station::Anvil, inputs: &[(Item::CopperIngot, 1)], out: (Item::CopperWire, 8), ticks: 80 },
    Recipe { name: "Knot", station: Station::Anvil, inputs: &[(Item::CopperIngot, 1), (Item::Quartz, 1)], out: (Item::Gate, 1), ticks: 100 },
    Recipe { name: "Thrummer", station: Station::Anvil, inputs: &[(Item::Crystal, 2), (Item::CopperIngot, 2), (Item::Glass, 1)], out: (Item::Thrummer, 1), ticks: 200 },
    Recipe { name: "Lamp", station: Station::Kiln, inputs: &[(Item::Glass, 1), (Item::Fungus, 2)], out: (Item::Lamp, 1), ticks: 90 },
    Recipe { name: "Vent", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Stone, 2)], out: (Item::Vent, 1), ticks: 90 },
    Recipe { name: "TNT", station: Station::Workbench, inputs: &[(Item::Coal, 2), (Item::Sand, 2)], out: (Item::Tnt, 1), ticks: 90 },
];

/// Tool tier 0 claws, 1 stone, 2 bronze, 3 iron. Higher tiers dig faster and unlock harder rock.
pub fn tool_tier(inv: &[u32; NITEM]) -> u8 {
    if inv[Item::IronTools as usize] > 0 { 3 } else if inv[Item::BronzeTools as usize] > 0 { 2 } else if inv[Item::StoneTools as usize] > 0 { 1 } else { 0 }
}

#[derive(Clone, Copy)]
pub enum Goal { Depth(f32), Have(Item, u32), Built(Station), Chests(u32), Pillars(u32) }
pub struct Quest { pub text: &'static str, pub goal: Goal, pub reward: &'static [(Item, u32)] }
pub static QUESTS: &[Quest] = &[
    Quest { text: "Dig down to 5 m", goal: Goal::Depth(5.), reward: &[(Item::Torch, 6)] },
    Quest { text: "Gather 6 wood", goal: Goal::Have(Item::Wood, 6), reward: &[(Item::Wood, 4)] },
    Quest { text: "Build a workbench", goal: Goal::Built(Station::Workbench), reward: &[(Item::Coal, 4)] },
    Quest { text: "Farm mushrooms: gather 4", goal: Goal::Have(Item::Mushroom, 4), reward: &[(Item::Spore, 4), (Item::Vent, 1)] },
    Quest { text: "Prop a tunnel: place 8 pillar cells", goal: Goal::Pillars(8), reward: &[(Item::WoodPillar, 8)] },
    Quest { text: "Craft stone tools", goal: Goal::Have(Item::StoneTools, 1), reward: &[(Item::Torch, 8)] },
    Quest { text: "Reach 15 m", goal: Goal::Depth(15.), reward: &[(Item::Clay, 6)] },
    Quest { text: "Build a kiln", goal: Goal::Built(Station::Kiln), reward: &[(Item::Coal, 6)] },
    Quest { text: "Open a buried chest", goal: Goal::Chests(1), reward: &[(Item::Crystal, 1)] },
    Quest { text: "Fire 10 bricks", goal: Goal::Have(Item::Brick, 10), reward: &[(Item::Coal, 8)] },
    Quest { text: "Build a furnace", goal: Goal::Built(Station::Furnace), reward: &[(Item::CopperOre, 4)] },
    Quest { text: "Smelt 4 copper ingots", goal: Goal::Have(Item::CopperIngot, 4), reward: &[(Item::TinOre, 4)] },
    Quest { text: "Alloy bronze", goal: Goal::Have(Item::BronzeIngot, 4), reward: &[(Item::Torch, 10)] },
    Quest { text: "Reach 40 m", goal: Goal::Depth(40.), reward: &[(Item::IronOre, 6)] },
    Quest { text: "Forge an anvil", goal: Goal::Built(Station::Anvil), reward: &[(Item::Quartz, 2)] },
    Quest { text: "Assemble a thrummer", goal: Goal::Have(Item::Thrummer, 1), reward: &[(Item::CopperWire, 16)] },
    Quest { text: "Reach 80 m - the deep", goal: Goal::Depth(80.), reward: &[(Item::Crystal, 4)] },
    Quest { text: "Open 4 chests", goal: Goal::Chests(4), reward: &[(Item::IronIngot, 4)] },
];
