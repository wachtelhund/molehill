# Refine run: UI and graphics (branch `refine/ui-graphics`)

Polish only: no new systems, lore names kept. Each entry lists what changed and how it was checked.

## Changes

### Structural blocks are rigid (bug report)
* Placed pillars, bricks and beams resting on sand or rubble had no load path in the connectivity check.
  They cracked, crumbled into sand and flowed away. Loose ground now carries whatever rests directly on it.
* Loads only reach a beam or pillar through material that touches it. Before, a ledge under a cave roof was
  charged the roof's weight through the air, and a free-standing stub took a share of the roof load from
  the voids beside it.
* Wood beams, doors and iron beams that fail now break into loose debris of their own material, not sand.
* Harness: `place T DX DY Item`, `static:<Mat>` (non-loose cells), `util:DX,DY` (stress use), `I` = iron beam.
* Scenarios: `pillar_holds_roof`, `pillar_on_rubble`, `beam_spans_gap`, `beam_overload`.

### Zoom (bug report)
* Settings zoom only zoomed out: Enter and clicks always stepped out, and the label showed the raw camera scale.
  Zoom is now one clamped, saved value (`sim/src/view.rs`, scale 0.12-0.75) shared by the Settings row
  (A/D or the left/right half of the row), `+`/`-` and `Shift`+wheel, shown as a magnification (1.0x default).
  In-game zoom used to be lost on New Holt; it now persists.

### Round brush for building, adjustable size (request)
* Pillars, bricks and iron beams place a round brush like digging (`World::place_brush`). Each new cell must
  touch something solid, so a stroke grows out of what it rests on and never floats.
* `[`/`]` or `Ctrl`+wheel resize the brush of the dig tool or block in hand, clamped per item by `brush_range`:
  pickaxe 0.5-3, drill 0.5-4, bore 1-5.5, pillar 0.5-3, brick 0.5-3.5, iron beam 0.5-2. Defaults keep the old
  bites, and single cells for pillars and beams. The ring shows the size (in the block's colour when placing),
  and the label above the hotbar shows `brush r`.

### HUD, tooltips
* The HUD hangs off one root that is hidden while any menu is open; the aim ring clears while paused.
* Hover tooltips: hotbar and pack slots show name, count and a one-line description. Resting the cursor on a
  seen cell for ~0.35 s names the device, copperline or material there (loose debris is marked). Nothing
  shows while a mouse button is held. Texts live in the sim (`Item/Dev/Mat/Station::desc`) with a lore check.
* Help, Controls and README list brush and zoom keys; removed the dead "New world N" hint.

## Open issues
* `--shot` writes an empty frame on macOS when the game is launched from a background shell (the surface is
  never presented). It needs a visible window, or Linux with Xvfb.
