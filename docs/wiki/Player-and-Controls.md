# Player and Controls

> Code: `World::step_player` and `TUNE` in `engine/sim/src/world.rs`; keys in `input` in `engine/game/src/main.rs`; the controls table in `menu::CONTROL_ROWS`.

You play one Delver: a body about **45 × 73 cm** (1.8 × 2.9 tiles of 0.25 m, i.e. 3.6 × 5.8 cells: the mole is about 4 cells wide) moving through a world of 12.5 cm cells. The movement is pure sim code, so every feel change can be tested headless (see [Testing](Testing.md)).

![Digging with the bore: the ring shows the bite](images/hud-digging.png)

## Controls

| input | action |
|---|---|
| `A` / `D` | Run. Instant start and stop. Steps of 1–2 tiles (up to 0.5 m) are climbed at full speed, and you shove through loose sand |
| `Shift` held while running | Sprint: 1.6× speed (instant), so jumps carry further; burns a little food. Shift-click is left free |
| `Space` (or a fresh `W` press off a wall) | Jump. Hold for a full jump, tap for a short hop |
| Into a wall in the air | Cling (side walls only: a ceiling just stops the jump). `W` / `S` climb up or down (speed eases in over ~3 ticks; a grab while falling catches at once); let go and you slide slowly. You grab as soon as a jump's rise slows to climbing speed, hold on across a notch, and ease out round a one-cell outcrop. `Space` wall-jumps away (also pressing away plus `Space`, or `Space` within 0.1 s of letting go); at the top you rise to the edge and roll over it smoothly (holding `W` or toward the wall); holding `W` through it doesn't hop |
| `W` / `Space` in water | Swim up |
| `S` | Dig down (on the ground) |
| Left mouse | Use the item in hand: dig, place a block brush, torch, copperline, device, spore, TNT or station |
| Right mouse | Interact: flip a lever, step a thrum pattern or ticker beat, light TNT or a fuse cord |
| `Ctrl` + right mouse | Pick up copperline or a device (a lit charge is defused) |
| `[` / `]`, `Alt` / `Option` + wheel | Brush size for the tool or block in hand, within its range |
| `1`–`0`, wheel | Hotbar slot (a trackpad flick moves one slot, momentum included). Sprinting doesn't change this |
| `I` | Pack (inventory grid): click a slot to pick it up, click another to drop or swap. The bottom row is the hotbar |
| `+` / `-`, `Ctrl` / `Cmd` + wheel (or pinch) | Zoom (clamped to scale 0.12–0.75 and saved) |
| `Q` | Headlamp lit / out (also in Settings; saved). The fungus-oil lamp on the mole's hat throws a beam toward the cursor and reveals the way ahead |
| `H` | Controls overlay |
| `Esc` | Pause menu, or close a panel |
| `K` / `L` | Quick save / quick load |
| `F3` | Developer info |

Menus pause the sim. A key or mouse button held while a menu closes is ignored until released, so the click or `Space` that started the game never digs or jumps.

## The feel target

Movement should feel like **Streets of Rogue**: instant start and stop, no input latency, and every action firing on press with immediate feedback. In practice:

* Running reaches top speed in under 8 ticks and stops almost at once (ground acceleration 900, deceleration 1400 tiles/s²).
* Reversing in mid-air takes about 4 ticks. Letting go in the air **coasts** (only 60 tiles/s² of drag), so wall-jump and run-off momentum carry.
* **Coyote time** and a **jump buffer** of 0.1 s each: a jump 4 ticks after leaving a ledge works; 12 ticks after, it doesn't. The same 0.1 s grace applies after letting go of a wall. A jump tap is latched until the next 60 Hz step, so a quick tap on a fast display is never lost, and the camera follows after the step, so a press shows the same frame.
* Releasing jump early triples gravity (`cut_mul`) for a short hop. Falling uses 1.7× gravity, so arcs are snappy rather than floaty.
* **Corner correction** (`nudge` 0.6 tiles): rising into a corner slides the head round it, and moving sideways in the air snaps the feet to the nearest row so you slot into tunnel mouths. Jumping into a 2-wide shaft half a cell off-centre still goes up.
* Feedback is immediate: the mining strike sounds on the first tick of a press and every 1/6 s after; a cleared bite or placed block pops on screen; any knock of more than 0.5 hp flashes the mole for 0.22 s and jolts the view in proportion; hard landings (8+ tiles, 2 m) thump the camera and squash the sprite.

