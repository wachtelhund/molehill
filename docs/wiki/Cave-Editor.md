# Cave editor (dev only)

> Code: `engine/tools/cave-editor` (crate `molehill-cave-editor`): `src/lib.rs` is the model and the checks (headless, tested), `src/main.rs` the window. It edits the [site templates](Site-Templates.md) in `assets/sites/`.

**The cave editor is a developer tool. It is not part of the game.** Players who download Molehill, natively or as the web build in `site/`, never get it:

* It is its own binary crate in the workspace, and **not** a default member (`default-members = ["sim", "game"]` in `engine/Cargo.toml`), so `cargo build` and `cargo run` build and run the game only.
* Neither the game nor the sim depends on it, and there's no feature flag that compiles it into the game.
* `build-web.sh` builds `-p molehill` only, and `site/` is that build's output.
* `cargo test --release -p molehill-cave-editor --test packaging` fails if any of that changes.

## Running it

From a clone, on a desktop (macOS, Windows or Linux with a display):

```bash
cd engine
cargo run --release -p molehill-cave-editor            # opens the abandoned mine
cargo run --release -p molehill-cave-editor -- fossil  # or a family: mine holt pumpworks fissure fossil crystal halt
```

It uses the game's own Bevy build, so after the game has been built once it compiles in seconds.

## What it does

The window shows one found place at one pixel per cell (12.5 cm) as the game would stamp it: materials in their table colours, host rock dark and hatched, linings dimmed behind open space, and markers for torches (yellow), furniture (blue), farms (green), ropes (brown lines), carts (grey), devices (red), copperline (orange) and the chest (gold). The panel on the left lists every command; click a row or press its key.

| key | does |
|---|---|
| `1`–`7` | open a family (mine, holt, pumpworks, fissure, fossil, crystal, halt); unsaved changes ask for a second press |
| `M` / `L` / `T` / `C` | paint materials / paint linings / set things down / move the chest |
| `,` `.` (or click) | pick from the palette: materials (and host rock), single-wall linings (and none), things (torch, each piece of furniture, farm, rope, cart, lever, floodgate, pump, lamp, door, plate, copperline, switched on) |
| `[` `]` | brush radius, 0.5 to 24 cells |
| left mouse | paint the brush, set a thing down, or set the chest |
| right mouse | pick the material under the cursor; take the nearest thing away; remove the chest |
| `V` / `B` (`Shift`: 256) | step the variant `var` the preview shows: slots (the mine's slump, rails and band, the salt hall's clusters) pick their options from it, and from 256 up the [seeded variation](Site-Templates.md#seeded-variation-variation) too (flipped, zones), as a placed site's would; the panel's `varied` row says what happened. A flipped preview is painted where you click it: the stroke lands on the template's own side |
| `N` | the halt's side (tube east or west) |
| `E` | the depth (10, 50 or 80 m) the preview places the piece at, for depth remaps |
| `R` | **can the mole walk it?** Stamps the piece in granite and flood-fills the mole's body (the found-places test's rule): green where a way in reaches, orange where open space is cut off; the panel names any way in that doesn't reach the chest. Gravity is ignored here (the mole climbs walls and ropes), so a long drop still counts as a way: `G` and the scenarios catch falls |
| `P` | **will it stand?** A full stress solve; red cells are over capacity and crack and fall in play |
| `G` | **run it 10 s** with a mole standing at the chest: shows the piece afterwards and counts the cells that changed, the solid ones that broke or fell (what `P` flags red comes down here), and the mole's hp |
| `Esc` | clear the check (painting, `V`/`B`, `N` and `E` clear it too, and say so) |
| `Ctrl`/`Cmd`+`Z`, `+Shift` or `Y` | undo, redo (a stroke is one step) |
| `Ctrl`/`Cmd`+`S` | save to `assets/sites/<family>.json` |
| wheel, `+`/`-`, arrows or `WASD`, `F` | zoom, pan, fit |

Painting edits the base layer. Cells a variant patches show the variant's value in the preview (step `V`/`B` to see each). Materials and linings join the file's palette and lining mixes as they are used, and leave them when nothing uses them any more.

The window opens at 1600 × 960 and can't be made shorter than the panel needs. The palette shows 9 entries at a time around the one picked; `,` `.` reach the rest.

## Checking it without looking (dev)

* `--canvas=FILE.bmp` writes the canvas (one pixel per cell, as drawn in the window) and quits without opening a window; add `--shot-keys=p` (or `r`, `g`, `n`, `b`, …) to run those keys first, so the file shows a check. The panel's text goes to the terminal.
* `--shot=FILE.bmp [--shot-at=N] [--shot-keys=…]` saves one frame of the real window (panel, fonts and canvas, 1600 × 960) and quits. The camera draws into an image instead of the screen for it, so it works from a background shell too (a macOS window that isn't really on screen reads back black).
* `cargo test --release -p molehill-cave-editor --test panel` clicks every panel row at its place, presses every key and paints strokes through the same code the window calls. It shows that every control does what its row says, and that every row fits the panel's width (38 characters; a longer one wrapped onto the row below). Mouse feel (dragging strokes, wheel zoom, panning) still needs a person at the window.

## Who owns a layout

A file the exporter wrote (`examples/export_sites.rs`, from the procedural layouts in `sites.rs`) is checked cell for cell against them (`templates_match_the_procedural_sites`). The first save from the editor marks the file `"source": "editor"`. From then on the file is the layout:

* `export_sites` prints `KEPT` for it and leaves it alone (`-- --force` overwrites it with the sites.rs layout again).
* The golden check skips it. Everything else still runs on it: the canonical-file check, the found-places walk test, the `site_*` scenarios.

So after painting a family, run the tests and scenarios as for any change (they walk every family), and commit the JSON.

## The checks are the game's own

`R`, `P` and `G` stamp the piece into a real `World` with `World::stamp_template` (the same cells, linings, chest and things as the game's site stamp, `stamp_template_matches_the_game_stamp`) and use the sim's body box, stress solver and step. The editor's tests (`cargo test --release -p molehill-cave-editor`) check painting, undo, saving and that the checks agree with today's layouts: every family is walkable, the fossil stands, a floating slab is caught, a wall across the salt hall is caught.
