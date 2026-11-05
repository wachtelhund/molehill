# Testing Molehill

Everything below runs headless. `molehill-sim` has no Bevy dependency, so these need no window, GPU or audio. All commands run from `engine/`.

## 1. Unit and integration tests

```bash
cargo test --release -p molehill-sim          # about 10 s; build the release profile, debug is very slow
cargo test --release -p molehill-sim --test sim -- jump coyote   # filter by name
```

`sim/tests/sim.rs` covers stress and collapse, fluids, digging tiers, and movement. The movement tests check jump height, the variable short hop, coyote time, wall cling, climbing and wall jumping. Helpers: `arena()` makes a cleared granite box around spawn, and `press(&mut w, n, |input| ...)` steps n ticks with a given input.

## 2. Scenario harness (`sim/examples/scenario.rs`)

```bash
cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/*.scn
cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/wall_jump.scn --json   # plus a final JSON state
```

Output is line-oriented and easy to grep:

* `PASS <file> (n passed, m failed, T ticks)` / `FAIL <file> ...`
* `  FAIL <file>:<line> tick T: <var> = <value>, expected <op> <n>`: one line per failed assertion.
* `DUMP <file> {json}`: a state snapshot from `dump T`.
* `ASCII <file> tick T` followed by the map region (`M` marks the mole body; legend below).
* The last line is `N scenarios, K failed`. The process exits with 1 if anything failed.

It is deterministic, so the same file always gives the same output. The sim runs at a fixed 60 ticks per second.

### File format (`.scn`)

The format is line-based. `//` starts a comment, and ticks start at 1.

```
// what this checks
seed 11                 // optional world seed (default 11); the world is generated and the map stamped over it
map                     // ASCII map, ends at `end`. '@' = the mole's lowest body cell (feet sit on the cell below)
##########
#........#
#.@......#
##########
end
give Tnt 5              // set inventory count (item names as in craft::Item, case-insensitive)
hold 1-30 right         // hold keys for ticks A-B: left right jump up down (several allowed)
hold 31-40 right jump
tool 41 3               // from tick 41 dig with tool tier 0 claws, 1 pickaxe, 2 drill, 3 bore
mine 41-60 3 0          // hold mine on cell (dx,dy) relative to '@' during ticks A-B
place 12 6 0 WoodPillar // at tick T place a round brush of that block centred at (dx,dy) (WoodPillar, Brick, IronBeam)
brush 10 2.5            // from tick T the brush radius in cells for digging and placing (0 = item default; clamped per item)
explode 70 5 -2 3       // TNT-strength blast at (dx,dy) with radius r at tick T
put 2 8 0 WoodPillar    // put exactly one block cell at (dx,dy) at tick T, no reach check (scripted building)
dump 50                 // print the JSON state after tick T
ascii 80                // print the map region after tick T
expect 60 rise >= 4.5   // assertion after tick T: <var> <op> <number>, ops: < <= > >= == !=
```

The map is stamped with its top-left so that `@` lands on the world spawn. A 20-cell granite frame is put around it, so the generated world can't leak in. Disasters are off. Rows shorter than the widest row are padded with `#`.

Map legend: `.` empty, `#` granite, `B` bedrock, `r` rock, `d` dirt, `c` clay, `s` sand, `g` gravel, `w` water, `l` lava, `o` coal, `i` iron ore, `a` basalt, `b` wood beam, `I` iron beam, `k` stone brick (`~` steam, `?` other, only in output).

**Assertion variables:**

| variable | meaning |
|---|---|
| `px` `py` `vx` `vy` | Player body (cells; py is the feet). Velocities are in cells/s; +y is down |
| `dx` `dy` | Offset from the start position (dy < 0 means higher) |
| `rise` | Highest point reached so far, measured above the start |
| `grounded` `cling` | 0 or 1; cling is -1 (left wall), 0 or 1 (right wall) |
| `hp` `breath` `deaths` | Mole vitals |
| `crumbled` | Cells that have crumbled (world counter) |
| `awake` | CA chunks overlapping the map that are still awake (0 = everything there has settled) |
| `count:<Mat>` | Cells of that material inside the map rectangle, e.g. `count:Water` |
| `is:DX,DY,<Mat>` | 1 if the cell at an offset from `@` is that material |
| `inv:<Item>` | Inventory count |
| `static:<Mat>` | Like `count:` but only cells that are fixed in place (not loose debris) |
| `util:DX,DY` | Stress utilisation of the cell at an offset from `@` (1.0 = at capacity; above that it cracks) |

