# Molehill

A colony sim about moles digging a falling-sand world where every tunnel has to carry real loads.

* **Play (web build):** https://molehill-prototype.vercel.app
* **v3 engine:** Rust + Bevy 0.20 in [`engine/`](engine/). The older TypeScript v2 prototype lives in [`legacy/`](legacy/).

## Layout

| path | what |
|---|---|
| `engine/sim` | `molehill-sim`: pure-Rust simulation (no Bevy). Falling sand/fluids on flat arrays with 64×64 chunks and dirty flags, a 4-phase checkerboard parallel update (rayon on native, single-threaded on wasm), structural stress, procgen, light, fog, colony AI, crafting and quests. `cargo test -p molehill-sim` |
| `engine/game` | `molehill`: the Bevy front-end. It composites the world into one GPU texture each frame (albedo × per-cell light, fog, cracks, designations, parallax background) and draws vector-style sprites for moles, items, chests, stations and torches, plus a text HUD. |
| `engine/web` + `engine/build-web.sh` | the wasm32 build (wasm-bindgen, then wasm-opt) written to `dist/` and deployed to Vercel |
| `legacy/` | the TS v2 (canvas) prototype, its tests and screenshots |

## Build

```bash
cd engine
cargo run --release -p molehill          # native (Linux/macOS)
cargo test --release -p molehill-sim     # simulation tests
./build-web.sh                           # wasm into ../dist (needs wasm-bindgen-cli 0.2.130 and the wasm32-unknown-unknown target)
```

## Controls

LMB designates or places with the current tool, RMB clears, and the mouse wheel or `+`/`-` zooms. Pan with WASD or the arrow keys (hold Shift to pan faster). `[` and `]` change the brush size.

| key | action |
|---|---|
| `1` | Dig |
| `2` | Wood pillar |
| `3` | Brick |
| `4` | Iron beam |
| `5` | Torch |
| `6` | TNT |
| `7` | Station (`Tab` cycles workbench, kiln, furnace, anvil) |
| `8` | Move the stockpile |
| `9` | Cancel |
| `Q` / `E` | Choose a recipe |
| `Enter` / `C` | Queue the recipe |
| `Space` | Pause |
| `F` | Change speed (×1, ×2, ×4) |
| `N` | New world with a new seed |
| `H` | Show or hide help |

## World generation (`sim/src/gen.rs`)

Each game uses a new seed. The grid is 1024×512 cells, and one cell is 0.25 m.

* **Biomes** vary along x (Loamlands, Karst caverns, Granite crags, Ashlands) and shift layer depths and cave density.
* **Strata by depth:**
  * grass on top;
  * topsoil dirt to about 6 m;
  * dirt and clay with sand or gravel lenses, which can be saturated aquifers (`F_WET`, so digging them releases water);
  * limestone or granite;
  * mixed sandstone and granite;
  * basalt and granite below about 90 m, where lava lakes sit.
* **Caves:** cellular-automata caves on a 2-cell lattice, plus winding worm tunnels.
* **Ore veins** follow thin ridged-noise bands: coal seams, copper, tin, iron and quartz, each in its own depth window.
* **Features:** crystal geodes and quartz on deep cave walls, glowing fungus on cave floors, root systems under the surface, and 24 treasure chests (on cave floors or buried).
* **Stability pass:** the world is settled for 240 CA ticks, then the structural solver runs and failing cells are reinforced or removed. A fresh world therefore stands, and collapses are caused by you, or by the deep lava/water front.

## Structural model (`sim/src/stress.rs`)

Units: cell edge s = 0.25 m, unit depth b = 1 m, g = 9.81 m/s². Densities are in kg/m³ and strengths in kPa. Strengths are rock-mass values (intact lab values scaled down for jointing), for example:

| material | ρ (kg/m³) | tensile (kPa) | compressive (kPa) |
|---|---|---|---|
| dirt | 1600 | 15 | 250 |
| clay | 1800 | 40 | 400 |
| limestone | 2500 | 400 | 9000 |
| granite | 2700 | 800 | 20000 |
| wood beam | 600 | 3000 | 8000 |
| iron beam | 7800 | 40000 | 120000 |

