# Refine run: worldgen (p6)

Short log of what this run changed, and what is still open.

## Changed
* **less-loose-upper** (`refine/less-loose-upper`): settled ground round the burrow (`gen::SETTLED`, `settled_at`,
  `gen::LENS`). Within 40 m of the burrow, easing off by 70 m, sand/gravel lenses need up to +0.14 more lens
  noise, none is soaked, and sand within 1 m of a natural cave is cemented to sandstone. Upper layers (effective
  depth < 10 m) get +0.012 everywhere. Deeper (10-35 m) and further-out lenses are unchanged: the hazard pockets stay.
  Tests: `sim/tests/settled_ground.rs`. Tool: `--example loosesurvey`.

### Numbers (`--example loosesurvey -- 16`, seeds 0-15, game window 1024×512 tiles, within 30 m of the burrow)
| | before | after |
|---|---|---|
| loose share of ground (sand + gravel + fungus + knocked loose) | 7.50 % | 0.68 % |
| sand | 6.65 % | 0.30 % |
| gravel | 0.62 % | 0.13 % |
| fungus (lamp oil, kept) | 0.23 % | 0.25 % |
| soaked ground (`F_WET`; what's left is the near grotto's moss and roots) | 7.66 % | 0.20 % |
| dig bot: generator-loose ground displaced per m of tunnel | 9.96 | 0.99 |
| dig bot: loose fill left in the tunnels per m | 20.7 | 20.1 |
| dig bot: cells crumbled (stress cave-ins) per m | 126 | 143 |

The bot's tunnel refill and cave-ins are dominated by roof rubble (sandstone and limestone roofs, coal and ore
seams; `WHY=1` breaks it down), not by the loose ground the generator places. That half is p2's
(collapse/slide thresholds); the crumbled numbers are passed on as their baseline, not changed here.

### Polish check on merged main (`c738448`, `loosesurvey -- 8`)
Spawn tunnels: loose share within 30 m 0.66 %, generator-loose ground displaced 1.01 cells per tunnel metre
(was 9.96); `digging_straight_down_never_kills` and `settled_ground` pass. Tunnel refill (18.9/m) is roof rubble.
Seed 7's cave-ins rose from 91 to 486 crumbled cells/m between the branch survey and merged main; that came in
from other streams' merges and is for p2 to look at.

## Final polish round (main `a9bcf4b`)
Settled ground re-measured on latest main (`loosesurvey -- 20`): loose share within 30 m 0.66 %, soaked 0.21 %,
generator-loose ground displaced 1.03 cells per tunnel metre; `settled_ground` and `first_hour` pass. Nothing to fix.

## Horizontally endless (Hampus 23:46, branch `refine/worldgen-infinite-x`)
The window now slides sideways as it slides down. No side walls.
* **Generator:** every placement is by absolute column (`i64`): `Gen::cell(x, wy)`, grottos, ruins, found places
  (`grottos_in` / `ruins_in` / `sites_in` over a column range), chests and Lid clues (per 256 m span; span 0 keeps
  its old hashes), the start zone and settled ground (`w0` = home width, burrow at `w0 / 2`). The bedrock side
  columns are gone; the stress solver and the seam's reach pass anchor the window's side columns as they do the
  top and bottom rows (the rock goes on). `sites.rs` layouts untouched (only placement types).
* **Storage:** tiles of 256 × 64 cells (`band::TW` × `BH`), keyed by (first column, first row): `band::Tile`
  (grab / put / cut / read / write). A slide down packs a row of 8 tiles, a slide sideways a column of 16.
* **`World::shift_x`** (32 m, one tile; `follow_colony` at 80 m from a side, `SIDE_MARGIN`, only where
  `roams_x`: a whole number of tiles wide and found-place wide, so scenario maps stay put). Column twins of
  every subsystem's row shift in `roam.rs` (carts, borer and wagons in one pass with ride indices remapped and
  the bench cleared, tubes, ore hoppers, boilers, works and casts, ropes, ladders, lifts, weights, gates, scales;
  dropped at the edge as the row slide does, agreed with production), plus grid, stress, logic and air
  `shift_cols`. The player's body keeps its speed through the slide; the camera stays on the same rock.
* **Stream:** a fresh tile is shored on its own with 16 m above, 8 m below and 16 m each side of context, so it
  is the same from any route (`fresh_ground_is_the_same_from_any_side`). Kept built: 3 slides above and below,
  2 tile columns each side (`AHEAD_X`), ordered by how far the player must go before a slide needs each tile
  (straight-line order starved the next column east behind the tiles over the new column). On the web the game
  now spends up to 6 ms a frame on slices (`PREFETCH_MS`; one slice a frame could not keep up with a walk).
* **Saves:** v12 writes `ox`, world columns for opened chests, clues and the spawn, and the tile list
  (`t<x>_<y>_<hash>` keys). v≤11 full-width bands are cut into tiles on load (`band::split_legacy`); a real
  v11 save from the old build is the fixture (`tests/data/v11_cpm8.bin`, `v11_save_still_loads`).