The JSON dump includes the tick (the world's absolute tick, a few ticks ahead of the scenario tick), player state, non-zero inventory, material counts in the map rectangle, and events as `"tick:Event"`.

### Included scenarios

| file | checks |
|---|---|
| `run_jump.scn` | Top speed in under 8 ticks; full-jump height 4.5-9 cells; lands; the 11 m granite ceiling doesn't crumble |
| `coyote.scn` / `coyote_late.scn` | A jump 4 ticks after leaving a ledge works; 12 ticks after, it doesn't |
| `wall_climb.scn` | Cling to a wall, then climb more than 6 cells with W |
| `wall_jump.scn` | Space while clinging launches you away from the wall |
| `dig_tiers.scn` | The pickaxe clears rock within 40 ticks; the bore within 10 |
| `water_flood.scn` | Mining a dirt plug floods the room below |
| `tnt_collapse.scn` | A blast removes rock, and the sand cap falls |
| `cave_stability.scn` | A small granite cave shows no crumbling over 600 ticks |
| `hall_sag_arrests.scn` | An over-wide granite hall sags out of midspan and the cave-in stops (< 60 cells) |
| `pillar_holds.scn` / `pillar_on_sand.scn` | Placed wood pillars stay rigid, also when standing on a sand floor |
| `pillar_wide_hall.scn` | A lone pillar in a wide, sound hall isn't crushed |
| `roof_unpropped.scn` / `roof_propped.scn` | A wide clay roof caves in; four pillar columns hold it |
| `water_levels.scn` / `lava_flows.scn` | A heap levels to the right depth (water by tick 120, lava much later), nothing lost, then sleeps |
| `sand_sinks.scn` / `sand_floats_lava.scn` | Sand sinks through water and piles on the bottom; it floats on lava |
| `player_lava_sink.scn` | Running onto a lava pool: you sink in, wade slowly, burn and die |
| `chunk_sinks_in_water.scn` / `wood_floats.scn` | An unsupported rock/ore clump in a pool gives way fast and its debris sinks; wood debris rises and floats |
| `lava_meets_water.scn` | Lava on water quenches to basalt with steam; the steam clears and the chamber sleeps |
| `beam_span.scn` / `beam_overload_sand.scn` | An iron beam bridges an 8-cell gap; a sand-loaded wood beam breaks into loose debris |
| `pillar_holds_roof.scn` | Placed pillars (full height, a short stub, a ledge off a wall) stay rigid |
| `pillar_on_rubble.scn` | Pillars, a ledge and bricks built on a sand floor stay rigid; no new sand appears |
| `beam_spans_gap.scn` | Iron beams placed across a pit stay put, and the mole runs across them |
| `beam_overload.scn` | A long wood span under deep sand cracks and falls as wooden debris |
| `brush_place.scn` | A radius-2 brick brush fills the open part of the disc and uses items 4 cells to one |
| `brush_clamp.scn` | Brush radius is clamped per item (iron beam max 2, pillar min 0.5, pickaxe max 3) |

### Writing a new scenario (for agents)

1. Copy the closest `.scn`, then draw the map. Leave 3 empty cells above `@` for the mole's body.
2. Add `dump`/`ascii` lines at the ticks you care about and run the file alone. Read the JSON to pick thresholds; for example, find the tick where `grounded` flips.
3. Turn what you saw into `expect` lines, then remove the noisy dumps.
4. Run the whole folder. Every scenario must PASS before you commit.

Tuning constants for movement live in one place: `TUNE` in `sim/src/world.rs`. After changing them, rerun both the tests and the scenarios.

Zoom limits and brush ranges also have unit tests in `sim/tests/sim.rs` (`zoom_is_clamped_both_ways`, `brush_sizes_clamp_per_item`), as do tooltip texts (`every_thing_has_a_lore_safe_tooltip`).

## 2b. Settling / performance check

`cargo run --release -p molehill-sim --example wakediag [seed]` generates a world, runs 1200 ticks and reports how many CA chunks are still awake and what is still changing. A settled world should be close to 0 awake chunks.

## 3. Visual check (optional)

`cargo build --release -p molehill`. On a headless Linux box, run it under `Xvfb :99` with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, wait about 8 s, and grab one frame with `ffmpeg -f x11grab`. The game opens on the main menu; press Enter to start a New Holt. One screenshot is enough; don't record videos unless asked.

Native builds also take a frame by themselves: `molehill --fresh --seed=4242 --shot=out.bmp [--shot-at=150] [--shot-ui=pause|inv|help|dev] [--shot-walk]` starts a holt, writes one BMP and quits. On macOS the window must be visible on screen: launched from a background shell the surface reads back empty.
