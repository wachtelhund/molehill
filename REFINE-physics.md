# Refine run: physics (branch `refine/physics`)

Polish only: no new features. Each item has a scenario in `engine/sim/tests/scenarios/`.

## Changed
* **Granite ceiling in `run_jump` cracked and rained gravel.** The roof-bending check loaded every
  beam with the full overburden above its arch cap (`qtop`), so any wide room failed and the
  failure peeled row after row (616 cells in 600 ticks). Now only loose rubble, sand or water lying
  directly on the beam counts as superimposed load, spread over the span. (`stress.rs`, `run_jump.scn`)
* **Cave-ins peeled whole rows.** Bending utilisation now follows the moment shape (x(L-x) for a
  beam resting at both ends, tip distance² for a cantilever), so an overloaded roof breaks at
  midspan, an arch forms and the collapse stops. An over-wide 15 m hall went from 840 crumbled
  cells to 12. (`hall_sag_arrests.scn`)
* **Placed pillars / beams crumbled and flowed away.** A pillar on a sand floor had no load path
  (connectivity only walked static cells), so it failed within 40 ticks and its debris was sand.
  Load now passes straight down through resting powder to rock. Tributary load on pillars/walls
  from open space beside them is capped at the arch's loosened zone (≈ opening width), so a lone
  prop in a sound hall isn't crushed. Broken wood beams/doors become loose splinters (`Roots`,
  which dig back into Wood) instead of free-flowing sand.
  (`pillar_holds`, `pillar_on_sand`, `pillar_wide_hall`, `roof_propped`, `roof_unpropped`,
  `beam_span`, `beam_overload`)
* Harness: `place T dx dy Item` directive, `I` = iron beam in maps; `World::place_block_at`.

* **Settled worlds never slept (performance).** 30 of 32 CA chunks stayed awake forever:
  pools whose top row is part-filled sloshed sideways endlessly, fungus (repose 1.0) and resting
  rubble re-woke whenever a diagonal was open, and lava woke itself every tick. Now liquids move
  sideways only toward a lower spot within 48 cells or when liquid presses from above (a heap
  levelling), grains that can't slide rest, and lava stays awake only while it has a real move.
  Seed 11: 1200-tick settle 874 → 177 ms, settled tick 0.76 → 0.13 ms; awake chunks 30 → ~0; the
  sim test suite 125 s → 9 s. (`water_levels`, `lava_flows`, `lava_meets_water`; `wakediag` example)
* CA `moved` stamps wrapped every 255 ticks, so some cells skipped a tick; cleared on wrap.
* Harness: `awake` variable, `a` = basalt in maps.
* **Sand floated on water.** A grain only sank into liquid with no powder above it, so any slab
  rested on a pool forever. Grains now sink by density (sand, gravel, scoria through water; sand
  floats on lava) and can slide diagonally into lighter liquid, so underwater piles spread.
  (`sand_sinks`, `sand_floats_lava`)
* **World window thrashed.** `follow_colony` used fixed 100-row margins with 128-row slides; on
  a 256-row window (tests, scenarios) a mole at y < 96 slid up and straight back down every
  30 ticks, rebuilding stress/light each time. Margins now scale with the window height.
  (`window_does_not_thrash_near_its_edge`)
* **Overloaded ground crushed all at once.** Compression now applies confinement: only cells at a
  run's free faces see unconfined strength, so overloaded bodies spall at faces.
* **The mole could walk on lava.** `passable()` treated lava (and fire) as solid. Lava is now a
  thick liquid for the mole: you sink at 3 cells/s, wade at a quarter speed, can barely claw up,
  can't wall-cling, and burn at 75 hp/s (the old double burn from the colony-mole code is gone for
  the player). TUNE: `lava_mul`, `lava_sink`, `lava_up`, `lava_dps`. (`player_lava_sink`)
* **Solid chunks floated on water.** A clump with no load path now gives way after 8 ticks instead
  of creaking for 40, and its debris sinks by density (see sand above). Light debris rises: loose
  wood (600 kg/m³) bobs up through water and floats. Moving grains out from under static rock, or
  `Grid::set` removing a prop, now asks for a stress re-solve, so slabs don't hang on vanished
  support. (`chunk_sinks_in_water`, `wood_floats`, `rock_falls_when_its_support_slumps_away`)
* Merged main's rigid-blocks work: kept its connectivity, stub-aware tributary load and
  own-material beam debris; kept this branch's roof load, arch cap, moment shape and confinement.
  Harness: `put T dx dy Item` (one cell, no reach) next to main's brush `place`.

