# Molehill: Progression

> This file is the same as [docs/wiki/Progression.md](docs/wiki/Progression.md). Edit both together.

> **Mostly design.** This page describes where Molehill is going. The game now has a **Survival** and a **Creative** mode, with the first slice of Survival in (see [Survival mode today](#survival-mode-today)) and **crafting** built (stations, recipes unlocked by touch, the tool ladder: see [Crafting](#crafting)); everything else here is design unless marked **built**. The other pages describe what is actually in the code. Everything here must follow [Lore](LORE.md).

## The idea in one line

**Depth is progression.** Going up is cool, wet and soft: easy digging, water everywhere, food and wood. Going down is hot, hard and rich: better ore and real power, and more danger. Each tier of tools opens the next band of rock, and each band holds what you need to make the next tier.

```
   ↑ shallower: cooler, wetter, softer          dirt · clay · water pockets · grottos (wood, food, glowmites)
   │
 ──┼── 0 m  the holt                            limestone · coal · copper
   │
   │                                            granite · tin · iron (about 40–85 m)
   │
   ↓ deeper: hotter, harder, richer, deadlier   basalt · lava (dominant below ~70 m) · Ancient ruins
```

The world generator already makes these layers (see [World and Generation](docs/wiki/World-and-Generation.md)): water pockets give way to lava with depth, and dirt gives way to granite and then basalt. Progression is the design that makes walking that gradient matter.

## The survival loop

A Delver's day is digging out, bringing things home, and keeping the holt alive. Each danger from the lore is a need to manage, and most hazards are also resources.

| need | threat | how you meet it |
|---|---|---|
| **Food** | hunger | Fungus farms (dark and damp, as now), roots dug from the upper layers, grubs turned up from soil. Grottos are the richest larder |
| **Air** | stale air in sealed tunnels | Vents, and later powered fans, to move air. Torches and crowds stale a room faster |
| **Light** | the dark; Delvers see poorly | Light costs **fuel**: torches and the headlamp burn **fungus oil**, pressed from fungus. Glowmite lanterns and lamps on current come later. A holt with no light is a holt with no work |
| **Heat** | lava burns, the deep is hot | Avoid it, wall it off, or **use it**: heatstones turn lava heat into current |
| **Water** | floods and drowning | Floodgates and pumps hold it back, and water wheels turn its flow into current |
| **Stone** | cave-ins | Read the strain, prop the roof, build in stronger material. The structural solver is the rule (see [Physics](docs/wiki/Physics.md#structure-and-stability)) |

**Death** is not the end of the holt. You wake in the holt's burrow (as today), but everything you carried stays **where you died**, as loose items in the world. Going back for it is the penalty, and sometimes the adventure: down a flooded shaft, under a fresh cave-in, next to the lava that got you.

## Tiers

Each tier is gated by a material from a deeper or harder band, and unlocks the next.

| tier | key material | you can dig | you can make | where it comes from |
|---|---|---|---|---|
| **1. Wood and stone** | wood, stone | dirt, clay, sand, limestone (**Pickaxe**) | wood pillars, planks, workbench, torches, simple furniture | the holt layers and the first grottos |
| **2. Copper** | copper ore, tin ore | (same, faster) | **copperline**, levers, doors, plates, knots, **thrummers**; **bronze** (copper and tin) for the **Drill** | copper seams just below the holt |
| **3. Iron** | iron ore | granite, iron ore (**Drill**) | iron beams, stronger props, the forge's better recipes | about 40–85 m, in granite |
| **4. Power** | heat and flowing water | (same) | **water wheels**, **heatstones**, **pumps**, **fans**, powered lamps and doors | lava and water pockets: hazards turned to current |
| **5. Heat-proof** | basalt and a heat-resistant alloy (working name *emberiron*) from the lava layer | basalt (**Bore**) | the Bore, lava-proof walls and pipes, deep outposts | below ~70 m, where lava dominates |
| **6. Ancient relics** | found, never made | – | nothing: relics are unique and **can't be crafted** | Ancient facilities and ruins (see below) |

The tool ladder matches the rock: the **Pickaxe** for soft ground, the **Drill** (a bronze-toothed auger driven by copperline) for granite, and the **Bore** (a heavy heat-bore) for basalt. The digging model already gates material by tool tier (see [Tools and Items](docs/wiki/Tools-and-Items.md)).

### Crafting

> **Built** (Survival; Creative keeps free access): the three stations below with their heat and power rules, recipes that unlock by touch, short chains, and the tool ladder: Pickaxe at the workbench, Drill at the workshop, Bore at the forge. Details and the recipe list: [Tools and Items](docs/wiki/Tools-and-Items.md#stations). Not yet: the forge's heat from a heatstone carried by pipes, fungus-oil fuel, and emberiron (the Bore uses iron, copper and crystal for now).

* **Stations, not menus.** You craft at a station you have built and stand at:
  * the **workbench** for wood, stone, simple devices and furniture;
  * the **forge**, which needs heat: a heatstone, or lava run to it, for smelting and metalwork;
  * the **workshop**, which needs copperline and power, for machines, thrummers and fine parts.
* **Recipes unlock by touch.** The first time you pick up a material, the recipes that use it appear. There's no tech tree screen: the world is the tree.
* **Short chains.** At most two steps from the ground to the thing: ore → ingot → tool. No intermediate parts lists.

## Furnishing and decor

> **Built:** decorative blocks (planks, polished stone, tiles, carved pillars), six pigments that paint blocks and dye cloth, and furniture (table, stool, nest, shelf, rug, banner, glowing fungus planter). Details: [Tools and Items](docs/wiki/Tools-and-Items.md#furnishing). Not yet: back walls, and Delvers moving in.

Making the holt **look like home** is a core goal, not a side feature. A Delver should be able to stand in their main hall and be proud of it.

* **Furniture:** tables, beds and nests, shelves, chests, rugs, banners, lamps, fungus planters.
* **Decorative blocks:** polished stone, brick, planks, tiles, carved pillars. They're built with the same brush as structural blocks, and they *are* structural: a carved pillar holds a roof like any pillar, by its material's numbers.
* **Back walls:** a background layer behind open cells, so a dug room can be panelled, bricked or tiled instead of showing raw rock.
* **Paint and dye** from minerals and fungi: ochre and rust from ores, pale from quartz, greens and blues from fungi and moss.
* **Delvers move in.** When the holt is comfortable (light, air, food, beds, a bit of beauty), other Delvers come and join it. Comfort is the colony's growth mechanic.

## Transportation

The deeper the holt reaches, the further its Delvers have to haul, so travel climbs the same tier ladder as everything else. Every way of getting about is a machine you build, which runs on weight, water, heat and current.

> **Built:** rails and mine carts (tier 2, copper): track as a built block, carts that roll downhill by gravity, are pushed, ridden and loaded, and stop at the end of the track. **Powered rails**: copperline laid along the track drives carts while it carries current, so they climb, and a lever or knot switches the line. **Rope ladders** (tier 1) and counterweighted **lifts** (tier 3), balanced with stone by hand or wound up by current. **Pressure tubes**: iron-banded glass pipe between hoppers whose current-driven blowers send capsules of items, or the mole, along the line at 12 m/s; a breach stops the capsule and it spills there. Details: [Transport](docs/wiki/Transport.md).

| tier | transport | how it works |
|---|---|---|
| **2** (**built**) | **Rails and mine carts** | Laid track and a cart that carries cargo (ore, stone, wood, a Delver). Push it on the level; on a slope, gravity does the work and it runs downhill on its own. Track needs support like any built block |
| **1** (**built**) | **Rope ladders** | Cheap vertical access down a shaft. Slow, but needs no power |
| **3–4** (**built**) | **Lifts** | A platform on a counterweight. A loaded cage rises when the counterweight is heavier, so loads are balanced by hand at first and driven by power later |
| **4** (**built**) | **Powered rails** | Track with copperline laid along it, fed by a water wheel or heatstone. Carts run faster and can climb. A lever or knot can switch them, so a powered line can be automated with the same logic as doors and pumps |
| **5** (**built**, at tier 3 for now: iron and glass) | **Pressure tubes** | A sealed tube network that blows capsules along it with air pressure from pumps. Fast and long-distance, and it climbs as easily as it runs flat. Tubes must hold their pressure: a cave-in or a breach stops the line |
| **6** | **Ancient waygates** | The endgame network: see below |

### Ancient waygates (open decision)

> **Open decision for Hampus:** how far the waygates should go, and whether they should exist at all.

The waygates are the closest Molehill comes to teleporters, and they must stay **physics-based and consistent with the lore**: there is no instant jump from one place to another. The proposal is that the Ancients built long **fast lines**: sealed, pumped-out tubes or high-speed rails that ran between their facilities across distances no holt has crossed since. You find a dead line, then **repair it**: restore its seals, clear its cave-ins, and feed it power. Each repaired line links two facilities you have found. Travel is very fast, but it is still travel. A capsule runs the tube, and a broken stretch stops it.

Options to decide between:
* **Repair only:** waygates exist only where the Ancients built them. You can restore them but never extend them. This keeps them rare and makes finding a facility matter.
* **Repair and extend:** late-game relics let you lay new stretches of fast line, so the network can reach your holt.
* **No waygates:** pressure tubes are the top of the ladder, and the Ancients' lines are only ruins to explore.

Whatever the choice, the name and the workings stay earthy and physical: a waygate is a station on a line, not a portal.

## Wood: grottos

Wood is the first material and must stay precious, so it doesn't come from the endless rock. It comes from **grottos**: rare, generated underground oases.

> **Built:** grottos, root-trees that regrow from their roots, moss, mycelium, seep pools and catchable glowmites are in the game. The generator is described in [World and Generation](docs/wiki/World-and-Generation.md#grottos). Glowmite lanterns (a crafted, placeable light) are still to come.

* **Cave trees (root-trees)** grow where water seeps through the rock. They drink the seep and the minerals in it; they need no light.
* **Glowing mycelium** threads the floor and walls, giving soft light. **Moss** covers the damp stone, and **small pools** collect the seep.
* **Glowmites** drift through the air: small glowing insects, ambient light that can be caught in a jar for a **lantern** that needs no fuel.
* **Where:** more common in the wet upper layers and rarer deeper down, following the lore's wet-up, dry-down rule.
* **Care:** cut a tree but leave its roots, and it **regrows slowly**. Clear-cut a grotto, or drain its seep, and it's gone. A good holt is built near a grotto and looks after it.

## Modes

| mode | what it is |
|---|---|
| **Survival** (the main game) | Everything above: start with almost nothing, dig, survive, build up the tiers, furnish the holt and grow the colony. The long goal is the **Lid**: following the old myth upward and through the **Ancients'** facilities, which hint at it. |
| **Creative** | Everything in the pack, nothing hurts. |

### Survival mode today

The first slice of Survival is in the game (`sim/src/survival.rs`); the rest of this page is still design.

* **New Holt** asks for a mode: **Survival** or **Creative**. The mode is kept in the save; a save from before modes loads as Creative.
* **Start kit:** a Pickaxe, 6 wood (enough for a workbench: wood is scarce near most burrows), 4 torches, 3 grubs, 2 roots and 2 measures of fungus oil. Nothing else.
* **Tools you own:** you dig only with tools in the pack; a tool you don't own is drawn faint in the hotbar, after what you hold. The Drill is made at the workshop and the Bore at the forge (see [Crafting](#crafting)).
* **Food:** hunger drains; below 40 the Delver eats from the pack on its own (grub 45, mushroom 35, root 30, fungus 25). Root tangles turn up edible **roots** when dug, soil turns up grubs, and fungus is food too. The log says each meal and what is left ("Ate a grub (2 left)."), and once, when hunger bites with nothing in the pack, where to dig for more. Starving hurts. **Hunger can be turned off** for a holt: a New Holt option, also in Settings for the current holt, and saved with it. Then food never drains and the food bar is hidden.
* **Light costs fuel:** a lit headlamp burns a measure of **fungus oil** every 3 minutes, and placing a torch soaks up a measure. With no oil left the pack presses 2 fungus into a measure; with neither, the headlamp goes dark and torches won't light. The log says when fungus is pressed (and how much is left), when the pack's last measure goes in, and once when the headlamp goes dark (scenarios `survival_food`, `survival_lamp_oil`). The kit lasts about 12 minutes of food and 9 of headlamp oil (test `survival_kit_sustains_a_short_explore`).
* **Crafting:** recipes appear as you pick up their ingredients, and you make things standing at a working station: see [Crafting](#crafting).
* **Stale air** works as before.
* **Death:** you wake in the burrow with an empty pack. Everything you carried lies where you fell as loose items (a pile you can walk back over). Items that burn are lost if that was in lava or fire.
* **Creative:** 999 of everything, nothing hurts, nothing needs fuel.

### The Ancients

Long before today's holts there were **Ancient Delvers**: moles, like everyone else, who dug deeper and built finer. What they left behind is the late game:

* **Facilities:** large, deliberate structures such as halls, shafts, and machine rooms built of materials and techniques no one makes any more.
* **Ruins:** rare, randomly placed remains with **lore fragments** (carvings, inscribed plates, patterns knocked into stone for a listener) and **unique loot**.
* **Relics:** one-of-a-kind items that can't be crafted. They are better-made versions of things Delvers know (a finer drill bit, a lens that throws light further, a lighter alloy), not new physics.

**Built (first slice):** rare ruins of three kinds (pumping halls, sealed vaults, collapsed tube stations), each with a carving read out the first time you come near, two relics (the Ancient bit and the Ground lens) in sealed vaults, and broken waygates at the tube stations as the hook for the fast lines (found, not yet mendable). See [World and Generation](docs/wiki/World-and-Generation.md#ruins-of-the-ancients).

Everything the Ancients built obeys the same physics as everything else: weight, heat, water, pressure and current. Nothing is supernatural, and nothing a modern word would name. Their ruins point toward the Lid, but **the game never shows the Lid**. The myth stays a myth, and the journey is the point.

## From today to here

| area | today | this design |
|---|---|---|
| Mode | New Holt picks **Survival** (small kit, food, fuel, the pack lost at death) or **Creative** (999 of everything); crafting at stations, recipes unlocked by touch | Survival as the main game; Creative kept |
| Tools | **built:** Pickaxe (workbench), Drill (workshop), Bore (forge), dig tiers 1–3; in Survival you dig with the best you own | the same three, unlocked by tiers 1, 3 and 5 |
| Stations | **built:** workbench, forge (needs heat), workshop (needs a live copperline) | workbench, forge (needs heat), workshop (needs copperline and power) |
| Recipes | **built:** unlocked by first pick-up; at most 2 steps | unlocked by first pick-up; at most 2 steps |
| Light | torches, lamps, a headlamp and a jarred glowmite in the paw; in Survival the headlamp and torches burn fungus oil (pressed from fungus), the glowmite needs no fuel | fungus-oil fuel for torches and headlamp; glowmite lanterns |
| Death | Survival: wake in the burrow, the pack stays where you died (Creative: nothing hurts) | wake in the holt; carried items stay where you died |
| Wood | **built:** grottos with regrowing root-trees (see [World and Generation](docs/wiki/World-and-Generation.md#grottos)) | grottos with regrowing root-trees |
| Decor | **built:** furniture, decorative blocks and pigments (see [Tools and Items](docs/wiki/Tools-and-Items.md#furnishing)); back walls not yet | furniture, decorative blocks, back walls, dyes |
| Colony | one mole | Delvers move in when the holt is comfortable |
| Late game | **built:** the Lid myth as clues and a journal, denser higher up; ruins of the Ancients with carvings in the same journal and two relics (see [Lore](docs/wiki/Lore.md#the-lid-in-the-game)) | Ancient facilities, ruins, relics, the Lid myth |
| Travel | walking, climbing, digging; **built:** rails and mine carts, powered rails, rope ladders, lifts, pressure tubes | carts and rails, ladders and lifts, powered rails, pressure tubes; Ancient waygates (open decision) |

Any of these that gets built should move from this page into the page that describes the system, as it lands on `main`.
