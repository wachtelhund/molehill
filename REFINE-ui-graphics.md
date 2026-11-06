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

## Steam art (branch `refine/steam-art`, on #49/#50)
* Boiler: drawn over its shell cells (`art::boiler`), a riveted drum with a dome and chimney over brickwork. Cold,
  the fire door is dark. Lit, the fire shows through its bars, a flickering glow sits round the door and three
  smoke puffs rise from the chimney.
* Steam engine cells: a metre unit per metre, a cylinder with a brass band, and a crank wheel whose pin goes round
  with the train's `turn`, the connecting rod swinging to it. Steam pipe cells: copper, a highlight along the run
  and brass flanges every half metre, oriented like shafts.
* Icons: the boiler and steam pipe placeholders were kept. On the sheet the engine (a small cylinder and wheel),
  ash (read like a stone) and slag (read like coal) were weak, so they were redrawn: an upright cylinder with rods to
  a spoked flywheel and a steam puff; a pale soft heap with cinders and an ember; a frothy clinker with a glassy
  blue-violet face, a rust streak and a set drip. Icon sheet regenerated (86 items, 21 devices).
* Stills: `--scene=steam` at frame 120 shows the lit boiler (fire through the door, smoke). The scene light is too
  dim to judge the engine and pipe cells, so their unlit swatches were checked: the crank pin and rod move
  between rest and 1.5 cells of travel. Rubble settled over the pipe in the still. That's the sim, not the art,
  and it was left alone.
* Not tested: a live window, and the engine actually turning under steam in a still (checked by swatch and test).
* Test `boiler_and_pipe_are_drawn`; the engine joins `machinery_moves_when_its_train_turns`.

## Steam art: polish (branch `refine/steam-art-polish`)
* Evidence: in the `--scene=steam` still, rubble lay heaped over the boiler, pipe, engine and belt, and the engine's
  cells didn't visibly change between frames 120 and 128. A headless copy of the scene (a throwaway example, since
  deleted) showed the engine at full speed from tick 40 and 653 gravel cells in the hall by tick 240. My scenes'
  limestone shell (a 15 m unsupported span) cracks under the stress sim and rains gravel; this was also the rubble on
  the floor of the production stills. All four scenes I added (production, rope, power, steam) now use a bedrock
  shell, and the headless copy then shows no gravel.
* With the clutter gone, the engine region changes between frames 120 and 128 (12 cells, up to 59 levels): the
  crank turns in the running game.
* Boiler: shell cells showed brick at the drum's top corners (the sprite was transparent there); the sprite now
  sets the drum against a brick back wall. Engine cells brightened (cylinder, band, wheel), and two low torches
  added beside the engine in the scene. Inside a metre-thick block the sim's light still falls off, so the engine
  stays dim in a still apart from its lit top and moving rod. That's the game's lighting for any thick solid, and
  it was left alone.
* Not tested: a live window.

## Refinement art (branch `refine/refine-art`, on #52)
* Works drawn over their shell cells (whose own colour is now a dark backing):
  * Stamp mill (`art::stamp_mill`): frame with back boards, cam shaft, guide beam and mortar box. Its two stamps
    follow `Works::stroke` half a stroke apart, a slow lift and a fast drop, the head resting in the mortar's mouth.
  * Washer (`art::washer_tub`): a hooped tub, with water while it has some. It rocks with its train's `turn`.
  * Smelter (`art::smelter`): its fire mouth shows four heat steps from `Works::temp`, with a glow from dull red
    upward and smoke while it burns.
* Cells: bellows' pleats breathe with the train, and moulds have an ingot hollow per half-metre tile that glows
  when setting.
* Icons: the machine placeholders were kept. The three concentrates read as sand, gravel or ash, and rust, so they
  were redrawn as washed grains in a shallow pan (copper), a deep clay bowl (tin) and a scoop (iron). One shared pan
  failed the silhouette check, so each got its own vessel. Icon sheet: 94 items, 21 devices.
