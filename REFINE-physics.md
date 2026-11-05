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
* **Generation (Phase 2 start):** stabilisation re-solved the full 2M-cell window every round (40-100
  rounds). Now the first round is a full solve and later rounds re-solve only around the cells the
  last round changed (`Stress::solve_cells`; exact, since every failing cell is changed). Roof rows
  are kept between solves and region solves use an 8 m side margin. Under the same machine load,
  `World::new` on the game window went from 6.6-9.6 s to 2.2 s. (`genbench` example)
* **Slides:** the stress state moves with the rock (`Stress::shift_rows`). Only the entering rows (plus 16 m)
  and the edge the window left are re-solved, and fresh-row stabilisation starts from that state.
  Into fresh rows went from about 1.0 to 0.77 s; back into stored rows from about 110 to 77 ms. It matches
  a full solve (`slide_keeps_stress_state`). Most of a fresh slide is still stabilisation rounds.
* **Phase 2, active region:** the falling sand runs only within 64 m sideways and 48 m vertically of the mole
  (`World::ACTIVE_HX/HY`; `Grid::active`). Chunks outside keep their wake flag and resume exactly where
  they left off, so a fresh holt settles only around the mole and the stress work stays local with it.
  (`ca_runs_only_near_the_mole`)
* **Phase 2, fresh-slide stabilisation:** a cell that keeps failing gets an ever-wider fill (radius grows
  every 3 rounds, up to 1 m), where a 1-cell fill barely helps at 8 cells/m. Rounds per fresh slide
  went from 42 to 18; a fresh slide from about 460 to 280 ms; `World::new` from about 1.5 to 1.1 s.
* **Web (M4, Chrome) after Phase 2:** 58 fps, p95 16.8 ms, already during a fresh holt's first 16 s
  (was 54-55 fps, p95 33 ms). 60 fps in steady state.
* Tests now take about a minute (the multi-seed stability runs cover 4× the cells).

## Open issues
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

## Worldgen for progression (PROGRESSION.md)
* **Ore by depth:** copper 2-30 m (fades over 10 m), tin 12-45 m, iron 40-85 m in the granite, quartz below
  50 m; the deep is mostly basalt. Seeded noise, the same for a seed every time. (`ore_sits_in_its_depth_bands`)
* **Grottos:** one chance per 40 m × 24 m cell: 20 % up high, 10 % at 10 m, 2 % by 50 m, none below 60 m.
  - Shape: a domed hollow with a 1.5 m limestone shell where the host is weak, a moss/mycelium floor,
    a seep pool and 1-3 root-trees. All generated inside the per-cell function, so it's free and deterministic.
  - The stability pass keeps grotto hollows open.
  (`grottos_follow_the_layers`, `generated_grottos_hold`)
* **New materials:** Root-tree (wood), Root-tree roots, Moss, Mycelium (glows). Saves read the
  discovered-materials list by name now, so adding materials doesn't break older saves.
* **Root-trees regrow** from their top root, about a cell every 6 s per column, up to the tree's own height,
  while there's seep water within about 6 m. A stateless sweep reads the grid. (`tree_regrow`, `tree_no_seep`)
  - Moss, mycelium and roots bear 2.5 MPa: they're a skin on the stone floor. At 250 kPa the floor's
    ends crushed, because the overburden model counts a cave's roof onto its floor.
* **Glowmites:** a swarm of 4-7 per grotto on looping paths, each a light source. Right mouse catches one
  (a Glowmite item, which lights when held). A caught one is back after 5 minutes. (`glowmite_catch`, `glowmites_drift...`)
* **Found on the way (also on main before this):** after a slide into fresh rows, the old rows next to them
  could roof a newly generated cave unchecked (5,929 cells failing on one seed). Separately, a roof
  reaching the window's top edge was judged thin. Stabilisation now covers 32 m of old rows, and a static
  column reaching the edge counts as full thickness. (`slides_leave_nothing_failing`)
* Still to come: glowmite lanterns (a crafted, placeable light), tree crowns or branches that regrow.

