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

## Open issues
* A resting pool can keep a 1-cell step if the lower spot is more than 48 cells away.
* Lava never cools on its own; dry lava resting on a basalt crust stays lava.
* Thin roofs are still judged only by their bottom fibre's tensile strength (a 1-cell clay skin
  under granite limits the whole roof).
* `site/` not rebuilt: the `wasm32-unknown-unknown` target isn't installed here (`rustup target add wasm32-unknown-unknown`, then the AGENTS.md rebuild line).
