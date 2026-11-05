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
| `mat.rs` | the material table, the one place material behaviour is defined: density, strengths, dig time and tier, blast, light, colour, repose, heat and ignition, cooling, liquid flow, lifetimes, category flags and the tooltip line; plus the thrumming wave helpers |
| `grid.rs` | cells and flags; the falling-sand automaton on 8 m chunks (64 × 64 cells) with sleeping |
| `stress.rs` | the structural solver: overburden, load path, bending, compression, cracking |
| `gen.rs` | seeded procgen per `(seed, x, absolute row)` and the stability pass |
| `world.rs` | `World`: the step order, the player (`step_player`, `TUNE`), colony moles, digging, placing, the brush, the endless window, quests and chests |
| `logic.rs` | copperline nets, devices, knots, fuses, TNT, thrumming |
| `blast.rs` | explosions as a pressure wave: Kinney-Graham overpressure over the volume the wave has filled, gas pressure in closed spaces, diffraction shadows, rock cracked or shattered by blast strength, loose ground, water and items thrown, moles and devices hit by overpressure |
| `transport.rs` | rails and mine carts: track as a built block, carts rolling by slope and friction, pushed, ridden and loaded, and powered rails driven by copperline current |
| `lifts.rs` | rope ladders and counterweighted lifts, balanced with stone or wound up by current |
| `tubes.rs` | pressure tubes: hoppers whose current-driven blowers send capsules of items or the mole along the pipe; a breach stops and spills them |
| `ruins.rs` | ruins of the Ancients: pumping halls, sealed vaults and tube stations placed per 160 × 80 m cell from the seed, their carvings, and the relics in vault chests |
| `debris.rs` | blast debris: loose ground, rubble and liquid thrown as ballistic cells that land with nothing lost |
| `lore.rs` | the Lid thread: clues (carvings, old notes, drafts, odd finds) placed per 32 m band from the seed, denser higher up, and the 24 journal fragments that hint but never confirm |
| `survival.rs` | game modes and their rules: **Survival** (start kit, dig only with owned tools, fungus-oil fuel, the pack dropped at death, optional hunger off), **Creative** (999 of everything, nothing hurts) and **Plain** (no mode rules, used by tests and scenarios) |
| `items.rs` | loose items as physics bodies: dug, dropped and thrown things fly, bounce, skid, float or sink, burn, and sleep when still; thrown TNT |
| `tips.rs` | the world hover text: what's under a cell, in plain Delver words |
| `light.rs` | per-cell RGB light and fog of war |
| `air.rs` | stale air on a 2 m block grid |
| `craft.rs` | items, stations, recipes, tool tiers, quests |
| `disasters.rs` | floods, lava surges, quakes |
| `band.rs` | the endless world off-window: 64-row bands holding cells, devices, air and things; the unit of saving |
| `save.rs` | save/load: a manifest plus band blobs, split into parts (only changed bands are rewritten) or one blob |
| `view.rs` | zoom rules shared by the settings menu, keys and wheel; `Wheel` turns wheel and trackpad scroll into notches (idle is still, one flick is one notch); `wheel_use` picks what a notch does (Shift/sprint never changes it) |

### One tick

`World::step()` runs in a fixed order:

1. the falling-sand automaton (`grid.step_ca`);
2. a local stress re-solve around the 8 m chunks that changed (every 6 ticks; fire and lava reactions batched every 24), then crack timers, crumbling and rubble bulking;
3. the player (`step_player`), loose items (`step_items`), then colony moles every second tick;
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
| `devart.rs` | pixel art for copperline tiles (one per neighbour mask, lit with a travelling glint), every device (off/on, animation frames) and the wiring view's schematic glyphs |
| `journal.rs` | the journal (`J`): the Lid fragments found, newest first, as a plain list |
| `decorart.rs` | pixel art for furniture (a base and a pale cloth layer the pigment tints) and the furnishing icons |
| `wiring.rs` | draws copperline and devices as sprites over the world (one per 1 m block, cached by block), and the wiring view (`V`) |
| `audio.rs` | procedural sound and the music synth (web only) |
| `music.rs` | the generative music composer: pure Rust, unit-tested natively, writes timed notes for `audio.rs` |

**Fixed step with interpolation.** Each frame adds `delta × 60` to an accumulator and runs `World::step()` once per whole tick, at most 3 a frame, discarding the backlog after a hitch. The mole is drawn interpolated between its last two positions, so movement is smooth at any refresh rate while the sim stays at exactly 60 Hz. Menus set `Game.paused`, which stops stepping.

**Rendering.** The window's cells are composited on the CPU into **128 chunk textures of 128 × 128 cells (16 m)** (material colour × a per-material pixel pattern (`art::mat_tex`: strata, brick bond, wood grain, rivets, ore glints) × edge shading × light, fog, cracks, designations, translucent overlays for darkness, stale air and steam), and **only chunks that changed are uploaded**. Light is recomputed every second frame over the view. The three parallax layers are GPU sprites behind the world, placed at `camera × factor`. Chunk rows beyond the simulated window are drawn as a static "far" view, so the camera can roam a little past it. The mole is 26 × 20 pixel art with idle, run, jump, fall, climb and dig poses, wearing a brass delving hat with a fungus-oil lamp (lens lit or dark; a small halo when lit). It visibly holds the selected item (`art::held_sprite`, at the mole's own pixel scale): a pickaxe, drill or bore turned toward the aim in 15° steps (the pickaxe swings while digging; the drill and bore shake), a block or charge that thrusts out when placed, a torch held upright with its flame (which lights, see Physics), or a small icon for anything else. Nothing is held while climbing. Then there are plus squash, stretch, landing and run dust, dig chips, water/lava splashes and torch embers for feedback.

