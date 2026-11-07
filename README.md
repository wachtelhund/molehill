# Molehill

A colony sim about moles digging through the Deepholt, an endless falling-sand underworld where every tunnel has to carry real loads. There is no sky and no surface (see [LORE.md](LORE.md)). This is a private project with no hosted build; play it locally.

* **Wiki:** [docs/wiki](docs/wiki/README.md) covers the lore, world generation, physics, controls, items, devices, testing and architecture.
* **Design:** [PROGRESSION.md](PROGRESSION.md) is the planned survival progression (not yet implemented).
* **v3 engine:** Rust + Bevy 0.20 in [`engine/`](engine/). The older TypeScript v2 prototype lives in [`legacy/`](legacy/).

## Play locally

**Native** (Linux/macOS/Windows, needs a Rust toolchain; on Linux also the ALSA headers for sound, `libasound2-dev` or `alsa-lib-devel`):

```bash
cd engine
cargo run --release            # add `-- --fresh` to skip loading the autosave
```

**Web** (the prebuilt wasm build is committed in `site/`):

```bash
python3 -m http.server -d site 8080
# then open http://localhost:8080/   (add ?fresh to skip loading the autosave)
```

To rebuild the web version, run `cd engine && ./build-web.sh` (needs wasm-bindgen-cli 0.2.130 and the wasm32-unknown-unknown target), then `rm -rf site && cp -r dist site`.

The game opens on a main menu: Continue (if there is a save), New Holt (with an optional seed), Settings (volume, music volume or off, default zoom, and fullscreen on native), Controls, and Quit (native). It opens at once, without building a holt first. A new holt is built behind one loading line that says what it is doing and how far along it is ("Shoring up the caves…  31 %"), and Continue and Load show "Opening the save…" while the save is read. Both then build the ground ahead of the window ("Shoring up the deeper rock…") before play starts. `?seed=N` / `--seed=N` opens straight into that holt.

Tests: `cd engine && cargo test --release -p molehill-sim`. Headless scenarios: `cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/*.scn` (see [TESTING.md](TESTING.md)). For agents, see [AGENTS.md](AGENTS.md). Collapse check: `cargo run --release -p molehill-sim --example settle`.

## Layout

| path | what |
|---|---|
| `engine/sim` | `molehill-sim`: pure-Rust simulation (no Bevy): sand and fluids, stress, procgen, light and fog, colony AI and needs, crafting and quests, copperline logic and thrumming, stale air, mushroom farms, endless world streaming, disasters, save/load |
| `engine/game` | `molehill`: the Bevy front-end. It composites the world into 128 chunk textures of 64×64 cells and uploads only the chunks that changed. It also draws the sprites, the HUD, the wire and device overlay, and the procedural sound (WebAudio on the web, a software synth through cpal natively). |
| `engine/web`, `engine/build-web.sh` | the wasm build, written to `dist/` |
| `site/` | committed copy of the web build, for local play |
| `docs/wiki/` | the wiki (markdown pages; start at [`docs/wiki/README.md`](docs/wiki/README.md)) |
| `legacy/` | the TS v2 prototype, plus the screenshot scripts (`legacy/scripts/shot-v3.mjs`) |

## Controls

You play one mole. The controls work like Terraria or Core Keeper.

