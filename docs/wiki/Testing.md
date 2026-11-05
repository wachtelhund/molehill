# Testing

> Full reference: [`TESTING.md`](../../TESTING.md) in the repository root. This page explains the idea and the workflow.

## Why the game can be tested without a screen

Every gameplay rule lives in `molehill-sim`, which has **no Bevy dependency**: no window, no GPU, no audio. The game crate only turns key presses into an `Input` struct and draws what the sim produces. So a test, or an AI agent, can drive the exact same `World::step()` the player does, and check the result as numbers.

The sim is also **deterministic**. Randomness is a hash of position and tick, the parallel falling-sand phases can't race, and disasters use their own random stream. The same scenario always gives the same output, so a failure is reproducible rather than flaky.

```
 .scn file ─▶ stamp map over a generated world ─▶ hold keys / mine / place / wire per tick ─▶ World::step() × N
                                                                              │
                                     PASS / FAIL lines, JSON dumps, ASCII maps ◀─ expect <var> <op> <n>
```

## The three layers

| layer | command (from `engine/`) | what it covers |
|---|---|---|
| Unit and integration tests | `cargo test --release -p molehill-sim` | stress and collapse, fluids, dig tiers, movement (jump height, short hop, coyote, cling, climb, wall-jump), zoom and brush clamps, lore-safe tooltips and world hover text, save/load, pack accounting. About 10 s; use release, because debug is very slow |
| Scenario harness | `cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/*.scn` | about 75 small, readable set pieces of gameplay with assertions |
| Crash hunts | `cargo run -p molehill-sim --no-default-features --example soak -- SEEDS TICKS`; `--example loadsave -- FILE [TICKS]` | random play on game-sized worlds (moving, digging, blocks, devices, copperline, TNT, save/load) in debug for overflow checks; loading and playing an old save |
| Benchmarks | `--example stepbench [seed] [ticks]`, `genbench [seed]`, `bandbench` | per-tick cost of `World::step` on the game window (mean, p95, p99, by tick class); where generation spends its time; the cost of a window slide, an autosave, a full save and a load |
| Settling check | `cargo run --release -p molehill-sim --example wakediag [seed]` | a generated world runs 1200 ticks; reports how many falling-sand chunks are still awake (should be close to 0) |

Both the tests and the scenarios must pass before every commit.

## Scenarios

A `.scn` file is a tiny, line-based script. An ASCII map is stamped over a generated world with `@` on the spawn point, a 5 m granite frame keeps the world out, and disasters are off. Scenarios are written in **tiles of 0.25 m**: each map character is a 2 × 2 block of cells, and offsets, radii, positions, speeds and counts are all in tiles, so a scenario means the same at any resolution:

```
// a full jump clears 4.5 tiles and lands
map
##########
#........#
#.@......#
##########
end
hold 1-30 right
hold 31-40 right jump
expect 60 rise >= 4.5
expect 120 grounded == 1
```

* **Directives** drive the mole and the world: `hold` keys over a tick range, `mine` / `dig` a cell, `tool` and `brush` to pick a tool and size, `place` / `put` blocks, `device` / `use` / `pickup` / `wire` for copperline and devices, `explode`, `give` items, `saveload` to round-trip the world mid-run, `slide` to send the scenario's rows into stored bands and back, and `throw` / `drop` to launch or place a loose item.
* **Assertions** (`expect T <var> <op> <n>`) read the player (`px`, `vx`, `rise`, `grounded`, `cling`, `hp`, `flash`…), the map (`count:Water`, `static:WoodBeam`, `is:DX,DY,Mat`, `util:DX,DY` stress utilisation, `strain:DX,DY` the hover strain level, `awake` chunks), devices (`dev:`, `on:`, `lit:`, `wire:`) loose items (`items:Stone`, position and speed `ix:` `iy:` `ivx:` `ivy:`, `irest:` asleep, `ilit:` a lit charge) and events (`ev:Explode`).
* **Output** is easy to grep: `PASS` / `FAIL` per file, one `FAIL <file>:<line> tick T: …` per broken assertion, optional `DUMP` JSON and `ASCII` maps, and a final `N scenarios, K failed` (exit code 1 on failure).

The variables, the map legend and the list of every included scenario are in [`TESTING.md`](../../TESTING.md).

## Writing one (the loop for agents)

1. Copy the closest `.scn` and draw the map. Leave 3 empty tiles above `@` for the mole's body.
2. Add `dump` / `ascii` lines at the ticks you care about and run that file alone. Read the JSON to choose thresholds, for example the tick where `grounded` flips.
3. Turn what you saw into `expect` lines and remove the noisy dumps.
4. Run the whole folder.

Feel changes are tested the same way: rise height, `vx` after N ticks, the coyote window, and how many ticks a mid-air turn takes. Every refinement run in the history added scenarios for what it changed (for example `hall_sag_arrests`, `player_lava_sink`, `tnt_thrum`, `shaft_jump`).

## Looking at it

Visual checks are optional and come last. `cargo build --release -p molehill`, then:

* `molehill --fresh --seed=4242 --shot=out.bmp [--shot-at=150] [--shot-ui=pause|inv|help|dev] [--shot-walk]` starts a holt, writes one frame and quits;
* `--dump-art=DIR` also writes every sprite at 8× and the composited world around the mole, all CPU-side, so it works even where the window can't be captured.

On macOS the window has to be visible on screen to capture a frame. On headless Linux, use Xvfb with the lavapipe Vulkan driver. One screenshot is enough, and only for visual changes.
