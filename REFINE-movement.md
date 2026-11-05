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

## Open issues
