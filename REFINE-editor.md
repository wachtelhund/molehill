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