1. **Overburden:** P(x,y) = Σ ρ·g·s / 1000 [kPa] over the non-empty cells above.
2. **Load path:** static solids that are not 4-connected to bedrock (through static solids) fail. Floating islands fall.
3. **Roof bending:** every horizontal run of static cells with nothing static under it is a beam with span L = n·s. Its depth h is the static thickness above it, capped at L (arching: only about one span of rock works as the beam). q is the superimposed load on top of that beam, counted only when the static thickness ends inside the cap.
   * M = k·(ρ g h + q)·L², with k = 1/8 when both ends rest on supported cells and k = 1/2 for a cantilever.
   * Z = b·h²/6.
   * σ_t = M/Z, compared with the weakest tensile strength along the run.
   * Example: a 1 m span of 1 m-thick dirt holds, while a 6 m span fails (tested). Wood pillars every 1 m save a wide dirt room (tested).
4. **Pillar and wall compression:** every horizontal run of static cells of width w carries its own overburden plus half the overburden of the voids on each side (tributary area). σ_c = (Σ_run P + ½Σ_left void P + ½Σ_right void P)/w, compared with each cell's compressive strength.
5. **Utilisation:** u = σ/strength. When u > 1 the cell cracks (visible), with a 40-tick countdown. If the load is relieved in time (by a pillar or by digging less), the crack heals. Otherwise the cell crumbles into its loose form (dirt, gravel or sand) and joins the falling-sand sim, which can load the next roof and cascade.
6. **When it re-solves:** after structural edits (every 6 ticks while anything is dirty) and after lava/fire reactions (every 24 ticks). A full solve of 512k cells takes about 10 ms natively.

## Light and fog (`sim/src/light.rs`)

* **Light** is per-cell RGB. It is seeded by sky light down open columns (with a day/night cycle), by emissive materials (lava, crystal, fungus, fire) and by point lights (torches, mole headlamps, kilns and furnaces). It spreads with a two-sweep 8-neighbour chamfer pass: L(i) = max(L(i), L(n)·t(i)), with transmittance t = 0.90 per open cell and about 0.3 into rock. It is computed only over the camera view plus a margin.
* **Fog of war:** cells stay hidden until a mole sees them (raycasts every few ticks, radius 7 m) or digs next to them.

## Parallax background

Open underground cells show procedural background layers:

* far rock strata with biome tint (parallax 0.8);
* cave silhouettes (0.6);
* hanging roots near the surface (0.35).

They are sampled per cell at a camera-dependent offset, so they scroll slower than the camera. Each cell is darkened by its light value (floor of 16%, so never pure black) and tinted by coloured light. Above ground there is a sky gradient with two parallax hill layers, all following the day/night cycle.

## Crafting, items and quests (`sim/src/craft.rs`, `sim/src/world.rs`)

* **Digging drops items** (one per 6 cells, or per 3 for gems). Moles haul the drops to the stockpile, and only stockpiled items count.
* **Stations:**

  | station | cost |
  |---|---|
  | workbench | 6 wood |
  | kiln | 8 stone + 4 clay |
  | furnace | 10 brick |
  | anvil | 5 iron ingots |

* **Recipes:**

  | station | makes |
  |---|---|
  | workbench | torches, wood pillars, stone tools, TNT |
  | kiln | brick, glass from sand, charcoal, fungus lamps |
  | furnace | copper, tin and iron ingots; bronze (3 Cu + 1 Sn) |
  | anvil | bronze and iron tools, iron beams, copper wire (1 Cu ingot gives 8), logic gates (Cu + quartz), LoRa nodes (2 crystal + 2 Cu + glass) |

* **Tool tiers** (claws, then stone, bronze, iron) speed up digging and gate the harder rock.
* **Quests:** 17 sequential quests (reach a depth, stockpile, build, craft, open chests) with item rewards.
* **Discoveries:** the first sighting of each ore, fluid or rock gives a small reward.
