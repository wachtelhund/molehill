# Tools and Items

> Code: `engine/sim/src/craft.rs` (items, stations, recipes, quests), `brush_range` and `place_brush` in `world.rs`, the hotbar and pack in `engine/game/src/main.rs` and `ui.rs`.

## Dig tools

There are three hand tools on the hotbar. Each tier takes a bigger bite, digs faster and cuts harder rock:

| tool | tier | default bite (radius, tiles of 0.25 m) | brush range | cuts |
|---|---|---|---|---|
| Pickaxe | 1 | 2.5 | 0.5–3 | dirt and sand almost instantly, soft stone (limestone, soft rock, copper and tin ore, brick) in about 0.3 s |
| Drill | 2 | 3.2 | 0.5–4 | adds granite, basalt, iron ore, quartz and crystal. A copperline-driven auger |
| Bore | 3 | 4.2 | 1–5.5 | everything but bedrock, fastest. A heavy heat-bore with the widest bite |

The colony also knows **pack tools** (stone, bronze and iron), crafted at stations, which set the tier of bare-handed digging (claws are tier 0). Bedrock can't be dug at all.

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
| TNT | Set down **inert** (left mouse, or right mouse on an empty spot); light it with right mouse (1.5 s fuse) or set it off with current, or pick it back up with `E` while unlit. Thrown with `Q`, it goes out lit |
| Spore | Planted on soil to start a mushroom plot |
| Station | Placed as a 2.5 × 1.75 m building (10 × 7 tiles) |
| Glowmite | A glowmite in a jar: held in the paw it lights the way like a small cold lamp, with no fuel |

Food (grubs, mushrooms, fungus) is eaten from the pack automatically when the mole gets hungry.

