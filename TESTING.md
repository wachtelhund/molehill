# Testing Molehill

Everything below runs headless. `molehill-sim` has no Bevy dependency, so these need no window, GPU or audio. All commands run from `engine/`.

## 1. Unit and integration tests

```bash
cargo test --release -p molehill-sim          # about 1 min; build the release profile, debug is very slow
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
dig 41-60 3 -1          // same, but (dx,dy) is relative to the mole's current cell (feet row), so it follows the mole
explode 70 5 -2 3       // TNT-strength blast at (dx,dy) with radius r at tick T
place 5 4 0 WoodPillar  // at tick T place a block item (WoodPillar, Brick, IronBeam) at (dx,dy), as a left click; a refusal prints NOTE
device 5 12 -1 Tnt      // place a device (logic::Dev name: Lever, Tnt, And, Tx, Rx, Lamp, Fuse, ...) in the 1 m block at (dx,dy)
use 10 12 -1            // right mouse on the device there: flip a lever, light TNT or a fuse, step a channel
pickup 40 12 -1         // Ctrl+right mouse: take the device and copperline there back into the pack
wire 4 1 -1 13 -1       // lay copperline from (dx,dy) to (dx2,dy2), one per 1 m block (a single block if only one point)
saveload 60             // save the world to bytes and load it back (checks persistence mid-scenario)
slide 60                // slide the window away until only its bottom band is left, and back (things go into bands and return)
put 2 8 0 WoodPillar    // put exactly one block cell at (dx,dy) at tick T, no reach check (scripted building)
throw 2 Stone 1 -1 1    // throw one from the pack toward direction (dx,dy), power 0..1 of a full throw (TNT goes out lit)
drop 1 Wood 26 -12 0 0  // a loose item centred on tile (dx,dy), optionally moving at (vx,vy) tiles/s
dump 50                 // print the JSON state after tick T
ascii 80                // print the map region after tick T
expect 60 rise >= 4.5   // assertion after tick T: <var> <op> <number>, ops: < <= > >= == !=
```

**Scenarios are written in tiles of 0.25 m**, the original cell size. Each map character is stamped as a K × K block of cells (K = `mat::S`, 2 at 8 cells/m). Every offset and radius in a directive is in tiles, and positions, speeds, `count:`/`static:`/`crumbled` and the ASCII dump are reported back in tiles. A scenario therefore means the same thing in metres at any resolution. `is:` is true when most of the tile is that material; `util:`/`strain:` take the tile's maximum. `put` fills the whole tile.

The map is stamped with its top-left so that `@` lands on the world spawn (the mole stands centred on the `@` tile, feet on its bottom edge). A 20-tile granite frame is put around it, so the generated world can't leak in. Disasters are off. Rows shorter than the widest row are padded with `#`.

Map legend: `.` empty, `#` granite, `B` bedrock, `r` rock, `d` dirt, `c` clay, `s` sand, `g` gravel, `w` water, `l` lava, `o` coal, `i` iron ore, `a` basalt, `b` wood beam, `I` iron beam, `k` stone brick (`~` steam, `?` other, only in output).

**Assertion variables:**

