# Refinement log: movement and feel (branch `refine/movement`)

Polish only: no new features or concepts. Each entry is one commit.

## Changes

1. **Placed structure is rigid (bug from Hampus).** Placed wood pillars and iron beams were crumbling and
   rolling away as debris. Cause: the stress solver's overburden ran straight through open space, so a beam
   laid across a pit "carried" the whole cave roof above it (~390 kPa). Floors under caves did too. Now
   overburden stops at voids: the rock above an opening is carried by the roof's bending. The load *through*
   a void is kept separately as the tributary load for walls and pillars beside it, and a run only takes that
   load if it reaches the void's roof, so a half-built pillar or a floor bump carries only what rests on it.
   Blocks still fail by stress (overload, too long a span) and then break into debris.
   **Superseded on merge:** `main` got its own fix for the same bug from the ui-graphics run (6bbb493: loose
   ground carries what rests on it, roof load only through contact). The merge keeps `main`'s `stress.rs` and
   harness `place`. My scenarios stay as extra coverage and pass on `main`'s solver: `beams_placed_across_pit`,
   `pillar_in_cave`, `clay_roof_falls` / `pillars_hold_clay_roof`, `beam_overload_wide`. The roof pair moved
   from dirt to clay because `main`'s overburden still passes through caves above, which crushes a dirt seam
   under the scenario's 20-cell granite frame whatever holds it up.
2. **Space on menu start no longer jumps.** The menu closes on Space/Enter/click and the game's input ran in
   the same frame, so the held Space was read as a fresh jump (and a click could dig, place or throw TNT).
   `World::swallow_held()` makes a held jump count only after release; the body re-sync (new world,
   respawn, load) does the same. The game ignores mouse buttons held since the menu closed until they're
   released. Test: `jump_held_through_a_menu_does_not_fire`.
3. **Tight spaces for the 1.8x2.9 body.** A 2-wide shaft left 0.1 cell of slack, so jumping up into one
   almost always bonked. Corner correction (`TUNE.nudge` 0.6): rising into a corner slides the body round it,
   and airborne sideways moves snap the feet to the nearest row to slot into a tunnel mouth (it used to take
   a cling and a slow slide down to line up). Dig aiming: the round bite left a lip at head or toe height
   next to the body, so pointing ahead with the pickaxe only tunnelled when aimed exactly at mid-body. Now a
   bite aimed level with the body (or straight up/down) also clears a body-sized passage to the aim point
   (`World::passage_cells`, at most 6 cells, only cells the tool can dig). Scenarios: `shaft_jump`,
   `tunnel_mouth`, `dig_tunnel`, `dig_tunnel_high`; harness `dig` directive (aim relative to the mole).
4. **Air control and climbing.** `air_accel` 260 -> 480: reversing in mid-air took about 7 ticks and now takes
   about 4. A new `air_decel` (60) applies when no direction is held, so letting go in the air coasts and keeps
   wall-jump momentum instead of braking like the ground does. `climb` 9 -> 10.5. Scenario: `air_turn`.
5. **Impacts and instant feedback.** Any knock of more than 0.5 hp in a tick (fall, lava, being buried,
   drowning) sets `pl.flash` (0.22 s, a white/red strobe on the mole sprite) and a view shake scaled by the
   damage. Hard landings (8+ cells) thump the view a little. Mining sounds its strike on the first tick of the
   press and every 1/6 s after, instead of only when the rock gives. A cleared bite and a placed block (or
   device, or brush) leave a `World::marks` entry. After the merge, `main`'s chip burst fires on the bite mark
   (not on `Event::Dig`, which now also sounds on press), and a placement flashes white for 0.2 s. Camera: the lookahead eases at 7/s, so turning doesn't whip the view; the follow stays tight, and it
   only looks down on real falls. Test: `digging_and_placing_give_instant_feedback`; scenario: `fall_hurt`;
   harness vars `flash`, `shake`.
6. **Step-up and mantle checked.** Both already behaved (steps of 1-2 cells at full speed; a climbed wall
   rolls you onto its top), so there was no tuning change, just coverage: scenarios `step_up` and `mantle`.

### Run 2

7. **Wall-jump grace.** Clinging and pressing away from the wall plus Space (the natural wall-jump input) let go
   of the wall first, so the jump found no wall and the mole just dropped. Leaving a cling now keeps a
   0.1 s grace (`TUNE.coyote`, like ground coyote) in which Space still kicks off that wall. Scenario:
   `wall_jump_away` (same-tick press, a late press 4 ticks after letting go, none after 8 ticks).
8. **One frame less lag on screen.** The camera follow and the mole's interpolated draw position were worked
   out in `input()`, before that frame's sim steps ran, so every frame showed the mole where it was a step
   earlier (about 16 ms behind the keys). They now run in `follow()` right after the steps (and after the old
   shake offset is removed), so a press shows on the same frame. Game-only, so there's no scenario; the
   cursor-to-cell mapping still uses the camera as drawn, which is what the player sees.
9. **Quick taps aren't lost on fast displays.** Keys are read once per frame but the sim steps at 60 Hz, so on
   a 120/144 Hz display a Space (or `W`) tap that went down and up between two steps was never seen. The press
   is now latched until the next step consumes it, then the jump input goes back to the key's real state (so a
   tap still gives the short hop). The menu-closing Space stays swallowed (`swallow_held`). Game-only.

## Open issues
* `main`'s solver still sends overburden through open caves, so dirt seams under deep rock are crushed in
  compression whatever supports them. That's a modelling choice from the other run; I left it alone.
* No screenshot of the hit-flash, pops or camera: this run was on macOS without a capture setup. They need a
  look in the real game.