### Hitch numbers (`digbench`, seed 4242, single thread CPU ms, game window)
| route (300 m, 6 m/s) | mean | p99 | worst | worst slide |
|---|---|---|---|---|
| east, worker, paced 60 Hz (`--realtime`) | 3.15 | 21.2 | 67.9 | 51.0 |
| west, worker, paced 60 Hz | 4.18 | 30.3 | 57.0 | 38.7 |
| east, web slices, paced 60 Hz (`--sliced --realtime`) | 10.04 | 31.6 | 64.6 | 64.6 |
| east, worker, unpaced | 2.55 | 22.2 | 58.2 | 58.2 |
| west, worker, unpaced | 2.83 | 24.5 | 63.3 | 20.7 |
| down 200 m, worker, unpaced | 5.28 | 77.1 | 101.6 | 35.8 |
Before the need-ordering, slide frames were 200-375 ms (worker) and 670-790 ms (slices): tiles were built in
the slide. Baseline on main `c2dd000` (unpaced, worker): `down` p99 70.9, worst 95.0, worst slide 34.3; `side`
(200 m between the old bedrock walls, no slides) p99 18.5, worst 60.9. So the worst frames left are the same
non-slide ticks main already has (stress passes), and a sideways slide costs about what a slide down does.

## Phase 2 polish (main `c65aa80`)
A slide down left the player's float body behind: `step_player` saw the mole's cell move and reset the body
(speed 0, `fall_from` = here, grounded). A fall that crossed a slide stopped dead in mid-air (120 → 15 cells/s
in `a_fall_carries_through_a_slide_down`) and the height fallen before the slide never counted at the
landing. `World::shift` now carries the body as `shift_x` does (`py`, `ppy`, `fall_from` move with the rock;
`sync` follows the mole).

## Biomes v2 (Hampus 03:41 "Kör biomerna härnäst"; branch `refine/biomes-v2`, successor of draft #10)
Redone on main `c3942b1` rather than rebased (#10's branch is pushed and predates infinite-x and the settled ground;
a rebase would need a force-push). Same design as #10: `Gen::biome_of(x, wy)` (2D: the column's leaning bent by
depth, edges wandering 15 m), five regions (Loamlands, Karst caverns, Granite crags, Fungal hollows, Ashlands) each
with strata offset, cave openness, fluid pockets, host rock, lenses, growth and natural lining; `Wall::Mycel`.
* **Under infinite x:** everything takes world columns (`i64`); the leaning is `biome_x`, defined forever east and
  west. Tests sample from a home width west of home to two east of it, and
  `regions_line_the_tiles_the_window_brings_in` checks tiles slid in from the west and from below.
* **With the settled ground:** a region's extra lenses (crags 0.04, karst 0.02, `Biome::lens`) fade out with
  `settled_at`, so the burrow's surroundings keep their lens rule (the first try failed `settled_ground`: 1.52 % near,
  seed 11 2.17 %).
* **The scenario flake** (why #10 stayed a draft): scenario `crumbled` was `World::crumbled`, world-wide. The
  generated world round the frame settles a cell or two (here a moss and a basalt cell 42 m east of the map, 2 s
  in), which failed 18 scenarios once the generator changed round the default seed's burrow, and made them flaky
  under rayon's run-to-run differences. It now counts the map and its frame only (`World::crumble_box`,
  `crumbled_in`; kept through slides and the harness's save/load). `tree_regrow`, `water_levels` and three more:
  8 of 8 parallel runs under load pass.
* **Hitches** (`digbench`, seed 4242, worker, paced 60 Hz; main `c3942b1` in brackets): down 200 m p99 67.5
  (67.0), worst slide 54.7 (62.7); west 300 m p99 53.1 (47.2), worst slide 49.0 (49.9); east 300 m p99 371.5
  (384.6), worst slide 83.7 (110.9). Slides are as cheap as on main. The east route's slow ticks are not
  slides on either: the bench's 4 m-tall tunnel brings ~44 000 cells of roof down and each stress pass then
  costs ~350 ms. That is main's physics (#81), passed to p2, not changed here.
* **Art is ui-graphics' (p3):** only a `Biome::Fungal` arm in `art::biome_tint` so the game builds. API for them:
  `Biome` (+ `name`, `NBIOME`), `Gen::biome_of(x, wy)` in world cells, `Wall::Mycel` (colour and pattern in walls.rs).

## Open / for others
* **p2:** the parallel CA (rayon) isn't deterministic on the first run after a rebuild. Seen on
  `digging_straight_down_never_kills` seed 6 (a limestone clump falls into a flooded shaft onto the mole on some runs),
  and on `tree_regrow` / `water_levels` on the biomes branch. Repeat runs of the same binary pass.
* Biomes v1 (PR #10) is parked behind this; it will be rebased onto it.