**Right mouse** on a glowmite drifting in a grotto catches it in a jar (see [World and Generation](World-and-Generation.md#grottos)); otherwise right mouse uses the device under the cursor.

## The pack

The pack (`I`) is a grid of 70 slots, and the bottom row is the hotbar (`1`–`0`). Click a slot to pick its item up, then click another to drop or swap. Counts come from one shared inventory. Hovering a slot shows its name, count and a one-line description. Resting the cursor on a seen cell for about 0.35 s explains what's there (`sim/src/tips.rs`): the material (and whether it's loose debris), a device, copperline (carrying current or idle), a station (what it makes; craft by standing at it with the pack open), a chest (opened or not) or a mushroom plot (growth % and whether it's too bright or too dry). A standing block also says how hard it is working, read from the stress solver: bearing weight, straining, overloaded, or cracking, so you can see a span is about to fail before it does.

> **Modes:** New Holt asks for **Survival** (a small kit: stone tools, 4 torches, 3 grubs, 2 roots, 2 fungus oil; you dig only with tools you own; the headlamp and torches burn fungus oil) or **Creative** (999 of every item). Crafting is still hidden. **Fungus oil** is pressed from fungus (2 a measure, done by the pack when it runs out); a **root** is food from root tangles. See [Progression](Progression.md#survival-mode-today). The sim's own starting pack (used by the tests) is small: 8 wood, 6 torches, 12 pillars, 12 grubs, 40 copperline, 6 knots, 3 thrummers, 6 spores and a vent.

## Digging drops

Digging drops items: one per 6 tiles dug, or one per 3 for gems. **Wood** comes from grottos: root-tree trunks and their roots (a trunk cut down but left with its roots grows back), and from tangled roots in the upper soil. Mycelium gives fungus. Loose items are small physics bodies (`items.rs`): they pop out of the bite, fall, bounce and skid on what they land on (stone is lively, dirt swallows a bounce), float or sink by density (wood floats, ingots sink, a grub sinks slowly), burn up in lava or fire if they burn, and fall asleep once still. Walking past picks them up.

## What each item does

One table (`Item::uses` in `craft.rs`) says whether an item is **set down**, **thrown**, or **picked back up**, and `Item::controls` turns it into the line of inputs shown in the slot tooltip and under the item name over the hotbar.

| Item | Set down | Throw (`Q`) | Pick up (`E`) |
|---|---|---|---|
| TNT | left/right mouse, unlit | yes, lit | while unlit |
| Wood pillar, brick, iron beam | left mouse (brush) | no | no (dig it) |
| Copperline | left mouse (drag) | no | one segment, sweep for a run |
| Torch | left mouse | no | yes |
| Lamp, vent; levers, knots, thrummers and other devices | left mouse, or right mouse on an empty spot | no | yes |
| Spore | left mouse (plant on soil) | yes | no |
| Loose materials, ingots, charcoal, grubs, mushrooms | no | yes | (lying loose: walk over it) |
| Tools, glass | no | no | no |

## Throwing

Hold `Q` to charge a throw of a throwable item in hand and release to throw it at the cursor; 0.8 s of charge is a full throw, which carries about 6 m on the level, and dots show the arc. You can't catch your own throw for half a second. A thrown **TNT** charge goes out lit: it flies, bounces and rolls, and goes off where its 1.5 s fuse runs out, with the same blast as placed TNT. Water puts it out (it lies there as a dud you can pick up again), and lava or fire lights one lying loose. A lit charge can't be picked up. Blocks, copperline, devices, torches and tools aren't thrown: `Q` says so and keeps them. Blasts throw loose items around. In the colony design, moles haul drops to the **stockpile**, and only stockpiled items count. The first sighting of each ore, fluid or rock is a **discovery** with a small reward.

## Stations

| station | cost | does |
|---|---|---|
| Workbench | 6 wood | saw and bench for wooden things |
| Kiln | 8 stone + 4 clay | bakes clay into brick and sand into glass |
| Furnace | 10 brick | smelts ore into metal |
| Anvil | 5 iron ingots | hammers metal into tools and beams |

## Recipes

| station | makes | from |
|---|---|---|
| Workbench | 4 torches | 1 wood + 1 coal (or charcoal) |
| | 4 wood pillars | 2 wood |
| | stone tools | 2 wood + 4 stone |
| | vent | 2 wood + 2 stone |
| | TNT | 2 coal + 2 sand |
| Kiln | 4 brick | 2 clay + 1 coal |
| | 2 glass | 2 sand + 1 coal |
| | 2 charcoal | 2 wood |
| | lamp | 1 glass + 2 fungus |
| Furnace | copper ingot / tin ingot | 2 ore + 1 coal |
| | iron ingot | 2 iron ore + 2 coal |
| | 4 bronze | 3 copper + 1 tin ingot |
| Anvil | bronze tools / iron tools | 3 ingots + 1 wood |
| | 4 iron beams | 1 iron ingot |
| | 8 copperline | 1 copper ingot |
| | knot | 1 copper ingot + 1 quartz |
| | thrummer | 2 crystal + 2 copper ingots + 1 glass |

The chain is the design: wood and stone get you a bench; clay and coal get you a kiln and brick; brick gets you a furnace and metal; iron gets you an anvil, beams and copperline; and crystal from the deep gets you a thrummer.

## Mushroom farms

Plant a spore on an open cell standing on soil. A plot grows fastest when it is both:

* **dark:** no torch, lit lamp, kiln or furnace within 10 tiles (2.5 m), or 12 tiles (3 m) for a kiln or furnace; and
* **damp:** water, wet soil or clay within 4 tiles (1 m).

With only one of the two it grows at ¼ speed, and with neither it doesn't grow. A ripe plot drops 2 mushrooms (with a 1-in-3 chance of a spore) and regrows. Farms breathe a little CO2.

## Quests

A chain of 18 quests leads through the game. Each one has an item reward:

1. Dig down to 5 m
2. Gather 6 wood
3. Build a workbench
4. Farm mushrooms: gather 4
5. Prop a tunnel: place 8 pillar cells
6. Craft stone tools
7. Reach 15 m
8. Build a kiln
9. Open a buried chest
10. Fire 10 bricks
11. Build a furnace
12. Smelt 4 copper ingots
13. Alloy bronze
14. Reach 40 m
15. Forge an anvil
16. Assemble a thrummer
17. Reach 80 m: the deep
18. Open 4 chests
