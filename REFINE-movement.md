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
10. **Wall jumps stretch too.** The take-off stretch only fired when leaving the ground; it now fires on any
    sudden kick upward when not clinging (ground jump, wall jump, a hop out of water), so a wall jump reads
    on the sprite the same frame. Climbing doesn't trigger it. Game-only.
11. **Ledge jumps checked.** Probed run-and-jump onto 3-, 4- and 5-cell ledges from 14 take-off points each:
    all land on top; a late take-off costs at most ~0.25 s (cling, then the mantle). No change.
12. **No clinging to ceilings (bug from Hampus).** The wall probe was the body box moved 0.08 sideways *and
    0.02 up*, so a head flush against a ceiling read as a wall on both sides: pressing sideways under a low
    roof clung to it, and `W` "climbed" the ceiling. Walls are now found by `side_wall`: the column just past
    the body's edge, over the rows the body spans, inset 0.1 at the head and feet. A ceiling only stops the
    rise and the mole falls. Scenario: `no_ceiling_climb` (16 take-offs both ways plus straight up, cling
    checked every tick).
13. **Sprint (request from Hampus).** `Shift` held while moving runs at 1.6x (`TUNE.sprint`), reached in 2
    ticks and dropped at once like the run; it applies in the air too, so a sprint jump goes ~11.5 cells
    instead of ~7.5. It burns 0.5 food/s on the ground. Shift-click is left free. Scenario: `sprint_speed`.
14. **Smooth wall climbing (request from Hampus).** Climbing felt clunky; traced tick by tick:
    * the grab waited for the jump's apex: now it catches as soon as the rise slows to climbing speed;
    * climb speed was set instantly: now it eases in at `climb_accel` 240 (about 3 ticks) and a grab while
      falling still catches at once;
    * the top of a wall was a 1-3 cell teleport (`TUNE.mantle`, removed): now the mole rises to the edge at
      climb speed and is carried over it at `mantle_v` 9 (`Player.mantle`), never moving more than a quarter
      cell a tick, with `W` alone or toward the wall;
    * a one-cell outcrop overhead stopped the climb dead: the mole now eases out round it 1.5 cells early
      (corner correction also tries one cell further while clinging) and back in after it;
    * a shallow notch pulled the mole in and bumped it back out: a cling now holds across a gap of up to a
      cell, and is only pulled into a wall that steps back for good;
    * `W` held through the mantle hopped on arrival: `W` now jumps only on a fresh press.
    The sprite needed no change: its claws already sit on the wall face. Scenarios: `climb_smooth`
    (constant `vy` every tick on a ragged wall), `climb_mantle` (harness var `step`: distance moved this tick).

15. **First hour: no hidden deaths under the feet (first-hour queue).** A new survey
    (`examples/firsthour.rs`) measured the drops around the burrow: within 30 m, about 1 seed in 7 has a
    ledge or a thin floor (under 1 m) over a drop that kills (~13.3 m from full health), e.g. seed 16 spawns
    1 m above a 0.5 m worm shaft ~17 m deep. Driving the real thing (hold `S` from the burrow for 15 s on 32
    seeds) found what actually kills: seed 9 digs into lava ~37 m down within 12 s. Fall damage is untouched.
    Instead a dig that cuts into the floor under the body and would leave less than 1 m of it over a drop that
    would kill at the hp left (onto rock; water is safe), or over lava, is held back at once: a warning thump
    and "The floor rings hollow..." / "The floor is hot under your claws...", once a press and at most every
    5 s (`World::fatal_drop`, `HOLLOW_FLOOR` = 1 m). It counts any thin column under the body, not only a
    full hole: the first version only refused a bite that opened the whole floor, and the scenario showed the
    stress solver snapping the 0.25 m slab a narrower bite left behind. Digging beside the drop still works,
    so you open it, look, and go down by choice. The out-of-reach refusal
    is bugs-logic's ("Too far to dig: step closer.", on main); the game now passes an out-of-reach press with
    the pick to the sim so it can actually say it. Scenarios `dig_hollow_floor`, `dig_down_lava`,
    `dig_down_shallow`, `dig_down_water`; test `tests/first_hour.rs` (fails without the rule: seed 9 burns).
    Message wording sits beside `dig_refusal` in `world.rs` (`HOLLOW`, `HOT`) for bugs-logic's message table.

16. **Falling sand isn't a wall (Hampus's playtest, 2026-11-06).** The mole's collision box, wall-cling and
    ledge checks treated every powder cell as solid, grains in mid-air included: a stream of sand pouring from
    a slot could be clung to and climbed like a wall (the mole climbed it 3 m and hung there for good), stood on
    in mid-air, and walked into as a wall (the mole stopped dead at the stream). Now `World::in_flight` (sand,
    gravel or soil that moved in the last two CA steps, from the grid's `moved` stamps, with air or another
    grain in flight under it) is ignored by `box_hits`, `side_wall_at` and `ledge` (`body_blocked`), and by the
    burial check, so the mole passes and drops through a pour unhurt. Grains rolling down a pile have the pile
    under them and still carry the mole, as do settled heaps. No shared flag: the grid already stamps each move.
    Scenarios `falling_loose_no_climb` and `falling_loose_walk_under` (both fail with the rule off).
    Polish run (headless playtest): digging up through a dirt ceiling into sand, the pour comes down past the
    mole unhurt and it shoves through the settled heap (about 0.5 s) and on; standing still under a pour, it
    rides up on the growing pile (at most 1 hp from a grain settling in it, regained). No change needed.

## Open issues
* Walking off a ledge you dug open is still a free choice: a dug-open cave shows itself, and run/stop is
  instant, so I left ledge drops alone. Fissure sites (26 m, ledges every 6-7 m) still hurt at each ledge.
* `main`'s solver still sends overburden through open caves, so dirt seams under deep rock are crushed in
  compression whatever supports them. That's a modelling choice from the other run; I left it alone.
* No screenshot of the hit-flash, pops or camera: this run was on macOS without a capture setup. They need a
  look in the real game.
