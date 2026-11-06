# Production

A mine's output moves as **real loose items**: every lump is a loose item in the world, in a cart, or in an ore hopper's bin (drawn as its heap). Nothing teleports and nothing is stored out of sight. Lumps get from the face to where they are wanted by falling, sliding and rolling, so a line can jam, spill and pile up. The code is `engine/sim/src/production.rs`, `sluice.rs` and `power.rs`; carts are in [Transport](Transport.md).

| stage | what | state |
|---|---|---|
| 1 | Chutes, shaker screens, bumpers (carts tip), ore hoppers that feed carts | **built** |
| 2 | Water sluicing: running water carries fines off, riffle bars catch heavy lumps, water drains through screens | **built** |
| 3 | Mechanical power: water wheels, shafts, gears, belts that carry and queue real lumps, counterweights | **built** |
| 4 | Coal power: boiler and steam engine; ash and slag to haul away | design |
| 5 | Refinement: stamp mill, washer, bellows smelter, foundry | design |
| 6 | Non-electric logic: plates, tipping scales, float valves, gates | design |

## Chutes

**Chute** (`Mat::Chute`, flag `CHUTE`): a smooth timber trough, laid with the block brush like rail. A loose item lying on a chute slides down its slope: gravity along the slope (g·sinθ) against the chute's low friction (0.12 × g·cosθ). A 1-in-2 chute runs lumps off fast; a level one holds them. The slope is read like a cart reads its track, from the chute's top up to 1 m either side, so a chute built in steps reads as its overall run. The rock a chute is laid on, and the drop past its end, don't count. A chute is a built block and needs support like any other.

Scenario `chute_slides`: a stone slides down a 1-in-2 chute and out past its foot; on bare granite steps of the same slope, or on a level chute, it stays put.

## Shaker screens

**Shaker screen** (`Mat::Screen`, flags `CHUTE | SCREEN`): an iron grating on a timber frame. Anything finer than its mesh (5 cm, `MESH_M`, from `Item::size_m`) falls straight through it: sand, dirt, clay, gravel and pigment powder. Lumps ride over it: ore, coal, stone. The iron grips (friction 0.45), so lumps lie still on a gentle screen. **Copperline current** laid along it (in the same 1 m blocks) works its knocker. While it shakes, the grip drops to 15 % (`SHAKE_GRIP`), and the lumps walk down the slope and off the end. Lumps lying asleep on a screen wake up when it starts shaking.

Scenario `shaker_sorts`: gravel and copper ore dropped on a 1-in-4 screen. The gravel falls into the pit under it; the ore stays on top until a lever powers the line, then walks off into the far pit. Nothing ends up in the wrong pit.

## Bumpers and tipping

