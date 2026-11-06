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

## Run 2

### Material pixel art (`art::mat_tex`)
* Solid cells get a per-material pattern in absolute world rows (so it doesn't jump when the window slides):
  strata bands in rock and limestone (with the odd fossil), salt-and-pepper granite, basalt columns, a 4×2
  running bond with dark mortar for brick, grain and knots in wood, rivets and plate seams on iron, glints in
  ore and coal, and quartz/crystal that twinkle. Loose debris stays plain.
* Edges read as shaped: a bright lip on faces open above plus a softer second row, shadowed undersides and
  slightly darker side walls.

### Hover explanations (`sim/src/tips.rs`)
* World hover text moved into the sim (`tips::describe`) so it's tested headless. New: stations say what they
  make and how to craft there, chests say whether they're empty, mushroom plots give growth % and what holds
  them back (too bright / too dry), copperline says whether it's carrying current, and standing blocks say how
  hard they're working (bearing / straining / overloaded / cracking) from the stress solver.
* Scenario `hover_strain` (new `strain:DX,DY` variable) and test `world_hover_explains_cells`.

### Animation and particles
* Run dust in the floor's colour, a splash crown when the mole drops into water or lava, and embers drifting up
  from torches. Particles carry their own gravity.
* Fix: placement pops shared a sprite-pool kind with torch halos, so a pop with the same index as a torch was
  never drawn.

## Phase 2: light, headlamp, held items, music

* **Light system** (`sim/src/light.rs`): sources (torches, lamps, kilns, the headlamp beam, a torch in the paws) cast rays
  that stop at rock, so walls throw shadows; point lights fall off as (1 − d/R)², beams as (1 − (d/R)²)²; sources add;
  a steep bounce pass softens edges; material glow (lava, crystal, fungus) is unchanged; the old sky path is gone.
  The renderer's own glow around the mole is gone (the sim's lights replace it) and remembered rock is a little darker.
* **Headlamp**: a brass delving hat with a fungus-oil lamp on every mole frame (lit/out sheets, lens halo). The beam
  follows the aim and reveals fog along it. `G` or Settings toggles it (saved). `Q` is throwing (refine/materials).
* **Held items**: pickaxe/drill/bore turned to the aim in 15° steps (the pick swings, the drill and bore shake), blocks
  and charges thrust on placing, a torch upright with its flame (and its light).
* **Music**: generative ambient (`game/src/music.rs` + WebAudio in `audio.rs`), its own volume and mute in Settings.
* Scenarios `light_torch`, `headlamp`, `headlamp_wall`, `held_torch` (directives `lamp`, `aim`, `wield`, `torch`;
  variable `light:`); composer unit tests.

## Unique item art and furnished found places (branch `refine/unique-item-art`)

* **One icon per item** (`game/src/itemart.rs`). The old icons were five shapes (lump, ingot, gem, torch, charge)
  tinted per item; blocks shared a striped square and pigments a pot. Now each of the 69 items has its own 16×16
  drawing (a cone of sand, three pebbles of gravel, a pig of tin, a coil of copperline, a scallop of verdigris,
  boards standing on end for a timber wall…). Furniture keeps its scaled sprite. The hotbar shows each device as
  its own sprite instead of one shared placeholder; the floodgate now differs from the door (iron, a brace while
  shut, a hand-wheel). Mine carts are a sprite (plank tub, bronze rim, iron wheels) instead of four rectangles.
* **Found-place furnishings** (`game/src/props.rs`): 19 drawn props placed per family from the site box and
  fitted against the real cells (standing on both feet, or leaning on a post; hanging under a roof; never in rock,
  furniture, a chest, a farm or a ladder). Winch, scaffold, dig tools, crates, mine sign,
  cold lamps, pipes, sluice gate, bridge stubs, ropes, crystal clusters, railcar, bench, station
  sign, kitchen, bedroll, pantry, barrels. Drawn behind furniture, lit like it, shown once seen. sites.rs is
  unchanged. After env-rework v2 built a headframe, ore chute, pump wheel and fossil scaffold in cells, the
  drawn versions of those were dropped rather than doubled.
* **`--dump-art`**: `view.bmp` now composites every visible world sprite over the cells (it was cells only;
  the sprite composite was `view_wire.bmp` and included hidden pool sprites). `view_cells.bmp` keeps the cells
  alone. A dump renders the whole window, so a site crop wider than the screen is no longer half unrendered.
  env-rework v2 had added a separate CPU re-blit of furniture, chests, torches, carts and
  ladders; it was replaced by compositing the real visible sprites (which also covers props and lighting).
  The seven site stills in the wiki were regenerated with their furnishings.
