# Production

A mine's output moves as **real loose items**: every lump is a loose item in the world, in a cart, or in an ore hopper's bin (drawn as its heap). Nothing teleports and nothing is stored out of sight. Lumps get from the face to where they are wanted by falling, sliding and rolling, so a line can jam, spill and pile up. The code is `engine/sim/src/production.rs`, `sluice.rs`, `power.rs`, `steam.rs`, `refine.rs`, `linkage.rs` and `borer.rs`; carts are in [Transport](Transport.md).

| stage | what | state |
|---|---|---|
| 1 | Chutes, shaker screens, bumpers (carts tip), ore hoppers that feed carts | **built** |
| 2 | Water sluicing: running water carries fines off, riffle bars catch heavy lumps, water drains through screens | **built** |
| 3 | Mechanical power: water wheels, shafts, gears, belts that carry and queue real lumps, counterweights | **built** |
| 4 | Coal power: a boiler raises steam from coal and real water; a steam engine drives a train; ash and slag come out to haul away | **built** |
| 5 | Refinement: a stamp mill crushes ore under falling stamps, a washer sorts by weight, a smelter blown by bellows melts it, moulds cast ingots | **built** |
| 6 | Non-electric logic: pull-rods from tread plates, tipping scales and floats to sluice gates, clutches and ore hoppers | **built** |
| 7 | Steam borer: the deep's boring machine, steered like a worm, its wagons following as body segments | **built** |

**Headless only.** Every stage's behaviour, the steam borer included, has been checked only in headless scenarios. None has been playtested in the running game yet.

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

**Ore hopper** (`Item::OreHopper`): a 1.5 m wide, 1 m tall timber bin on legs. Its spout is 0.75 m over the floor, so a cart fits under it. Set it down over the track: both legs need a floor. It can't stand over a tube hopper, nor a tube hopper inside its bin or legs; the refusal names which hopper is there (test `hoppers_do_not_stack`).
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

