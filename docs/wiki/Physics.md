# Physics

> Code: `mat.rs` (material table), `grid.rs` (falling sand and fluids), `stress.rs` (structure), `light.rs`, `air.rs`, `logic.rs` (thrumming).

Molehill's design rule is that **nothing happens by decree**. Collapse, flooding and signal reach all come out of a few physical models that share one material table. When the game feels wrong, the fix goes into a model or a number in that table, not into a special case.

## Units

| quantity | unit |
|---|---|
| cell edge `s` | 0.125 m (1 m = 8 cells, `mat::CPM`); depth into the screen `b` = 1 m |
| tile | 0.25 m = 2 × 2 cells: the size everything was first tuned at. Lengths below given in *tiles* scale with `mat::S` (cells per tile), so behaviour is the same in metres at any resolution |
| time | 60 ticks per second, fixed |
| density ρ | kg/m³ |
| strength, stress, pressure | kPa |
| `g` | 9.81 m/s² |

## Materials

Every cell holds one of 28 materials. Each has a **kind**, which picks how the falling-sand automaton treats it:

| kind | materials | behaviour |
|---|---|---|
| Solid | bedrock, granite, rock, limestone, basalt, dirt, clay, gravel, coal and ores, quartz, crystal, roots, grass, glass, wood beam, stone brick, iron beam, door | Static until it cracks or is knocked **loose**; loose solids fall like powder |
| Powder | sand, fungus | Always fall and slide by angle of repose |
| Liquid | water, lava | Fall, then spread sideways to level out |
| Gas | steam, fire | Rise and fade |

Strengths are **rock-mass** values: lab values for intact rock scaled down (about ×0.02–0.05) for joints and flaws, as rock mechanics does with GSI and Hoek-Brown. A selection:

| material | ρ (kg/m³) | tensile (kPa) | compressive (kPa) | tool tier | breaks into |
|---|---|---|---|---|---|
| dirt | 1600 | 15 | 250 | claws | dirt |
| clay | 1800 | 40 | 400 | claws | clay |
| sand | 1600 | 0 | 150 | claws | (powder) |
| limestone | 2500 | 400 | 9000 | pickaxe | gravel |
| soft rock | 2300 | 300 | 8000 | pickaxe | sand |
| basalt | 2900 | 700 | 18000 | drill | gravel |
| granite | 2700 | 800 | 20000 | drill | gravel |
| crystal | 2600 | 300 | 20000 | drill | gravel |
| wood beam | 600 | 3000 | 8000 | claws | wood splinters |
| stone brick | 2400 | 600 | 15000 | pickaxe | gravel |
| iron beam | 7800 | 40000 | 120000 | drill | iron debris |
| water | 1000 | – | – | – | – |
| lava | 2700 | – | – | – | – |

Each material also carries a dig time, a blast resistance, a light emission and opacity, a colour, and a **repose** value (0 flows freely, 1 never slides). Dirt is 0.15, clay 0.45 and gravel 0.4.

The rest of a material's behaviour is in the same row (`Props` in `mat.rs`), and the rules read it rather than naming materials:

* **Heat.** `heat` is what it gives off (a flame 1, lava 2): it burns a mole, feeds the air's fire load and lights fuses. `ignite` is the heat needed to set it alight: wood, roots, fungus and doors catch from any flame; coal only from lava. `cools_to` is what it sets into when it loses heat: lava becomes basalt (shattered scoria when water quenches it), and a flame touching water becomes steam.
* **Flow.** Liquids carry `spread` (how far a cell runs along a surface per tick, in 0.25 m tiles: water 6, lava 2) and `flow_in_10` (on how many ticks in 10 it moves: water 10, lava 3). `life` is how long steam and fire last.
* **Thrum conduction** is derived from density and compressive strength (see Thrumming below).
* **Flags** sort materials for the few rules that need a category: `SOIL` (a mushroom plot can grow on it), `POROUS` (lets stale air seep away), `GRUBS` (digging turns up grubs), `BUILT` (placed by moles: breaks into its own debris, never bulks, a quake doesn't loosen it), `NOTABLE` (a discovery note the first time it's seen) and `OPEN` (a placed block displaces it).

A new or retuned material is a single row edit. `material_table_is_consistent` checks that the rows hang together.

## Falling sand

The grid is a Noita-style cellular automaton on **64 × 64-cell chunks** (8 m):

* Chunks update in **four checkerboard phases**. Chunks in the same phase are at least 64 cells apart, and no cell moves more than 6·S cells a tick, so a phase can run in parallel (rayon on native) and still be deterministic. Randomness is a hash of `(x, y, tick)`, never a shared stream.
* Rows run bottom-up, alternating left-right and right-left each tick to avoid drift.
* **Sleeping chunks.** A chunk only updates while something in it can move. A grain with nowhere to slide, a pool with a level surface or lava with no real move all rest without waking their chunk. A settled world runs at about 0 awake chunks (`wakediag` checks this), and a settled tick costs about 0.13 ms.

**Grains** (powder, or solid cells flagged loose) fall into empty space or gas, then try the two lower diagonals. A grain **sinks through a lighter liquid** (sand, gravel and scoria through water, more slowly than falling) and **floats on a denser one** (sand on lava). Light debris such as wood splinters bobs up through water and floats.

