# Architecture

Molehill is written in **Rust** with **Bevy 0.20**. The code is split so that the game's rules can be run, tested and reasoned about without any engine at all.

## Repository layout

| path | what |
|---|---|
| `engine/sim` | `molehill-sim`: the whole simulation in pure Rust. No Bevy, no I/O except save bytes |
| `engine/game` | `molehill`: the Bevy front-end (input, camera, rendering, sprites, HUD, menus, sound) |
| `engine/web`, `engine/build-web.sh` | the wasm build, written to `dist/` (gitignored) |
| `site/` | a committed copy of the web build, for local play |
| `engine/shots` | screenshots |
| `legacy/` | the TypeScript v2 prototype that the v3 engine was ported from, plus screenshot scripts |
| `docs/wiki/` | this wiki |

## The split: sim and engine

```
┌──────────────────── molehill (Bevy) ─────────────────────┐
│ keys / mouse ─▶ Input ┐                  ┌─▶ chunk textures, parallax, sprites, HUD, sound
│                       ▼                  │
│            ┌──── molehill-sim ───────────┴──┐
│            │ World::step()  (60 Hz, fixed)  │   no Bevy, no window, deterministic
│            │  grid CA · stress · player ·   │
│            │  moles · logic · disasters ·   │
│            │  air · farms · window slide    │
│            └────────────────────────────────┘
└───────────────────────────────────────────────────────────┘
          ▲
  tests / scenario harness / examples drive the same World headless
```

The rule is simple: **gameplay goes in the sim.** If it changes what happens in the world, it belongs in `molehill-sim`, where a scenario can check it. The game crate reads `World` and writes `World::input`, and nothing more. Text the player reads about the world (tooltips for items, devices and materials, and the world hover text in `tips.rs`) also lives in the sim, so a test can check it against the lore.

### `molehill-sim` modules

| module | role |
|---|---|
| `mat.rs` | the material table (density, strengths, dig time and tier, blast, light, colour, repose) and the thrumming wave helpers |
| `grid.rs` | cells and flags; the falling-sand automaton on 64 × 64 chunks with sleeping |
| `stress.rs` | the structural solver: overburden, load path, bending, compression, cracking |
| `gen.rs` | seeded procgen per `(seed, x, absolute row)` and the stability pass |
| `world.rs` | `World`: the step order, the player (`step_player`, `TUNE`), colony moles, digging, placing, the brush, the endless window, quests and chests |
| `logic.rs` | copperline nets, devices, knots, fuses, TNT, thrumming |
| `light.rs` | per-cell RGB light and fog of war |
| `air.rs` | stale air on an 8 × 8 block grid |
| `craft.rs` | items, stations, recipes, tool tiers, quests |
| `disasters.rs` | floods, lava surges, quakes |
| `save.rs` | binary save/load |
| `view.rs` | zoom rules shared by the settings menu, keys and wheel |

### One tick

`World::step()` runs in a fixed order:

1. the falling-sand automaton (`grid.step_ca`);
2. a stress re-solve if anything structural changed (every 6 ticks) or after reactions (every 24), then crack timers and crumbling;
3. the player (`step_player`), then colony moles every second tick;
4. hit flash and view shake from knocks;
5. logic: nets, sources, knots, thrumming links, sinks, fuses, then TNT blasts;
6. disasters; stale air every 15 ticks; farms, the window slide, quests and chests every 30.

### `molehill` (the game crate)

| file | role |
|---|---|
| `main.rs` | input, the fixed-step loop, the camera, world rendering and sprites |
| `ui.rs` | the HUD: vitals, quest line, toasts, hotbar, pack grid, tooltips, dev text |
| `menu.rs` | main and pause menus, settings, the shared controls table |
| `art.rs` | pixel-art sprites and icons built in code |
| `audio.rs` | procedural sound (web only) |

**Fixed step with interpolation.** Each frame adds `delta × 60` to an accumulator and runs `World::step()` once per whole tick, at most 3 a frame, discarding the backlog after a hitch. The mole is drawn interpolated between its last two positions, so movement is smooth at any refresh rate while the sim stays at exactly 60 Hz. Menus set `Game.paused`, which stops stepping.

**Rendering.** The window's cells are composited on the CPU into **128 chunk textures of 128 × 128 cells (16 m)** (material colour × a per-material pixel pattern (`art::mat_tex`: strata, brick bond, wood grain, rivets, ore glints) × edge shading × light, fog, cracks, designations, translucent overlays for darkness, stale air and steam), and **only chunks that changed are uploaded**. Light is recomputed every second frame over the view. The three parallax layers are GPU sprites behind the world, placed at `camera × factor`. Chunk rows beyond the simulated window are drawn as a static "far" view, so the camera can roam a little past it. The mole is 26 × 20 pixel art with idle, run, jump, fall, climb and dig poses, plus squash, stretch, landing and run dust, dig chips, water/lava splashes and torch embers for feedback.

## Native and web

| | native | web |
|---|---|---|
| build | `cargo run --release` (in `engine/`) | `./build-web.sh` → `dist/`, then copy to `site/` |
| target | Linux / macOS (x11, wayland) | `wasm32-unknown-unknown`, WebGL2, `wasm-bindgen` 0.2.130 |
| sim threads | rayon (`parallel` feature) | single-threaded (feature off) |
| saves | files next to the binary | localStorage (base64) |
| sound | silent | procedural WebAudio, starts after the first input |
| seeds and flags | `--fresh`, `--seed=`, `--shot=`, `--dump-art=`, `--scene=` | `?fresh`, `?seed=` |

Bevy is built with hand-picked features (no scenes, picking, gizmos, animation, post-processing, gamepad or image formats) to keep the wasm small. The `wasm-release` profile adds `opt-level = "z"`, full LTO, `panic = "abort"` and stripping, and `wasm-opt -Os` runs when it is installed.

Rebuild the committed web copy with:

```bash
cd engine && ./build-web.sh && cd .. && rm -rf site && cp -r dist site
```

## Saves

A save starts with the bytes `MOLEHILL`, a format version (3), followed by length-prefixed sections. Cell layers are byte run-length encoded. Materials are stored **by name** with a name table, so the material enum can grow or be reordered. Loading regenerates the world from the seed and then overwrites all mutable state: cells, the window offset and cached rows, moles, the pack, devices and fuses, farms, air and quests. Every read is bounds-checked, so a bad save gives a readable error and never panics. Newer optional sections (such as burning fuses and the death count) trail at the end, so older v3 saves still load. The game autosaves every 30 s and loads the autosave at start. `K` and `L` use a separate manual slot. Autosaves (`*.save`) are gitignored.

## Working on the code

See [`AGENTS.md`](../../AGENTS.md) for the loop: change the sim, add a scenario, run tests and scenarios, build the game, rebuild `site/` if the game changed, then commit and push. **Merges that change behaviour must update the matching page in `docs/wiki/`.**
