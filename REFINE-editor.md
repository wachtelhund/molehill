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
