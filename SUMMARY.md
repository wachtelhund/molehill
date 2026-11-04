# Disasters — summary and design notes (v2/disasters, WIP)

**Status:** this branch has a working TypeScript port onto the v2 falling-sand engine. It was stopped there because Molehill is moving to Bevy (Rust). Everything below is written to guide the Rust port. The v1 tile-grid version lives on branch `feat/disasters`.

## Feature
These are random disasters. Each one is preceded by a warning countdown (default 10 s) and then stays active for a few seconds:
- **Flood 🌊:** water springs from a horizontal source in the upper half of the world, preferring existing underground water pockets.
- **Lava surge 🌋:** lava springs from a source in the bottom 40% of the world, preferring existing lava pockets.
- **Earthquake 🌐:** screen shake, plus soft overhang cells near the epicentre get loosened. The engine's own physics and cave-in pass does the rest; disasters never retune it.

Calm gaps are 90–180 s. Everything is deterministic from a seed because the director owns its own RNG and never draws from the world's.

## Architecture (port this shape to Rust)
```
DisasterDirector (pure logic, no engine imports)  ←→  DisasterWorld trait  ←→  engine adapter
         ↑ state for UI                                                      (only file that knows engine internals)
   banner + site marker (presentation)
```

### The `DisasterWorld` interface: the whole contract with the engine
| method | meaning |
|---|---|
| `width`, `height` | world size in cells |
| `is_open(x,y)` | a cell that can hold fluid or be walked through (empty, liquid or gas) |
| `is_loose(x,y)` | soft, static, non-anchored material that a quake may knock loose |
| `fluid_at(x,y,kind)` | amount of water or lava in the cell (0..1; for a sand engine, 1 if the cell is that material) |
| `add_fluid(x,y,kind,amount)` | inject a fractional amount; the adapter decides how it becomes cells |
| `dislodge(x,y)` | loosen the cell and let the engine's own collapse take over |
| `shake(intensity)`, `say(text)` | screen shake and event log |

In Rust: `trait DisasterWorld`. The director can be generic over it, or the adapter can be a short-lived view struct borrowing the world inside a system.

### Director (`src/disasters.ts`)
- **State machine:** `calm → warning → active → calm`. The UI reads `phase`, `kind`, `x`, `y` (site) and `ticks_left` (`seconds_left = ticks_left / hz`).
- **Resolution-independent config:**
  - Times are in seconds, converted with `hz`.
  - Fluid volume is a fraction of the world's cell count (`floodVolume 0.012`, `lavaVolume 0.006`), spread evenly over the active duration.
  - Source width is a fraction of world width (`0.03`).
  - The quake's cell budget is a fraction of the cell count (`0.0015`), released evenly over its duration so it rumbles rather than pops.
  - The quake samples `0.003 × cells` random cells per tick in a square of half-size `0.3 × width` around the epicentre. It only takes loose cells with open space below (overhangs).
- **Site picking:** random samples within a depth band. It prefers covered sites (solid somewhere above) and, for fluids, existing pockets of that fluid. If random sampling fails, it does a full scan of the band from a random offset, so sparse worlds still find a site. If no site exists, nothing is scheduled.
- `schedule(kind)` forces a warning now (debug, scripts, URL `?disaster=quake`). `enabled = false` (`?disasters=0`) stops random rolls.

### Sand adapter (`src/disasters-sand.ts`): lessons for the Bevy engine
- **Classify materials only through property tables** (`KIND`, `ANCHOR`, `SPAN`), never numeric ids, because the material list will grow. Soft means static solid, structural (`SPAN > 0`), not an anchor, not wood (beams), and not already loose.
- **Quake:** `loose[i] = 1; structDirty = true; wake(x, y)`. The falling-sand CA crumbles the cell, and the support pass brings down what it held. In a test, a quake on a stable 16-wide dirt ceiling made 79 cells fall.
- **Fluid:** keep a fractional balance per source cell and spawn whole cells once it reaches 1. Place them at the nearest free cell reachable through connected liquid, using a flood-fill limited to 1024 visited cells (spring pressure). A straight-up column overflowed short caverns and lost about 30% of the volume. With the fill, flood delivers 1105 of 1106 target cells.

### Presentation (`src/disasters-ui.ts`)
- DOM banner: `⚠ 🌊 Flood in 7s`, then `🌊 FLOOD! (3s)`. It blinks for the last 3 s.
- Canvas marker: a pulsing ring with an icon at the site. `drawDisasterMarker(ctx, d, cellSize, time, size)` takes the marker size separately from the cell size, because on a fine grid one cell is too small for a visible marker.

## Hooks in `src/main.ts` (v2)
```ts
import { createDisasters } from "./disasters-sand";
import { updateDisasterBanner, drawDisasterMarker } from "./disasters-ui";
let disasters = createDisasters(world, seed, params), disastersFor = world;
function stepDisasters() { /* re-creates director when R/M replaces world */ ... disasters.step(); }
(window as any).__disaster = (k: any) => disasters.schedule(k);
// after every world.step() (main loop and __advance):  stepDisasters();
// in draw(), before hover:  drawDisasterMarker(ctx, disasters, 1, performance.now() / 1000, 2 * B);
// after hud(hb):  updateDisasterBanner(disasters);
```
`src/sand/*` is untouched.

## Files on this branch
- `src/disasters.ts`: director and interface, copied from `feat/disasters` unchanged.
- `src/disasters-ui.ts`: banner and marker. Same as v1 except for the new optional `size` argument.
- `src/disasters-sand.ts`: SandWorld adapter and `createDisasters`. New.
- `test/disasters.test.ts`: engine-free tests against fake worlds at two resolutions, plus SandWorld integration tests.

## Tests worth carrying to Rust
- A warning always precedes an event, the countdown timing is exact, and the director returns to calm.
- All three kinds get rolled; when disabled, nothing fires.
- Volumes and quake budgets scale with world area.
- Floods land in the upper half, lava sites are deep, and a fully solid world gets no site.
- The same seed gives the same disasters.
- A quake loosens only soft cells and brings down a stable ceiling.
- Fluid volume matches the config.
- A spring still adds fluid into a full pocket.

Run with: `npx esbuild test/disasters.test.ts --bundle --platform=node --outfile=/tmp/d.js && node /tmp/d.js`. It passes, as do `test/sand.test.ts`, `test/sim.test.ts`, `tsc` and the build.
