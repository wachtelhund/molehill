# v2/saveload — summary (WIP, frozen for the Bevy/Rust port)

Status: TypeScript port onto the v2 falling-sand engine was stopped part-way by the switch to Bevy. At the time of the freeze `tsc`, `bash scripts/build.sh`, `test/sand.test.ts` and `test/saveload.test.ts` all passed. It hasn't been tested by hand in a browser. Earlier v1 work is on `feat/saveload`.

## Files
- `src/saveload.ts`: the engine-agnostic core, copied from feat/saveload. One backward-compatible addition: named secondary grids.
- `src/saveload-sand.ts`: the adapter for `SandWorld` + `Logic`. Engine id `sand-v2`.
- `src/saveload-ui.ts`: Save / Load / Reset buttons, hotkeys, autosave. Hotkeys can now be configured, because `L` is the LoRa TX tool in v2.
- `test/saveload.test.ts`: tests against the sand adapter.
- Hook lines in `src/main.ts`. `src/sand/*` is untouched.
  ```ts
  import { initSaveLoad } from "./saveload-ui";
  import { sandAdapter } from "./saveload-sand";
  const saves = initSaveLoad(sandAdapter, { getWorld: () => world, setWorld: w => { world = w; }, newWorld: () => makeWorld(seed, null) }, { keys: { save: "k", load: "j" } });
  if (!params.has("scenario") && !params.has("fresh")) saves.resume();
  // in frame(): if (!paused) { saves.tick(dt * 1000); ...
  ```

## Design notes (carry these over to the Rust port)
1. **The core never sees engine internals.** A world reaches the core only through an adapter, `{ engine id, required layers, capture(world), restore(data) }`. `capture` returns:
   - main grid dimensions,
   - optional named secondary grids,
   - named flat layers (typed arrays),
   - a JSON-able state blob (entities, resources, tick, counters).

   In Rust this maps to a trait. A Bevy version would probably be a `SaveAdapter` resource or system that reads components and resources into a `Snapshot` struct.
2. **Save envelope:** `{ format: "molehill-save", version, engine, savedAt, width, height, grids?, layers, state }`.
   - Layer keys are `"name"` for the main grid and `"grid:name"` for a secondary grid. Layer length must equal that grid's width × height.
   - Each layer is `{ kind: u8|u16|i16|i32|u32|f32, len, data }`, where `data` is RLE(byte runs) and then base64. That's about 290 KB for 384×240 cells plus all other layers.
   - For Rust: keep the same envelope, or use serde with bincode plus zstd. Keep the version, engine id and per-grid dimension checks either way.
3. **Versioning:** `version` is an integer. `MIGRATIONS[v]` upgrades a save from v to v+1, applied in a chain. A save newer than the build is rejected. A save from a different engine id is rejected.
4. **Loading never panics or throws.** It returns either ok or a readable error. These are all rejected cleanly:
   - missing, truncated, non-JSON or foreign data,
   - a bad version or the wrong engine,
   - wrong dimensions, an undeclared or reshaped grid,
   - bad encoding, a wrong layer length, an unknown layer kind,
   - out-of-range material ids, entities out of bounds.

   Optional data that is missing or has the wrong type falls back to defaults, and the result is still sanitized: index lists are filtered to valid ranges, and moles are merged onto a template mole. The storage layer (a `KV` trait) returns errors such as a full quota instead of throwing. A corrupt autosave at startup is discarded and reported.
5. **Generic enumeration:** the adapter walks every field of the world and its logic sub-object.
   - Typed arrays become layers. Arrays of cell length go to the main grid, arrays of block length go to the `block` grid, and any other length gets its own 1-D grid.
   - Plain values, arrays, objects and Sets become JSON state.
   - Closures are skipped: rng, the `scripts` used by demo scenarios, and back-references.
   - New fields are therefore saved without editing the adapter.
   - Rust has no reflection, so use `#[derive(Serialize, Deserialize)]` on the world/components, or `bevy_reflect`, to get the same effect. Mark scratch and derived data `#[serde(skip)]`.
6. **Materials are stored by name.** The save includes the material name table, and loading remaps ids by name. A renamed material keeps its slot if that slot is free, and an unknown material rejects the load. This lets the material enum grow or be reordered. In Rust, serialize the material enum by variant name, or write the table out the same way.
7. **Determinism:** a loaded world must evolve identically to the original; the tests compare hashes 300 ticks after a load, including a wiring/LoRa demo and a save taken mid-collapse.
   - To make that work, save everything that affects stepping, including "scratch" data that isn't really scratch: dirty rects, `moved` stamps, support, pending.
   - Save the RNG *position*. `SandWorld`'s mulberry32 closure can't report it, so on the first capture the adapter swaps in an equivalent RNG that can, seeded from the current one. The live world and the save then continue identically.
   - In Rust, use an RNG whose state can be serialized, such as `rand_pcg` or `rand_xoshiro` with serde.
8. **UI and autosave:**
   - Two slots: `auto` and `manual`.
   - Autosave every 30 s of *unpaused* time, plus when the tab is hidden or closed.
   - The game resumes from the autosave at startup unless `?scenario` or `?fresh` is in the URL.
   - Load uses the manual save first and falls back to the autosave.
   - Reset asks for confirmation, clears both slots and starts a new world.

## Known gaps
- Demo `scripts` (closures) are not saved, so a demo saved before its scripted events won't replay them.
- Autosave keeps running during `?scenario=demo`, so the next plain visit resumes the demo.
- Saving the main grid as byte RLE is fine at 384×240. For a much larger Rust world, save per chunk and skip unchanged chunks.
