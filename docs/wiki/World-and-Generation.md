# World and Generation

> Code: `engine/sim/src/gen.rs` (procgen), `World::shift` and `follow_colony` in `world.rs` (the endless window), `disasters.rs`.

## The shape of the Deepholt

The Deepholt has no top and no bottom, so the game never holds "the world" in memory. It holds a **window**: a grid of **2048 × 1024 cells**. One cell is **0.125 m** (`mat::CPM` = 8 per metre), so the window is 256 m wide and 128 m tall. Depth 0 is the layer you start in. Positive depth is deeper and negative is shallower, and both run on forever.

```
          … generated on demand …
   ┌──────────────────────────────┐  ← window row 0 = absolute row `oy`
   │ bedrock │  2048 × 1024   │ bedrock │
   │  wall   │  live cells     │  wall   │   the sim, stress, light and air run only here
   └──────────────────────────────┘
          … stored bands / not yet generated …
```

* **Vertically endless.** When the camera or colony gets near the top or bottom edge, the window **slides by 32 m** (`SLIDE` = 32 × CPM rows). The margin is `min(25 m, (h − SLIDE) / 2)`, so small test windows don't thrash. Rows that leave are packed into 64-row **bands** (`band.rs`) with everything in them: cells, copperline and devices (a lit fuse keeps its time), air, torches, drops, stations and mushroom plots. So your tunnels and builds are still there when you come back. Until format 4, only the cells were kept and builds vanished. Rows that have never been seen are generated fresh. The structural state slides with the rock (`Stress::shift_rows`), so after a slide only the entering rows (plus 16 m) and the edge the window left are re-solved, and stabilising fresh rows starts from that state. A slide into fresh rows takes about 0.3 s and one back into stored rows about 80 ms (`slide_keeps_stress_state` checks it matches a full solve).
* **Horizontally bounded.** The two outermost columns on each side are bedrock.
* **Deterministic.** Every generated cell is a pure function of `(seed, x, absolute row)`. The same seed always gives the same Deepholt, wherever you start looking. New rows are stabilised just like the first window (see below).
* **Chests** come 6 per 32 m band (`CHEST_BAND`), each on a cave floor, chosen by hashing `(k, band, seed)`. Opened chests are remembered by absolute position across slides.

## Effective depth

Generation works on an **effective depth** `dd` rather than the raw depth, so layer boundaries wander:

```
dd = depth_m + (fbm noise − 0.5) × 10 m + biome offset
```

The biome offset is −4 m for Loamlands, +2 m for Karst caverns, +8 m for Granite crags and +4 m for the Ashlands. A crag column is "deeper" than its depth suggests, so hard rock rises closer to the start there.

## Biomes

Biomes vary along x only (a 2-octave fbm at scale 65 m). Generation noise is sampled in 0.25 m tiles, so every feature keeps its size in metres at any resolution. so each one is a tall column you can dig straight down through.

| biome | share of noise | character |
|---|---|---|
| Loamlands | < 0.36 | Soft and shallow-feeling: more dirt and clay, ordinary caves |
| Karst caverns | 0.36–0.52 | More and larger caverns (cave threshold +0.04) |
| Granite crags | 0.52–0.66 | Granite from depth 0; fewer caves (−0.03) |
| Ashlands | ≥ 0.66 | Basalt below 70 m everywhere; hot and deep-feeling |

## Strata

| effective depth | rock |
|---|---|
| above −30 m | dirt, with clay lenses |
| −30 to 0 m | dirt, clay and limestone patches |
| 0 to 30 m | limestone and soft rock (granite in the crags) |
| 30 to 70 m | granite and soft rock |
| below 70 m | granite and basalt (all basalt in the Ashlands) |

Above 35 m, lenses of **sand** and **gravel** form in the rock. The wettest are aquifer-saturated (`F_WET`): they seep water when you dig into them, and they make a mushroom farm damp.

**Ore veins** follow thin ridged-noise bands:

| ore | effective depth |
|---|---|
| coal | −20 to 9 m (thin seams up to −60 m); never where lava runs |
| copper ore | below 0 m |
| tin ore | below 10 m |
| iron ore | below 35 m |
| quartz | below 45 m |

**Cave walls** are decorated: **crystal** (which glows) below 45 m, **quartz** below 35 m, **glowing fungus** on cave floors above 9 m, and **roots** above −10 m.

## Caves

A cell is open when either test passes:

* **caverns:** a 3-octave fbm (scale 46, stretched 1.6× vertically) plus the biome band is above 0.66: big blobby halls;
* **worm tunnels:** a ridged fbm at scale 110 is within 0.012 of its midline: thin winding passages that link the halls.

## Fluid pockets: wetter up, hotter down

Open cells inside a separate pocket-noise field (`> 0.63`) are filled with fluid. Each 6 m × 6 m patch picks **one** fluid with a hash, so a pocket never mixes:

```
lava share  = clamp((dd − 10) / 60, 0, 0.92)
water share = clamp((40 − dd) / 60, 0.08, 1) of what is left
```

At the start layer pockets are almost all water. By 40 m half of them are lava, and from 70 m on lava dominates. A little water survives at every depth. This is the lore's "cooler and wetter up, hotter and drier down" written as two clamps.

## The start burrow

The holt begins in a **domed chamber about 7.75 m wide**, carved at depth 0 in the middle of the window:

* the roof is limestone and steps down toward the walls (a parabola, 3 m of drop), so no roof run is long enough to fail in bending;
* side walls tie the dome to a clay floor, and if a cavern lies underneath, limestone legs drop down to the next solid ground;
* water and lava near the burrow are dried to clay, and nothing that flows in during the settling pass stays inside;
* the floor stays soil, so mushrooms can grow there, and nothing just below it is granite, so even claws can dig out;
* a **vent** is placed above the burrow and two torches inside it, and the area 15 m to each side is revealed.

## The stability pass

Random noise makes caves that real rock could not hold. After filling, the generator runs the full structural solver (see [Physics](Physics.md#structure-and-stability)) in a loop:

1. Solve with a **safety factor** (`thr = 0.45`): fresh caves must survive fluids shifting about later.
2. Cells with no load path (floating islands) are removed.
3. Other failing cells are reinforced: weak cells become limestone in the first passes, then granite, and a cell that keeps failing gets an ever-wider fill of the voids around it (the radius grows every 3 rounds, up to 1 m) to thicken roofs and widen pillars. A fresh slide now needs about 18 rounds instead of 42.
4. After 40 rounds the margin is dropped to 1.0, so the loop only has to make sure nothing actually fails.

Only the first round is a full solve. Every later round re-solves just the regions around the cells the previous round changed (`Stress::solve_cells`), with an 8 m margin at the sides and the roof rows kept between solves. This is exact, because every failing cell is changed. Together with the growing fill, it makes a new holt several times faster to generate: `World::new` on the game window takes about 1.1 s instead of 7–10 s (`genbench` measures it).

Then the world runs **240 falling-sand ticks** so loose material and fluids settle, and the pass runs again. A fresh world therefore stands: with no player action the start burrow holds and only a little collapses in the first two minutes (this is tested). Collapses are caused by you, or by fluid fronts you open up.

## Disasters

`disasters.rs` keeps its own random stream, so it never disturbs the world's determinism. It cycles calm → warning → active:

| disaster | where | what happens |
|---|---|---|
| Flood | upper half of the window, preferring water pockets | a spring adds water (0.4 % of the world's cells) |
| Lava surge | bottom 40 %, preferring lava pockets | a spring adds lava (0.2 %) |
| Earthquake | the middle band | loosens soft overhangs near the epicentre (0.15 % budget); stress and falling sand do the rest |

Each one gives a **10 s warning** banner with its depth, lasts **6 s** with camera shake, and is followed by a calm of **90–180 s**. The first comes after about 2.5 minutes.

> **Current build:** the game turns disasters off while the game is being tuned (`make_world` in `game/src/main.rs`), and the pack comes from the mode picked at New Holt (Survival's kit or Creative's 999 of everything). The sim and the tests keep disasters.

## Fog, light and the background

* **Fog of war:** cells stay hidden until a mole sees them (raycasts every few ticks, 7 m radius) or digs next to them.
* **Light** comes only from emitters (lava, crystal, fungus, fire) and lights (torches, lamps, kilns, furnaces, the mole's headlamp, a torch in its paws, the mole's own small glow). Rock casts shadows. See [Physics](Physics.md#light).
* **Parallax:** open cells show three procedural deep-earth layers: far strata with biome tint (factor 0.8), cave silhouettes (0.6) and mineral veins (0.35), rusty near the start and pale quartz or ember deeper down. They follow absolute depth, so they stay continuous when the window slides, and they are drawn as GPU sprites that move smoothly with the camera.
