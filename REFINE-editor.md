# REFINE-editor (p10) log

External cave/site editor (dev-only, never shipped) plus data-driven found-place templates and an assembler.

## Step 1: data format + loader + identical export (`refine/cave-editor`)
* Survey (2026-11-06): no open sites.rs edits in other worktrees; env-rework v3 layouts are on main (8add868, de54c93). p8 registry (`refine/item-registry`) keeps Rust const tables keyed by variant names, so templates name materials, walls, items and devices by their Rust variant names (as scenarios do). No fork of formats.
* Format: JSON (`serde`, `serde_json` with `preserve_order`), one file per family in `assets/sites/`, compiled in with `include_str!` (web and native identical). RLE rows, palette, lining mixes (`hash % len`), roles (mine ore), slots (sparse patches picked from `var` or side), chest, things, tagged connectors (entrance / shaft / tube).
* sites.rs: only behaviour-preserving plumbing. Carve functions read explicit `Knobs` instead of `self.var` (5 expressions); the wall, chest and stamp_one bodies became `*_reference` functions (the stamp body emits a `Thing` list instead of calling try_*). Layout code itself untouched. `Site::cell` / `wall` / chest / `stamp_one` now read templates.
* Export: `examples/export_sites.rs` cuts each layout into base + per-knob patches, checks slots are disjoint and every reachable variant matches, then writes. `--check` reports staleness.
* Golden: `tests/templates.rs::templates_match_the_procedural_sites` (all 7 families, var 0..96 + large, both halt sides; cells, linings, chest, things). Plus a throwaway world-hash comparison against origin/main 4322396: 5 seeds at 2048×1024 and 56 stamped variants hashed identical (grid, flags, linings, chests, furniture, torches, farms, ladders, carts, logic).
* Green: sim tests (all suites), 162/162 scenarios, game builds, sim checks on wasm32.
* PR #14 opened after merging main fd458d9: sim tests green, 168/168 scenarios, game builds.
* Polish: whole-cell thing positions are written as integers (`64`, not `64.0`); the cells are unchanged.
* Stage 2 (editor UI) waits until #14 is merged, per the watchdog.

## Stage 1 polish (`refine/cave-editor-polish`)
* #14 merged at b842698, after green runs on each main tip up to bef5986. It was merged onto da1b741 (crafting balance, no site code) through a clean merge; this branch re-runs the full suite on top of that.
* Test `template_files_are_canonical_and_parse_fast`: the files on disk are canonical writer output; all seven parse in 8 ms.
* Removed unused `Template::lining` (base only, it ignored variants) and `slot_names`.
* Layout traps (fossil scaffold collapse, the fissure's 0.5 m slot at the east bridge, the pumpworks gallery drop): Hampus decided at 12:41 (confirmed after the 12:54 watchdog) that they are fixed in the templates as their own PR with scenarios, after this polish and before stage 2. Each fix goes through the sites.rs reference and `export_sites`.
* Polish green on main b842698: sim tests, 180/180 scenarios, game builds.

## Site layout traps (`refine/site-layout-traps`, Hampus 12:41)
* Pumpworks: walking west off the wheel was a 7.5-10 m drop (16-43 hp). Added a timber landing on the viewing window's sill (`sites::PUMP_LANDING`, 12 m), keyed into the wheel's rim, and a 7 m rope off its west end to the dry hall floor. Scenario `site_pumpworks_rope_to_chest`: chest opened at 100 hp.
* Fissure: the east mouth's 0.5 m slot is gone (the mouth floor is the 24 m deck's top). The watchdog (13:38) asked to make the decks stand too. Under the solver about 4400 cells were over capacity: decks stopping 0.5 m short of both walls floated, mixed wood/iron halves were judged at wood strength with iron weight, root cords and the ceiling roots crushed under the open span's load, and middle cords hung from nothing. Now every deck is keyed into both walls with each half one material, cords are timber and hang only under decks (none from the ceiling or in the broken middles), and the 6 m dangling top roots are gone (the back wall keeps its roots). Down to 96 over capacity, none in decks or cords. Scenario `site_fissure_east_bridge`: 10 s on the bridge, 100 hp, no debris.
* Fossil (Hampus 13:50: "beast on the floor"): the body's bones lie 3 m lower, on the floor, bedded in a rock mound that runs through each rib tip a quarter metre down (`Site::fossil_mound_top`), so climbing over the ribs is a run of small steps, not 1 m pits the 0.45 m-wide mole drops into. Seven ribs (an eighth would stand past the mound's end, under the snout). The skull stays resting on the east wall, without the old bone gaps that cut its snout tip loose, and the mole walks in under it. Gone: scaffold posts and walkways, their two ladders, and the dangling roof roots. Added: a rope off the mound's 4 m west face to the floor by the chest. The table moves from 8 m (now in the mound) to 3 m and the centre torch goes above the mound. About 1600 cells over capacity down to 18, none in bone or mound. Scenario `site_fossil_over_the_beast`: chest opened at 100 hp, `crumbled` 0, no debris; `site_fossil_traverse` now walks in to the beast's east end.
* All through the sites.rs reference and `export_sites`; `templates_match_the_procedural_sites` stays the guard. The movement agent's `site_fissure_traverse` and `site_pumpworks_traverse` are updated (new mouth floor; comments now point to the new walks).

