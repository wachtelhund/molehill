# v2/needs — WIP (TypeScript port stopped; Molehill is moving to Bevy/Rust)

Status: unfinished. The engine-agnostic core was copied over from `feat/needs` unchanged. A first SandWorld adapter (`src/needs-sand.ts`) is written, but nothing in `src/main.ts` calls it yet. It also has no tests and has not been run. `src/sand/*` is not touched.

## Feature
Each mole has three meters: **hunger**, **sleep** and **oxygen**. They decay over time, change behaviour and affect health. Food comes from **grubs**, which turn up while digging, and from buildable **mushroom farms**.

## Engine-agnostic design (use this for the Rust port)
- **Core and host interface.** `src/needs.ts` (`NeedsSystem`) never touches cells. It sees the world only through a small `NeedsHost` trait:
  - Grid: `width`, `height` and `cellScale` (cells per mole length).
  - Timing and RNG: `tickHz`, `seed`.
  - Queries: `isWalkable`, `isSolid`, `isSubmerged`, `isHazard`, `isSky`, `airCost` (<0 = airtight).
  - Moles: `moles()` returns `{id, x, y, dead, hasJob}`. Actions are `moveMole`, `hurtMole`, `healMole`, `dropJob` and `setOxygen` (mirrors the value into the engine's own o2 field).
  - Economy and messages: `spend`/`refund` resources, `stockpile()` (home point), `say`.
  - In Bevy this maps naturally to a resource/system that takes a `NeedsHost` impl over the world plus mole queries.
- **Units.** Rates are per second (converted with `tickHz`). Distances and search budgets are in mole lengths × `cellScale`. Nothing hardcodes grid size.
- **Grid.** The needs grid is the grid moles walk on. On v2 that is the 4×4-cell block layer (`mole.bx/by`, `cellScale = 1`), not the fine cells. Air searches over fine cells would be too slow.
- **Grid helpers.** `src/needs-grid.ts` has a bounded BFS path search and a bounded Dijkstra (`costTo`) for air. Visited sets are maps, not width×height arrays, so cost scales with the search budget, not the world size.
- **Determinism.** The core has its own seeded mulberry32 RNG and never consumes the engine's RNG.
- **Per-mole state.** Each mole has `MoleNeeds {hunger, sleep, oxygen, activity, path, target, timers}`. Activities are `none | toFood | eating | sleeping | toAir | toBuild | building`.
- **Engine integration points.** The engine needs to provide these:
  1. Call `step()` once per sim tick, after moles move.
  2. Don't give jobs to (or run job work for) moles where `busy(id)` is true.
  3. Gate job work with `workTick(id)`. Weak moles work at half speed.
  4. Call `onDug(materialName, moleId)` per dug cell. Grub chance is per cell, scaled by 1/cellScale².
  5. Forward the `farm`/`cancel` tools with `designate(tool, x, y)`.
  6. The needs system owns oxygen, or mirrors it into the engine's drowning logic.

## Rules and default tuning (`NEEDS_DEFAULTS`)
- **Hunger** −0.33/s, ×1.5 on a job. Below 40, an idle mole goes to eat. Below 15, it drops its job to eat. At 0 it starves at 1 hp/s.
- **Sleep** −0.24/s, ×1.5 on a job. Below 30, an idle mole sleeps in place. At 0 it collapses asleep even mid-job. Sleep refills at +2.5/s and the mole wakes at 95.
- **Oxygen**
  - Underwater it drops 10/s. The mole paths to the nearest dry, non-lava cell when oxygen falls below 35.
  - Out of water, fresh air must reach the mole from the sky within a cost budget (`ventReach`). Otherwise the air is stale: oxygen drops `o2Stale`/s and the mole suffocates at 3 hp/s.
  - **v2 decision:** air passes through open cells only, not porous soil. The v2 starting chamber is sealed under dirt, so stale air is gentle there (0.5/s) and the log says "dig a shaft".
- **Health and speed.** Hunger or sleep below 20 halves work speed. When hunger > 60, sleep > 50 and oxygen > 80, a mole regains 0.5 hp/s.
- **Grubs.** Digging dirt, clay, gravel or rubble can turn one up. They are stored at home and eaten there (+45 hunger).
- **Mushroom farm.** It needs a walkable cell with a solid floor and costs 4 wood.
  - An idle mole builds it in 6 s.
  - It grows one mushroom per 25 s, up to 5, and pauses while flooded. Moles eat at the farm (+35 hunger).
  - It is destroyed if buried or hit by lava. Cancel removes an unbuilt site and refunds the wood.

## How the v2 adapter works (`src/needs-sand.ts`)
SandWorld has no plugin hooks, so the adapter wraps `step()` and `designate()` on the world instance:
- **Grub detection.** Snapshot the blocks being dug before each step and diff them after; the rubble count caps how many cells count as dug.
- **Holding busy moles.** Give them an inert placeholder job (`{type:"dig", b:-1}`). `assignJobs` skips them, while the sim's flee check still runs.
- **Fleeing.** A fleeing mole is reported as inactive, so the needs system hands control back to the sim.
- **Drowning.** The sim already does it from the mirrored `o2`, so `drownDmg` is set to 0.

In Bevy these should be real system ordering and component flags rather than wrappers.

## Reference
The finished v1 version (grid sim, UI overlays/panel, 34 tests including a mock-host test) is on branch `feat/needs`.
