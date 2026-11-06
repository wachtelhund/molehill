# Site templates

> Code: `engine/sim/src/templates.rs` (format, loader), `assets/sites/*.json` (the data), `engine/sim/examples/export_sites.rs` (export from the procedural reference), `engine/sim/tests/templates.rs` (golden check). The procedural reference layouts stay in `engine/sim/src/sites.rs`.

The seven [found-place](World-and-Generation.md#found-places) families are stamped from **templates**: data files under `assets/sites/`, one per family (`mine`, `holt`, `pumpworks`, `fissure`, `fossil`, `crystal`, `halt`). The game compiles them in (`include_str!`), so native and web read the same data and nothing is loaded from disk at run time. Placement (which site cell holds which family, where its box goes, its `var` and side) is unchanged and still lives in `sites.rs`; the template says what the box holds.

## What a template holds

A template is one piece at cell resolution (`cpm` must equal `mat::CPM`, 8 cells per metre; a file drawn at another resolution is refused with a message):

| field | what it is |
|---|---|
| `family`, `size` | the family and the box in cells (must equal `Kind::size`) |
| `palette` | one character per material (`mat::Mat` names), or `@role` for a role; `_` leaves the host rock |
| `cells` | material rows, top to bottom, run-length coded: `12.3#o` is twelve `.`, three `#`, one `o` |
| `linings`, `lining` | lining mixes (lists of `walls::Wall` names) and a row grid of which mix each cell takes. A cell gets entry `hash % len` of its mix, from the same per-cell hash the procedural linings used, so a mine's back is still mostly soot with a gap, a beam, a crack or bare rock here and there |
| `roles` | a material decided per site: the mine's `ore` (`*`) is copper three times in four, else coal (`var % 4 == 3`) |
| `slots` | layout alternatives: each option is a sparse patch of row spans over the base (and may move the chest). A slot picks its option from the site's `var` (`(var >> shift) % modulo`, then `table`) or from its side (`left`). Slots patch disjoint cells |
| `chest` | the chest's cell from the box's top-left |
| `things` | what the place owns besides cells, in the order it is set down: `torch`, `furniture` (`item`), `farm`, `ladder` (`anchor`), `cart`, `device` (a `logic::Dev` name), `wire` (copperline), `on` (switch the device there on). `when` limits one to a slot option |
| `connectors` | tagged ways in: `entrance` (open floor reaching a side), `shaft` (through the roof or floor), `tube` (a pressure tube leaving the box), with side, start and length. The assembler (coming) stitches pieces at these |

Names are the Rust variant names used everywhere else (scenario `give`, the item registry in `craft.rs`), so a template, a `.scn` file and the code all say `WoodBeam`, `Shelf`, `Floodgate`.

The exported slots today: the mine's `slump` (which bay from 20 m caved, 6), `rails` (which run of track rusted, 4) and `band` (where rock bands cross the worked face, 8); the salt hall's `hang` (where the hanging clusters start, 4); the cargo halt's `side` (tube east or west, which also moves the warehouse, its furniture and the chest).

## The export, and the golden check

The procedural layouts in `sites.rs` are the reference the templates were exported from. `Site::cell_reference`, `wall_reference`, `chest_reference` and `things_reference` take the layout knobs (`sites::Knobs`) explicitly, so the exporter can set each knob on its own and cut the layout into a base and per-slot patches. Before writing a file it checks that slots patch disjoint cells and that every variant a site can be today (`var` 0–95 covers every knob's period, plus a few large values, and both halt sides) comes out the same, cell for cell, with the same linings, chest and things.

```bash
cd engine
cargo run --release -p molehill-sim --example export_sites            # rewrite assets/sites/*.json
cargo run --release -p molehill-sim --example export_sites -- --check # SAME/STALE per file, exit 1 if stale
```

`templates_match_the_procedural_sites` (in `cargo test`) makes the same comparison against the files compiled in, so **if you change a layout in `sites.rs`, re-run the export** or the test fails and names the first cell that differs. When the export landed, whole generated worlds (seeds 1, 11, 4242, 777, 90210 at 256 × 128 m, grid, linings, chests and every thing) hashed the same before and after the switch to templates.

`template_files_are_canonical_and_parse_fast` checks that every file on disk is byte for byte what `templates::to_json` writes from its data (so a diff only shows real changes) and that parsing all seven takes well under a frame budget's worth of startup (8 ms on an M4; the limit is 250 ms). The game parses them once, on first use. `template_files_round_trip_and_explain_errors` checks that a file written by `templates::to_json` parses back to the same data and that a broken file says what is wrong (row length, resolution, unknown names).

## Writing a file by hand

Files are written by `templates::to_json`: pretty JSON with each row, span, thing and connector on one line, so a diff shows what changed. Keep `_` for host rock, never use a digit as a palette character (digits are run lengths), and keep the box size equal to the family's `Kind::size`. The external cave editor (dev only, never shipped with the game) will write these files; until then the exporter is the only writer.