## Physics-based explosions
* `World::explode` (a fixed-radius disc) is replaced by `World::blast(x, y, kg)`, a pressure wave (`blast.rs`).
  - The wave runs Dijkstra over open, gas and liquid cells.
  - Kinney-Graham overpressure on the equivalent radius of the volume filled, so tunnels carry it and
    caves dissipate it.
  - A gas-pressure term for confinement, and diffraction loss by detour, so walls shadow.
  - Faces crack or shatter by blast strength (15 kPa × 2^blast from the material table).
  - Loose ground and water are thrown (water in a plume, none lost); items fly along the wave.
  - Mole damage comes from local overpressure; charges, fuses and devices react by pressure on their block;
    listeners hear the knock.
  - Deterministic and bounded (400k cells, 3 kPa floor); about 2 ms per charge.
* A TNT charge is 0.25 kg. Its reach changed: a thrower 6 m off in the same closed room now loses about 13 hp
  (`thrown_tnt_explodes_at_landing` expects hp > 80 there). The harness's `explode` takes kg.
* Scenarios `blast_tunnel_vs_cave`, `blast_shadow`, `blast_granite_vs_dirt`, `blast_pushes_water`; `tnt_chain` and the
  other TNT scenarios still pass. Harness variable `p:DX,DY`.

## TNT tuning (umph)
* A charge is now 0.5 kg. Blast strengths come from an explicit table (`BLAST_KPA`), not 15 kPa × 2^blast:
  dirt 150, sandstone/limestone 360, granite 1900, basalt 3500 kPa. Faces crack above 1.4×.
* The load runs on into the rock (e^(−d/λ), λ = 0.6 m × (1 + 2·confinement)), so craters have depth and a
  tamped charge breaks more: soft rock loses ~100 tiles in a pocket, ~80 on a tunnel floor (crater ~2.5 m each
  way), ~30 on a cave floor. Granite chips 0.5 m close in; basalt loses one tile.
* Feel: Quake and Crumble events with the bang, screen shake by charge and by pressure at the player,
  rubble thrown a tile per 15 kPa up to 4 m, harder item impulses.
* Thresholds: 130 kPa sets off a charge, 60 lights a fuse, 400 wrecks devices. Mole damage (P − 40)/3.
* Scenarios `blast_soft_rock_tunnel`, `blast_pocket`, `blast_cave_soaks`, `blast_granite_chips`,
  `blast_basalt_holds`; `thrown_tnt_explodes_at_landing` expects hp > 65.
* Loose ground and liquids fly: thrown cells leave the grid as ballistic debris (`debris.rs`), pushed by the
  pressure (1 m/s per 7 kPa, up to 30 m/s), deeper into a loose bed (fading over 0.5 m), thrown back out of a
  rock-backed surface. They land in the last open cell before what they hit, so none is lost. Deterministic.
* The blast keeps its wave (path distance, peak, direction per cell); the game draws the front following the
  tunnels (slowed to 22 m/s), with dust that rides it and settles, smoke at the charge and debris in flight.
* Blasts no longer leave fire in the crater (it burned furniture the blast had knocked loose).
* Scenarios `blast_moves_sand`, `blast_wave_front`; harness vars `debris`, `debris:Mat`, `wave:DX,DY`, `front`,
  `frontcells`, and a `blast` section in `dump`. Dev flag `--shot-blast=K`.

## TNT playtest pass (more destruction, no outline)
* The drawn white front and its shimmer are gone; the wave shows only through debris and dust. Dust is about a
  third as much, smaller and fainter, and still rides the wave out and settles.
* More real destruction: faces crack at 1.0× strength (was 1.4×) and shatter at 2.5× (was 3×); the load on the
  walls is raised by (1 + confinement); the march into rock runs radially (round craters) with λ = 0.8 m ×
  (1 + 2·confinement); rubble flies at 0.9 of the push. Loose beds are pushed 0.8 m deep, down to 8 kPa.
* Basalt and iron beams moved up the table (6500 / 9000 kPa) so they stay very hard.
* Soft rock now loses ~250 tiles in a pocket (was 100), ~155 in a tunnel (was 84), ~38 in a cave (was 31);
  granite ~22 (was 7), basalt ~4 (was 3).
* Debris buried by a heap lands at the nearest open cell (out to 3 m) instead of hovering and being dropped.