## Native and web

| | native | web |
|---|---|---|
| build | `cargo run --release` (in `engine/`) | `./build-web.sh` → `dist/`, then copy to `site/` |
| target | Linux / macOS (x11, wayland) | `wasm32-unknown-unknown`, WebGL2, `wasm-bindgen` 0.2.130 |
| sim threads | rayon (`parallel` feature) | single-threaded (feature off) |
| saves | a manifest file and a folder of band files next to the binary | localStorage keys (base64): the manifest and one key per band |
| sound | silent | procedural WebAudio effects and generative music, starts after the first input |
| seeds and flags | `--fresh`, `--seed=`, `--shot=`, `--dump-art=`, `--scene=` | `?fresh`, `?seed=` |

On the web, `console_error_panic_hook` is installed, so a panic prints its message and source line in the browser console instead of a bare `unreachable`.

Bevy is built with hand-picked features (no scenes, picking, gizmos, animation, post-processing, gamepad or image formats) to keep the wasm small. The `wasm-release` profile adds `opt-level = "z"`, full LTO, `panic = "abort"` and stripping, and `wasm-opt -Os` runs when it is installed.

Rebuild the committed web copy with:

```bash
cd engine && ./build-web.sh && cd .. && rm -rf site && cp -r dist site
```

## Saves

The world is saved in **bands** (`band.rs`): horizontal strips of 64 rows (8 m). A band blob holds everything in its rows: cells and flags, copperline and devices (with fuse state), air, and the drops, torches, stations and mushroom plots anchored in it. Layers are byte run-length encoded.

A save (format 9; 5 added the game mode, headlamp oil and hunger option, 6 the items ever picked up, so unlocked recipes survive, 7 the journal and which Lid clues were read, 8 the relics found and the ruins visited, 9 the transport machines: carts and their cargo, ladders, lifts and tube hoppers) is a small **manifest** plus band blobs. The manifest starts with the bytes `MOLEHILL`, the version and the engine tag (which names the cell resolution). It then holds the globals (seed, ticks, the pack, the mole, quests, the log, the window offset, opened chests, the spawn point) and the list of bands: start row, content hash and length for every band of the window and every band stored off-window. Materials are stored **by name** with a name table, so the material enum can grow or be reordered.

* **Split parts** (`save::save_parts`, the game's slots): each band is stored under its own key, `b<start row>_<hash>`. Unchanged bands keep their key, so an autosave writes only the bands that changed (in practice the window, about 0.3 MB), however far you have dug. Bands are written first and the manifest last, then keys no longer named are removed. A failed write (storage full) therefore leaves the previous save whole. On the web the keys are `molehill-auto/b…` in localStorage (base64); natively they are files in a `molehill-auto/` folder next to `molehill-auto.save`.
* **One blob** (`save::save` / `save::load`, used by tests and the harness): the manifest followed by every band inline.
* **Loading** builds an empty world shell for the seed (`World::blank`, no generation, so a load takes about 50 ms) and unpacks the window's bands into it. Each blob is checked against its hash. A missing or damaged band in the window fails the load; one off-window is dropped (the generator makes it again) and the log says so. Every read is bounds-checked, so a bad save gives a readable error and never panics (`corrupt_saves_never_panic`). A save from a build with a different world size is refused. Format 3 saves (one blob with a row cache of cells) still load, and their cached rows become bands.

The holt's mode and its hunger option are saved with it; a save from before modes loads as Creative. The game autosaves every 30 s and loads the autosave at start. `K` and `L` use a separate manual slot. Autosaves (`*.save`) are gitignored.

## Working on the code

See [`AGENTS.md`](../../AGENTS.md) for the loop: change the sim, add a scenario, run tests and scenarios, build the game, rebuild `site/` if the game changed, then commit and push. **Merges that change behaviour must update the matching page in `docs/wiki/`.**

## Music

The music is generated while you play; no audio files ship. `music.rs` is a slow composer. It plays a piece of 70–130 s, then rests for 40–95 s of near-silence (it also opens with 12 s of quiet). Each piece picks a texture: pads with plucks, a lone kalimba, pads with bells, or a drone with the odd pluck. It also picks a mood from where the mole is:

| mood | where | key | feel |
|---|---|---|---|
| Seep | above −15 m (wet layers) | D major pentatonic | bright, open, higher |
| Loam | home layers | A minor pentatonic, dorian sixth | earthy |
| Deep | below 25 m | E minor pentatonic | lower, darker filter, drone |
| Ember | below 70 m, or lava around the mole | C# with a flat second | low and slow, strong drone |

Melodies walk gently on the scale at a slow pulse (0.85–1.3 s), with long gaps. A big change of place winds the current piece down early, so the next one starts in the new mood. `audio.rs` plays the notes with WebAudio: pads and drones go through a lowpass set by the mood; plucks (a sine with a short 4th-harmonic tick, like a kalimba or marimba) also go into a 0.62 s darkening echo; everything goes into a 3.5 s generated cave reverb. Music has its own bus and volume (Settings > Music, default 35% of an already quiet level; Enter or a click mutes). The composer's tests (`cargo test --release -p molehill music`) check that it opens quietly, stays sparse, has long rests, sits higher up shallow and lower near lava, never schedules a note twice, and doesn't burst after the tab sleeps.