| input | action |
|---|---|
| `A` / `D` | Run: instant start and stop. Steps of 1-2 cells are climbed automatically, and you shove through loose sand |
| `Space` (or `W` off a wall) | Jump. Hold for a full jump, tap for a hop. Coyote time and a jump buffer are about 0.1 s each |
| Into a wall in the air | No wall climbing: you slide down bare rock. A jump pressing toward a ledge rolls onto it if its top is within 0.75 m of your feet (ledges up to about 2 m). Higher, hang a rope ladder or use the **rope gun** (left mouse throws its hook into rock; `W` grabs the rope, `W` / `S` climb, `A` / `D` swing, `Space` lets go) |
| Water / lava | `W` / `Space` swim up (water); lava burns |
| `S` | Dig down (on the ground) |
| Left mouse | Use the item in hand. A dig tool carves a round bite under the cursor and a block (pillar, brick, beam) places a round brush; the ring shows its size. Other items place a torch, copperline, device, spore, TNT or station. TNT goes down inert |
| `[` / `]`, `Option`/`Alt`+wheel | Brush size for the dig tool or block in hand, within that item's range (bigger tools allow bigger bites). `[` and `]` work on any layout (Option+8 / Option+9 on a Swedish Mac). The size is written beside the outline, in metres |
| `X` | Grid snap for blocks: square blocks of 0.25, 0.5 or 1 m on a fixed grid (the size keys step it), and a drag lays a straight row or column |
| Right mouse | Interact: flip a lever, set a thrum channel, light a TNT charge (1.5 s fuse) or a fuse cord. On an empty spot it sets down the device or TNT in hand (unlit) |
| `Q` held, then released | Throw the item in hand (only throwable things: loose materials, food, spores, TNT). A thrown TNT charge goes out lit |
| `E` | Pick up the device, unlit TNT, torch or copperline segment under the cursor; hold and sweep to take up a run of copperline. No digging needed |
| `1`-`0` / wheel | Select a hotbar slot (also while sprinting) |
| `I` | Inventory grid. Click a slot to pick its item up, then click another slot to drop or swap it (the hotbar is the bottom row). Standing at a station, its recipes are listed on the left: click one to make it |
| `+` / `-`, `Ctrl`/`Cmd`+wheel | Zoom (also in Settings; clamped and saved) |
| `G` | Headlamp lit / out |
| `J` | Journal: the Lid myth fragments found on clues (carvings, old notes, drafts, odd finds), more common the higher you go, and the carvings in the Ancients' ruins |
| `V` | Wiring view: a bold schematic of copperline (amber when carrying current), devices by role, knot inputs/outputs and names, thrum links and reach; the world dims. `V` again for the normal view |
| `H` | Controls overlay |
| `Esc` | Pause menu (Resume, Restart same seed, New Holt, Save, Load, Settings, Controls, Main Menu), or close a panel |
| `K` / `L` | Save / load |
| `F3` | Developer info |

There are three dig tools. Each tier gives a bigger bite, faster digging and harder rock:

* **Pickaxe:** dirt and sand go almost instantly; soft stone takes about 0.3 s.
* **Drill:** a copperline/heatstone-driven auger. It cuts granite, basalt and ore.
* **Bore:** a heavy heat-bore with the widest bite.

New Holt asks for a mode. **Survival** starts with a small kit: you dig with the tools you own, find food, and burn fungus oil for light, and when you die your pack stays behind on the last dry ground near where you fell. **Creative** has every item and nothing hurts. Crafting is still hidden.

## Systems

* **Falling sand and fluids:** sand, gravel, water and lava. Lava meeting water becomes loose basalt scoria.
* **Structural stress:** overburden, bending, compression and connectivity. Cracked cells crumble after 40 ticks. With no player action, the start burrow stands and a fresh world collapses only a little in the first 2 minutes (this is tested). Some natural caves may still settle in the fog away from the colony.
* **Endless Deepholt:** you start in the middle layer of earth (depth 0 m). The 2048×1024-cell grid (256 × 128 m) is a sliding window over a world endless every way: when the colony nears the top or bottom edge the window slides by 32 m, and when it nears a side (80 m) it slides 32 m sideways. What leaves is stored as 32 m × 8 m tiles with everything in them (your digging, devices, torches and stations are kept), and new ground is generated on demand from a pure per-cell function of (seed, absolute column, absolute row), built and shored ahead of the window on a worker thread, so the world is deterministic per seed. There are no side walls.
* **Seeded procgen:** biomes, strata by depth, caves, and pockets of water and lava. Water is common in shallow layers and lava deeper down (details below).
* **Lighting:** unlit tunnels stay dark. Torches, powered lamps, forges, a torch in the paws and the mole's fungus-oil headlamp (`G`) cast light with soft falloff and rock shadows; lava, crystal and fungus glow. There is also fog of war and a deep-earth parallax background. There is no sky light and no day cycle.
* **Moles:**
  * They dig, haul, build and craft.
  * Their needs are hunger, sleep and breath. They eat grubs (+45), mushrooms (+35) and fungus (+25) at the stockpile, and sleep when tired.
  * In stale air (CO2 above 0.45) their breath drains and then they lose health. Above 0.55 they flee to fresher air.
  * Low needs halve their work speed, and starving hurts them. Digging soil sometimes turns up grubs.