**Bumper** (`Mat::Bumper`, flag `BUMPER`): a timber buffer built at the end of a track. A **loaded** cart that runs into one **tips**. It is held against the buffer while its load comes out over the front, one item every 3 ticks, thrown forward at 3 m/s and up at 1 m/s. Then it rights itself, empty and still on the rails, and its running direction flips, so a powered line drives it back. If there is no room for a lump to come out (the buffer stands above the rim, or the spill has piled up), the load **jams** and stays in the cart ("The cart's load is jammed against the buffer."). An empty cart, or a plain track end with no bumper, just stops as before. A loaded cart standing against a bumper tips too: one eased up to it, or one loaded back from a save taken mid-tip (the tip itself isn't saved, so it simply starts again with what is left).

Scenario `cart_tips_at_bumper`: a cart with 20 stones rolls down a 1-in-4 slope into a bumper, tips, and all 20 land in the pit beyond. Scenario `cart_tip_resumes`: saved mid-tip, it finishes after loading.

## Ore hoppers

**Ore hopper** (`Item::OreHopper`): a 1.5 m wide, 1 m tall timber bin on legs. Its spout is 0.75 m over the floor, so a cart fits under it. Set it down over the track: both legs need a floor.
* **Filling:** loose items that fall into its mouth are its load, and so is loose sand and rubble poured in as cells (24 cells make one item, as when digging). It holds up to 60 items.
* **The gate:** a cart standing under the spout with room left trips the gate (items still falling count against that room). The load comes down into the cart one item every 5 ticks. With no cart there, the gate stays shut. **Right mouse** holds the gate open by hand: it then empties onto whatever is under it. Right mouse again lets it go.
* `E` empties its load into the pack, and `E` on an empty hopper picks it up.
* **Footing:** dig away the floor under both legs and it falls, spilling its load as loose items.
* Ore hoppers and their loads are saved (transport save v4) and travel with the window.

Scenario `ore_hopper_feeds_cart`: ten stones and a pour of sand go in. The gate stays shut until it's held open, and two lumps drop onto the track. A cart rolled down under the spout scoops those up and is filled until the hopper is empty. The load survives a save.

## A whole line

Scenario `production_line` runs the line end to end with nothing carried by hand:
1. Gravel and copper ore go into an ore hopper, which fills a cart under its spout.
2. A lever powers copperline on the level stretch, which drives the cart off. It rolls down to a bumper and tips.
3. The load slides down a chute onto a steep shaker screen. The gravel drops through into the pit under it, and the ore slides off its foot into the far pit.

The mole stands by the hopper throughout and picks up nothing, because lumps let down from a hopper or tipped from a cart can't be picked up for their first third of a second.

## Sluicing

Running water sorts what lies in it by how heavy it is for its size (`sluice.rs`). Nothing is moved by hand: the water's own flow from the falling-sand sim pushes loose items and loose grains downstream, and what it can't shift stays.

* **The current.** The CA moves water cell by cell and keeps no velocities. So the current is read from the shape of the water, as in an open channel: water runs down the slope of its own surface. Under water, the top of the water is found up to 1 m either side (the nearest columns that hold water). Its slope `s` gives the speed, 4·√|s| m/s (`WASH_V`), so a 1-in-4 sheet runs about 2 m/s. With the depth `h` above the bed it also gives the stress on the bed, `h`·|s|. A pool, level on top, has no current (test `still_water_has_no_current`). Only a free surface counts: water capped by rock, such as a flooded passage under its roof, isn't a downhill surface (test `water_under_a_roof_is_not_a_current`). The hover tip on running water gives its direction and speed.
* **What moves.** A lump's mobility is its weight in water over its face: (ρ/ρw − 1) × size (`mobility`, with the size from `Item::size_m`). Running water carries a lump when `h`·|s| exceeds that (`WASH_K` = 1), and holds it up off the bed at twice that.

  | lump | mobility (m) | in a sluice |
  |---|---|---|
  | sand, dirt, clay | 0.001 to 0.016 | carried, held up |
  | gravel, coal | about 0.03 | carried, held up |
  | ore and stone | 0.26 to 0.34 | stay |

  Test `fines_wash_and_lumps_stay_by_mobility`. Deep, fast water near a reservoir's mouth can nudge ore a tile; a sluice's thin sheet can't.
* **Loose items** in running water are dragged toward the current's speed. On the bed they press with only their weight less buoyancy, and less again while the flow holds them up. A held-up lump the current drives into something is lifted up and over it, as the flow goes over a riffle bar.
* **Loose grains** on the bed of running water (sand, and a blast's rubble: anything loose) are washed one cell downstream every 4 ticks (`WASH_EVERY`), as often as the current's speed says. They move along the bed or down it, or, when held up, over a lip.
* **Riffle bar** (`Mat::Riffle`, `Item::Riffle`): a low timber bar across a sluice floor. Lumps the water rolls along the bed fetch up behind it, while fines held up in the flow go over the top. Sand washed along as grains also collects behind riffles, so a riffled sluice wants clearing now and then, as a real one does.
* **Water through screens:** a shaker screen's mesh lets water through (`water_through_screen`), so a screen can stand under a sluice or drain a wash.

**Blasting ore frees it.** Ore and coal break into loose lumps of themselves (`crumbles_to` in the material table), in a blast or a cave-in alike, so the rubble a sluice washes still has its ore. Before, they crumbled to gravel and a blast destroyed the ore it broke (scenario `blast_ore_leaves_ore`). Loose ore still needs its tool to dig by hand (a drill for iron), but an ore hopper takes rubble poured into it.

A sluice is a sloping channel, rock or timber rather than a slippery chute, with riffle bars along it and water running down it from a pocket or a breach. Scenarios:
* `sluice_sorts`: a reservoir drains down a stepped 1-in-4 channel with a riffle at the foot of each step. Gravel, coal and sand tipped in at the top are carried into the tailings pond. Copper ore, iron ore and stone settle in the riffle traps, both copper ores behind the first bar.
* `sluice_washes_rubble`: after a blast, the same channel without riffles has heaps of loose rubble on two steps. Every grain of sand and gravel is washed into the pond, and the iron ore stays where it lay.

## Mechanical power

Drive trains of iron and timber carry a stream's push to belts and machines (`power.rs`). Nothing here is current. The lore's water wheels are the source, and a hanging weight stores what they give.

* **Trains.** Machinery cells that touch make one drive train, with one rule. A plain **iron shaft** passes turning only along its own line, left-right or up-down, so turning a corner or branching needs a **gear**. Gears, water wheels, belts and weight drums join on all four sides. Trains are found again every 6 ticks over the active region (test `a_straight_shaft_needs_a_gear_to_turn_a_corner`).
* **Power** is balanced per train, in watts. The train turns at the share of its demand it can meet, at most full; a belt then runs at 1 m/s times that.
  * **Water wheel:** undershot. Its bottom paddles reach a quarter metre down into a stream running under it. The current, read just beside the wheel where the water's surface is free (see [Sluicing](#sluicing)), pushes them with ½·ρ·A·v³·η: about 150 W per paddle cell in a 2 m/s stream.
  * **Belt:** costs 2 W a cell to keep rolling, and 5 W for each lump riding on it.
  * **Shaker screen:** a turning train within a tile of one shakes it, as copperline current does, for 3 W a cell.
  * **Counterweight:** a drum under a roof and a 200 kg iron weight on up to 8 m of rope. When the train is short, the weight falls (m·g·v, up to 0.25 m/s) and makes up the difference. Spare power winds it back up. A metre of fall is about 2 kJ.
* **Belts** (`Mat::Belt`, which runs right, and `Mat::BeltL`, which runs left): a woven fungus-fibre belt over wooden rollers. A belt runs the way the mole faces as it is laid, and right mouse turns a whole run round (test `right_mouse_turns_a_belt_run_round`).
  * A loose item lying on a turning belt is gripped by it and carried at the belt's speed. A still belt holds it like any floor.
  * **Lumps queue:** one can't pass the lump ahead of it on the same belt. So a blocked end backs the line up, one lump to a tile, and a full belt pushes its tail off the back. If there's no room behind, the lump stays wedged: a jam. An open end drops them off.
* Scenarios:
  * `belt_counterweight`: a falling weight drives a belt through a shaft, gears and a corner. Stones ride at 1 m/s and drop off the end. With the weight taken down, the belt is still.
  * `belt_backs_up`: fourteen stones on a 3 m belt that runs into its drive gear. Twelve queue one to a tile, and two are pushed off the back.
  * `water_wheel`: a stream under a wheel gives several hundred watts, shakes a screen and winds a counterweight up. When the reservoir runs dry, the weight falls and carries on.

**Every block item can be built.** The brush now builds any block item from one table (`world::BUILDS`). Chutes, screens, bumpers and riffle bars had been missing from it, so in the game they couldn't be placed at all (test `every_block_item_can_be_built`).

## How they look

Chutes, screens and bumpers are cells with their own patterns (`art::mat_color`). A chute has a pale worn lip and boards along it. A screen is iron bars over a dark mesh under a timber rail. A bumper is red-painted timber with a bright head and iron bands. The ore hopper is drawn: its heap is tinted with what it holds, it shows through the open front of the bin and rises with the load, and the gate under the spout swings open when it lets a lump down. `--scene=production` stamps a small works (chute onto screen, a hopper of copper ore over track, a bumper) for stills. See [Item and Place Art](Item-and-Place-Art.md).

## Hover tips

The chute, screen (shaking or not) and bumper say what moves things along them. The ore hopper's tip gives its load and capacity, and says whether the gate is held open or opens for a cart (test `production_hover_tips_say_what_to_do`).

## Making them

| station | makes | from |
|---|---|---|
| Workbench | 8 chute | 2 wood |
| Workbench | 2 bumper | 2 wood + 1 stone |
| Workbench | 8 riffle bar | 2 wood |
| Workbench | 2 gear | 2 wood |
| Workbench | 8 water wheel | 4 wood |
| Workbench | 8 belt | 2 wood + 2 fungus |
| Forge | 8 iron shaft | 1 iron ingot |
| Forge | counterweight | 4 iron ingots + 2 wood |
| Forge | 4 shaker screen | 1 iron ingot + 1 wood |
| Forge | ore hopper | 6 wood + 1 copper ingot |

Chutes and bumpers come from the workbench (8 chute from 2 wood; 2 bumper from 2 wood and 1 stone), the hopper needs copper but no iron (forge: 6 wood and 1 copper ingot), and the screen needs iron (forge: 4 from 1 iron ingot and 1 wood) (test `production_is_reachable_in_survival`; all recipes are on [Tools and Items](Tools-and-Items.md#recipes)). Digging a chute or bumper gives wood back.