| variable | meaning |
|---|---|
| `flash` `shake` | Hit-flash time left (s); view-shake strength (decays each tick) |
| `step` | Distance the body moved this tick (tiles); catches teleport pops that a velocity check misses |
| `items:Name` | How many of that item lie loose |
| `ix:Name` `iy:Name` `ivx:Name` `ivy:Name` | The first loose item of that kind: centre in tiles from the `@` tile's centre (iy 0 = resting on the `@` floor), velocity in tiles/s |
| `irest:Name` `ilit:Name` | 1 if that item is asleep (lying still); 1 if it's a lit charge |
| `px` `py` `vx` `vy` | Player body in tiles from the map's top-left (py is the feet). Velocities are in tiles/s; +y is down |
| `dx` `dy` | Offset from the start position (dy < 0 means higher) |
| `rise` | Highest point reached so far, measured above the start |
| `grounded` `cling` | 0 or 1; cling is -1 (left wall), 0 or 1 (right wall) |
| `hp` `breath` `deaths` | Mole vitals |
| `crumbled` | Tiles' worth of cells that have crumbled (world counter) |
| `awake` | CA chunks overlapping the map that are still awake (0 = everything there has settled) |
| `count:<Mat>` | Tiles' worth of that material inside the map rectangle (cells / K²), e.g. `count:Water` |
| `is:DX,DY,<Mat>` | 1 if the cell at an offset from `@` is that material |
| `inv:<Item>` | Inventory count |
| `static:<Mat>` | Like `count:` but only cells that are fixed in place (not loose debris) |
| `util:DX,DY` | Stress utilisation of the cell at an offset from `@` (1.0 = at capacity; above that it cracks) |
| `strain:DX,DY` | The hover tooltip's strain level for that cell (`tips::strain_at`): 0 idle, 1 bearing, 2 straining, 3 overloaded, 4 cracking |
| `dev:DX,DY,<Dev>` | 1 if the 1 m block at that offset holds that device |
| `lit:DX,DY` / `on:DX,DY` / `wire:DX,DY` | The block's fuse is burning / its device is powered / it has copperline |
| `ev:<Event>` | How many of that event (Explode, Click, Place, ...) happened so far |

Devices sit on a 1 m (4×4-cell) block grid. The spawn cell is block-aligned, so blocks start at offsets that are multiples of 4.