* Tests: `every_item_has_its_own_icon` (no two icons within 20 px of pattern or 8 px of silhouette, judged on
  brightness bands, so a tint scores 0), `every_device_has_its_own_sprite`, `every_prop_has_its_own_sprite`,
  `found_places_are_furnished`. Wiki: new page Item and Place Art (full catalogue and icon/prop sheets).
* Not seen in a live window: the props' scale against the mole in play, and whether the cold lamps read as
  lamps at play zoom. The holt is crowded, so its kitchen stands on the table and its pantry hangs.

## Unique item art: polish (branch `refine/unique-item-art-polish`)

* Measured slot coverage of every shipped icon from a `--dump-art` run: the nest (69 px of 256, its 9 × 3-tile
  sprite scaled), the fuse (64) and the pressure plate (74) were floor-level slivers against a median of 135.
  The nest gets its own icon (a basket bed with blanket and pillow). The fuse and plate keep their in-world
  sprites but get hotbar icons (`devart::hotbar_icon`): a coil of cord with a lit end, the plate from above.
* Test `every_slot_icon_fills_its_slot` (≥ 80 px; slim shapes like sand, torch and crystal sit at 86-89).
  Icon sheet regenerated.
## Spawn fog stays local (branch `refine/spawn-fog-local`, playtest bug)

* Cause confirmed: `World::new` marked a 30 × 23.5 m box around the burrow seen with `light::reveal_box`, which
  ignores rock, so the whole start chunk opened on the first frame and every material in it counted as
  discovered.
* Now: `light::reveal` raycasts from three spots inside the chamber (6 m, stopping at the first rock), then
  `light::reveal_skin` marks a 1 m skin around the open cells that saw. The rest stays dark until vision,
  digging or the headlamp finds it. The discovered-materials loop runs on that, so it only counts what's in
  sight. Seen at spawn: 712 m² before (the box), 36 m² after (five seeds, 2319 cells each); 5-8 materials
  start discovered.
* Test `spawn_reveals_only_burrow_neighbourhood` (five seeds). Wiki World and Generation, TESTING.md.
* Polish run: a fresh seed 4242 dumped at frame 150 shows the burrow as a small torch-lit dome in dark rock,
  nothing beyond its walls. Found on the way: `--dump-art` without `--scene` cropped to the last found place in
  the window (off-window, so it panicked); only a stamped scene crops to its site now.

## Production art (branch `refine/production-art`, on PR #9's production stage 1)
* PR #9 had already given the chute, shaker screen, bumper and ore hopper their own icons; in the world the three
  blocks drew flat colour and the hopper was placeholder rectangles.
* Blocks: `art::mat_color` patterns. Chute: a pale lip, boards with half-metre seams. Screen: a timber top rail,
  iron bars every half metre, the dark mesh between. Bumper: red-painted timber, a bright head, iron bands every metre.
* Ore hopper: `art::ore_bin` parts (back boards, front frame with the gate shut or open, leg, heap). The heap is
  tinted with the first item it holds and fills the straight part of the bin. The front is outlined only round the
  silhouette, so the window onto the heap stays open (the first dump showed black bands there).
* `--scene=production`: a sealed limestone works (a water pocket over the first try flooded it; the chute and screen
  needed the wall and a post or they fell as items). Dumped at frame 90: the copper heap shows through the bin's
  front, the chute reads as a sloped trough onto the screen, and the bumper is a red block at the track's end.
* Tests `production_blocks_have_their_own_look`, `ore_hopper_is_a_drawn_bin`.

## Rope gun art, riffle bars (branch `refine/rope-gun-art`, on #32 and #33)
* Icons: the rope gun is a root-wood bow-spring launcher (stock, bent spring, cord to the catch, crank, stone hook
  in the muzzle); the hook rope a twisted coil running out to a barbed stone hook. The uniqueness test passes.
* Held: `Held::RopeGun(loaded)`, with a hook in the muzzle only while a hook rope is ready and none is in flight.
* World: each rope segment is a twisted-fibre sprite turned along it (`art::rope_seg`), and the hook a stone
  head with two barbs (`art::rope_hook`). It points into the rock it bit, or along its flight while thrown.
* Found on the way: the placeholder rope used sprite kinds 43/44, which the tube hoppers also use. With a hopper
  and a rope on screen, the pool could hand one's sprite to the other. Ropes now use kinds 51/52.
