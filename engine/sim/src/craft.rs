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
    // production (production.rs): chute and shaker screen blocks, a cart bumper, an ore hopper that feeds carts
    Chute, Screen, Bumper, OreHopper,
    // the rope gun (ropes.rs) and the hook ropes it throws
    RopeGun, HookRope,
    // sluicing (production.rs, sluice.rs): riffle bars for a sluice floor
    Riffle,
    // mechanical power (power.rs): shafts, gears, water wheels, belts and a counterweight on its drum
    Shaft, Gear, WaterWheel, Belt, Counterweight,
    // coal power (steam.rs): the boiler, a steam engine and its pipe, and what burning leaves
    Boiler, Engine, SteamPipe, Ash, Slag,
    // refinement (refine.rs): the works, bellows, ingot moulds, and the concentrates a stamp mill and washer make
    StampMill, Washer, Smelter, Bellows, Mould, CopperFines, TinFines, IronFines,
    // non-electric logic (linkage.rs): pull-rods, a tread plate, a float, a sluice gate, a tipping scale, a clutch
    Rod, TreadPlate, Float, SluiceGate, Scale, Clutch,
}
pub const NITEM: usize = 100;
/// What an item can do in the world (see `Item::uses`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Uses { pub place: bool, pub throw: bool, pub pick_up: bool }
/// Paint colours by pigment index (0 = unpainted): ochre, rust, chalk, verdigris, woad, soot.
pub const PIGMENTS: [[u8; 3]; 7] = [[0, 0, 0], [206, 160, 70], [164, 70, 46], [232, 226, 210], [84, 162, 132], [74, 112, 190], [50, 46, 52]];
/// A piece of furniture's footprint in 0.25 m tiles, and how it's held up: standing on the floor, or
/// hanging from the ceiling. `cloth` = it has a dyeable part (a pigment colours it).
#[derive(Clone, Copy, Debug)] pub struct Furn { pub w: usize, pub h: usize, pub hang: bool, pub cloth: bool }