## Site traps polish (`refine/site-traps-polish`)
* #22 merged at 0fc643d. Suite green there: sim tests, 189/189 scenarios, export SAME, game builds. Rebuilt `site/` so the web build has the new pumpworks, fissure and fossil layouts.
* Not done: the wiki's site screenshots (images/site-*.png) still show the old fossil and fissure.

## Stage 2: editor UI (`refine/cave-editor-ui`)
* Crate `engine/tools/cave-editor` (`molehill-cave-editor`), a workspace member but not in `default-members`; neither the game nor the sim depends on it; `build-web.sh` is untouched (`-p molehill`). `tests/packaging.rs` checks all three and that `site/` holds nothing of it.
* `src/lib.rs` (headless): `Doc` opens a family's file, paints materials and linings with a round brush (palette and one-wall lining mixes grow and shrink with use), sets things down (ropes tie to the nearest solid cell), moves the chest, undo/redo per stroke, saves canonically. Checks on the real sim: `reach` (the found-places body flood fill), `stress` (full solve, cells over capacity), `simulate` (10 s with a mole at the chest). `draw` makes the canvas pixels. `tests/model.rs`: 6 tests.
* `src/main.rs` (Bevy, the game's exact feature set, so it reuses the game's compiled Bevy): canvas sprite, clickable text panel, keys as in the wiki page.
* Sim: `SiteFile::source` (`export` is left out of files, so the exported JSON didn't change; `editor` after an editor save); `World::stamp_template` in templates.rs (no sites.rs change), checked against the game stamp by `stamp_template_matches_the_game_stamp`. `export_sites` prints KEPT for editor-owned files (`--force` overwrites); the golden test skips them.
* The window opens and runs (10 s, no errors in the log), but this shell can't capture the screen (`screencapture`: could not create image from display), so the layout isn't visually checked; Hampus needs to look at it once.
* Wiki: new Cave-Editor page (how to run; not shipped; keys; ownership); README, Architecture, Site-Templates, Testing; TESTING.md; AGENTS.md.

## Stage 2 polish (`refine/cave-editor-ui-polish`)
* Visual check: `--shot` (Bevy's screenshot of the window, as the game's `--shot`) reads back an all-black 3200×1858 frame from this background shell, the known macOS limit in TESTING.md, and `screencapture` is refused. **The window's look (fonts, spacing, colours, the panel over the canvas) is not visually verified;** Hampus should open it once.
* What was checked instead: `--canvas=FILE.bmp` writes the canvas from the model with no window. Looked at the fossil plain, with `R` (green everywhere the mole fits, the skull's eye socket orange as a sealed pocket) and the mine with `P`.
* The panel logic moved out of the Bevy file into `Editor` in lib.rs (rows, what each does, keys, canvas presses). `tests/panel.rs` clicks every row at its y, picks every palette entry of every tool, presses every key, paints a drag as one undo step, checks unsaved work asks twice, things and the chest from the canvas. Found and fixed: the panel needed 55 rows against 54 laid out (the bottom row clipped; on a shorter window much more). The palette now shows 11 at a time, the window has a minimum size, and `panel_fits` is tested.
* Undo snapshots held whole grids as 4-byte chars plus the whole file, 200 deep (about 1 MB a step on the mine). Now bytes and only what an edit changes, 100 deep (about 0.2 MB).
* Thing and chest markers are drawn over a check's tint, not under it. The walk message no longer says "1 way(s) in" (it counted connected spaces, not mouths) and says gravity is ignored.
* Seen with `P`: the mine's upper deck between the chute and the shaft, posts included, is over capacity (most of its 2583 cells). That's pre-existing and not changed here; flagged for the layout owner.

## Stage 3: seeded assembler variation (`refine/cave-assembler`)
* Format: `SiteFile::variation` (mirror; depth remaps; zones `chest` / `furniture` / `may-collapse`). Carried over by `export_sites`, ignored by the golden check, so `Template::cell` stays the exported layout.
* Placed lookups in templates.rs (`placed_cell`, `placed_lining`, `placed_chest`, `placed_things`, `Thing::mirrored`); `Site::placement()` and four one-line call changes in sites.rs, no layout code. Variation only from `var` 256 (`VARY_FROM`); rolls are `hash3(var >> 8, k)`.
* Data: mine flips, granite below 40 m, face caves in 35 %; holt flips, banner sets; pumpworks flips; fissure and fossil flip, basalt below 60 m; salt hall flips, chest 75 %; halt doesn't flip (tube link), chest 50 %. Mines and holts keep their chest (the first-hour find).
* Tests: `found_places_entrances_reach_every_room_and_the_chest` now walks seeded variants per family and seed (chosen to cover flipped and not, each zone on and off), 54 s; `tests/variation.rs` (5); `stamp_template_matches_the_game_stamp` with varied vars.
* Editor: previews and checks through the placed lookups at a chosen depth (`E`), shows what varied, paints a flipped preview on the template's own side; Shift+V/B steps 256. Palette now 9 rows (`panel_fits`).
* Not in this PR: stitching extra pieces at connectors. It changes a site's box (Kind::size and the site grid assume it's fixed) and there are no extra pieces yet; follow-up once pieces are painted.

## Stage 3 polish (`refine/cave-assembler-polish`)
* #41 merged at 69f367d (GitHub merged it over main's blast/ore change, 867dd17). Re-gated on that merged main.
* Looked at a varied mine (var 512 at 80 m, `--canvas`): flipped (face, ballast and chute east; headframe, shaft ropes, carts and chest west by the adits) and the worked face caved in (the gravel block by the face that settles as rubble in play). As intended.
* Wiki Testing lists the variation tests.
* This round's queue is done: stages 1-3 and their polish. Not done, as agreed: connector stitching. Still open from before: the window's look is unverified (needs a person), and the mine upper deck is over capacity under the stress check (flagged in #36).

## Final polish (`refine/cave-editor-final-polish`, on main a9bcf4b)
* The window seen for the first time. `--shot` now points the camera (and the panel, `UiTargetCamera`) at a 1600 × 960 image instead of the window, so it captures from this background shell: the real panel, font and canvas. Looked at the fossil, mine (plain, `R`, Shift+B), halt (`T`, `P`) and pumpworks (plain, `G`).
* Found and fixed from those frames:
  * Rows longer than the panel wrapped onto the row below: `F fit (wheel or +/- zoom, arrows/WASD pan)`, and the message (wrapped at 44 characters, the panel holds 38), which overprinted "right"/"might". Message wraps at `PANEL_CHARS` (38), two rows shortened, panel text no-wrap and clipped to the panel. `every_row_fits_the_panel` checks every family, tool, palette entry, slot row and check message.
  * `G` showed the wrong rock: the sim slides its window after the mole (`follow_colony`), the piece sits near the window's bottom, so after 10 s it read the generated holt the slide brought in (89 333 of 101 376 mine cells "changed"). `simulate` now reads the piece's rows through the window offset (`oy`): mine 2 613 changed (its over-capacity deck), pumpworks 136, fossil 0. `a_run_shows_the_piece_it_ran` (fails on the old read). The vacuous `assert_eq!(s.cells.len(), s.cells.len())` went.
  * A check's message stayed after its overlay was cleared (`R` then `B`: "Walkable ... green" over a canvas with no green). Painting, `V`/`B`, `N`, `E` and `Esc` now say the check was cleared. `a_cleared_check_takes_its_message_with_it`.
* Not live-tested: mouse feel (dragging strokes, right-click, wheel zoom, panning, resizing the window) and the Retina scale (the offscreen frame is 1 × scale). No screen recording permission here, so no input was driven through a real on-screen window.

## Phase 2 polish (`refine/cave-editor-phase2-polish`, on main c65aa80)
* Re-checked on c65aa80 (horizontally endless world, stress and physics changes): editor tests green, `export_sites --check` SAME, every family walkable (`R`). `P`: mine 2583, halt 286, holt 164, pumpworks 16 over capacity; fissure, fossil, crystal sound.
* Found: `G` and `P` disagreed both ways.
  * The mine's deck that `P` flags red comes down whole in a `G` run, but `G` said "1 crumbled": the world's `crumbled` counter doesn't count what the stress solver brings down. `G` now counts solid cells that broke or fell (solid before the run, not after): mine 2535.
  * The fissure (sound under `P`) "lost" 2983 solid cells, its top decks: it's 320 cells tall, so the window's slide after the mole packed its top rows into a stored band, and the run read anything outside the window as empty. `simulate` now reads those cells from their stored band (`World::stored_cells`; columns too, for a sideways slide): fissure 0.
  * `a_run_shows_the_piece_it_ran` checks the mine's broke count against `P` and that the fossil and fissure lose nothing.
* Not changed: the mine's deck itself (layout owner); the editor's test world is 1024 wide, under `SITE_MIN_W`, so it never slides sideways today (read anyway).
* Not live-tested: the GUI was again only seen through `--shot` frames and `--canvas`; mouse feel (drag, right-click, wheel, pan, resize) still needs a person at the window.