* `--dump-art` `view.bmp` now composites rotated sprites (sampled back through their rotation), so ropes and the
  held item show in stills. `--scene=rope`: a hook rope in the ceiling (hanging into a slack pile on the floor) and
  a hook in flight paying out its rope. Checked at frame 5: the hook points along its flight, and the rope sags
  back toward the mole.
* Riffle bar (from #33): cell pattern of upright slats with a dark gap every half metre and pale water-worn tops;
  added to `--scene=production` and to `production_blocks_have_their_own_look`. Tests `rope_gun_art_is_drawn`.

## Rope gun art: polish (branch `refine/rope-gun-art-polish`)
* Evidence: the wiki's icon sheet (`docs/wiki/images/item-icons.png`) still showed 69 items. The chute, shaker
  screen, bumper, ore hopper, rope gun, hook rope and riffle bar were missing. It was regenerated from a fresh
  `--dump-art` sprite dump: all 76 items, then the 21 devices as the hotbar shows them.
* Read back on the sheet, the new icons sit with the rest (same scale, outline and lighting). No art change was needed.
* `site/` rebuilt on main 99ec464 (the rope gun art and riffles had merged without a web rebuild).
* Not seen in a live window: everything was checked through dump stills. The gun in the paws isn't in any still
  (the scenes hand the mole a pickaxe); its loaded/empty sprites were checked in the sprite dump and by test.

## Mechanical power art (branch `refine/power-art`, on #45)
* The five kinds from #45 (iron shaft, gear, water wheel, belt, counterweight) had drawn, unique pack icons
  already, kept. In the world they drew flat table colour, and the counterweight was a 1-px rope and a grey box.
* The cells move: `Game::turn` keeps, per drive train, how far it has run (sim ticks since the last frame × speed
  × `BELT_V`), and `art::mat_color` takes that `turn` (plus `horiz` for shafts). The renderer redraws every visible
  cell each frame, so nothing else was needed. Shaft: a twist band slides along (it lies the way its run is
  longer, so a 2-wide vertical shaft isn't read as horizontal). Gear: half-metre cogs turning, neighbours
  opposite ways. Wheel: metre wheels, spokes and paddles turning. Belt: chevrons pointing and sliding its way,
  rollers under. Drum: windings ticking.
* Counterweight: a fibre rope (the rope gun's rope sprite turned upright) to a cast-iron weight on its eye (`art::weight`).
* Stills: `--scene=power` (counterweight, shaft, gears, belt; a water wheel standing by, with no stream). Belt
  rows changed between frames 20 and 35 (22 of 60 px a row); frames 20 and 50 were identical there, exactly one
  4-cell chevron period apart at 1 m/s. The rope pays out by frame 400. The wheel read as a black lattice at first
  (open interior near-black), so the disc is now boarded in darker timber. `--dump-art` writes unlit swatches of
  every production and machinery cell (`cells-<Mat>-0/1.bmp`), since scene light hides cell art.
* Not seen: a water wheel actually turning in a stream (the still has none, so it was checked by swatch and test
  only), and anything in a live window.
* Test `machinery_moves_when_its_train_turns`.

## Mechanical power art: polish (branch `refine/power-art-polish`)
* The wiki icon sheet was regenerated (81 items, 21 devices); the shaft, gear, water wheel, belt and counterweight
  were missing from it.
* Read back on the sheet: the water wheel item's icon (a spoked wheel over blue water) and the water wheel device's
  hotbar icon (a spoked wheel in a frame) are alike at a glance. They can be told apart by the water, so this is
  noted, not redrawn.
* `site/` rebuilt on main 91d4943 (the power art merged without a web rebuild).
* Not tested: a water wheel turning in a real stream, and a live window (everything via dump stills and swatches).

## Open issues
* `--shot` writes an empty frame on macOS when the game is launched from a background shell (the surface is
  never presented). It needs a visible window, or Linux with Xvfb.
* Nothing here has been seen in a live window: this machine's shell can't capture or drive one. Sprites and the
  composited world were checked through `--dump-art`; still unchecked by eye are the climb pose's placement against
  a wall, squash/stretch, particles, the halos' strength, tooltip placement near screen edges, and the menu grid.
* Light, the hat, held items and music were checked headlessly (light scenarios, sprite dumps, composer tests),
  not in a live window or by ear.
* Remembered rock is quite dark (light floor 0.22). That's deliberate (Delvers see poorly) but worth a look in play.
* Stations, chests and mushrooms are snapped from vector art rather than drawn as pixel art from scratch.
