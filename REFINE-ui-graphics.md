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

### Pixel-art mole and animation
* The mole is now crisp pixel art (26×20, one texel per world unit, nearest sampled, outlined, top-lit) built
  on a small hard-edged pixel grid (`art::Px`). Poses: idle (breathing), run (4 frames, phase follows distance
  so the feet don't skate), jump, fall, climb (2 frames, nose up, belly to the wall), dig (3-frame claw swing).
* Feedback: squash on landing (harder landings squash more) and stretch on take-off, settling in about 0.12 s;
  a puff of dust on landing; chips of the target material spray while digging, and a burst when a bite breaks.

### Light sources
* Torches: pixel-art stick, bracket and three-frame flame with a warm halo that breathes (render-only, so the
  sim stays deterministic). Lit lamps get a steady pale halo.

### Parallax
* The deep-earth layers were sampled per cell on the CPU with an integer offset, so they stepped a whole cell
  (about 12 px) at a time while the world scrolled smoothly, and every camera move re-uploaded every open cell.
  They are now GPU sprites placed continuously at `camera × factor`, and open cells in the world texture are a
  translucent overlay (darkness, light tint, stale-air haze, steam) blended in linear space to match the old look.

### Menus and icons
* Controls (menu screen and the H overlay) are one shared two-column table (`menu::CONTROL_ROWS`), so the
  columns line up in the pixel font; the overlay is centred whatever its height.
* Menus tick softly when the choice moves, a screen changes or a row is chosen.
* Item and tool icons redrawn as 16×16 outlined pixel art in the mole's style (TNT is a bound charge with a
  fuse). The hotbar shows them at exactly 2× (32 px) and dropped items at an exact half scale. Chests, built
  stations and mushroom plots are snapped to the same hard-edged, outlined pixel style.

### Dev tools
* `--dump-art=DIR` (with `--shot`) writes every sprite at 8× and the composited world around the mole, so art
  can be checked headlessly.

## Open issues
* `--shot` writes an empty frame on macOS when the game is launched from a background shell (the surface is
  never presented). It needs a visible window, or Linux with Xvfb.
