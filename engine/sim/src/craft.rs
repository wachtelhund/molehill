//! Items, stations, recipes, tool tiers, quests.
use crate::mat::Mat;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Item {
    Dirt, Clay, Sand, Gravel, Stone, Coal, CopperOre, TinOre, IronOre, Quartz, Crystal, Wood, Fungus,
    Brick, Glass, Charcoal, CopperIngot, TinIngot, BronzeIngot, IronIngot,
    Torch, WoodPillar, IronBeam, CopperWire, LoraNode, Gate, Tnt, Lamp,
    StoneTools, BronzeTools, IronTools,
}
pub const NITEM: usize = 31;
pub const ITEMS: [Item; NITEM] = [
    Item::Dirt, Item::Clay, Item::Sand, Item::Gravel, Item::Stone, Item::Coal, Item::CopperOre, Item::TinOre, Item::IronOre, Item::Quartz, Item::Crystal, Item::Wood, Item::Fungus,
    Item::Brick, Item::Glass, Item::Charcoal, Item::CopperIngot, Item::TinIngot, Item::BronzeIngot, Item::IronIngot,
    Item::Torch, Item::WoodPillar, Item::IronBeam, Item::CopperWire, Item::LoraNode, Item::Gate, Item::Tnt, Item::Lamp,
    Item::StoneTools, Item::BronzeTools, Item::IronTools,
];
impl Item {
    pub fn name(self) -> &'static str {
        ["Dirt", "Clay", "Sand", "Gravel", "Stone", "Coal", "Copper ore", "Tin ore", "Iron ore", "Quartz", "Crystal", "Wood", "Fungus",
         "Brick", "Glass", "Charcoal", "Copper ingot", "Tin ingot", "Bronze ingot", "Iron ingot",
         "Torch", "Wood pillar", "Iron beam", "Copper wire", "LoRa node", "Logic gate", "TNT", "Lamp",
         "Stone tools", "Bronze tools", "Iron tools"][self as usize]
    }
    pub fn color(self) -> [u8; 3] {
        use Item::*;
        match self {
            Dirt => [122, 88, 58], Clay => [168, 104, 78], Sand => [222, 196, 132], Gravel => [140, 134, 124], Stone => [160, 156, 150],
            Coal => [50, 48, 54], CopperOre => [200, 120, 70], TinOre => [180, 190, 200], IronOre => [150, 90, 80], Quartz => [235, 232, 240],
            Crystal => [120, 200, 255], Wood => [150, 105, 60], Fungus => [96, 200, 120], Brick => [170, 80, 60], Glass => [190, 230, 240],
            Charcoal => [70, 60, 60], CopperIngot => [230, 140, 80], TinIngot => [210, 215, 225], BronzeIngot => [205, 150, 70], IronIngot => [190, 190, 200],
            Torch => [255, 190, 90], WoodPillar => [180, 130, 80], IronBeam => [120, 130, 150], CopperWire => [240, 150, 90], LoraNode => [150, 120, 255],
            Gate => [120, 220, 160], Tnt => [220, 60, 50], Lamp => [255, 240, 170], StoneTools => [160, 156, 150], BronzeTools => [205, 150, 70], IronTools => [200, 205, 220],
        }
    }
}
/// What digging a cell yields (one item per `CELLS_PER_ITEM` cells of that material).
pub const CELLS_PER_ITEM: u16 = 6;
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
    pub fn cost(self) -> &'static [(Item, u32)] {
        match self { Station::Workbench => &[(Item::Wood, 6)], Station::Kiln => &[(Item::Stone, 8), (Item::Clay, 4)], Station::Furnace => &[(Item::Brick, 10)], Station::Anvil => &[(Item::IronIngot, 5)] }
    }
    /// footprint in cells
    pub const W: usize = 10; pub const H: usize = 7;
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
    Recipe { name: "Copper wire x8", station: Station::Anvil, inputs: &[(Item::CopperIngot, 1)], out: (Item::CopperWire, 8), ticks: 80 },
    Recipe { name: "Logic gate", station: Station::Anvil, inputs: &[(Item::CopperIngot, 1), (Item::Quartz, 1)], out: (Item::Gate, 1), ticks: 100 },
    Recipe { name: "LoRa node", station: Station::Anvil, inputs: &[(Item::Crystal, 2), (Item::CopperIngot, 2), (Item::Glass, 1)], out: (Item::LoraNode, 1), ticks: 200 },
    Recipe { name: "Lamp", station: Station::Kiln, inputs: &[(Item::Glass, 1), (Item::Fungus, 2)], out: (Item::Lamp, 1), ticks: 90 },
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
    Quest { text: "Stockpile 6 wood", goal: Goal::Have(Item::Wood, 6), reward: &[(Item::Wood, 4)] },
    Quest { text: "Build a workbench", goal: Goal::Built(Station::Workbench), reward: &[(Item::Coal, 4)] },
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
    Quest { text: "Assemble a LoRa node", goal: Goal::Have(Item::LoraNode, 1), reward: &[(Item::CopperWire, 16)] },
    Quest { text: "Reach 80 m - the deep", goal: Goal::Depth(80.), reward: &[(Item::Crystal, 4)] },
    Quest { text: "Open 4 chests", goal: Goal::Chests(4), reward: &[(Item::IronIngot, 4)] },
];
