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
   Harness: `place T dx dy Item` and the `I` map char. Scenarios: `beam_spans_gap`, `pillar_in_cave`,
   `dirt_roof_falls`, `pillar_holds_roof`, `beam_overload`.
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

## Open issues
