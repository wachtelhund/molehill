# Tools and Items

> Code: `engine/sim/src/craft.rs` (items, stations, recipes, quests), `brush_range` and `place_brush` in `world.rs`, the hotbar and pack in `engine/game/src/main.rs`, `ui.rs` and `hotbar.rs` (Survival's hotbar order).

## Dig tools

There are three hand tools on the hotbar. Each tier takes a bigger bite, digs faster and cuts harder rock:

| tool | tier | default bite (radius, tiles of 0.25 m) | brush range | cuts |
|---|---|---|---|---|
| Pickaxe | 1 | 2.5 | 0.5–3 | dirt and sand almost instantly, soft stone (limestone, soft rock, copper and tin ore, brick) in about 0.3 s |
| Drill | 2 | 3.2 | 0.5–4 | adds granite, basalt, iron ore, quartz and crystal. A copperline-driven auger |
| Bore | 3 | 4.2 | 1–5.5 | everything but bedrock, fastest. A heavy heat-bore with the widest bite |

The colony also knows **pack tools** (the Pickaxe, the bronze Drill and the iron Bore, see [Tool tiers](#tool-tiers)), crafted at stations, which set the tier of bare-handed digging (claws are tier 0). Bedrock can't be dug at all.

## The round brush

Dig tools and building blocks share one idea: a **round brush** centred under the cursor, shown as a ring.

* `[` / `]` or `Alt` / `Option` + wheel resize it within the item's range, one step (an eighth of a metre of radius) at a time. Each dig tool and block keeps its own size (`view::BrushSizes`). The size is written beside the outline and over the hotbar, in metres across (scenario `brush_size_keys`).
* **Grid snap** (`X`, blocks only): the brush becomes a square of 0.25, 0.5 or 1 m on a fixed grid (the size keys step between them), drawn as a crisp square outline. Holding left mouse and dragging lays a straight row or column of squares from where the drag started, along the axis the drag mostly follows; the outline shows the whole line. The usual rules hold: open cells in reach, never the mole, and each square grows from something solid (scenario `place_grid_snap`).
* Radii here are in tiles (0.25 m = 2 cells). Digging carves a disc of cells; **blocks go down in whole tiles** on the global tile grid, so a radius of 0.5 places one tile (2 × 2 cells) and 1.0 a plus of five tiles.
* **Placing grows from support.** Each new cell of a block brush must touch something solid, so a stroke grows out of what it rests on and never floats.

| block | material | brush range | default |
|---|---|---|---|
| Wood pillar | wood beam | 0.5–3 | one cell |
| Brick | stone brick | 0.5–3.5 | a small plus |
| Iron beam | iron beam | 0.5–2 | one cell |

Placed blocks are ordinary cells, judged by the same structural solver as the rock (see [Physics](Physics.md#building-that-holds)). A wood beam that fails breaks into **wood splinters** that float on water and dig back into wood. A failed iron beam becomes loose iron.

## Other things in hand

| item | left mouse |
|---|---|
| Torch | Placed on a wall or floor. Lights a few metres around it and slowly stales the air |
| Copperline | Lays wire, one per 1 m block, as you drag |
| Device (lever, knot, lamp, vent…) | Placed in the 1 m block under the cursor. See [Devices and Copperline](Devices-and-Copperline.md) |
| TNT | Set down **inert** (left mouse, or right mouse on an empty spot); light it with right mouse (1.5 s fuse) or set it off with current, or pick it back up with `E` while unlit. Not set down on or beside a live copperline (cut the power first). Thrown with `Q`, it goes out lit |
| Spore | Planted on soil to start a mushroom plot |
| Station | Placed as a 2.5 × 1.75 m building (10 × 7 tiles) |
| Glowmite | A glowmite in a jar: held in the paw it lights the way like a small cold lamp, with no fuel |

Food (grubs, mushrooms, fungus) is eaten from the pack automatically when the mole gets hungry.

**Right mouse** on a glowmite drifting in a grotto catches it in a jar (see [World and Generation](World-and-Generation.md#grottos)); otherwise right mouse uses the device under the cursor.

## The pack

The pack (`I`) is a grid ten slots wide with a slot for everything there is (every item, device, station and dig tool, so a newly added item always has one: `NSLOT`), and the bottom row is the hotbar (`1`–`0`). Creative holds 999 of every item, the steam borer and wagons included (tests `creative_has_999_of_every_item`, `the_pack_holds_everything_and_creative_has_999`). Click a slot to pick its item up, then click another to drop or swap. Counts come from one shared inventory, and every item has its own hand-drawn icon (see [Item and Place Art](Item-and-Place-Art.md)). In Survival the hotbar puts what you hold first, then free things (stations to plan, devices that cost nothing); a slot for something you have none of is drawn faint, and a newly gained thing moves into the first faint slot (details on [Player and Controls](Player-and-Controls.md)). Hovering a slot shows its name, count and a one-line description. Resting the cursor on a seen cell for about 0.35 s explains what's there (`sim/src/tips.rs`): the material (and whether it's loose debris), a device, copperline (carrying current or idle), a station (what it makes; craft by standing at it with the pack open), a chest (opened or not) or a mushroom plot (growth % and whether it's too bright or too dry). A standing block also says how hard it is working, read from the stress solver: bearing weight, straining, overloaded, or cracking, so you can see a span is about to fail before it does.

> **Modes:** New Holt asks for **Survival** (a small kit: a Pickaxe, 6 wood, 4 torches, 3 grubs, 2 roots, 2 fungus oil; you dig only with tools you own; the headlamp and torches burn fungus oil) or **Creative** (999 of every item). Crafting works at stations (below). **Fungus oil** is pressed from fungus (2 a measure, done by the pack when it runs out); a **root** is food from root tangles. See [Progression](Progression.md#survival-mode-today). The sim's own starting pack (used by the tests) is small: 8 wood, 6 torches, 12 pillars, 12 grubs, 40 copperline, 6 knots, 3 thrummers, 6 spores and a vent.

## Digging drops

Digging drops items: one per 6 tiles dug, or one per 3 for gems. **Wood** comes from grottos: root-tree trunks and their roots (a trunk cut down but left with its roots grows back), and from tangled roots in the upper soil. Mycelium gives fungus. Loose items are small physics bodies (`items.rs`): they pop out of the bite, fall, bounce and skid on what they land on (stone is lively, dirt swallows a bounce), float or sink by density (wood floats, ingots sink, a grub sinks slowly), burn up in lava or fire if they burn, and fall asleep once still. Walking past picks them up.

## What each item does

One row per item in the item registry (`ITEM_DEFS` in `craft.rs`, read through `Item::uses`) says whether an item is **set down**, **thrown**, or **picked back up**, and `Item::controls` turns it into the line of inputs shown in the slot tooltip and under the item name over the hotbar.

| Item | Set down | Throw (`Q`) | Pick up (`E`) |
|---|---|---|---|
| TNT | left/right mouse, unlit | yes, lit | while unlit |
| Wood pillar, brick, iron beam | left mouse (brush) | no | no (dig it) |
| Rock / timber / masonry / beam / pipe / carved wall | left mouse panels the back (hold; `X` snaps a line); right mouse scrapes | no | no (scrape it) |
| Copperline | left mouse (drag) | no | one segment, sweep for a run |
| Torch | left mouse | no | yes |
| Rail | left mouse (brush; `X` snaps a straight run) | no | no (dig it) |
| Mine cart | left mouse, on the track (right mouse: get in) | no | its cargo, then the empty cart |
| Rope ladder | left mouse in a drop: it hangs from the rock above or a ledge's lip | no | yes |
| Rope gun | left mouse: throws a hook rope's hook at the cursor; it bites solid rock or a block and the rope stays (`W` grab, `W`/`S` climb, `A`/`D` swing, `Space` let go; see [Transport](Transport.md#rope-gun)) | no | it's kept in the pack; `E` on a rope takes the rope back |
| Hook rope | the rope gun's rope: one per throw (16 m); `E` on a fixed rope takes it back | yes | a missed one lies loose |
| Pressure tube | left mouse (brush; `X` snaps a straight run) | no | no (dig it) |
| Chute, shaker screen, bumper, riffle bar | left mouse (brush) | no | no (dig it; a chute, bumper or riffle bar gives wood back) |
| Iron shaft, gear, water wheel, belt | left mouse (brush; `X` snaps a shaft's straight run); a belt runs the way you face as it is laid, and right mouse turns a run round | no | no (dig it; a gear, wheel or belt gives wood back) |
| Counterweight | left mouse: hang the drum under a roof over a drop, joined to the drive | no | yes (`E` takes it down) |
| Boiler | left mouse: stand it in a sump or by water; drop coal, charcoal or wood on it to feed it | no | `E` rakes out its fuel, then picks it up |
| Steam engine, steam pipe | left mouse (brush; `X` snaps a pipe's straight run): an engine against a boiler or at a pipe's end, joined to a shaft, gear or belt | no | no (dig it) |
| Ash, slag | what a boiler leaves at its ash door: haul it off (ash is light, so water washes it away and a screen lets it through) | yes | yes |
| Stamp mill, washer, smelter | left mouse: stand it on a level floor (a stamp mill or washer against a turning gear or shaft, a washer by water); drop lumps on its top to feed it | no | `E` empties it, then picks it up |
| Bellows, ingot mould | left mouse (brush): build against a smelter (bellows joined to a turning shaft or gear) | no | no (dig it; bellows give wood back) |
| Copper, tin or iron concentrate | what a stamp mill makes from ore: wash it, then smelt it for more metal than raw ore gives | yes | yes |
| Pull-rod, tread plate, float, clutch | left mouse (brush; `X` snaps a rod line): rods run from a plate, scale or float to a gate, clutch or ore hopper; a tread plate goes into a floor, a float where the water should reach, a clutch into a drive line | no | no (dig it) |
| Sluice gate | left mouse in an opening up to 2 m tall, joined to pull-rods; right mouse turns it round (pulled shuts instead of opens) | no | yes (`E` takes it up) |
| Tipping scale | left mouse on a floor; right mouse puts a stone on its counter-pan (stone in paw) or takes one off | no | yes (`E` takes it up) |
| Steam borer | left mouse: set it down heading your way; right mouse with coal, charcoal or wood in paw feeds it, with an empty paw climbs into its cab; in the cab `A`/`D`/`W`/`S` (and diagonals) steer it through the rock, `Space` climbs out | no | `E` rakes out its fuel, then picks it up |
| Store, workshop, bunk or coal wagon | left mouse: set it down (at a borer train's tail it couples on); right mouse climbs in or out; walk into a loose one to shove it; see [Production](Production.md#wagons) for what each does | no | `E` unloads it, then uncouples it (the last of a train), then picks it up |
| Ore hopper | left mouse over the track: both legs need a floor | no | `E` empties its load into the pack, then picks up the empty hopper |
| Tube hopper | left mouse on a tube (drop items in to send; right mouse: ride, or pick where it sends) | no | yes |
| Lift | left mouse in a shaft: snaps to the shaft (a ghost shows the cage, its stops and the counterweight); `W` / `S` ride or call it; right mouse on head or cage balances it with stone | no | yes, with its ballast |
| Lamp, vent; levers, knots, thrummers and other devices | left mouse, or right mouse on an empty spot | no | yes |
| Spore | left mouse (plant on soil) | yes | no |
| Loose materials, ingots, charcoal, grubs, mushrooms, roots, fungus oil | no | yes | (lying loose: walk over it) |
| Tools, glass | no | no | no |

## Furnishing

Making the holt look like home: decorative blocks, paint and furniture. In Survival they're crafted (all at the workbench except tiles, fired at the forge) and unlock when you first hold their materials. In Creative they're in the pack.

**Decorative blocks** are built with the round block brush (`[`/`]` resize, grid snap works too). They are structural, by their own numbers (see [Physics](Physics.md#materials)):

| block | recipe | look | strength |
|---|---|---|---|
| Planks | 2 wood → 8 | boards with seams and staggered joints | light, like a wood prop (6000 kPa) |
| Polished stone | 4 stone + 1 sand → 4 | smooth slabs, fine joints, a faint sheen | as strong as brick |
| Tiles | 2 clay + 1 coal → 6 (forge) | glazed squares with pale grout | brittle under a roof |
| Carved pillar | 4 stone → 2 | a fluted column | the strongest stone prop (22000 kPa) |

**Pigments** are ground at the workbench: ochre (iron ore + clay), rust (iron ore), chalk (quartz), verdigris (copper ore), woad (fungus), soot (charcoal); each makes 4. With a pigment in hand, left mouse paints a brush of standing blocks (any solid block: brick, planks, even raw rock). The pigment takes over most of the colour and the block keeps its pattern and shading. One pigment covers 16 tiles. Painting a rug, banner or nest dyes its cloth. Paint goes with the block (a painted block that falls is still painted) and is kept in saves.

**Furniture** is placed with left mouse and taken back with `E`. A pale ghost shows where it will go. It isn't solid, so the mole walks past it.

| piece | size | stands | notes |
|---|---|---|---|
| Table | 2 × 1 m | on the floor | with a cup and a bowl |
| Stool | 0.75 m | on the floor | |
| Nest | 2.25 × 0.75 m | on the floor | a basket bed; its blanket takes dye |
| Shelf | 1.5 × 2 m | on the floor | pots, jars, a crystal, a tablet |
| Rug | 2.5 m | on the floor | lies under other furniture; takes dye |
| Banner | 0.75 × 1.5 m | hangs from the ceiling | takes dye |
| Fungus planter | 1.25 m | on the floor | glows soft green (a ray-cast light) |

Furniture finds the floor below the cursor, or the ceiling above it for a banner. It needs two thirds of its width supported and room for its whole footprint, and pieces can't overlap (except a rug). If its floor or ceiling is dug or blasted away, it falls loose as its item.

### Back walls

A **background wall** panels the back of a room. It is not a block: you walk in front of it, it never collides, and it holds no roof (see [World and Generation](World-and-Generation.md#background-walls)). The six pack items are crafted; scraping returns the nearest made wall.

| wall | station | from | look |
|---|---|---|---|
| Rock wall ×8 | workbench | 2 stone | carved host rock |
| Timber wall ×8 | workbench | 2 wood | panelling, board seams |
| Carved wall ×4 | workbench | 2 stone | fluting and marks |
| Beam wall ×4 | workbench | 1 wood pillar | decorative beams (no weight) |
| Masonry wall ×8 | forge | 2 brick | brick courses |
| Pipe wall ×4 | forge | 1 copper ingot | old copper pipes (they carry nothing) |

With a wall item in hand, left mouse panels (the same round brush as a plank; `X` snaps a line). Right mouse scrapes lining. Neither edits the rock in front. Digging a solid cell that has no lining leaves one that matches what you cut. Shift is sprint, never a wall modifier. Creative has 999 of each; Survival crafts them. Saves from before format 10 infer lining on load.

## Relics

Relics of the Ancients can't be made: they're found in sealed vaults in the ruins, each once. The **Ancient bit** makes every bite a quarter faster while it's in the pack; the **Ground lens** halves the headlamp's oil. See [World and Generation](World-and-Generation.md#ruins-of-the-ancients).

## Throwing

Hold `Q` to charge a throw of a throwable item in hand and release to throw it at the cursor; 0.8 s of charge is a full throw, which carries about 6 m on the level, and dots show the arc. You can't catch your own throw for half a second. A thrown **TNT** charge goes out lit: it flies, bounces and rolls, and goes off where its 1.5 s fuse runs out, with the same blast as placed TNT. Water puts it out (it lies there as a dud you can pick up again), and lava or fire lights one lying loose. A lit charge can't be picked up. Blocks, copperline, devices, torches and tools aren't thrown: `Q` says so and keeps them. Blasts throw loose items around. In the colony design, moles haul drops to the **stockpile**, and only stockpiled items count. The first sighting of each ore, fluid or rock is a **discovery** with a small reward.

## Modes and crafting

In **Survival** (see [Progression](Progression.md#survival-mode-today)) crafting is earned: recipes appear as you pick up their ingredients and you make things at a working station. In **Creative** every recipe is listed and can be made anywhere. The sim's default (Plain, used by tests and scenarios) crafts freely too. This is `Rules::crafting` in `survival.rs`. Saves keep what you have found (format 6); older saves count what is in the pack as found.

## Tool tiers

| tier | tool | made at | from | digs |
|---|---|---|---|---|
| 0 | claws | (always) | | dirt, clay, sand, gravel; soft stone (sandstone, limestone, coal, copper and tin ore) slowly |
| 1 | Pickaxe | workbench (Survival starts with one) | 2 wood + 4 stone | the same, faster |
| 2 | Drill | workshop | 4 bronze ingots | granite and iron ore |
| 3 | Bore | forge | 6 iron ingots + 2 copper ingots + 2 crystal | basalt |

In Survival a dig tool you don't own sits faint in the hotbar (see [The pack](#the-pack)), and holding a better tool's slot doesn't help: digging uses the best tool in the pack (`World::tool`, scenario `tool_tiers_survival`). Without any tool, claws still take soft stone, slowly.

**Dig tempo.** A bite takes the slowest cell in it: `dig × 4 / (1 + tier) + 1` ticks, at least 4 (`World::mine_time`; `dig` is the material table's), and takes the whole round bite at once. The time doesn't depend on the bite's size: a bigger brush clears more in the same time (a design decision), so widening it is how to clear a room faster. Soft soil goes at once, and stone is work, so a dug room is earned. Clearing a 4 × 3 m room with the default bite (`examples/digtempo.rs` prints every material):

| material | claws | Pickaxe | Drill | Bore |
|---|---|---|---|---|
| dirt, gravel, roots | 2.8 s | 0.8 s | 0.4 s | 0.3 s |
| clay | 5.4 s | 1.5 s | 0.6 s | 0.3 s |
| coal | 9.2 s | 2.5 s | 1.0 s | 0.5 s |
| copper or tin ore, sandstone | 15.5 s | 4.2 s | 1.7 s | 0.9 s |
| limestone | 18.0 s | 4.8 s | 1.9 s | 1.0 s |
| iron ore | – | – | 2.3 s | 1.2 s |
| granite | – | – | 3.0 s | 1.5 s |
| basalt | – | – | – | 1.3 s |

Each better tool clears about twice the room a second of the one before, or more, and the kit's pickaxe bites any rock it can cut in half a second or less, with all the rock within 25 m of a new burrow its own (test `dig_tempo_is_earned_but_the_first_hour_is_open`).

The [steam borer](Production.md#steam-borer) cuts by the same `dig` numbers: each cell of its round face costs its head a fixed amount of steam per unit of dig, so it goes as fast as its cutting power grinds (granite about 2 m/s), and in soft ground at its top speed (5 m/s). The `digtempo` example's last columns print, from the borer's own constants, its speed through each rock and how many metres of its 1.5 m tunnel a lump of coal's steam bores.

## Stations

Stations, not menus: you craft standing at one you have built. Open the pack (`I`) there and its recipes are listed down the left; click a bright one to make it. The list shows 30 rows at a time: with more (Creative lists every station's recipes), its first or last line says how many more there are, and the wheel over the list scrolls it, three rows a notch (test `every_recipe_can_be_scrolled_to`). It starts at the top each time the pack opens. A dim recipe is missing something.

| station | cost | works when | makes |
|---|---|---|---|
| Workbench | 4 wood + 4 stone | always | the pickaxe, wood pillars, torches, vents, TNT, simple furniture |
| Forge | 10 stone + 4 clay | **hot**: lava or fire within 3 m of it, or a heatstone fed by lava within 3 m; otherwise it burns one coal (or charcoal) from the pack per job (scenario `forge_coal_fuel`) | brick, glass, charcoal, ingots, bronze, copperline, iron beams, lamps, the bore |
| Workshop | 2 wood + 6 stone + 6 copperline | **powered**: a live copperline in or touching its footprint (a thrown lever on the line will do) | the drill, knots, thrummers |

A cold forge or an unpowered workshop says why in the list (scenario `craft_stations`). Saves from before list the old stations: a kiln or furnace becomes a forge, an anvil a workshop.

## Recipes unlock by touch

A recipe appears the first time **any** of its inputs reaches the pack: picked up off the floor, dug, crafted, found in a chest or given as a reward. The log says what opened ("New recipes: Pickaxe, Wood pillar x4."). There is no tech-tree screen: the world is the tree (scenario `craft_unlock_by_touch`).

## Recipes

Chains are short: nothing is more than two steps from the ground (ore → ingot → tool), checked by the test `recipes_are_short_chains`. The balance follows a survey of what lies around the burrow across seeds (`survey` example): wood is scarce near spawn, so the kit carries it; lava is 20–34 m away, so a forge can burn coal until then; copper and tin come before iron, so the drill is bronze, their alloy (test `bronze_makes_the_drill`). Bronze can be co-smelted straight from the two ores, so the bronze drill stays two steps from the ground. The test `survival_ladder_is_reachable` checks the whole tool ladder can be made from the kit and the ground. `craft::raw_bill` works out what the ground has to give for anything (the cheapest recipes, a coal per cold-forge job): the drill with its three stations is 6 wood (the kit's), 20 stone, 4 clay, 8 copper ore, 2 tin ore and 4 coal (test `the_drill_costs_a_first_hour_haul`); `examples/progression.rs` prints the whole ladder and when a digging bot's pack could first have paid each rung.

| station | makes | from |
|---|---|---|
| Workbench | Pickaxe | 2 wood + 4 stone |
| | 4 wood pillars | 2 wood |
| | 4 torches | 1 wood + 1 coal (or charcoal) |
| | vent | 2 wood + 2 stone |
| | TNT | 2 coal + 2 sand |
| | rope ladder | 2 wood + 2 fungus |
| | rope gun | 2 wood + 3 stone + 2 fungus |
| | hook rope (reusable: `E` takes a thrown rope back) | 3 fungus + 1 stone |
| | 8 chute | 2 wood |
| | 2 bumper | 2 wood + 1 stone |
| | 8 riffle bar | 2 wood |
| | store wagon / bunk wagon | 6 wood + 2 iron ingots / 6 wood + 4 fungus + 2 iron ingots |
| | workshop wagon | 8 wood + 4 stone + 2 iron ingots |
| | 2 gear / 8 water wheel | 2 wood / 4 wood |
| | 8 belt | 2 wood + 2 fungus |
| | washer | 6 wood + 2 clay |
| | 4 bellows | 2 wood + 2 fungus |
| | 4 ingot mould | 2 sand + 1 clay |
| | 2 float / sluice gate | 2 wood / 4 wood + 1 stone |
| | 2 fungus oil / 1 fungus oil | 3 fungus / 2 mushrooms (the oil press: better than the pack's 2 fungus a measure) |
| | 8 rock wall / 8 timber wall | 2 stone / 2 wood |
| | 4 carved wall | 2 stone |
| | 4 beam wall | 1 wood pillar |
| Forge | 4 brick | 2 clay |
| | 2 glass | 2 sand |
| | 2 charcoal | 2 wood |
| | copper ingot / tin ingot | 2 ore |
| | iron ingot | 2 iron ore + 1 coal |
| | 4 bronze | 3 copper + 1 tin ingot |
| | 2 bronze | 3 copper ore + 1 tin ore (co-smelted) |
| | 8 copperline | 1 copper ingot |
| | 4 iron beams | 1 iron ingot |
| | 8 pressure tube | 1 iron ingot + 2 glass |
| | 8 rail | 1 copper ingot + 1 wood |
| | mine cart | 2 copper ingots + 4 wood |
| | ore hopper | 6 wood + 1 copper ingot |
| | 4 shaker screen | 1 iron ingot + 1 wood |
| | 8 iron shaft | 1 iron ingot |
| | counterweight | 4 iron ingots + 2 wood |
| | boiler | 4 iron ingots + 6 brick |
| | 4 steam engine | 3 iron ingots + 1 copper ingot |
| | 8 steam pipe | 1 copper ingot |
| | coal wagon | 4 iron ingots + 2 wood |
| | stamp mill | 4 iron ingots + 4 wood |
| | smelter | 10 brick + 4 clay |
| | 8 pull-rod / 2 tread plate | 1 iron ingot / 1 iron ingot |
| | 2 clutch | 2 iron ingots |
| | tipping scale | 2 iron ingots + 1 bronze ingot + 2 wood |
| | lamp | 1 glass + 2 fungus |
| | 8 masonry wall | 2 brick |
| | 4 pipe wall | 1 copper ingot |
| | Bore | 6 iron ingots + 2 copper ingots + 2 crystal |
| Workshop | Drill | 4 bronze ingots |
| | knot | 1 copper ingot + 1 quartz |
| | thrummer | 2 crystal + 2 copper ingots + 1 glass |
| | lift | 2 iron ingots + 4 wood + 1 copper ingot |
| | tube hopper | 2 iron ingots + 2 copper ingots + 1 glass |
| | steam borer (listed once you've been down to 80 m) | 20 iron ingots + 10 bronze ingots + 8 brick + 6 crystal + 4 quartz |

The chain is the design: wood and stone give a bench and a pickaxe; heat (fire at first, lava deeper) lights a forge for copper and iron; copper and tin alloy into bronze, and copperline and a lever power a workshop to make the bronze drill; and iron, copper and crystal from the deep make the bore.

The refinement works ([Production](Production.md#refinement)) sit beside the forge, not above it. A smelter turns raw ore into ingots at the forge's yield (2 ore an ingot) on half the coal of a warm forge and a quarter of a cold one's (one lump burns through four charges, half a coal an ingot), but takes 20 s an ingot where the forge takes about 3, and it needs bellows on a turning train. It is the patient way to metal, and the forge is the quick one. The better yield, three quarters of an ingot a lump of concentrate, needs the stamp mill, which is iron, so it comes after the drill. The smelter (10 clay and 4 coal from the ground), bellows, moulds and washer are all made from what the pickaxe digs (test `the_smelter_is_patient_not_a_shortcut`).

Rod linkages (weight and water logic, [Production](Production.md)) are cheap once there is iron. The rods are iron, so they come after the drill, but a float, a sluice gate and 8 m of rod are 6 wood, 1 stone, 2 iron ore and 2 coal. No part asks more than two iron ingots: the tipping scale, the dearest, is 4 iron ore, 3 copper ore, 1 tin ore, 2 wood and 6 coal (test `rod_linkages_are_cheap_once_there_is_iron`).

**Adding a recipe** (furniture, decor): add the item at the **end** of `Item` (saves store items by index) with its name, description, colour and `uses`, then one `Recipe` line with its station. Unlocking and the crafting list need nothing more.

## Mushroom farms

Plant a spore on an open cell standing on soil. A plot grows fastest when it is both:

* **dark:** no torch or lit lamp within 10 tiles (2.5 m), and no forge within 12 tiles (3 m); and
* **damp:** water, wet soil or clay within 4 tiles (1 m).

With only one of the two it grows at ¼ speed, and with neither it doesn't grow. A ripe plot drops 2 mushrooms (with a 1-in-3 chance of a spore) and regrows. Farms breathe a little CO2.

## Quests

A chain of 21 quests leads through the game. Each one has an item reward. The HUD shows the current one, and the log says "Next: ..." when one is done (and at the start of a Survival holt, after a line listing the pack). The first seven are the first hour: each says what it takes, and none is already done by the Survival kit (test `survival_start_says_what_next`).

Then the burrow becomes a home: **Make it a holt** asks for 2 pieces of furniture standing within 12 m of the workbench (a stool, table or nest: the wooden recipes are listed from the start, since the kit carries wood). It pays 16 timber wall and says how to use it: "Panel the room: timber wall in hand, hold left mouse to line the back." (test `holt_quest_asks_for_furniture_by_the_bench`, scenario `holt_home`). The HUD counts the pieces (1/2).

After that the chain gives a reason to come back with better gear: **find iron ore** (seeing it is enough; it sits in granite 30 to 40 m down, and the pickaxe can't cut it), then build the workshop and make the drill, then **go back for the iron**. No earlier reward hands the iron over (test `quests_send_you_back_with_a_drill`, scenario `explore_clues`).

1. Dig down to 5 m
2. Build a workbench (4 wood, 4 stone)
3. Find wood: dig a root tangle (have 6)
4. Dig 4 copper ore (green-flecked rock)
5. Build a forge (10 stone, 4 clay)
6. Smelt 4 copper ingots
7. Draw 8 copperline at the forge
8. Make it a holt: set 2 furniture by the workbench (stool, table, nest)
9. Farm mushrooms: gather 4
10. Prop a tunnel: place 8 pillar cells
11. Reach 15 m
12. Open a buried chest
13. Fire 10 bricks
14. Alloy bronze
15. Find iron ore (your pickaxe can't cut it yet)
16. Build a workshop and power it
17. Make a drill
18. Go back for the iron: dig 4 iron ore
19. Assemble a thrummer
20. Reach 80 m: the deep
21. Open 4 chests