## Run 2
* **Cave-ins in dirt chimneyed through the whole layer.** Rubble took exactly the room the solid
  did, so a void just migrated upward forever. Broken ground now bulks: 3 in 10 crumbling cells
  leave an extra cell of rubble in the nearest open cell reachable through loose ground (never
  through rock). A 10×4 void under 30 rows of dirt now rises ~7 rows and chokes: 378 cells
  instead of 830 by tick 1500 and still climbing. Built blocks don't bulk. (`dirt_cavein_chokes`)
* **A buried mole popped out through rock.** The vertical collision snap searched upward without
  limit, so a mole wedged in sand under a granite slab came out 4 cells higher, in the room above.
  The snap now searches at most one cell and otherwise stays put; the "wriggle up" only passes
  through loose ground. (`buried_under_rock`, `buried_in_sand`)
* **Lava never cooled.** Thin lava (fewer than 10 lava cells in its 5×5 neighbourhood) resting on
  solid ground now freezes into basalt: a 1-in-24 chance per 30-tick sweep, so a splash sets in
  ~10-20 s. Deep pools keep their heat; only their thin edges crust. It freezes only on solid
  ground, because a crust on molten lava would just break up again. A spreading sheet crusts from
  its thin front. (`lava_cools`; `lava_flows` now allows for the crust)

## Cell resize (refine/cells, Phase 1)
Cells are half the size: 8 per metre (`mat::CPM`; `S` = 2 cells per legacy 0.25 m tile). The mole is
about 4 cells wide. Everything sized in cells goes through the knob, and the code at CPM = 4 is
bit-identical to before (checked before flipping it).
* **Scaled through `S`/`CPM`:** TUNE and the body; brushes and reach; stress (arch 16 m, caps); worldgen
  (noise sampled in tiles, burrow, chest bands, decoration skins); the window (2048 × 1024 = 256 × 128 m)
  and its slides; device blocks (1 m = 8 cells) and their cell-count thresholds; air blocks (2 m);
  light transmittance per cell (the tile factor's S-th root); fog rays; items per area dug; the
  rendering (CPX = 4/S world units per cell, 128-cell texture chunks, tile-sized texture patterns,
  rims, sprite offsets). Saves are tagged with the resolution; an old save asks for a new holt.
* **Same in metres:** grains and liquids fall up to S cells a tick, liquids spread 6·S, and a pressed
  liquid spreads a tile a tick. Placed blocks go down in whole tiles, so building is unchanged.
* **Harness:** scenarios stay in tiles. Maps are stamped K × K, and values come back in tiles. All 65
  existing scenarios kept their meaning. Six needed a threshold loosened for finer discretisation:
  a bite disc within a tile of the old count, a beam's end sliver, water levelling by tick 160
  instead of 120 (pressure crosses twice the cells), and lava sleeping by about 1500.
* **Found on the way:** a single sand grain inside the mole's body could wedge it for good (walking
  only shoves the leading column). Stray grains are now shoved aside. (`grain_in_body`)
* **Performance:** a full stress solve on the 4× window is about 40 ms. The world now re-solves locally:
  dirty 8 m chunks cluster, each cluster is solved in a region (exact: it matches a full solve), nearest
  first within a budget of an eighth of the window per pass. The air survey and the lava-cooling sweep
  sample per tile. Web (M4, Chrome): 57-58 fps steady state, the same as before. During the first
  ~15 s of a fresh holt it's 54-55 fps (p95 33 ms) while the world settles. Before this fix the resize
  gave 26-34 fps.
* TNT and fuse chaining past the blast radius were still bare cells (+6/+4), so the margin shrank to
  0.75/0.5 m. They're 1.5 m/1 m again (`tnt_chain`; reported by the wiki session).
* Tests now take about a minute (the multi-seed stability runs cover 4× the cells).

## Open issues
* A fresh holt still settles everywhere for its first ~15 s (more chunks dirty than one pass's
  budget): that's Phase 2's active-region work.
* No liquid pressure: water held in a column doesn't push through a U-bend under a wall (a full
  column beside a tunnel just stands there). It spreads sideways only when pressed from directly
  above. A first try at it (rise when liquid joined below stands 2+ higher) sloshed forever,
  because the risen cell fell back into its own gap. The better fix is to move liquid out of the
  high column (2 rows above our surface, within 16 cells, so parallel-safe) onto the low surface.
  Parked for after the cell resize.
* Resting lava never ignites wood next to it (its chunk sleeps); the lava cooling sweep is the
  natural place to add a slow heat check.
* A resting pool can keep a 1-cell step if the lower spot is more than 48 cells away.
* Thin roofs are still judged only by their bottom fibre's tensile strength (a 1-cell clay skin
  under granite limits the whole roof).
