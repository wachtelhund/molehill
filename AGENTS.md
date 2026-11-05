# AGENTS.md: working on Molehill

Molehill is a falling-sand mole platformer and colony sim in Rust and Bevy 0.20. Read [LORE.md](LORE.md) first. It is binding: there is no sky or surface, the tech is handmade (copperline, thrummers, knots), and nothing gets modern names.

## Layout
* `engine/sim` (`molehill-sim`) is the whole simulation, in pure Rust with no Bevy. Put gameplay rules here so they can be tested headless.
  * Resolution is one knob: `mat::CPM` cells per metre (8). Size anything in cells through it: metres × `CPM`, or legacy 0.25 m tiles × `mat::S`. Never write a bare cell count. Scenarios are written in tiles (see TESTING.md).
  * `world.rs`: World, the player mole (`step_player`), mining and placing. `TUNE` holds every movement constant.
  * `mat.rs` is the material table: every material property and category lives in its row (`Props`). Rules read the table; don't add `matches!(m, Mat::A | Mat::B)` lists.
  * `grid.rs` cells; `stress.rs` structure; `gen.rs` procgen; `light.rs`; `logic.rs` devices; `disasters.rs`; `craft.rs`; `band.rs` (the world off-window, in 64-row bands); `save.rs` (manifest + band blobs).
* `engine/game` (`molehill`) is the Bevy front-end.
  * `main.rs`: input, the fixed 60 Hz sim step with render interpolation, the camera, rendering, sprites.
  * `ui.rs` HUD; `menu.rs` main and pause menus; `art.rs` sprites and icons; `audio.rs` WebAudio (silent on native).
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
* `A`/`D` run (`Shift` held: sprint); `Space` jumps (also a fresh `W` press when you're not on a wall).
* Into a wall in the air you cling: `W`/`S` climb, `Space` wall-jumps.
* `S` digs down. Left mouse uses the item in hand; right mouse interacts; hold `Q` to charge and release to throw the item in hand (TNT goes out lit). `1`-`0` or the wheel pick a hotbar slot; `I` opens the pack.
* Dig tools and blocks use a round brush: `[`/`]` or `Alt`+wheel resize it within the item's range (`brush_range` in `world.rs`). `+`/`-` or `Ctrl`/`Cmd`+wheel zoom. `Shift` never changes the wheel: it is sprint (`view::wheel_use`).
* `Esc` opens the pause menu; `K`/`L` quick save and load; `F3` shows dev info.
