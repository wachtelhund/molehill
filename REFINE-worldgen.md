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

## Open / for others
* **p2:** the parallel CA (rayon) isn't deterministic on the first run after a rebuild. Seen on
  `digging_straight_down_never_kills` seed 6 (a limestone clump falls into a flooded shaft onto the mole on some runs),
  and on `tree_regrow` / `water_levels` on the biomes branch. Repeat runs of the same binary pass.
* Biomes v1 (PR #10) is parked behind this; it will be rebased onto it.
