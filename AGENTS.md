# AGENTS.md: working on Molehill

Molehill is a falling-sand mole platformer and colony sim in Rust and Bevy 0.20. Read [LORE.md](LORE.md) first. It is binding: there is no sky or surface, the tech is handmade (copperline, thrummers, knots), and nothing gets modern names.

## Layout
* `engine/sim` (`molehill-sim`) is the whole simulation, in pure Rust with no Bevy. Put gameplay rules here so they can be tested headless.
  * Resolution is one knob: `mat::CPM` cells per metre (8). Size anything in cells through it: metres × `CPM`, or legacy 0.25 m tiles × `mat::S`. Never write a bare cell count. Scenarios are written in tiles (see TESTING.md).
  * `world.rs`: World, the player mole (`step_player`), mining and placing. `TUNE` holds every movement constant.
  * `mat.rs` is the material table: every material property and category lives in its row (`Props`). Rules read the table; don't add `matches!(m, Mat::A | Mat::B)` lists.
  * `survival.rs`: game modes (Survival, Creative, and Plain for tests) and their rules: start kit, tools you own, headlamp and torch fuel, the pack dropped at death. Scenarios set a mode with `mode survival`.
  * `templates.rs`: found places as data (`assets/sites/*.json`, compiled in). `sites.rs` keeps the procedural reference; after changing a layout there, re-run `--example export_sites` (the golden test `templates_match_the_procedural_sites` fails until you do). A file saved from the cave editor (`"source": "editor"`) is its own layout: the exporter keeps it and the golden test skips it. See `docs/wiki/Site-Templates.md`.
  * `grid.rs` cells; `stress.rs` structure; `gen.rs` procgen; `light.rs`; `logic.rs` devices; `disasters.rs`; `craft.rs`; `band.rs` (the world off-window, in 64-row bands); `save.rs` (manifest + band blobs).
* `engine/game` (`molehill`) is the Bevy front-end.
  * `main.rs`: input, the fixed 60 Hz sim step with render interpolation, the camera, rendering, sprites.
  * `ui.rs` HUD; `menu.rs` main and pause menus; `art.rs` sprites and icons; `audio.rs` WebAudio (silent on native).
* `engine/tools/cave-editor` (`molehill-cave-editor`) is the dev-only cave editor for the templates in `assets/sites/` (`cargo run --release -p molehill-cave-editor`). It must never ship: keep it out of `default-members`, out of the game's dependencies and out of `build-web.sh` (its `packaging` test checks). See `docs/wiki/Cave-Editor.md`.
* `docs/wiki/` is the wiki (start at `docs/wiki/README.md`). There is no GitHub wiki.
* `site/` is a committed copy of the web build. Rebuild it with `cd engine && ./build-web.sh && cd .. && rm -rf site && cp -r dist site` (`dist/` is gitignored).

## Loop
1. Change the sim, and write or adjust a scenario (`engine/sim/tests/scenarios/*.scn`) or test. See [TESTING.md](TESTING.md).
2. From `engine/`, run `cargo test --release -p molehill-sim` and `cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/*.scn`. Both must pass.
3. Run `cargo build --release -p molehill`. Add one screenshot only if the change is visual; don't make videos unless asked.
4. If behaviour changed, update the matching `docs/wiki/` page. If the game changed, rebuild `site/`. Commit and push to `origin main`, then check that `git fetch && git rev-parse HEAD origin/main` match.

## Rules
* The only remote is the private `origin` (github.com/wachtelhund/molehill). No GitHub Pages, Vercel or other hosting, and no work accounts. Never force-push.
* Autosaves (`*.save`, including the band files in `molehill-auto/`) are gitignored. Don't commit them.
* The feel target is Streets of Rogue-snappy: instant start and stop, no input latency, and actions that fire on press with immediate feedback. Keep `TUNE` responsive, and test feel changes with scenarios (rise, `vx` after N ticks, coyote).
* Menus pause the sim (`Game.paused`). The headless harness never touches the game crate.
* Behaviour-changing merges must update `docs/wiki/` (the wiki: plain markdown on `main`, index `docs/wiki/README.md`, relative links). Change the page that describes the rule, number, control, device or build step in the same merge. The wiki's Lore page must match `LORE.md`, which wins if they disagree.

## Controls (in game)
* `A`/`D` run (`Shift` held: sprint); `Space` jumps (also a fresh `W` press, except by a rope ladder or rope, where `W` grabs).
* No wall climbing: a jump pressing toward a ledge rolls onto it if its top is within 0.75 m of the feet; higher, rope ladders and the rope gun (left mouse throws its hook into rock; by the rope `W` grabs, `W`/`S` climb, `A`/`D` swing, `Space` lets go; `E` takes a rope back).
* `S` digs down. Left mouse uses the item in hand; right mouse interacts (or, on an empty spot, sets down the device or TNT in hand); hold `Q` to charge and release to throw a throwable item (TNT goes out lit); `E` picks up the hovered device, unlit TNT, torch or copperline segment (hold to sweep). What each item allows lives in `Item::uses` / `Item::controls` (`craft.rs`). `1`-`0` or the wheel pick a hotbar slot; `I` opens the pack.
* Dig tools and blocks use a round brush: `[`/`]` (read by the character typed, so Option+8/9 on a Swedish Mac works) or `Alt`/`Option`+wheel resize it within the item's range (`brush_range` in `world.rs`, `view::BrushSizes`); the size is written beside the outline. `X` toggles grid snap for blocks (`World::place_line`). `+`/`-` or `Ctrl`/`Cmd`+wheel zoom. `Shift` never changes the wheel: it is sprint (`view::wheel_use`).
* `G` lights or puts out the headlamp; `V` toggles the wiring view; `J` opens the journal. `Esc` opens the pause menu; `K`/`L` quick save and load; `F3` shows dev info.
