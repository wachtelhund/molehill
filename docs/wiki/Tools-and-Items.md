# Tools and Items

> Code: `engine/sim/src/craft.rs` (items, stations, recipes, quests), `brush_range` and `place_brush` in `world.rs`, the hotbar and pack in `engine/game/src/main.rs` and `ui.rs`.

## Dig tools

There are three hand tools on the hotbar. Each tier takes a bigger bite, digs faster and cuts harder rock:

| tool | tier | default bite (radius, cells) | brush range | cuts |
|---|---|---|---|---|
| Pickaxe | 1 | 2.5 | 0.5–3 | dirt and sand almost instantly, soft stone (limestone, soft rock, copper and tin ore, brick) in about 0.3 s |
| Drill | 2 | 3.2 | 0.5–4 | adds granite, basalt, iron ore, quartz and crystal. A copperline-driven auger |
| Bore | 3 | 4.2 | 1–5.5 | everything but bedrock, fastest. A heavy heat-bore with the widest bite |

The colony also knows **pack tools** (stone, bronze and iron), crafted at stations, which set the tier of bare-handed digging (claws are tier 0). Bedrock can't be dug at all.

## The round brush

Dig tools and building blocks share one idea: a **round brush** centred under the cursor, shown as a ring.

* `[` / `]` or `Ctrl` + wheel resize it within the item's range. The label above the hotbar shows `brush r`.
* A radius of 0.5 is one cell, and 1.0 is a plus of five.
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
| TNT | Placed **inert**. Light it with right mouse (1.5 s fuse) or set it off with current |
| Spore | Planted on soil to start a mushroom plot |
| Station | Placed as a 10 × 7-cell building |

Food (grubs, mushrooms, fungus) is eaten from the pack automatically when the mole gets hungry.

## The pack

The pack (`I`) is a grid of 70 slots, and the bottom row is the hotbar (`1`–`0`). Click a slot to pick its item up, then click another to drop or swap. Counts come from one shared inventory. Hovering a slot shows its name, count and a one-line description. Resting the cursor on a seen cell for about 0.35 s explains what's there (`sim/src/tips.rs`): the material (and whether it's loose debris), a device, copperline (carrying current or idle), a station (what it makes; craft by standing at it with the pack open), a chest (opened or not) or a mushroom plot (growth % and whether it's too bright or too dry). A standing block also says how hard it is working, read from the stress solver: bearing weight, straining, overloaded, or cracking, so you can see a span is about to fail before it does.

> **Current build:** for testing, the game starts with all three tools and 999 of every item, and crafting is hidden. The sim's own starting pack (used by the tests) is small: 8 wood, 6 torches, 12 pillars, 12 grubs, 40 copperline, 6 knots, 3 thrummers, 6 spores and a vent.

## Digging drops

Digging drops items: one per 6 cells dug, or one per 3 for gems. In the colony design, moles haul drops to the **stockpile**, and only stockpiled items count. The first sighting of each ore, fluid or rock is a **discovery** with a small reward.

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

* **dark:** no torch, lit lamp, kiln or furnace within 10 cells; and
* **damp:** water, wet soil or clay within 4 cells.

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