* **Trains.** Machinery cells that touch make one drive train, with one rule. A plain **iron shaft** passes turning only along its own line, left-right or up-down, so turning a corner or branching needs a **gear**. Gears, water wheels, belts and weight drums join on all four sides. Trains are solved every 6 ticks. Machinery is found by a scan of the active region once a second, and cells join the moment they are built (test `a_straight_shaft_needs_a_gear_to_turn_a_corner`).
* **Power** is balanced per train, in watts. The train turns at the share of its demand it can meet, at most full; a belt then runs at 1 m/s times that.
  * **Water wheel:** undershot. (This is the timber paddle-wheel block that turns a drive train. The copperline device of the same name makes current instead: see [Devices and Copperline](Devices-and-Copperline.md#sources).) Its bottom paddles reach a quarter metre down into a stream running under it. The current, read just beside the wheel where the water's surface is free (see [Sluicing](#sluicing)), pushes them with ½·ρ·A·v³·η: about 150 W per paddle cell in a 2 m/s stream.
  * **Belt:** costs 2 W a cell to keep rolling, and 5 W for each lump riding on it.
  * **Shaker screen:** a turning train within a tile of one shakes it, as copperline current does, for 3 W a cell.
  * **Counterweight:** a drum under a roof and a 200 kg iron weight on up to 8 m of rope. When the train is short, the weight falls (m·g·v, up to 0.25 m/s) and makes up the difference. Spare power winds it back up. A metre of fall is about 2 kJ. Several on one train share the shortfall: each covers only what the ones before it leave. Its floor is whatever lies under it now: build under it and it stops on top, and clear the way and it can go further (`counterweight_stops_on_build`).
* **Belts** (`Mat::Belt`, which runs right, and `Mat::BeltL`, which runs left): a woven fungus-fibre belt over wooden rollers. A belt runs the way the mole faces as it is laid, and right mouse turns a whole run round (test `right_mouse_turns_a_belt_run_round`).
  * A loose item lying on a turning belt is gripped by it and carried at the belt's speed. A still belt holds it like any floor.
  * **Lumps queue:** one can't pass the lump ahead of it on the same belt. So a blocked end backs the line up, one lump to a tile, and a full belt pushes its tail off the back. If there's no room behind, the lump stays wedged: a jam. An open end drops them off.
  * **The mole rides a belt:** standing on a turning belt carries it along like a moving floor, and walking adds to it (`belt_carries_mole`).
* Scenarios:
  * `belt_counterweight`: a falling weight drives a belt through a shaft, gears and a corner. Stones ride at 1 m/s and drop off the end. With the weight taken down, the belt is still.
  * `belt_backs_up`: fourteen stones on a 3 m belt that runs into its drive gear. Twelve queue one to a tile, and two are pushed off the back.
  * `water_wheel`: a stream under a wheel gives several hundred watts, shakes a screen and winds a counterweight up. When the reservoir runs dry, the weight falls and carries on.

**Every block item can be built.** The brush now builds any block item from one table (`world::BUILDS`). Chutes, screens, bumpers and riffle bars had been missing from it, so in the game they couldn't be placed at all (test `every_block_item_can_be_built`).

## Coal power

A boiler burns real lumps, boils real water out of the ground, and its steam drives an engine on a drive train (`steam.rs`). What burns leaves ash and slag as loose items that have to be taken away.

* **Boiler** (`Item::Boiler`): 1.5 m wide and 1.25 m tall, standing on a level floor on the tile grid. Its body is solid shell cells (`Mat::Boiler`), so water stays out of it and an engine can rest on it. Break the shell up (dig or blast it) and the boiler is wrecked, spilling its fuel.
  * **Fuel:** coal, charcoal or wood dropped on its top goes in through the hatch. One lump burns at a time, giving 6 kW of heat: 40 s for coal or charcoal, 15 s for wood. Anything else lies on its top.
  * **Water:** it draws water cells from the ground touching it, one every 6 ticks while its drum has room (32 cells), so it wants to stand by a sump or a channel, and the water level visibly drops. Every 40 kJ of steam boils away one cell; this is a game measure, so a drum lasts a few minutes of hard firing. Dry, the fire only heats iron (`boiler_dry`).
  * **Steam:** heat becomes steam at 70 %, held up to 600 kJ. Past that its safety valve lifts and the excess goes up as steam cells.
  * **Ash and slag:** each lump burnt drops a lump of **ash** out of the ash door. The door is on its right side at the floor, or its left if rock stands against the right. Walled in on both sides, it can't be raked out and the grate is choked (`boiler_ash_door_side`, `boiler_walled_in`). Every third coal also drops a lump of **slag**. Ash is fine (it falls through a screen and running water washes it off); slag is a heavy lump to haul. With 10 of them lying against the door, the grate chokes and the fire is smothered until they're cleared, and the log says so (`boiler_ash_chokes`).
  * A lit firebox stales the air like four torches.
* **Steam engine** (`Mat::Engine`): machinery, joining a train like a gear. Engine cells touching the boiler, or joined to it by **steam pipe** (`Mat::SteamPipe`), take its steam; steam fills a whole engine block. Each engine cell gives up to 25 W. An engine covers what its train is short of after any water wheel, drawing steam at 1/0.15 of what it gives, and puffs exhaust steam from its top where there is open air.
* Scenarios:
  * `boiler_drives_belt`: coal and a sump drive a belt through an engine on the boiler. Ash and slag come out, and the boiler is saved.
  * `boiler_ash_chokes`: a choked ash door holds the fire until the mole picks up the ash.
  * `boiler_dry`: no water, no steam.
  * `steam_pipe_feeds_engine`: steam goes along a pipe to an engine 3.5 m away. Cut the pipe and the belt stops.

## Refinement

Ore is crushed under falling stamps, washed apart by weight, smelted in a fire blown by bellows, and poured into moulds where it sets into ingots (`refine.rs`). Every lump goes in and comes out as a loose item. The power comes off drive trains and the heat from burning real fuel.

The three works stand on a level floor on the tile grid. Their bodies are solid shell cells, like a boiler's, so a broken shell wrecks them. They are fed by dropping lumps on their tops and emptied through a spout at the floor on one side (the other if rock is against it). With no room at either side, a works holds the lump it has done and the log says its spout is walled in (`stamp_mill_walled_in`).

* **Stamp mill** (`Item::StampMill`): machinery that joins the train it touches. A cam lifts two 50 kg iron stamps 0.3 m and drops them on what lies in its mortar, a blow a second at full speed. That is m·g·h ≈ 300 W, which the train pays while it works. Six blows break a lump:
  * copper, tin or iron ore → its **concentrate** plus a lump of **sand** (the rock it was in)
  * stone and slag → gravel
  * anything else stays on its top

  A 200 kg counterweight's 2 m fall crushes about two lumps; a mill wants a water wheel or an engine (`stamp_mill_crushes`).
* **Washer** (`Item::Washer`): a tub of water that a train rocks (80 W). It needs water touching it and uses a cell every 4 lumps. Lumps come out sorted by weight, and each only ever leaves by its own side, so a blocked spout jams rather than mixing:
  * 2500 kg/m³ or more (ore, concentrate, slag) → out of its **left** spout
  * lighter (sand, gravel, coal, ash) → out of its **right**

  (`washer_sorts`)
* **Smelter** (`Item::Smelter`): coal or charcoal dropped in burns a lump at a time, with ore or concentrate.
  * **Heat:** it tends to 800 °C. **Bellows** (`Mat::Bellows`, machinery) touching it raise that up to 1350 °C with full air. They take 10 W a cell from their train while it's lit, and blow in proportion to how fast they turn; 8 turning cells (the whole bellows block touching it counts) is full air.
  * **Melting:** at copper's or tin's 1100 °C, or iron's 1250 °C, a lump melts in 10 s.
  * **Yield:** raw ore gives half an ingot and half a lump of **slag**; washed concentrate gives three quarters and a quarter, so crushing and washing pay. Slag comes out of its spout (`smelter_iron_slag`: raw iron ore waits until the heat passes 1250 °C, then casts an iron ingot and leaves a lump of slag).
* **Ingot mould** (`Mat::Mould`): a tile of mould built against a smelter is filled when it holds an ingot's worth of molten metal. It glows as it sets (`Mat::MouldHot`, a flame's heat) for 15 s, then it is empty again and the ingot lies on top (`smelter_bellows_mould`).

## Non-electric logic

Weight, water and rods, with no current (`linkage.rs`): the mechanical side of the lore's levers and pressure plates, next to copperline (see [Devices and Copperline](Devices-and-Copperline.md)).

A **linkage** is pull-rod cells and the sensors and actuators touching them. When any of its sensors is pressed, the rods are pulled and every actuator on it moves. Several sensors on one linkage act as *either*; a gate turned round acts as a *turn*. Linkages are solved every 6 ticks over the active region.

* **Pull-rod** (`Mat::Rod`): iron rods on guides, laid with the brush. They can run through rock, so a line can go up a wall and along a roof.
* **Sensors:**
  * **Tread plate** (`Mat::TreadPlate`, built into a floor): pressed by 20 kg or more on it. That's the mole (30 kg), four lumps, or a cart, including a cart on rails laid over the plate: a treadle in the track bed (`cart_treadle`).
  * **Tipping scale** (`Item::Scale`): a pan 1 m wide on a floor, with a counter-pan of stones (10 to start, 5 kg each). It tips when what lies on its pan outweighs them. Right mouse with a stone in the paw puts one on; with an empty paw it takes one off, back into the pack. That sets the batch it weighs out.
  * **Float** (`Mat::Float`): lifted, and so pressed, while water touches it.
* **Actuators:**
  * **Sluice gate** (`Item::SluiceGate`): a timber gate set into an opening, filling it top to bottom (up to 2 m). Pulled, it lifts and its cells open, so water and lumps pass. Let go, it drops again, pushing what's in its way aside through the water it's in to the nearest open cell, so a pool rises instead of losing what the gate displaced (`float_valve` keeps every cell). Right mouse turns it round so that it *shuts* when pulled.
  * **Clutch** (`Mat::Clutch`): machinery that joins its drive train only while its rod is pulled.
  * **Ore hopper:** a pulled rod touching or alongside its bin holds its gate open (`ore_hopper_rod`).
* Scenarios:
  * `tread_plate_gate`: the mole on a plate lifts a gate under a reservoir, and stepping off drops it.
  * `float_valve`: a turned-round gate fills a tank until the water reaches a float, and then shuts.
  * `scale_clutch`: a scale weighing out 10 kg tips on the third stone and engages a clutch, and a belt starts.

## Steam borer

The deep's boring machine (`borer.rs`), unlocked late: a steam cab with its own firebox and a round, toothed bore head. It goes through the rock like a worm, in every direction, and its wagons follow behind it as body segments.

* **Unlocking and cost:** in Survival its recipe is listed only once the mole has been down to 80 m (`craft::BORER_DEEP_M`; reaching it says so). It is the workshop's longest job: 20 iron ingots, 10 bronze ingots, 8 brick, and 6 crystal and 4 quartz for its cutters (`steam_borer_opens_in_the_deep`). The last quest is to build one.
* **Fuel:** it runs on coal alone (charcoal and wood burn in it too). Right mouse with fuel in the paw puts a lump from the pack into its firebox. A lump burns at a time, as in a boiler (6 kW, 70 % to steam), and it holds up to 300 kJ of steam. It has no water tank to fill: a save from when it had one loads with the tank's water gone. A coupled coal wagon stokes it. Ash drops out behind the cab.
* **Riding:** right mouse with an empty paw climbs into the cab, and Space climbs out into the tunnel beside it. The camera follows the cab, and the mole isn't drawn while it's inside. The cab (and a wagon the mole sits in) shelters its rider: loose gravel or sand that pours down over the machine, as it does when the head bores up into a pocket of it, doesn't bury the mole inside (`the_borers_cab_shelters_its_rider`; it used to kill a rider in about two seconds, held in the cab with no way to wriggle clear). Water still drowns and bad air still chokes in there, and climbing out into the gravel is being buried as anywhere. Its headlamp shines from the bracket over the cab's nose, along the heading.
* **Light:** while coal burns, the fire behind the firebox door lights the bore round the cab and the first wagon (4.5 m, deep orange, with shadows like any light: [Physics](Physics.md#light)). A borer whose fire is out stands in the dark. Test `a_borers_fire_lights_its_bore_and_its_lamp_looks_ahead`.
* **Steering:** in the cab the walking keys steer. A, D, W and S and any two together (the diagonals) point the way to go. The head turns toward it at up to 4 rad/s, so it swings round in a curve (about 1.25 m round at top speed, tighter going slower) and never snaps round. Held, it gets up to speed in a quarter of a second; let go, it stops at once. W steers in the cab: it doesn't jump (`World::borer_takes_up`).
* **Speed:** up to 5 m/s through soft ground or open tunnel. Through rock it goes as fast as its 5 kW head can grind: a cell costs 0.0012 kJ per unit of the rock's dig time. That makes granite about 2 m/s, sandstone 3.6 m/s, and dirt or clay full speed. Driving costs up to 600 W, plus 100 W for each wagon. Boring granite flat out uses about 5.2 kW against the firebox's 4.2 kW, so its steam store runs down over a few minutes. A lump of coal bores about 60 m of granite (`--example digtempo`). Its head pays for each cut before it moves: with its steam store empty it goes only as far as the steam its firebox raises, about 1.6 m/s in granite, slowing smoothly (`starved_borer_bores_at_what_its_fire_raises`). With the fire out too, it stops and says so. Nothing cuts bedrock: the head stops against it, and it slides along bedrock it only grazes, so it can always turn away.
* **The tunnel:** the head cuts every cell of its round face, 1.5 m across, wherever it goes, so it leaves a smooth round tunnel along its path: straight, round curves, up and down shafts.
* **Spoil:** its head grinds the rock fine and rams the grit into the tunnel wall round it, so no rubble is left behind. What's worth keeping comes off its sieve: coal, ore, quartz and crystal (an item per as many cells as a pickaxe takes) go into the first wagon of the train with room, coal into a coal wagon first. If no wagon has room, they drop behind the cab.
* **Grip:** it holds itself in its tunnel by the walls. With nothing within a cell of its head's or cab's sides, and no wagon standing on anything, it falls, and the train pours down its path after it. Falling, it cuts nothing: it lands on the rock below.
* Scenarios:
  * `borer_bores`: fed coal in a dry chamber and ridden east into granite, it bores at about 2 m/s. It leaves a clean 1.5 m tunnel with its roof and floor whole and no rubble, and it is saved.
  * `borer_steers`: from a chamber into solid granite it is steered east, then down (a curve, not a snap), then on the diagonal, then up (turning back the short way), then west.
  * `borer_needs_steam`: with no coal it raises no steam, doesn't move and says so.
  * `borer_bedrock`: against a bedrock face it stops without cutting any; steered down, it slides off along the bedrock and bores down beside it.
  * `borer_falls`: driven off a ledge over a pit, it falls with the mole in its cab and lands on the pit floor, cutting nothing.

### Wagons

A borer's wagons are its body segments, a mobile base for deep expeditions (`wagon.rs`). Everything in a wagon is really in it, and is hauled with it.

* **Following:** each coupled wagon rides at its place along the head's own path: its middle 2.4 m behind the head for the first wagon, then every 1.25 m. Wherever the head has bored, round curves and down shafts, the wagons wind after it like a worm's body, each lying along the path. They never get stuck: they ride the tunnel the head cut. The train is kept in the borer's list of its segments (`Borer::segs`, by `Wagon::id`).
* **Coupling:** a loose wagon set down, or shoved by hand (walk into a loose one, up to 1.5 m/s), so that its middle comes within 1 m of where the train's next segment would ride couples on with a clank, and is set on the path.
* **E on a wagon** takes its load out into the pack. Then, if it is the last of its train, it uncouples it (a wagon in the middle says to uncouple the ones behind it first). Then, empty with nobody in it, it picks it up. An uncoupled wagon stands loose (it falls if nothing is under it) and won't couple again until the train has moved off from it.
* **Climbing in:** right mouse with an empty paw climbs into a wagon and Space gets out. The borer is steered only from its cab. The camera follows the mole in a wagon as anywhere.
* **Kinds** (each 1 m long, 0.75 m tall). There is no water wagon (the borer needs no water): one in a save from before (v12 or older) loads as a store wagon, its water gone, and one in the pack, a cargo or on the floor as a store wagon item (`old_water_wagon_save_loads`).
  * Store wagon: up to 200 items. Loose things that fall in ride along, and so does what the borer's sieve keeps. Inside it, right mouse stows the whole stack in paw.
  * Coal wagon: up to 40 lumps of coal, charcoal or wood, thrown in or shovelled in (fuel in paw, right mouse); the coal the borer cuts goes in first. Coupled, its stoking chute feeds the borer's firebox a lump at a time, but only while the borer's steam is below half full. With steam up, a catch holds the chute shut, so a parked borer doesn't burn the bunker away (`wagon_stoker_waits`).
  * Bunk wagon: climbing in is going to sleep. You wake there after a death, and the game saves.
  * Workshop wagon: inside it you work as at a workbench.
* **Old saves** (transport blob 13 or older, when the borer was a cab on wheels boring east or west): the borer loads heading the way it faced, with its head at the old cab's front and a straight path behind it. The wagons that were coupled behind it are its segments, in order and with their loads (`old_borer_train_loads_as_a_worm`, from a real v13 save, `data/v13_borer_train.bin`).
* Scenarios:
  * `wagon_train`: a store wagon and a coal wagon couple on as segments and the coal wagon stokes the borer. The iron ore it bores through goes into the store wagon, and no rubble is left. Turned down, the wagons follow round the bend and down the shaft, and the train is saved coupled.
  * `wagon_couple`: set down where the segments ride, two wagons couple on. E on the middle one is refused; E on the last uncouples it, and it doesn't hook straight back on. Set down again at the train's new tail, it couples again.
  * `wagon_stoker_waits`: under a parked borer, the coal wagon gives up one lump and then holds its chute shut once steam is up.
  * `wagon_kinds`: the store wagon catches what falls in and stows the paw's stack. The workshop wagon is a workbench, and the bunk wagon moves the wake point.

## How they look

Chutes, screens and bumpers are cells with their own patterns (`art::mat_color`). A chute has a pale worn lip and boards along it. A screen is iron bars over a dark mesh under a timber rail. A bumper is red-painted timber with a bright head and iron bands. The ore hopper is drawn: its heap is tinted with what it holds, it shows through the open front of the bin and rises with the load, and the gate under the spout swings open when it lets a lump down. `--scene=production` stamps a small works (chute onto screen, a hopper of copper ore over track, a bumper) for stills. See [Item and Place Art](Item-and-Place-Art.md).

Machinery moves with its drive train. Each train keeps how far it has run (cells of belt travel, advanced by its speed every frame), and the cells draw from that. An iron shaft's twist band slides along it, a gear's teeth turn round its hub, and a water wheel's spokes and paddles turn. A belt's stitched chevrons point the way it carries and slide that way, and the weight drum's windings creep. A still train shows them at rest. The counterweight hangs as a cast-iron weight on a fibre rope that pays out as it falls. `--scene=power` stamps a counterweight driving a shaft, gears and a belt, with a water wheel standing by. `--dump-art` also writes each production and machinery cell's art as an unlit swatch, at rest and 1.5 cells in (`cells-<Mat>-0/1.bmp`).

A boiler is drawn over its shell cells: a riveted drum with a steam dome and chimney over a brick firebox. While it burns, the fire shows through the door, a warm glow flickers round it and smoke puffs from the chimney. A steam engine's crank pin goes round with its train and the connecting rod swings to it. Steam pipes are copper with brass flanges. `--scene=steam` stamps a lit boiler piping steam to an engine that drives a belt.

The refinement works are drawn over their shell cells. A stamp mill's two stamps rise slowly on the cam and drop fast, half a stroke apart. A washer's tub rocks while its train turns, with water in it while it has some. A smelter's fire mouth goes from dark through dull red and orange to white-hot with its heat, and smoke rises while it burns. Bellows' pleats squeeze and open with their train, and a setting mould's hollow glows. `--scene=refine` stamps a counterweight driving a stamp mill, a washer and bellows on a lit smelter, with an empty and a setting mould.

Linkage parts show their state. A pulled rod's brass collar slides past its guide, and a pressed tread plate sinks with a dark gap along its top. A lifted float's highlight bobs, and a clutch's teeth mesh and turn when it's in, with a dark gap between when it's out. A lifted sluice gate's leaf is drawn raised out of the opening, and the lever on its head points down when it's turned round. A tipping scale has a post, a beam that tips and a counter-pan showing its stones. `--scene=logic` stamps every part: a tread plate in the burrow floor under the mole, rodded to a gate holding a pool (the mole's weight lifts it in the running game), a turned-round gate standing lifted, a scale with three stones, and a float in a tank rodded to a clutch in a drive train.

## Hover tips

The chute, screen (shaking or not) and bumper say what moves things along them. The ore hopper's tip gives its load and capacity, and says whether the gate is held open or opens for a cart (test `production_hover_tips_say_what_to_do`).

## Making them

| station | makes | from |
|---|---|---|
| Workbench | 8 chute | 2 wood |
| Workbench | 2 bumper | 2 wood + 1 stone |
| Workbench | 8 riffle bar | 2 wood |
| Workbench | washer | 6 wood + 2 clay |
| Workbench | 4 bellows | 2 wood + 2 fungus |
| Workbench | 4 ingot mould | 2 sand + 1 clay |
| Workbench | 2 float | 2 wood |
| Workbench | store wagon | 6 wood + 2 iron ingot |
| Workbench | workshop wagon | 8 wood + 4 stone + 2 iron ingot |
| Workbench | bunk wagon | 6 wood + 4 fungus + 2 iron ingot |
| Forge | coal wagon | 4 iron ingot + 2 wood |
| Workbench | sluice gate | 4 wood + 1 stone |
| Workbench | 2 gear | 2 wood |
| Workbench | 8 water wheel | 4 wood |
| Workbench | 8 belt | 2 wood + 2 fungus |
| Forge | 8 iron shaft | 1 iron ingot |
| Forge | boiler | 4 iron ingots + 6 brick |
| Forge | stamp mill | 4 iron ingots + 4 wood |
| Forge | smelter | 10 brick + 4 clay |
| Forge | 8 pull-rod | 1 iron ingot |
| Forge | 2 tread plate | 1 iron ingot |
| Forge | 2 clutch | 2 iron ingots |
| Forge | tipping scale | 2 iron ingots + 1 bronze ingot + 2 wood |
| Workshop | steam borer (only once you've been down to 80 m) | 20 iron ingots + 10 bronze ingots + 8 brick + 6 crystal + 4 quartz |
| Forge | 4 steam engine | 3 iron ingots + 1 copper ingot |
| Forge | 8 steam pipe | 1 copper ingot |
| Forge | counterweight | 4 iron ingots + 2 wood |
| Forge | 4 shaker screen | 1 iron ingot + 1 wood |
| Forge | ore hopper | 6 wood + 1 copper ingot |

Chutes and bumpers come from the workbench (8 chute from 2 wood; 2 bumper from 2 wood and 1 stone), the hopper needs copper but no iron (forge: 6 wood and 1 copper ingot), and the screen needs iron (forge: 4 from 1 iron ingot and 1 wood) (test `production_is_reachable_in_survival`; all recipes are on [Tools and Items](Tools-and-Items.md#recipes)). Digging a chute or bumper gives wood back.