/// Everything the game knows about an item, in one row of `ITEM_DEFS` (as `mat::Props` is for
/// materials). A new item is a new `Item` variant plus one row; the `Item` methods only read the row.
#[derive(Clone, Copy, Debug)]
pub struct ItemDef {
    pub item: Item,
    pub name: &'static str,
    /// one plain line for the hover tooltip: what it is and what it's for
    pub desc: &'static str,
    pub color: [u8; 3],
    /// the material a loose item is a lump of: its density and burning come from the material table
    pub mat: Option<Mat>,
    /// kg/m³ lying loose, for items that are no lump of a material (else the material's)
    pub density: f32,
    /// heat that burns it up when it is no lump (0 = never; a lump burns as its material lights)
    pub ignite: u8,
    /// a decorative block's material (built with the block brush)
    pub block: Option<Mat>,
    /// a pigment's paint index (1..6, see `PIGMENTS`); 0 = not a pigment
    pub pigment: u8,
    pub furn: Option<Furn>,
    /// a decorative background wall (walls.rs): panels the back, holds no roof
    pub wall: bool,
    pub uses: Uses,
    /// the input line for the item in hand ("" = the default: the throw hint if it throws, else nothing)
    pub controls: &'static str,
    /// its lump size (m): the coarsest piece in it. A shaker screen lets through what is finer than its mesh
    /// (`production::MESH_M`); 0.25 m (a whole lump) unless set
    pub size_m: f32,
}
const fn it(item: Item, name: &'static str, color: [u8; 3], desc: &'static str) -> ItemDef {
    ItemDef { item, name, desc, color, mat: None, density: 2000., ignite: 0, block: None, pigment: 0, furn: None, wall: false,
        uses: Uses { place: false, throw: false, pick_up: false }, controls: "", size_m: 0.25 }
}
impl ItemDef {
    const fn lump(mut self, m: Mat) -> Self { self.mat = Some(m); self }
    const fn dens(mut self, d: f32) -> Self { self.density = d; self }
    const fn ignite(mut self, h: u8) -> Self { self.ignite = h; self }
    const fn block(mut self, m: Mat) -> Self { self.block = Some(m); self }
    const fn pigment(mut self, p: u8) -> Self { self.pigment = p; self }
    const fn furn(mut self, w: usize, h: usize, hang: bool, cloth: bool) -> Self { self.furn = Some(Furn { w, h, hang, cloth }); self }
    const fn wall(mut self) -> Self { self.wall = true; self }
    const fn place(mut self) -> Self { self.uses.place = true; self }
    const fn throw(mut self) -> Self { self.uses.throw = true; self }
    const fn pick_up(mut self) -> Self { self.uses.pick_up = true; self }
    const fn controls(mut self, c: &'static str) -> Self { self.controls = c; self }
    const fn size(mut self, m: f32) -> Self { self.size_m = m; self }
}

/// The item registry, in `Item` order (checked at compile time below).
pub static ITEM_DEFS: [ItemDef; NITEM] = ITEM_DEFS_CONST;
const ITEM_DEFS_CONST: [ItemDef; NITEM] = {
    use Item::*;
    [
    it(Dirt, "Dirt", [122, 88, 58],
       "Loose earth. Packs into burrow walls; weak under load.").size(0.01).lump(Mat::Dirt).throw(),
    it(Clay, "Clay", [168, 104, 78],
       "Stiff, wet earth. Holds a little better than dirt; fired into brick.").size(0.02).lump(Mat::Clay).throw(),
    it(Sand, "Sand", [222, 196, 132],
       "Flows like water when nothing holds it. Melts into glass.").size(0.002).lump(Mat::Sand).throw(),
    it(Gravel, "Gravel", [140, 134, 124],
       "Rubble. Shifts when disturbed.").size(0.03).lump(Mat::Gravel).throw(),
    it(Stone, "Stone", [160, 156, 150],
       "Broken rock. Builds kilns; cut into brick.").size(0.2).lump(Mat::Rock).throw(),
    it(Coal, "Coal", [50, 48, 54],
       "Burns hot. Lights torches and feeds the furnace.").size(0.08).lump(Mat::Coal).throw(),
    it(CopperOre, "Copper ore", [200, 120, 70],
       "Green-flecked rock. Smelted into copper for copperline.").size(0.12).lump(Mat::CopperOre).throw(),
    it(TinOre, "Tin ore", [180, 190, 200],
       "Pale ore. Alloyed with copper into bronze.").size(0.12).lump(Mat::TinOre).throw(),
    it(IronOre, "Iron ore", [150, 90, 80],
       "Heavy red ore. Smelted into iron for beams and heatstones.").size(0.12).lump(Mat::IronOre).throw(),
    it(Quartz, "Quartz", [235, 232, 240],
       "Clear stone that lets light through.").size(0.1).lump(Mat::Quartz).throw(),
    it(Crystal, "Crystal", [120, 200, 255],
       "Glowing stone from deep down. Lights the dark.").size(0.1).lump(Mat::Crystal).throw(),
    it(Wood, "Wood", [150, 105, 60],
       "Roots and timber. Pillars, torches and workbenches.").lump(Mat::WoodBeam).throw(),
    it(Fungus, "Fungus", [96, 200, 120],
       "Soft glowing growth. Edible in a pinch.").size(0.04).lump(Mat::Fungus).throw(),
    it(Brick, "Brick", [170, 80, 60],
       "Fired stone block. Strong in a wall, heavy on a roof.").lump(Mat::StoneBrick).place()
       .controls("Left mouse: build (hold to keep building)"),
    it(Glass, "Glass", [190, 230, 240],
       "Clear pane. Lets light through a wall.").lump(Mat::Glass),
    it(Charcoal, "Charcoal", [70, 60, 60],
       "Wood baked black. Burns cleaner than wood.").size(0.08).dens(400.0).ignite(1).throw(),
    it(CopperIngot, "Copper ingot", [230, 140, 80],
       "Drawn into copperline.").dens(8900.0).throw(),
    it(TinIngot, "Tin ingot", [210, 215, 225],
       "Alloyed with copper into bronze.").dens(7300.0).throw(),
    it(BronzeIngot, "Bronze ingot", [205, 150, 70],
       "Tough metal: the drill's teeth are bronze.").dens(8800.0).throw(),
    it(IronIngot, "Iron ingot", [190, 190, 200],
       "Strongest metal. Beams and heatstones.").lump(Mat::IronBeam).throw(),
    it(Torch, "Torch", [255, 190, 90],
       "Lights a few metres around it. Breathes the air stale over time.").lump(Mat::WoodBeam).place().pick_up()
       .controls("Left mouse: set down · E: pick up"),
    it(WoodPillar, "Wood pillar", [180, 130, 80],
       "Wooden prop. Holds up a roof or spans a short gap.").lump(Mat::WoodBeam).place()
       .controls("Left mouse: build (hold to keep building)"),
    it(IronBeam, "Iron beam", [120, 130, 150],
       "Iron girder. Spans wide gaps and carries heavy roofs.").lump(Mat::IronBeam).place()
       .controls("Left mouse: build (hold to keep building)"),
    it(CopperWire, "Copperline", [240, 150, 90],
       "Copper wire. Carries current between levers, knots and lamps.").dens(3000.0).place().pick_up()
       .controls("Left mouse: lay (drag) · E: take up (drag)"),
    it(Thrummer, "Thrummer", [150, 120, 255],
       "Knocks a pattern into the rock for a listener to hear. Stone carries it; dirt and water muffle it.").dens(2500.0)
       .controls("Set down from its device slots · E: pick up"),
    it(Gate, "Knot", [120, 220, 160],
       "Mechanical knot: joins two lines into one answer (both, either, turn, odd, catch).").dens(2500.0)
       .controls("Set down from its device slots · E: pick up"),
    it(Tnt, "TNT", [220, 60, 50],
       "Blasting charge. Shatters rock in a wide round.").dens(1500.0).place().throw().pick_up()
       .controls("Left/right mouse: set down unlit · right mouse on it: light · Q: throw lit · E: pick up"),
    it(Lamp, "Lamp", [255, 240, 170],
       "Lights when current flows through it.").place().pick_up()
       .controls("Left/right mouse: set down · E: pick up"),
    it(Pickaxe, "Pickaxe", [160, 156, 150],
       "Stone pick: breaks soil and soft stone (limestone, sandstone, coal, copper and tin ore).").dens(2500.0),
    it(Drill, "Drill", [205, 150, 70],
       "Bronze-toothed auger driven by copperline: cuts granite and iron ore.").dens(5000.0),
    it(Bore, "Bore", [200, 205, 220],
       "Heavy heat-bore: grinds through basalt.").dens(5500.0),
    it(Grub, "Grub", [235, 215, 170],
       "Fat burrowing larva. A good meal.").size(0.06).dens(1050.0).ignite(1).throw(),
    it(Spore, "Spore", [200, 170, 230],
       "Mushroom spore. Plant it on soil to grow food.").size(0.005).dens(300.0).ignite(1).place().throw()
       .controls("Left mouse: plant on soil · Q: throw"),
    it(Mushroom, "Mushroom", [214, 120, 90],
       "A ripe mushroom. Fills the belly.").size(0.06).dens(600.0).ignite(1).throw(),
    it(Vent, "Vent", [150, 200, 210],
       "Draws stale air out of the tunnels around it.").dens(2500.0).place().pick_up()
       .controls("Left/right mouse: set down · E: pick up"),
    it(Glowmite, "Glowmite", [190, 255, 160],
       "A glowmite caught in a jar. Glows softly, needs no fuel.")
       .controls("Held, it lights the way · right mouse on a glowmite in a grotto: catch one"),
    it(FungusOil, "Fungus oil", [200, 220, 120],
       "Pressed from fungus. Fuels the headlamp and soaks a torch so it burns.").dens(900.0).ignite(1).throw(),
    it(Root, "Root", [190, 140, 90],
       "A fat, starchy root from a root tangle. A fair meal.").size(0.06).dens(1100.0).ignite(1).throw(),
    it(Planks, "Planks", [182, 134, 84],
       "Sawn boards for floors and panelling. A light prop.").lump(Mat::Planks).block(Mat::Planks).place()
       .controls("Left mouse: build (hold to keep building)"),
    it(PolishedStone, "Polished stone", [176, 170, 160],
       "Stone dressed smooth. Fine walls and floors, as strong as brick.").lump(Mat::Polished).block(Mat::Polished).place()
       .controls("Left mouse: build (hold to keep building)"),
    it(Tiles, "Tiles", [190, 108, 74],
       "Fired clay tiles in a neat grid. Bright, but brittle under a roof.").lump(Mat::Tiles).block(Mat::Tiles).place()
       .controls("Left mouse: build (hold to keep building)"),
    it(CarvedPillar, "Carved pillar", [164, 158, 148],
       "Fluted stone column. Holds a roof better than brick.").lump(Mat::Carved).block(Mat::Carved).place()
       .controls("Left mouse: build (hold to keep building)"),
    it(Ochre, "Ochre", PIGMENTS[1],
       "Yellow earth pigment, ground from iron ore and clay.").size(0.005).dens(1800.0).pigment(1).place()
       .controls("Left mouse: paint blocks, or dye a rug, banner or nest (hold to keep painting)"),
    it(Rust, "Rust", PIGMENTS[2],
       "Red-brown pigment, ground from iron ore.").size(0.005).dens(1800.0).pigment(2).place()
       .controls("Left mouse: paint blocks, or dye a rug, banner or nest (hold to keep painting)"),
    it(Chalk, "Chalk", PIGMENTS[3],
       "Pale pigment, ground from quartz.").size(0.005).dens(1800.0).pigment(3).place()
       .controls("Left mouse: paint blocks, or dye a rug, banner or nest (hold to keep painting)"),
    it(Verdigris, "Verdigris", PIGMENTS[4],
       "Green pigment, scraped from weathered copper ore.").size(0.005).dens(1800.0).pigment(4).place()
       .controls("Left mouse: paint blocks, or dye a rug, banner or nest (hold to keep painting)"),
    it(Woad, "Woad", PIGMENTS[5],
       "Blue pigment, pressed from fungus.").size(0.005).dens(1800.0).pigment(5).place()
       .controls("Left mouse: paint blocks, or dye a rug, banner or nest (hold to keep painting)"),
    it(Soot, "Soot", PIGMENTS[6],
       "Black pigment, from charcoal.").size(0.005).dens(1800.0).pigment(6).place()
       .controls("Left mouse: paint blocks, or dye a rug, banner or nest (hold to keep painting)"),
    it(Table, "Table", [160, 112, 66],
       "A sturdy board on legs. Somewhere to eat.").lump(Mat::WoodBeam).furn(8, 4, false, false).place().pick_up()
       .controls("Left mouse: set down on the floor · E: pick up"),
    it(Stool, "Stool", [160, 112, 66],
       "A low seat.").lump(Mat::WoodBeam).furn(3, 3, false, false).place().pick_up()
       .controls("Left mouse: set down on the floor · E: pick up"),
    it(Nest, "Nest", [150, 170, 110],
       "A soft bed of moss and cloth. Dye it.").dens(300.0).furn(9, 3, false, true).place().pick_up()
       .controls("Left mouse: set down on the floor · E: pick up"),
    it(Shelf, "Shelf", [160, 112, 66],
       "A tall rack of shelves.").lump(Mat::WoodBeam).furn(6, 8, false, false).place().pick_up()
       .controls("Left mouse: set down on the floor · E: pick up"),
    it(Rug, "Rug", [176, 92, 70],
       "A woven floor rug. Dye it.").dens(300.0).furn(10, 1, false, true).place().pick_up()
       .controls("Left mouse: set down on the floor · E: pick up"),
    it(Banner, "Banner", [200, 70, 60],
       "A cloth banner hung from the ceiling. Dye it.").dens(300.0).furn(3, 6, true, true).place().pick_up()
       .controls("Left mouse: hang from the ceiling · E: pick up"),
    it(Planter, "Fungus planter", [110, 200, 130],
       "A box of glowing fungus. A soft green light.").lump(Mat::WoodBeam).furn(5, 3, false, false).place().pick_up()
       .controls("Left mouse: set down on the floor · E: pick up"),
    it(AncientBit, "Ancient bit", [150, 200, 210],
       "A relic of the Ancients: a bit of a metal no forge makes now. Carried, every bite digs a quarter faster.").dens(7000.0),
    it(GroundLens, "Ground lens", [210, 235, 250],
       "A relic of the Ancients: a lens ground true. Carried, the headlamp burns half the oil.").dens(2600.0),
    it(Rail, "Rail", [104, 84, 78],
       "Track: copper straps on wooden sleepers. Lay it on a floor; mine carts run along it, downhill on their own.").lump(Mat::Rail).block(Mat::Rail).place()
       .controls("Left mouse: lay track (X: snap to a straight line) · carts run along its top"),
    it(Cart, "Mine cart", [130, 96, 60],
       "A copper-bound cart on four wheels. Push it, ride it, throw cargo in. Rolls downhill on its own.").dens(1200.0).place().pick_up()
       .controls("Left mouse: set on the track · right mouse: get in · walk into it: push · E: unload, then pick up"),
    it(RopeLadder, "Rope ladder", [170, 140, 90],
       "Fungus-fibre rope and wooden rungs. Hang it from a ledge: it unrolls up to 8 m. Slow, but needs no power.").dens(500.0).place().pick_up()
       .controls("Left mouse in a drop: hang from the rock above or a ledge's lip · W/S: climb · E: take down"),
    it(Lift, "Lift", [150, 150, 160],
       "A cage on a rope with a counterweight. Load stone on to lift the cage, or wind it with current.").dens(3000.0).place().pick_up()
       .controls("Left mouse in a shaft: the head hangs under its ceiling · right mouse on head or cage: stone on (stone in paw) or off · E: take down"),
    it(Tube, "Pressure tube", [150, 190, 186],
       "Iron-banded glass pipe. Lay a line of it between tube hoppers to blow capsules along.").lump(Mat::Tube).block(Mat::Tube).place()
       .controls("Left mouse: lay pipe (X: snap to a straight line) · hoppers on a line blow capsules along it"),
    it(TubeHopper, "Tube hopper", [190, 150, 80],
       "A mouth and a blower on a pressure tube. Items dropped in are blown along the line to another hopper.").dens(3000.0).place().pick_up()
       .controls("Left mouse: set on a tube · drop items in to send them · right mouse: ride (empty paw) or pick where it sends · E: pick up"),
    it(RockWall, "Rock wall", [118, 108, 96],
       "Carved rock lining for the back of a room. Walk in front of it; it holds no roof.").wall().place()
       .controls("Left mouse: panel the back (hold) · right mouse: scrape lining · never edits the rock in front"),
    it(TimberWall, "Timber wall", [140, 102, 66],
       "Timber panelling for the back of a holt. Walk in front of it; it holds no roof.").wall().place()
       .controls("Left mouse: panel the back (hold) · right mouse: scrape lining · never edits the rock in front"),
    it(MasonryWall, "Masonry wall", [132, 78, 62],
       "Brick lining for the back of a room. Walk in front of it; it holds no roof.").wall().place()
       .controls("Left mouse: panel the back (hold) · right mouse: scrape lining · never edits the rock in front"),
    it(BeamWall, "Beam wall", [118, 84, 54],
       "Decorative beams on the back. They hold no weight.").wall().place()
       .controls("Left mouse: panel the back (hold) · right mouse: scrape lining · never edits the rock in front"),
    it(PipeWall, "Pipe wall", [148, 96, 64],
       "Old copper pipes on the back. They carry nothing.").wall().place()
       .controls("Left mouse: panel the back (hold) · right mouse: scrape lining · never edits the rock in front"),
    it(CarvedWall, "Carved wall", [124, 116, 106],
       "Carved marks on the back. They hold no roof.").wall().place()
       .controls("Left mouse: panel the back (hold) · right mouse: scrape lining · never edits the rock in front"),
    // production (production.rs)
    it(Chute, "Chute", [168, 128, 82],
       "Smooth timber trough. Lay it sloping: loose things slide down it.").lump(Mat::Chute).block(Mat::Chute).place()
       .controls("Left mouse: lay a sloping trough (hold) · loose things slide down it"),
    it(Screen, "Shaker screen", [120, 116, 112],
       "Iron grating that sorts by size: sand, dirt and gravel drop through, lumps of ore ride over.").lump(Mat::Screen).block(Mat::Screen).place()
       .controls("Left mouse: lay a sloping grating (hold) · fines drop through · copperline current shakes it"),
    it(Bumper, "Bumper", [150, 70, 48],
       "Timber buffer for the end of a track. A loaded cart that runs into it tips its load out.").lump(Mat::Bumper).block(Mat::Bumper).place()
       .controls("Left mouse: build at the end of the track · a loaded cart that runs into it tips out"),
    it(OreHopper, "Ore hopper", [140, 104, 66],
       "A timber bin on legs over the track. Fill it from above; it lets its load down into a cart.").dens(900.0).place().pick_up()
       .controls("Left mouse: stand it over the track · fill it from above · a cart under it is filled · right mouse: hold the gate open · E: empty, then pick up"),
    it(RopeGun, "Rope gun", [128, 96, 64],
       "A crank-wound spring throws a stone hook on 16 m of rope. It bites solid rock, not loose ground.").dens(700.0)
       .controls("Left mouse: throw the hook at the cursor (uses a hook rope) · W on the rope: grab · W/S climb · A/D swing · Space: let go · E on a rope: take it back"),
    it(HookRope, "Hook rope", [196, 170, 112],
       "A stone hook on 16 m of fungus-fibre rope, coiled for the rope gun. E takes a thrown one back.").dens(500.0).throw(),
    it(Riffle, "Riffle bar", [122, 88, 54],
       "Low timber bars for a sluice floor. Running water washes the fines on; heavy lumps settle behind them.").lump(Mat::Riffle).block(Mat::Riffle).place()
       .controls("Left mouse: build bars across a sloping channel (hold) · run water down it to wash what's tipped in"),
    it(Shaft, "Iron shaft", [140, 144, 156],
       "Iron rod on bearings. Lay it in a straight line to carry turning; a gear takes it round a corner.").lump(Mat::Shaft).block(Mat::Shaft).place()
       .controls("Left mouse: lay a straight line (hold; X: snap) · join it to a wheel, gear or belt"),
    it(Gear, "Gear", [160, 120, 72],
       "Toothed wooden wheels. One joins shafts, belts and wheels on all sides and turns a drive round a corner.").lump(Mat::Gear).block(Mat::Gear).place()
       .controls("Left mouse: build where shafts meet or turn"),
    it(WaterWheel, "Water wheel", [128, 96, 60],
       "Timber paddles on an axle. Build it over a running stream, its foot in the water: the current turns it.").lump(Mat::Wheel).block(Mat::Wheel).place()
       .controls("Left mouse: build it over a stream so its lowest paddles dip in · gear or shaft it to what it drives"),
    it(Belt, "Belt", [92, 64, 44],
       "Woven fungus-fibre belt over wooden rollers. Turned, it carries what lies on it the way it was laid.").lump(Mat::Belt).block(Mat::Belt).place()
       .controls("Left mouse: lay it (it runs the way you face) · right mouse: turn the run round · join it to a turning shaft or gear"),
    it(Counterweight, "Counterweight", [118, 112, 108],
       "A rope drum and an iron weight. Spare power winds it up; when the drive runs short, it falls and turns it.").dens(5000.0).place().pick_up()
       .controls("Left mouse: hang the drum under a roof over a drop · join it to the drive · E: take down"),
    it(Boiler, "Boiler", [120, 84, 70],
       "Brick firebox under an iron drum. Coal in, water from the ground; makes steam and leaves ash.").dens(4000.0).place().pick_up()
       .controls("Left mouse: stand it in a sump or by water · drop coal, charcoal or wood in · haul its ash away · E: rake out fuel, then pick up"),
    it(Engine, "Steam engine", [96, 92, 98],
       "Iron cylinder and crank. Touching a boiler, or piped to one, it turns the drive train it's part of.").lump(Mat::Engine).block(Mat::Engine).place()
       .controls("Left mouse: build against a boiler or at a steam pipe's end · join it to a shaft, gear or belt"),
    it(SteamPipe, "Steam pipe", [176, 108, 66],
       "Copper pipe. Lay it from a boiler to a steam engine to carry the steam.").lump(Mat::SteamPipe).block(Mat::SteamPipe).place()
       .controls("Left mouse: lay it from a boiler to an engine (hold; X: snap)"),
    it(Ash, "Ash", [150, 146, 140],
       "Fine grey ash from a firebox. Light: water washes it off and a screen lets it through.").size(0.001).dens(600.0).throw(),
    it(Slag, "Slag", [70, 66, 74],
       "A glassy lump of fused clinker from burnt coal. Heavy, and it has to be hauled off.").size(0.08).dens(2500.0).throw(),
    it(StampMill, "Stamp mill", [112, 100, 92],
       "Iron stamps on a cam, over a mortar. Turned by a drive train, they crush ore into concentrate and sand.").dens(4000.0).place().pick_up()
       .controls("Left mouse: stand it on a floor, against a turning gear or shaft · drop ore on it · E: empty, then pick up"),
    it(Washer, "Washer", [104, 112, 120],
       "A rocking tub. Stood in water and turned, it sorts lumps by weight: heavy out left, light out right.").dens(1200.0).place().pick_up()
       .controls("Left mouse: stand it by water, against a turning gear · drop crushed ore in · E: empty, then pick up"),
    it(Smelter, "Smelter", [136, 70, 54],
       "A brick stack for melting ore. Coal and ore go in at the top; bellows beside it make it hot enough.").dens(2400.0).place().pick_up()
       .controls("Left mouse: stand it on a floor · drop coal and ore in · bellows and an ingot mould against it · E: empty, then pick up"),
    it(Bellows, "Bellows", [140, 108, 72],
       "Leaf-and-board bellows on a crank. Built against a smelter and turned by a drive train, they blow its fire.").lump(Mat::Bellows).block(Mat::Bellows).place()
       .controls("Left mouse: build against a smelter · join them to a turning shaft or gear"),
    it(Mould, "Ingot mould", [150, 128, 96],
       "A sand-and-clay mould. Built against a smelter, it's filled with molten metal and gives an ingot.").lump(Mat::Mould).block(Mat::Mould).place()
       .controls("Left mouse: build a tile of it against a smelter's side"),
    it(CopperFines, "Copper concentrate", [176, 140, 100],
       "Crushed and washed copper ore. Heavy and fine; it smelts to more copper than raw ore does.").size(0.005).dens(4200.0).throw(),
    it(TinFines, "Tin concentrate", [160, 160, 168],
       "Crushed and washed tin ore. Very heavy and fine; it smelts to more tin than raw ore does.").size(0.005).dens(6900.0).throw(),
    it(IronFines, "Iron concentrate", [140, 76, 64],
       "Crushed and washed iron ore. Heavy and fine; it smelts to more iron than raw ore does.").size(0.005).dens(5200.0).throw(),
    it(Rod, "Pull-rod", [128, 120, 116],
       "Iron rods on guides. Lay them from a plate, scale or float to a gate or clutch: a pull goes along them.").lump(Mat::Rod).block(Mat::Rod).place()
       .controls("Left mouse: lay a line (hold; X: snap) from a plate, scale or float to a gate, clutch or ore hopper"),
    it(TreadPlate, "Tread plate", [150, 140, 128],
       "A sprung iron plate for a floor. A mole, a few lumps or a cart on it pulls the rods joined to it.").lump(Mat::TreadPlate).block(Mat::TreadPlate).place()
       .controls("Left mouse: build into a floor · join pull-rods to it"),
    it(Float, "Float", [184, 150, 96],
       "A sealed timber float on its rod. Where water reaches it, it lifts and pulls its rods.").lump(Mat::Float).block(Mat::Float).place()
       .controls("Left mouse: build it where the water should reach · join pull-rods to it"),
    it(SluiceGate, "Sluice gate", [116, 86, 58],
       "A timber gate for a channel. Its rods lift it to let water and lumps through; turned round, they shut it.").dens(700.0).place().pick_up()
       .controls("Left mouse: set it in an opening (up to 2 m tall) · join pull-rods to it · right mouse: turn it round · E: take up"),
    it(Scale, "Tipping scale", [168, 140, 88],
       "A pan and a counter-pan of stones. It tips when what's on the pan outweighs the stones, and pulls its rods.").dens(3000.0).place().pick_up()
       .controls("Left mouse: stand it on a floor · right mouse: a stone on (stone in paw) or off its counter-pan · E: take up"),
    it(Clutch, "Clutch", [110, 116, 126],
       "An iron dog clutch for a drive. It joins its drive train only while its pull-rod is pulled.").lump(Mat::Clutch).block(Mat::Clutch).place()
       .controls("Left mouse: build it into a drive line · join a pull-rod to it"),
    ]
};
/// Every item, in order.
pub const ITEMS: [Item; NITEM] = {
    let mut a = [Item::Dirt; NITEM];
    let mut i = 0;
    while i < NITEM { a[i] = ITEM_DEFS_CONST[i].item; i += 1; }
    a
};
// Each row sits at its item's index.
const _: () = { let mut i = 0; while i < NITEM { assert!(ITEM_DEFS_CONST[i].item as usize == i); i += 1; } };
impl Item {
    /// This item's row in the registry.
    #[inline] pub fn def(self) -> &'static ItemDef { &ITEM_DEFS[self as usize] }
    /// Its lump size (m), for a shaker screen's mesh (see `ItemDef::size_m`).
    pub fn size_m(self) -> f32 { self.def().size_m }
    pub fn name(self) -> &'static str { self.def().name }
    pub fn desc(self) -> &'static str { self.def().desc }
    pub fn color(self) -> [u8; 3] { self.def().color }
    /// The material a loose item is a lump of, if it is one: its density and burning come from the table.
    pub fn mat(self) -> Option<Mat> { self.def().mat }
    /// kg/m³ of the item lying loose (decides whether it floats or sinks): its material's, else its own.
    pub fn density(self) -> f32 { match self.def().mat { Some(m) => m.p().density, None => self.def().density } }
    /// Does heat `h` burn it up (a loose item in lava or fire)? TNT doesn't burn: it lights.
    pub fn burns(self, h: u8) -> bool {
        let d = self.def();
        match d.mat { Some(m) => m.lit_by(h), None => d.ignite > 0 && h >= d.ignite }
    }
    /// What the item in hand can do: set it down in the world, throw it, or neither (a crafting
    /// material). Blocks, copperline, devices, torches and tools are never thrown.
    pub fn uses(self) -> Uses { self.def().uses }
    /// One line naming the inputs for the item in hand (hover tooltip and the hint over the hotbar).
    pub fn controls(self) -> &'static str {
        let d = self.def();
        if !d.controls.is_empty() { d.controls } else if d.uses.throw { "Q: throw (hold for further)" } else { "" }
    }
    /// A decorative block's material (built with the block brush).
    pub fn block(self) -> Option<Mat> { self.def().block }
    /// A pigment's paint index (1..6), see `PIGMENTS`.
    pub fn pigment(self) -> Option<u8> { match self.def().pigment { 0 => None, p => Some(p) } }
    /// Furniture: footprint (tiles) and how it's held up.
    pub fn furn(self) -> Option<Furn> { self.def().furn }
    /// A decorative background wall: it panels the back, never the rock in front, and holds no roof.
    pub fn is_wall(self) -> bool { self.def().wall }
}
/// What digging a cell yields (one item per `CELLS_PER_ITEM` cells of that material).
pub const CELLS_PER_ITEM: u16 = 6 * (crate::mat::S * crate::mat::S) as u16; // 6 legacy tiles
pub fn drop_of(m: Mat) -> Option<Item> {
    use Mat::*;
    Some(match m {
        Dirt | Grass => Item::Dirt, Clay => Item::Clay, Sand => Item::Sand, Gravel => Item::Gravel,
        Granite | Rock | Limestone | Basalt | StoneBrick => Item::Stone, Coal => Item::Coal, CopperOre => Item::CopperOre, TinOre => Item::TinOre,
        IronOre => Item::IronOre, Quartz => Item::Quartz, Crystal => Item::Crystal, Roots | WoodBeam | Door | RootTree | TreeRoot => Item::Wood, Fungus | Mycelium => Item::Fungus, Moss => Item::Dirt,
        IronBeam => Item::IronIngot, Glass => Item::Glass, Planks | Chute | Bumper | Riffle | Gear | Wheel | Belt | BeltL | Bellows | Float | SluiceGate => Item::Wood, Polished | Carved => Item::Stone, Tiles => Item::Clay,
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
    // the rope gun (ropes.rs): the spring and stock, and the hook ropes it throws (reusable: `E` takes a rope back)
    Recipe { name: "Rope gun", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Stone, 3), (Item::Fungus, 2)], out: (Item::RopeGun, 1), ticks: 120 },
    Recipe { name: "Hook rope", station: Station::Workbench, inputs: &[(Item::Fungus, 3), (Item::Stone, 1)], out: (Item::HookRope, 1), ticks: 60 },
    Recipe { name: "Rope ladder", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Fungus, 2)], out: (Item::RopeLadder, 1), ticks: 90 },
    // the oil press: a bench presses fungus better than the pack does on the go (3 make 2 measures, where the
    // pack takes 2 a measure), and a mushroom farm's crop can feed the lamp instead of the belly
    Recipe { name: "Fungus oil x2", station: Station::Workbench, inputs: &[(Item::Fungus, 3)], out: (Item::FungusOil, 2), ticks: 60 },
    Recipe { name: "Chute x8", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::Chute, 8), ticks: 60 },
    Recipe { name: "Washer", station: Station::Workbench, inputs: &[(Item::Wood, 6), (Item::Clay, 2)], out: (Item::Washer, 1), ticks: 200 },
    Recipe { name: "Bellows x4", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Fungus, 2)], out: (Item::Bellows, 4), ticks: 120 },
    Recipe { name: "Ingot mould x4", station: Station::Workbench, inputs: &[(Item::Sand, 2), (Item::Clay, 1)], out: (Item::Mould, 4), ticks: 90 },
    // non-electric logic: timber floats and gates, an iron plate, rods and clutch, a brass-pinned scale
    Recipe { name: "Float x2", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::Float, 2), ticks: 60 },
    Recipe { name: "Sluice gate", station: Station::Workbench, inputs: &[(Item::Wood, 4), (Item::Stone, 1)], out: (Item::SluiceGate, 1), ticks: 120 },
    Recipe { name: "Gear x2", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::Gear, 2), ticks: 90 },
    Recipe { name: "Belt x8", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Fungus, 2)], out: (Item::Belt, 8), ticks: 120 },
    Recipe { name: "Water wheel x8", station: Station::Workbench, inputs: &[(Item::Wood, 4)], out: (Item::WaterWheel, 8), ticks: 150 },
    Recipe { name: "Riffle bar x8", station: Station::Workbench, inputs: &[(Item::Wood, 2)], out: (Item::Riffle, 8), ticks: 60 },
    Recipe { name: "Bumper x2", station: Station::Workbench, inputs: &[(Item::Wood, 2), (Item::Stone, 1)], out: (Item::Bumper, 2), ticks: 60 },
    Recipe { name: "Fungus oil (mushroom)", station: Station::Workbench, inputs: &[(Item::Mushroom, 2)], out: (Item::FungusOil, 1), ticks: 60 },
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
    // production: the ore hopper is timber bound with copper; the shaker screen is an iron grating
    // mechanical power: an iron shaft and a counterweight cast in iron (gears, wheels and belts are workbench timber)
    // coal power: an iron drum on a brick firebox, an iron engine, copper steam pipe
    // refinement: an iron-stamped mill, a timber washer tub, a brick smelter, bellows and sand moulds
    Recipe { name: "Stamp mill", station: Station::Forge, inputs: &[(Item::IronIngot, 4), (Item::Wood, 4)], out: (Item::StampMill, 1), ticks: 300 },
    Recipe { name: "Smelter", station: Station::Forge, inputs: &[(Item::Brick, 10), (Item::Clay, 4)], out: (Item::Smelter, 1), ticks: 300 },
    Recipe { name: "Pull-rod x8", station: Station::Forge, inputs: &[(Item::IronIngot, 1)], out: (Item::Rod, 8), ticks: 90 },
    Recipe { name: "Tread plate x2", station: Station::Forge, inputs: &[(Item::IronIngot, 1)], out: (Item::TreadPlate, 2), ticks: 90 },
    Recipe { name: "Clutch x2", station: Station::Forge, inputs: &[(Item::IronIngot, 2)], out: (Item::Clutch, 2), ticks: 150 },
    Recipe { name: "Tipping scale", station: Station::Forge, inputs: &[(Item::IronIngot, 2), (Item::BronzeIngot, 1), (Item::Wood, 2)], out: (Item::Scale, 1), ticks: 200 },
    Recipe { name: "Boiler", station: Station::Forge, inputs: &[(Item::IronIngot, 4), (Item::Brick, 6)], out: (Item::Boiler, 1), ticks: 300 },
    Recipe { name: "Steam engine x4", station: Station::Forge, inputs: &[(Item::IronIngot, 3), (Item::CopperIngot, 1)], out: (Item::Engine, 4), ticks: 240 },
    Recipe { name: "Steam pipe x8", station: Station::Forge, inputs: &[(Item::CopperIngot, 1)], out: (Item::SteamPipe, 8), ticks: 90 },
    Recipe { name: "Iron shaft x8", station: Station::Forge, inputs: &[(Item::IronIngot, 1)], out: (Item::Shaft, 8), ticks: 120 },
    Recipe { name: "Counterweight", station: Station::Forge, inputs: &[(Item::IronIngot, 4), (Item::Wood, 2)], out: (Item::Counterweight, 1), ticks: 200 },
    Recipe { name: "Ore hopper", station: Station::Forge, inputs: &[(Item::Wood, 6), (Item::CopperIngot, 1)], out: (Item::OreHopper, 1), ticks: 200 },
    Recipe { name: "Shaker screen x4", station: Station::Forge, inputs: &[(Item::IronIngot, 1), (Item::Wood, 1)], out: (Item::Screen, 4), ticks: 150 },
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
/// What the ground has to give for `n` of `it`, made from nothing the cheapest way (fewest raw items in
/// all): raw items (those no recipe makes) by index, counting a coal per forge job, as a cold forge burns
/// one (with lava in reach it burns nothing). Stations aren't counted: add `bill_of_station`.
pub fn raw_bill(it: Item, n: u32) -> [u32; NITEM] {
    let mut b = [0u32; NITEM]; add_bill(it, n, &mut b, 0); b
}
/// The raw bill of building a station (its cost, made from nothing).
pub fn bill_of_station(s: Station) -> [u32; NITEM] {
    let mut b = [0u32; NITEM]; for &(it, n) in s.cost() { add_bill(it, n, &mut b, 0); } b
}
fn add_bill(it: Item, n: u32, b: &mut [u32; NITEM], depth: u32) {
    if n == 0 { return; }
    let best = RECIPES.iter().filter(|r| r.out.0 == it && depth < 4).min_by_key(|r| {
        let mut t = [0u32; NITEM]; add_job(r, 1, &mut t, depth); t.iter().sum::<u32>() * 1000 / r.out.1.max(1) });
    match best { Some(r) => add_job(r, n.div_ceil(r.out.1), b, depth), None => b[it as usize] += n }
}
fn add_job(r: &Recipe, jobs: u32, b: &mut [u32; NITEM], depth: u32) {
    for &(i, k) in r.inputs { add_bill(i, k * jobs, b, depth + 1); }
    if r.station == Station::Forge { b[Item::Coal as usize] += jobs; }
}
/// The recipe with this name (scenarios, UI).
pub fn recipe_by_name(n: &str) -> Option<usize> { RECIPES.iter().position(|r| r.name.eq_ignore_ascii_case(n)) }