* `--scene=refine`: in the dump at frames 90 and 100 the stamps moved (18 px) and the tub rocked (10 px). The smelter
  shows white-hot; the empty and setting moulds both read. The stamp mill's open frame showed the dark shell
  backing as a black box, so back boards were added. The bellows (a half-metre block, dim) barely change in the
  still (2 px); their motion is checked by swatch and test.
* Not tested: a live window; the smelter's real heat curve over a firing (the still starts it hot).
* Test `refinement_works_are_drawn`; the bellows join `machinery_moves_when_its_train_turns`.

## Refinement art: polish (branch `refine/refine-art-polish`)
* Smelter heat steps: a headless run of the refine scene with a cold smelter (a throwaway example, since deleted)
  climbed 667 / 1000 / 1171 / 1323 °C at 10 / 20 / 30 / 60 s with the bellows blowing, then settled toward 800 °C
  once the weight ran out. Two rough edges: white-hot started at 1150 °C though copper and tin smelt from 1100
  (`refine::charge`), and the unblown fire settles right on the 800 °C orange step, so it could flicker. The steps
  are now dull red from 300, orange from 700 and white-hot from 1100 (smelting heat).
* Bellows: in the #55 still a half-metre block changed 2 px between frames, because the pleat bands were scaled to
  a whole metre. Its creases now slide a whole cell each way, and the same block changes 21 of 64 px between
  frames 90 and 97 (located from the frame difference in `view.bmp`; `view_cells.bmp` is on another scale, so crop
  boxes taken from one don't fit the other).
* Checked and left: at rest the stamps' heads sit in the mortar's mouth; the tub tilts with its water; the moulds
  and concentrate icons read as before.
* Not tested: a live window.

## Non-electric logic art (branch `refine/logic-art`, on #57/#60)
* `art::mat_color` takes `on` (pulled, pressed or lifted, from `World::pulled`) for the LINK cells. A float's
  `turn` is a tick phase, so it bobs.
* Cells:
  * Pull-rod: an iron rod through timber guides, oriented like shafts, its brass collar sliding past the guide when pulled.
  * Tread plate: studded chequer plate; pressed, a dark gap opens along its top.
  * Float: a tarred timber barrel whose highlight bobs when lifted.
  * Sluice gate leaf: planks with iron straps.
  * Scale pan: an iron pan with a rim.
  * Clutch: two halves with a dark gap out, and meshed teeth turning in.
* Sprites:
  * A lifted sluice gate's leaf is drawn raised out of the opening (its cells are open then).
  * The gate lever points up, or down when turned round.
  * Tipping scale: a post at the pan's end, a beam tipping pan-down once the pan outweighs the stones, a rod
    down to the pan, and the counter-pan with up to 8 stones.
* Icons: the placeholders were kept apart from the pull-rod, which read like the iron shaft (rods on a bracket).
  It's now a bell crank. Icon sheet: 100 items, 21 devices.
* `--scene=logic` at frame 90: the shut gate with the pool behind and its lever up, the turned-round gate standing
  lifted with its lever down, the tread plate's edge, and the scale (beam pan-side up, empty pan, three stones) all
  read. The rod's collars show over the tank. The float and clutch sit dim in the scene light, so their pulled
  states were checked by the `cells-*` swatches (second frame pulled) and the test.
* Not tested: a live window; a gate actually lifting and dropping, a scale tipping under load, and a plate pressed
  by the mole in a still (each state was drawn from staged data, with motion from swatches and tests).
* Test `linkage_parts_show_their_state`.

## Logic art: polish (branch `refine/logic-art-polish`)
* Evidence: #61's still drew every state from staged data, so no state had been seen changing in the running game, and
  the float and clutch end of the hall read too dark (mean luminance 29 over that crop).
* `--scene=logic` now puts the tread plate in the burrow floor right under the mole, rodded under the floor to the
  pool's gate. In the running game at frame 90 the mole's weight presses the plate (its dark sunk edge shows) and
  the gate has lifted (its leaf drawn raised, lever up), next to the turned-round gate (lever down). That's a
  pressed-plate and lifted-gate state reached by the sim, not staged.
* Two scene torches (inside the tank over the float, and beside the clutch): the crop's mean luminance goes from 29
  to 39, and the water and the float's lit edge show. The clutch's middle column stays dim, so its in/out state
  still rests on the `cells-*` swatches and `linkage_parts_show_their_state`.
* No art change was needed: the rod collars, gate levers, scale and plate read as shipped.
* Stills only (dump frames), no live window. Stills plus tests cover each state: the plate and gate change in the
  running sim, while the float bob, clutch mesh and scale tip are covered by swatch and test.

## Steam borer art (branch `refine/borer-art`, on #62/#64)
* The placeholders (kinds 120–123: a grey box, a brass box and two dark squares) are replaced with `art::borer`,
  flipped to the borer's facing:
  * Body: brick firebox with its door dark or glowing, a riveted barrel, the stack, a seat and drive lever, and the
    spoil belt over the top.
  * The toothed bore head turns through three frames while it drives or grinds with steam up.
  * Two spoked wheels turn with the ground covered.
  * While it burns, a door glow and three smoke puffs.
* Icon: the placeholder (a grey box with a brass blob) was redrawn to match: firebox, barrel, stack and puff,
  toothed cone, spoked wheels. Icon sheet: 101 items, 21 devices.
* `--scene=borer` seats the mole on a lit borer in a tunnel facing a dirt face. With `--shot-walk` the running game
  drives it (`tr.borer_ride` plus the walk input). At frames 60 and 120 it has moved up, cut into the face and piled
  spoil behind, with its door glowing, head in the dirt and wheels on the floor. That's driving reached by the sim,
  not staged.
* Not tested: a live window; smoke from the stack in a still (the riding mole sits over the stack); the borer
  facing left (drawn by flipping, not seen).
* Test `borer_is_drawn`.

## Steam borer art: polish (branch `refine/borer-art-polish`)
* Evidence: #65 had never shown a borer facing left, and the stack's smoke was hidden under the riding mole.
  `--scene=borer` now also parks a lit, unridden borer facing west behind the ridden one. At frame 60 the flip
  mirrors everything: the head points west, the firebox and its glowing door sit at the east end, and the stack and
  wheels are in place. Faint smoke puffs show above both stacks. No art change was needed.
* The first try parked it over the burrow's own torch (the scene cuts the tunnel's cells but kept world torches
  inside it). The scene now drops torches inside the tunnel and parks the borer clear.