The JSON dump includes the tick (the world's absolute tick, a few ticks ahead of the scenario tick), player state, non-zero inventory, material counts in the map rectangle, and events as `"tick:Event"`.

### Included scenarios

| file | checks |
|---|---|
| `air_turn.scn` | Reversing in mid-air takes under 5 ticks; letting go in the air coasts instead of braking |
| `fall_hurt.scn` | A 30-cell fall hurts, flashes the mole and jolts the view; a normal jump landing doesn't |
| `step_up.scn` | Running into a 1-cell then a 2-cell step hops up them at full speed |
| `mantle.scn` | Climb a 9-cell wall with W and roll over the top onto the ledge |
| `run_jump.scn` | Top speed in under 8 ticks; full-jump height 4.5-9 cells; lands; the 11 m granite ceiling doesn't crumble |
| `coyote.scn` / `coyote_late.scn` | A jump 4 ticks after leaving a ledge works; 12 ticks after, it doesn't |
| `wall_climb.scn` | Cling to a wall, then climb more than 6 cells with W |
| `wall_jump.scn` | Space while clinging launches you away from the wall |
| `wall_jump_away.scn` | Pressing away plus Space, or Space within 0.1 s of letting go, still wall-jumps; later it doesn't |
| `no_ceiling_climb.scn` | Jumping into a low ceiling while holding sideways and `W` never clings; the mole falls back down |
| `throw_arc.scn` | A full throw flies a ~6 m arc, bounces, skids to rest; a weaker one falls shorter; walking over them picks them up |
| `item_bounces_rests.scn` | A stone bounces off granite but not off dirt; both fall asleep on the floor without drifting |
| `item_sinks_floats.scn` | Wood floats in water, an iron ingot sinks straight to the bottom, a grub sinks slowly |
| `thrown_tnt_explodes_at_landing.scn` | A thrown lit charge flies, rolls and blows a crater where it stops, far from the thrower; one lobbed into water is doused |
| `sprint_speed.scn` | `Shift` sprint reaches 1.6x run speed in 2 ticks, stops at once, and a sprint jump carries further |
| `climb_smooth.scn` | Climbing a ragged wall (a notch, an outcrop overhead) holds a constant speed every tick, easing round the outcrop |
| `climb_mantle.scn` | Climbing over the top with only `W`: never moves down, never moves more than 0.25 cell in a tick, ends on the ledge |
| `dig_tiers.scn` | The pickaxe clears rock within 40 ticks; the bore within 10 |
| `water_flood.scn` | Mining a dirt plug floods the room below |
| `tnt_collapse.scn` | A blast removes rock, and the sand cap falls |
| `cave_stability.scn` | A small granite cave shows no crumbling over 600 ticks |
| `hall_sag_arrests.scn` | An over-wide granite hall sags out of midspan and the cave-in stops (< 60 cells) |
| `pillar_holds.scn` / `pillar_on_sand.scn` | Placed wood pillars stay rigid, also when standing on a sand floor |
| `pillar_wide_hall.scn` | A lone pillar in a wide, sound hall isn't crushed |
| `roof_unpropped.scn` / `roof_propped.scn` | A wide clay roof caves in; four pillar columns hold it |
| `water_levels.scn` / `lava_flows.scn` | A heap levels to the right depth (water by tick 120, lava much later; its thin sheet crusts to basalt), then sleeps |
| `sand_sinks.scn` / `sand_floats_lava.scn` | Sand sinks through water and piles on the bottom; it floats on lava |
| `player_lava_sink.scn` | Running onto a lava pool: you sink in, wade slowly, burn and die |
| `chunk_sinks_in_water.scn` / `wood_floats.scn` | An unsupported rock/ore clump in a pool gives way fast and its debris sinks; wood debris rises and floats |
| `lava_cools.scn` | A lava splash on the floor sets into basalt; a deep pool stays molten |
| `lava_meets_water.scn` | Lava on water quenches to basalt with steam; the steam clears and the chamber sleeps |
| `beam_span.scn` / `beam_overload_sand.scn` | An iron beam bridges an 8-cell gap; a sand-loaded wood beam breaks into loose debris |
| `pillar_holds_roof.scn` | Placed pillars (full height, a short stub, a ledge off a wall) stay rigid |
| `pillar_on_rubble.scn` | Pillars, a ledge and bricks built on a sand floor stay rigid; no new sand appears |
| `beam_spans_gap.scn` | Iron beams placed across a pit stay put, and the mole runs across them |
| `beam_overload.scn` | A long wood span under deep sand cracks and falls as wooden debris |
| `dirt_cavein_chokes.scn` | A void under a deep dirt bank caves in but bulking rubble chokes it off (< 600 cells, then sleeps) |
| `buried_under_rock.scn` / `buried_in_sand.scn` | A mole buried under a rock slab stays put (no popping through rock); under loose sand it wriggles out |
| `brush_place.scn` | A radius-2 brick brush fills the open part of the disc and uses items 4 cells to one |
| `hover_strain.scn` | The hover warns of an overloaded wood span (cracking) before it falls; floor and loose debris read idle |
| `brush_clamp.scn` | Brush radius is clamped per item (iron beam max 2, pillar min 0.5, pickaxe max 3) |
| `tnt_chain.scn` | A blast sets off a second charge 3.16 m away (charges chain up to 1.5 m past the 2.25 m radius) |
| `grain_in_body.scn` | A sand grain lying inside the mole's body (low tunnel, no wriggle room) is shoved aside instead of wedging it |
| `pillar_built_in_room.scn` | A wood pillar built cell by cell in a granite room stays rigid |
| `pillars_rock_roof.scn` / `roof_needs_pillar.scn` | Two pillars at the third points save a 12 m rock roof that caves in (bending) without them |
| `pillar_sand_floor.scn` | A pillar built on a sand floor rests on it |
| `beam_bridge_ledges.scn` | A wood beam bridging a 3.5 m gap between ledges high in a cave stays put |
| `tnt_inert.scn` | Placed TNT is inert; right mouse lights a 1.5 s fuse, then it blows |
| `tnt_lever.scn` / `tnt_thrum.scn` | Remote detonation: a lever on copperline, or a thrummer knocking through 2 m of granite to a listener |
| `tnt_defuse.scn` / `tnt_douse.scn` | A lit charge picked up is defused (and comes back unlit); water puts a fuse out |
| `knots.scn` | Truth tables of the both/either/turn/odd/catch knots, swapped in place |
| `door.scn` | A lever opens a door; it waits for the mole to clear the doorway before shutting; no door on yourself |
| `plate.scn` | A pressure plate above the floor powers a lamp while the mole stands on it |
| `saveload_devices.scn` | A thrown lever, a lit lamp and a burning TNT fuse survive save/load |
| `band_keeps_devices.scn` | The same build survives its rows leaving the window and coming back, also across a save taken while they are away |
| `beam_overload_iron.scn` | A wood beam under a sand heap breaks into wood splinters; the same iron span holds |
| `beams_placed_across_pit.scn` | Wood beams placed one by one across an 8-wide pit, and an iron stub, stay put |
| `pillar_in_cave.scn` | A wood pillar stands while half-built and once it reaches the roof of a 40-wide cave |
| `clay_roof_falls.scn` / `pillars_hold_clay_roof.scn` | A 20-wide clay roof falls alone; four placed pillars hold it |
| `beam_overload_wide.scn` | A 30-wide wood beam under a sand heap breaks and the sand pours through |
| `shaft_jump.scn` | Jumping half a cell off-centre under a 2-wide shaft slides round the corner and goes up |
| `tunnel_mouth.scn` | Jumping at a tunnel mouth 3 cells up a wall slots the mole in on the way up |
| `dig_tunnel.scn` / `dig_tunnel_high.scn` | Walking right while pointing at the rock ahead (foot / head height) tunnels through |

### Writing a new scenario (for agents)

1. Copy the closest `.scn`, then draw the map. Leave 3 empty cells above `@` for the mole's body.
2. Add `dump`/`ascii` lines at the ticks you care about and run the file alone. Read the JSON to pick thresholds; for example, find the tick where `grounded` flips.
3. Turn what you saw into `expect` lines, then remove the noisy dumps.
4. Run the whole folder. Every scenario must PASS before you commit.

Tuning constants for movement live in one place: `TUNE` in `sim/src/world.rs`. After changing them, rerun both the tests and the scenarios.

Zoom limits and brush ranges also have unit tests in `sim/tests/sim.rs` (`zoom_is_clamped_both_ways`, `brush_sizes_clamp_per_item`), as do tooltip texts (`every_thing_has_a_lore_safe_tooltip`) and world hover text from `sim/src/tips.rs` (`world_hover_explains_cells`).

## 2b. Settling / performance check

`cargo run --release -p molehill-sim --example wakediag [seed] [ticks]` generates a world, runs 1200 ticks (or `ticks`) and reports how many CA chunks are still awake and what is still changing. A settled world should be close to 0 awake chunks (at 8 cells/m a fresh world takes about 4800 ticks to settle).

`cargo run --release -p molehill-sim --example stepbench [seed] [ticks]` times `World::step` on the game-sized window from a fresh holt: mean/p50/p95/p99/max per tick, the average per tick class (stress passes every 6 ticks, air every 15), a full stress solve and a light pass. Add `--no-default-features` for single-threaded numbers, which are closer to the web build.

`cargo run --release -p molehill-sim --example genbench [seed]` times world generation: fill, one full stress solve, stabilisation, and `World::new` in total.

**Web frame time:** serve `dist/` (`python3 -m http.server`) and measure `requestAnimationFrame` intervals in Chrome after starting a holt. At 8 cells/m on an M4: 57-58 fps steady state (p95 16.8 ms, same as the 4 cells/m build), 54-55 fps during the first ~15 s while a fresh world settles.

## 3. Visual check (optional)

`cargo build --release -p molehill`. On a headless Linux box, run it under `Xvfb :99` with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, wait about 8 s, and grab one frame with `ffmpeg -f x11grab`. The game opens on the main menu; press Enter to start a New Holt. One screenshot is enough; don't record videos unless asked.

Native builds also take a frame by themselves: `molehill --fresh --seed=4242 --shot=out.bmp [--shot-at=150] [--shot-ui=pause|inv|help|dev] [--shot-walk]` starts a holt, writes one BMP and quits. Add `--dump-art=DIR` to also write every sprite (8×) and the composited world around the mole (`view.bmp`); these are CPU-side, so they work even where the window frame doesn't. On macOS the window must be visible on screen: launched from a background shell the surface reads back empty.