/// Dig tool tier owned: 0 claws, 1 Pickaxe, 2 Drill, 3 Bore. Higher tiers dig faster and open harder rock.
pub fn tool_tier(inv: &[u32; NITEM]) -> u8 {
    if inv[Item::Bore as usize] > 0 { 3 } else if inv[Item::Drill as usize] > 0 { 2 } else if inv[Item::Pickaxe as usize] > 0 { 1 } else { 0 }
}

#[derive(Clone, Copy)]
/// `Found(m)`: material `m` seen for the first time (`World::discovered`), whether or not it can be dug yet.
/// `Furnished(n)`: n pieces of furniture standing within `HOME_M` of the workbench (`World::home_furniture`).
pub enum Goal { Depth(f32), Have(Item, u32), Built(Station), Chests(u32), Pillars(u32), Found(Mat), Furnished(u32) }
/// How far from the workbench furniture counts as the holt's (metres).
pub const HOME_M: f32 = 12.;
/// `then`: an optional line said when it's done, after the reward (how to use what it gave).
pub struct Quest { pub text: &'static str, pub goal: Goal, pub reward: &'static [(Item, u32)], pub then: &'static str }
pub static QUESTS: &[Quest] = &[
    // The first hour, in order: none is done on arrival in Survival (the kit has a pickaxe and 6 wood, so the
    // first wood quest comes after the workbench spends 4 of it). Each says what it takes, so the HUD line and
    // "Next:" in the log are enough to go on: wood and stone make a bench, copper ore and a forge make copperline.
    Quest { text: "Dig down to 5 m", goal: Goal::Depth(5.), reward: &[(Item::Torch, 6)], then: "" },
    Quest { text: "Build a workbench (4 wood, 4 stone)", goal: Goal::Built(Station::Workbench), reward: &[(Item::Coal, 4)], then: "" },
    Quest { text: "Find wood: dig a root tangle (have 6)", goal: Goal::Have(Item::Wood, 6), reward: &[(Item::Wood, 4)], then: "" },
    Quest { text: "Dig 4 copper ore (green-flecked rock)", goal: Goal::Have(Item::CopperOre, 4), reward: &[(Item::Coal, 2)], then: "" },
    Quest { text: "Build a forge (10 stone, 4 clay)", goal: Goal::Built(Station::Forge), reward: &[(Item::Coal, 6)], then: "" },
    Quest { text: "Smelt 4 copper ingots", goal: Goal::Have(Item::CopperIngot, 4), reward: &[(Item::TinOre, 4)], then: "" },
    Quest { text: "Draw 8 copperline at the forge", goal: Goal::Have(Item::CopperWire, 8), reward: &[(Item::Torch, 4)], then: "" },
    // the burrow as a home: furniture by the bench, then the reward's timber wall to panel the room
    Quest { text: "Make it a holt: set 2 furniture by the workbench (stool, table, nest)", goal: Goal::Furnished(2), reward: &[(Item::TimberWall, 16)], then: "Panel the room: timber wall in hand, hold left mouse to line the back. It holds nothing up; it makes the place yours." },
    Quest { text: "Farm mushrooms: gather 4", goal: Goal::Have(Item::Mushroom, 4), reward: &[(Item::Spore, 4), (Item::Vent, 1)], then: "" },
    Quest { text: "Prop a tunnel: place 8 pillar cells", goal: Goal::Pillars(8), reward: &[(Item::WoodPillar, 8)], then: "" },
    Quest { text: "Reach 15 m", goal: Goal::Depth(15.), reward: &[(Item::Clay, 6)], then: "" },
    Quest { text: "Open a buried chest", goal: Goal::Chests(1), reward: &[(Item::Crystal, 1)], then: "" },
    Quest { text: "Fire 10 bricks", goal: Goal::Have(Item::Brick, 10), reward: &[(Item::Coal, 8)], then: "" },
    Quest { text: "Alloy bronze", goal: Goal::Have(Item::BronzeIngot, 4), reward: &[(Item::Torch, 10)], then: "" },
    // a reason to come back: find iron the pickaxe can't cut (granite country, 30-40 m), make the drill, then
    // go back for it. The discovery line and the refusal both say "come back with a drill".
    Quest { text: "Find iron ore (your pickaxe can't cut it yet)", goal: Goal::Found(Mat::IronOre), reward: &[(Item::Coal, 4)], then: "" },
    Quest { text: "Build a workshop and power it", goal: Goal::Built(Station::Workshop), reward: &[(Item::Quartz, 2)], then: "" },
    Quest { text: "Make a drill", goal: Goal::Have(Item::Drill, 1), reward: &[(Item::Crystal, 2)], then: "" },
    Quest { text: "Go back for the iron: dig 4 iron ore", goal: Goal::Have(Item::IronOre, 4), reward: &[(Item::Coal, 6)], then: "" },
    Quest { text: "Assemble a thrummer", goal: Goal::Have(Item::Thrummer, 1), reward: &[(Item::CopperWire, 16)], then: "" },
    Quest { text: "Reach 80 m - the deep", goal: Goal::Depth(80.), reward: &[(Item::Crystal, 4)], then: "" },
    Quest { text: "Open 4 chests", goal: Goal::Chests(4), reward: &[(Item::IronIngot, 4)], then: "" },
];