* **Stale air (`sim/src/air.rs`):** CO2 lives on an 8×8-cell block grid (0 is fresh, 1 is lethal). Breathing moles, torches, fire and lava add CO2. It diffuses conservatively between open neighbouring blocks, seeps slowly into porous soil faces, and is pulled out strongly by **vents** (×0.5 per update in the vent's block, ×0.8 around it). A sealed tunnel system therefore goes stale (tested). Stale air shows as an olive haze, and the HUD shows each mole's breath and air.
* **Mushroom farms:** plant spores (dug out of fungus, or found on harvest) with `P`. A plot grows fastest when it is **dark** (no torch, lit lamp or forge nearby) **and damp** (water, wet soil or clay within 4 cells). It grows at ¼ speed with only one of those and not at all with neither. A ripe plot drops 2 mushrooms (with a 1-in-3 chance of a spore) and regrows.
* **Crafting, quests and chests:** stations, a recipe queue and a quest chain.
* **Wiring and logic:**
  * Copperline on a 1 m block layer forms nets. Knots (both = AND, either = OR, turn = NOT, odd = XOR, catch = latch) switch one tick after their inputs change.
  * Devices include levers, plates, sensors, doors, floodgates, pumps, lamps, TNT and tickers. TNT is inert until lit (right mouse, fire or lava: a 1.5 s fuse that water puts out) or set off at once by current on its copperline (lever, knot or a listener: remote detonation by thrumming), by a blast nearby, or by a fuse cord burning into it. Doors never shut on a mole in the doorway. Power comes from **heatstones** (live while lava touches neighbouring blocks) and **water wheels** (turn while water flows through their block).
  * **Thrumming** sends a knock through the rock from a thrummer to listeners on the same channel; relay thrummers pass it on. It is modelled as an elastic wave, with every number derived from the material table:
    * stiffness E ≈ 300 × compressive strength (rock-mass modulus ratio); water uses the bulk modulus 2.2 GPa and air 142 kPa;
    * wave speed c = √(E/ρ) and impedance Z = ρc;
    * quality factor Q = 5 + 4 × UCS in MPa for intact solids (granite ≈ 85, dirt ≈ 6), 4 for loose or powdery ground, 20 for water and 2 for air;
    * absorption 8.686·π·f/(Q·c) dB/m at f = 200 Hz, applied per cell along the ray;
    * at every material boundary only T = 4Z₁Z₂/(Z₁+Z₂)² of the energy passes, so a rock/air boundary costs about 30 dB and an open tunnel reflects the knock away;
    * cylindrical spreading of 10·log₁₀(r) dB.
    * A link holds while 60 dB − losses > 0. Dense stone carries the knock tens of metres, loose dirt muffles it, and tunnels block it (tested).
* **Disasters:** floods, lava surges and quakes. Each starts with a 10 s warning banner and lasts 6 s, with camera shake. The first comes at about 2.5 min, then one every 90–180 s.
* **Sound:** procedural effects plus an ambient rumble that changes with depth, on every build. The recipes are shared (`game/src/audio.rs`): the web plays them through WebAudio, and native builds render the same graph in software (`game/src/synth.rs`) out through cpal. Sound starts after the first input and follows Settings > Volume and Music.
* **Music:** generative ambient (`game/src/music.rs`), no audio files: soft pads, kalimba-like plucks, rare bells and low drones through a cave reverb and echo. Pieces of 1–2 minutes alternate with long quiet rests. The mood follows the mole: airy and major up in the wet layers, earthy at home, darker deeper, low and smouldering near lava. It has its own quiet volume in Settings (default 35%; Enter mutes).
* **Save/load:** a small manifest plus one run-length-encoded blob per 32 m × 8 m tile of the world, named by position and content hash. Autosaves (every 30 s) rewrite only the tiles that changed; saves from before tiles still load. It goes to localStorage on web and to files on native, and autoloads at start.

## World generation (`sim/src/gen.rs`)

Each game uses a new seed. The window is 1024×512 cells, one cell is 0.25 m, and every cell is a pure function of (seed, x, absolute row). Depth 0 is the starting layer; positive is deeper and negative is shallower, and both are unbounded.

* **Biomes** vary along x (Loamlands, Karst caverns, Granite crags, Ashlands) and shift layer depths and cave density.
* **Strata by depth:** dirt and clay above about −30 m, mixed limestone near the start, then rock, granite and basalt deeper down.
* **Caves:** fbm caverns plus ridged worm tunnels.
* **Fluid pockets** use a separate noise field, and each pocket holds one fluid. The lava share is clamp((d − 10)/60, 0, 0.92) and the water share is clamp((40 − d)/60, 0.08, 1), where d is depth in metres. Pockets near the start burrow are dried to clay.
* **Ore veins** follow thin ridged-noise bands (coal, copper, tin, iron, quartz). Crystal and quartz line deeper caves, and glowing fungus grows on cave floors.
* **Chests:** 6 per 32 m band and 256 m span, placed deterministically. Opened chests are remembered across window slides.
* **Start burrow:** a domed 31-cell chamber with a granite roof and a clay floor. The roof steps down toward the walls, so no roof run is long enough to fail.
* **Stability pass:** the world is settled for 240 CA ticks, then the structural solver runs and failing cells are reinforced or removed. Newly generated rows are stabilised the same way when the window slides. A fresh world therefore stands, and collapses are caused by you, or by the deep lava/water front.

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

1. **Overburden:** P(x,y) = Σ ρ·g·s / 1000 [kPa] over the non-empty cells in the 16 m (64 cells) above. This is the pressure-arch height: load from deeper overhead is carried around openings by the ground arch, as in Terzaghi arching. The window's top and bottom rows count as anchors, because the rock continues beyond them.
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

* **Light** is per-cell RGB. It is seeded by emissive materials (lava, crystal, fungus, fire) and by light sources (torches, lamps, forges, the mole's headlamp beam and a held torch). Glow spreads with a two-sweep 8-neighbour chamfer pass: L(i) = max(L(i), L(n)·t(i)), with transmittance t = 0.90 per open cell and about 0.3 into rock. Sources cast rays that stop at rock, so they throw shadows (see the wiki's Physics page). It is computed only over the camera view plus a margin.
* **Fog of war:** cells stay hidden until a mole sees them (raycasts every few ticks, radius 7 m) or digs next to them.

## Parallax background

Open underground cells show procedural background layers:

* far rock strata with biome tint (parallax 0.8);
* cave silhouettes (0.6);
* mineral veins, rusty near the start and pale quartz or ember veins deeper down (0.35).

They are sampled per cell at a camera-dependent offset, so they scroll slower than the camera. Each cell is darkened by its light value (floor of 16%, so never pure black) and tinted by coloured light. The bands follow absolute depth, so they stay continuous as the window slides.

## Crafting, items and quests (`sim/src/craft.rs`, `sim/src/world.rs`)

* **Digging drops items** (one per 6 cells, or per 3 for gems). Moles haul the drops to the stockpile, and only stockpiled items count.
* **Modes:** New Holt picks **Survival** (a near-empty pack, tools and recipes earned) or **Creative** (every tool, 999 of everything, free crafting).
* **Stations** (craft standing at one, pack open): **workbench** (6 wood; always works), **forge** (10 stone + 4 clay; needs lava or fire within 3 m, or a lava-fed heatstone), **workshop** (4 wood + 4 stone + 6 copperline; needs a live copperline to it).
* **Recipes unlock by touch:** a recipe appears the first time any of its ingredients reaches the pack. Chains are at most two steps (ore → ingot → tool). Full list: [Tools and Items](docs/wiki/Tools-and-Items.md#recipes).
* **Tool tiers:** claws, then the **Pickaxe** (workbench), **Drill** (workshop: granite, iron ore) and **Bore** (forge: basalt). In Survival you dig with the best tool you own.
* **Quests:** 21 sequential quests (reach a depth, find, stockpile, build, furnish, craft, open chests) with item rewards.
* **Discoveries:** the first sighting of each ore, fluid or rock gives a small reward.