## Liquids and lava

* A liquid falls first, then tries the lower diagonals.
* **Free fall** is up to S cells (one tile) a tick for grains and liquids, so things fall as fast in metres at any resolution.
* **Sideways flow is purposeful.** A surface cell looks up to 48 cells (6 m) along its row for a lower spot and moves toward it (water up to 6 tiles a tick, lava 2). It also spreads when liquid presses on it from above, so a heap levels out. Otherwise it stays put, so a pool with a part-filled top row doesn't slosh forever.
* **Lava is viscous:** it only moves on about 30 % of ticks. Flowing lava sets wood, roots, fungus, coal and doors on fire. Lava that has come to rest sleeps with its chunk, so it doesn't set fire to wood beside it.
* **Thin lava cools.** Every 30 ticks a quarter of the rows is swept, each lava cell there with a 1-in-6 chance to be checked (the same rate as 1-in-24 for every row). If it has fewer than 10 tiles' worth of lava in its 1.25 m (5 × 5-tile) neighbourhood and rests on solid ground, it freezes into basalt. A splash sets in about 10–20 s, and a spreading sheet crusts from its thin front. Deep pools keep their heat, and only their thin edges crust. Lava never freezes on top of molten lava, because a crust there would just break up again (`lava_cools`).
* **Lava meeting water** quenches: the lava cell becomes loose basalt **scoria** that sinks, and the water becomes **steam**. Steam drifts up, fades after about 1.5 s and sometimes condenses back into a drop of water.
* **Fire** spreads to wood, roots, fungus and doors, rises, burns out, and turns to steam on touching water.
* **For the mole,** water is swimmable and lava is a thick, burning liquid. You sink into it, wade at a quarter speed and burn at 75 hp/s (see [Player and Controls](Player-and-Controls.md#water-and-lava)).

## Structure and stability

The structural solver (`stress.rs`) is the heart of the game. A full solve covers the whole window (2 M cells, about 40 ms on native), so after the first one the world re-solves **locally**: every structural change (a cell set, a grain slipping out from under rock, a fire or quench) flags its 8 m chunk; flagged chunks cluster, and each cluster is solved in a region (the cluster plus 32 m to the sides, the 16 m arch height above and 8 m below; roof rows and runs reach past it exactly) and written back for the cluster plus 4 m. Clusters go nearest the mole first, about an eighth of the window per pass; the rest wait 6 ticks. A local re-solve matches a full one (`local_stress_resolve_matches_full_solve`). It runs after structural edits (every 6 ticks while anything is dirty) and after fire and lava reactions (every 24 ticks).

### 1. Overburden

The pressure on a cell is the weight of the column above it:

```
P(x, y) = Σ ρ · g · s / 1000   [kPa]   over non-empty cells in the 16 m above
```

Only 16 m counts. That is the **pressure-arch height** (Terzaghi arching): load from deeper overhead flows around an opening through the ground arch.

### 2. Load path

A breadth-first search starts from bedrock and from the window's top and bottom rows (the Deepholt continues past them). **Static cells carry each other in all four directions.** **Loose ground** (sand, rubble) carries only what rests directly on it, so a pillar built on a sand floor still has a load path down to rock. A static cell that isn't reached has no load path; it gives way after **8 ticks**, and its debris sinks or floats by density.

### 3. Roofs in bending

Every horizontal run of static cells with nothing supporting it from below is treated as a **beam**:

* span `L = n · s`;
* depth `h` = the static thickness above it, **capped at about one span** (the Voussoir effect: only about L of rock acts as the beam, and the rest arches);
* `q` = superimposed load from **loose material lying directly on it** (rubble, sand or water). Static rock above the cap carries itself, and a cave above is carried by its own roof.

```
M  = k · (ρ g h + q) · L²      k = 1/8 if both ends rest on support, 1/2 for a cantilever
Z  = b · h² / 6
σt = M / Z                     compared with the weakest tensile strength along the run
```

The moment isn't spread evenly: it follows `x(L − x)` for a beam resting at both ends (peaking at midspan) and the distance from the free tip squared for a cantilever. An overloaded roof therefore **breaks where the moment peaks**, an arch forms, and the cave-in stops. It doesn't peel the room row by row. In a test, an over-wide 15 m hall went from 840 crumbled cells to 12 after this change.

Examples from the tests: a 1 m span of 1 m-thick dirt holds and a 6 m span fails; a 12 m rock roof caves in, but two pillars at its third points save it.

### 4. Pillars and walls in compression

Every horizontal run of static cells of width `w` carries its own overburden plus half of the load over the open space on each side (**tributary area**):

```
σc = (Σrun P + ½ Σleft void P + ½ Σright void P) / w
```

Two refinements keep this honest:

* **Tributary load needs contact.** A run only takes load from a void if its own static column reaches that void's roof. A half-built pillar or a floor bump carries only what rests on it. The load from beside an opening is capped at its loosened zone, about the opening's width in rock, so a lone prop in a sound hall isn't crushed.
* **Confinement.** Only the cells at a run's free faces see their full unconfined strength. Cells held in on both sides are judged at half the stress (Mohr-Coulomb with σ3 ≈ K0·σv), so an overloaded body **spalls at its faces** instead of turning to rubble all at once.

### 5. Cracking and crumbling

Utilisation is `u = σ / strength`. When `u > 1` the cell **cracks** visibly and a **40-tick** countdown starts. If the load is relieved in time, for example by building a pillar or digging less, the crack heals. Otherwise the cell crumbles into its loose form (dirt, gravel, sand, or splinters of its own material for wood and iron) and joins the falling sand. That debris can load the next roof, so failures can cascade.

**Broken ground bulks.** Rubble takes more room than the solid it came from: 3 in 10 crumbling cells of natural ground leave an extra cell of rubble. It spills to the nearest open cell reachable from the break through loose ground, level with the break or lower, searching up to 48 tiles (12 m). It never passes through rock, so rubble can't appear in a sealed room. A cave-in therefore fills its own void and chokes off instead of chimneying up through a whole dirt layer: a 10 × 4 void under 30 rows of dirt rises about 7 rows and stops (`dirt_cavein_chokes`). Built blocks (wood, iron, doors, brick, glass) don't bulk.

```
 dig wide ─▶ beam σt > tensile ─▶ crack (40 ticks) ─▶ crumble ─▶ debris loads the roof below ─▶ …
                     ▲                    │
                     └── prop it in time ◀┘  (crack heals)
```

### Building that holds

Placed blocks are judged by the same rules, which is the point: you learn to build the way the rock does.

* **Wood pillars** are light and strong in compression (8000 kPa): prop a dirt or clay roof every metre or two.
* **Iron beams** have about 13 times the tensile strength of wood: they bridge wide gaps and carry heavy roofs.
* **Stone brick** is strong in a wall but heavy on a roof.
* Everything you place must touch something solid, so a stroke grows out of what it rests on. See [Tools and Items](Tools-and-Items.md#the-round-brush).

### Known limits

* There's no liquid pressure. Water in a tall column doesn't push through a U-bend under a wall; it spreads sideways only when pressed from directly above. This is parked until after the cell resize.
* Thin roofs are judged by their bottom fibre only: a one-cell clay skin under granite limits the whole roof.
* Resting lava never ignites wood beside it.
* A resting pool can keep a 1-cell step if the lower spot is more than 6 m away.
* Overburden still passes through caves higher up, which can crush a dirt seam under deep rock whatever supports it.

## Light

Light is per-cell RGB. It is seeded by emissive materials (lava, crystal, fungus, fire) and point lights (torches, lit lamps, kilns, furnaces, a faint glow around each mole), then spread with a two-sweep 8-neighbour chamfer pass:

```
L(i) = max(L(i), L(n) · t(i) · d)     t = 0.90 open, 0.80 liquid, ≈ 0.3 into rock;  d = 0.93 on diagonals
```

It is computed only over the camera view plus a margin, every second frame. There is no sky light and no day cycle.

## Stale air

CO2 lives on a **2 m block grid** (16 × 16 cells; 0 is fresh, 1 is lethal) and updates every 15 ticks; the survey reads one cell per tile:

* **Sources:** breathing moles (0.08, half while asleep), torches (0.05), fire and lava (0.03), mushroom farms (0.01).
* **Exchange:** conservative diffusion between neighbouring open blocks, scaled by the smaller volume.
* **Sinks:** **vents** pull it out strongly (×0.5 per update in the vent's block, ×0.8 around it), and it seeps slowly into porous soil faces (dirt, sand, gravel, roots, clay).

The Deepholt is sealed, so a closed tunnel system slowly goes stale (tested). Above 0.45 a mole's breath drains and then its health; above 0.55 colony moles flee to fresher air. Stale air shows as an olive haze.

## Thrumming

Thrumming is how a signal crosses rock without copperline. It is modelled as an **elastic wave**, with every number derived from the material table:

| step | formula |
|---|---|
| stiffness | E ≈ 300 × compressive strength (rock-mass modulus ratio); liquids 2.2 GPa (bulk modulus); air 142 kPa |
| wave speed, impedance | c = √(E/ρ), Z = ρc |
| damping | Q = 5 + 4 × UCS [MPa] for intact solids (granite ≈ 85, dirt ≈ 6); 4 for loose or powdery ground; 20 for water; 2 for air |
| absorption | 8.686 · π · f / (Q · c) dB/m at f = 200 Hz, applied per cell along the ray |
| boundaries | only T = 4Z₁Z₂ / (Z₁+Z₂)² of the energy passes each material change: rock to air costs about 30 dB |
| spreading | 10 · log₁₀(r) dB (cylindrical, from 1 m) |

A thrummer starts with **60 dB**. A link to another thrum device holds while 60 dB minus all losses along the straight line between them stays at or above 0. The upshot is the lore, derived rather than scripted: **dense stone carries a knock tens of metres, loose dirt muffles it, water dampens it, and an open tunnel reflects it away.** How to use it is on [Devices and Copperline](Devices-and-Copperline.md#thrumming).