## `TUNE`

Every movement constant lives in one place, `TUNE` in `world.rs`. Values in the table are in tiles (0.25 m) and seconds, with +y down; in code they're multiplied by `S` (cells per tile).

| constant | value | meaning |
|---|---|---|
| `run` | 15 | top run speed (tiles/s, about 3.75 m/s) |
| `ground_accel` / `ground_decel` | 900 / 1400 | speeding up / braking on the ground |
| `air_accel` / `air_decel` | 480 / 60 | steering / drag in the air |
| `gravity`, `fall_mul`, `cut_mul` | 150, 1.7, 3.0 | gravity; extra on the way down; extra after releasing jump |
| `jump_v` | 42 | take-off speed (a full jump rises about 6 tiles, 1.5 m) |
| `terminal` | 60 | maximum fall speed |
| `coyote`, `buffer` | 0.1, 0.1 | grace windows (s) |
| `climb`, `slide`, `climb_accel` | 10.5, 3, 240 | wall climb speed; slow slide when clinging without input; how fast climbing speeds up |
| `wall_jump_x`, `wall_jump_y`, `wall_lock` | 17, 36, 0.13 | wall-jump kick and how long steering is locked after it |
| `step` | 2 | cells stepped up automatically on the ground |
| `mantle_v` | 9 | sideways speed rolling over a ledge at the top of a climb |
| `sprint` | 1.6 | run speed multiplier with `Shift` |
| `nudge` | 0.6 | corner-correction reach |
| `swim_mul`, `swim_grav`, `buoyancy`, `swim_up`, `swim_max` | 0.55, 0.35, 40, 70, 11 | water |
| `lava_mul`, `lava_sink`, `lava_up`, `lava_dps` | 0.25, 3, 22, 75 | lava |
| `safe_fall` | 20 | tiles (5 m) you can fall without harm |

Change `TUNE`, then rerun the tests and scenarios. The feel scenarios (`run_jump`, `coyote`, `coyote_late`, `air_turn`, `wall_jump`, `wall_jump_away`, `no_ceiling_climb`, `wall_climb`, `climb_smooth`, `climb_mantle`, `mantle`, `sprint_speed`, `step_up`, `shaft_jump`, `tunnel_mouth`) assert the numbers that matter: rise, `vx` after N ticks, and the coyote window.

## Water and lava

* **Water** slows you to 55 %, mostly cancels gravity, and lets you swim up with `W` or `Space`. Your breath runs down while you're under; when it runs out you lose health.
* **Lava** is thick: you sink into it at 3 tiles/s, wade at a quarter speed, can barely claw up, can't cling to walls, and burn at 75 hp/s.

## Vitals and death

The mole has **health**, **food** and **breath**, shown as the three bars in the top-left corner.

* **Falls:** each tile (0.25 m) beyond 20 tiles (5 m) costs 3 hp.
* **Hunger** drains over time. Below 40 the mole eats from the pack on its own: a grub restores 45, a mushroom 35 and fungus 25. Starving hurts.
* **Breath** drains in stale air (CO2 above 0.45), underwater and when buried by falling rock or sand. A mole buried in loose ground wriggles up through it. Under rock it stays put: the collision snap searches at most one cell, so it can never pop out through a slab into the room above (`buried_under_rock`, `buried_in_sand`).
* **Death:** after 2.5 s you respawn in the burrow with full health and breath (the window slides back to it if needed). The death count is kept in saves.

## Digging and reach

You can act on cells within **4.5 m** of the mole. A dig tool carves a **round bite** under the cursor. A cell takes `dig × 4 / (1 + tier) + 1` ticks (never fewer than 4), and the tool's tier must reach the material's tier. If you aim level with the body (or straight up or down), the bite also clears a body-sized passage to the aim point (at most 6 tiles, 1.5 m, and only cells the tool can dig), so pointing ahead always tunnels through. Tools and the brush are on [Tools and Items](Tools-and-Items.md).
