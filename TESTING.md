# Testing Molehill

Everything below runs headless. `molehill-sim` has no Bevy dependency, so these need no window, GPU or audio. All commands run from `engine/`.

## 1. Unit and integration tests

```bash
cargo test --release -p molehill-sim          # about 50 s; build the release profile, debug is very slow
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
explode 70 5 -2 3       // TNT-strength blast at (dx,dy) with radius r at tick T
place 5 4 0 WoodPillar  // at tick T place a block item (WoodPillar, Brick, IronBeam) at (dx,dy), as a left click; a refusal prints NOTE
device 5 12 -1 Tnt      // place a device (logic::Dev name: Lever, Tnt, And, Tx, Rx, Lamp, Fuse, ...) in the 1 m block at (dx,dy)
use 10 12 -1            // right mouse on the device there: flip a lever, light TNT or a fuse, step a channel
pickup 40 12 -1         // Ctrl+right mouse: take the device and copperline there back into the pack
wire 4 1 -1 13 -1       // lay copperline from (dx,dy) to (dx2,dy2), one per 1 m block (a single block if only one point)
saveload 60             // save the world to bytes and load it back (checks persistence mid-scenario)
dump 50                 // print the JSON state after tick T
ascii 80                // print the map region after tick T
expect 60 rise >= 4.5   // assertion after tick T: <var> <op> <number>, ops: < <= > >= == !=
```

The map is stamped with its top-left so that `@` lands on the world spawn. A 20-cell granite frame is put around it, so the generated world can't leak in. Disasters are off. Rows shorter than the widest row are padded with `#`.

Map legend: `.` empty, `#` granite, `B` bedrock, `r` rock, `d` dirt, `c` clay, `s` sand, `g` gravel, `w` water, `l` lava, `o` coal, `i` iron ore, `b` wood beam, `k` stone brick, `I` iron beam (`~` steam, `?` other, only in output).

**Assertion variables:**

| variable | meaning |
|---|---|
| `px` `py` `vx` `vy` | Player body (cells; py is the feet). Velocities are in cells/s; +y is down |
| `dx` `dy` | Offset from the start position (dy < 0 means higher) |
| `rise` | Highest point reached so far, measured above the start |
| `grounded` `cling` | 0 or 1; cling is -1 (left wall), 0 or 1 (right wall) |
| `hp` `breath` `deaths` | Mole vitals |
| `crumbled` | Cells that have crumbled (world counter) |
| `count:<Mat>` | Cells of that material inside the map rectangle, e.g. `count:Water` |
| `is:DX,DY,<Mat>` | 1 if the cell at an offset from `@` is that material |
| `inv:<Item>` | Inventory count |
| `dev:DX,DY,<Dev>` | 1 if the 1 m block at that offset holds that device |
| `lit:DX,DY` / `on:DX,DY` / `wire:DX,DY` | The block's fuse is burning / its device is powered / it has copperline |
| `ev:<Event>` | How many of that event (Explode, Click, Place, ...) happened so far |

Devices sit on a 1 m (4×4-cell) block grid. The spawn cell is block-aligned, so blocks start at offsets that are multiples of 4.

The JSON dump includes the tick (the world's absolute tick, a few ticks ahead of the scenario tick), player state, non-zero inventory, material counts in the map rectangle, and events as `"tick:Event"`.

### Included scenarios

| file | checks |
|---|---|
| `run_jump.scn` | Top speed in under 8 ticks; full-jump height 4.5-9 cells; lands |
| `coyote.scn` / `coyote_late.scn` | A jump 4 ticks after leaving a ledge works; 12 ticks after, it doesn't |
| `wall_climb.scn` | Cling to a wall, then climb more than 6 cells with W |
| `wall_jump.scn` | Space while clinging launches you away from the wall |
| `dig_tiers.scn` | The pickaxe clears rock within 40 ticks; the bore within 10 |
| `water_flood.scn` | Mining a dirt plug floods the room below |
| `tnt_collapse.scn` | A blast removes rock, and the sand cap falls |
| `cave_stability.scn` | A small granite cave shows no crumbling over 600 ticks |
| `pillar_holds.scn` | A wood pillar built cell by cell in a granite room stays rigid |
| `pillar_holds_roof.scn` / `roof_needs_pillar.scn` | Pillars every ~1 m save a 6 m dirt roof that caves in without them |
| `pillar_on_sand.scn` | A pillar built on a sand floor rests on it |
| `beam_spans_gap.scn` | A wood beam bridging a 3.5 m gap high in a cave stays put |
| `tnt_inert.scn` | Placed TNT is inert; right mouse lights a 1.5 s fuse, then it blows |
| `tnt_lever.scn` / `tnt_thrum.scn` | Remote detonation: a lever on copperline, or a thrummer knocking through 2 m of granite to a listener |
| `tnt_defuse.scn` / `tnt_douse.scn` | A lit charge picked up is defused (and comes back unlit); water puts a fuse out |
| `knots.scn` | Truth tables of the both/either/turn/odd/catch knots, swapped in place |
| `door.scn` | A lever opens a door; it waits for the mole to clear the doorway before shutting; no door on yourself |
| `plate.scn` | A pressure plate above the floor powers a lamp while the mole stands on it |
| `saveload_devices.scn` | A thrown lever, a lit lamp and a burning TNT fuse survive save/load |
| `beam_overload.scn` | A wood beam under a sand heap breaks into wood splinters; the same iron span holds |

### Writing a new scenario (for agents)

1. Copy the closest `.scn`, then draw the map. Leave 3 empty cells above `@` for the mole's body.
2. Add `dump`/`ascii` lines at the ticks you care about and run the file alone. Read the JSON to pick thresholds; for example, find the tick where `grounded` flips.
3. Turn what you saw into `expect` lines, then remove the noisy dumps.
4. Run the whole folder. Every scenario must PASS before you commit.

Tuning constants for movement live in one place: `TUNE` in `sim/src/world.rs`. After changing them, rerun both the tests and the scenarios.

## 3. Visual check (optional)

`cargo build --release -p molehill`. On a headless Linux box, run it under `Xvfb :99` with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, wait about 8 s, and grab one frame with `ffmpeg -f x11grab`. The game opens on the main menu; press Enter to start a New Holt. One screenshot is enough; don't record videos unless asked.