* Stills only, no live window. Stills plus `borer_is_drawn` cover both facings, the lit door, the turning head and
  driving (with `--shot-walk`).

## Wagon art (branch `refine/wagon-art`, on #67)
* The placeholders (ids 130–133: a tinted box, two dark squares, a fill rectangle, a grey bar) are replaced, with the
  same ids plus 134 for the water gauge:
  * Each kind has its own body (`art::wagon`): store, a low open plank box; workshop, a steep gable on posts over a
    saw bench; bunk, a closed car under a low canvas barrel roof with a lit window; water, a riveted tank with a cap
    and a gauge glass; coal, a flared riveted bunker with a stoking chute.
  * Two spoked wheels turn with the ground covered.
  * The load heaps above the rim as a store or coal wagon fills, and the water level rises in the gauge glass.
  * The coupling is an iron bar with a link at each end.
* The icons are built from the same bodies (`art::wagon_px`), on wheels, the store, coal and water ones showing a
  load. The first workshop and bunk roofs shared a silhouette (8 px apart), so the workshop got a steep gable and the
  bunk a low barrel roof. Icon sheet: 106 items, 21 devices.
* `--scene=wagons` at frame 60: the five read apart in a row behind the ridden borer: the bunk's lit window, the
  workshop's gable and saw, the store's crates above its rim, the coal heap, and the water gauge at about 60%. The
  coupling bars join them.
* Not tested: a live window; the train hauled through a run (`--shot-walk` drives it but no still was taken
  moving); wagons coupled behind a borer facing west (the bar is placed by the borer's facing).
* Test `wagons_have_their_own_silhouettes`.

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
