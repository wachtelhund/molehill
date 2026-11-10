# Testing Molehill

Everything below runs headless. `molehill-sim` has no Bevy dependency, so these need no window, GPU or audio. All commands run from `engine/`.

## 1. Unit and integration tests

```bash
cargo test --release -p molehill-sim          # about 1 min; build the release profile, debug is very slow
cargo test --release -p molehill-sim --test sim -- jump coyote   # filter by name
```

The game crate has a few headless unit tests of its own for HUD and art logic that needs no window: `cargo test --release -p molehill` (the Survival hotbar layout and faint slots, the refusal ring's colour and jolt, the vitals notes and hover tips, quest progress, rail and ore pixel colours, the music composer, the sound recipes and the native synth rendered offline (`audio::tests`, `synth::tests`: the first of each event sounds the moment the sound opens, the reverb, every event heard and ended, the two volumes apart, no clipping, and, `--ignored` and run alone, `renders_faster_than_real_time` (about 25x); `opens_the_sound_card` is `--ignored` too and needs speakers), and the art checks: every item icon, device sprite and furnishing is its own drawing, and every found place gets its furnishings standing in open space; see [Item and Place Art](docs/wiki/Item-and-Place-Art.md#checks)).

`spawn_reveals_only_burrow_neighbourhood` checks a fresh holt starts with only its burrow and a 1 m skin seen (nothing 10 m out, through rock or not, and no unseen material counted as discovered).

`sim/tests/sim.rs` covers stress and collapse, fluids, digging tiers, and movement. The movement tests check jump height, the variable short hop, coyote time, and that a bare wall can't be climbed (no cling, no climb). Rope-gun edge cases live there too: a window slide (up, down or sideways) keeps the mole on its rope, and riding from a rope lets go of it. `ropes_ladders_and_lifts_come_back_with_the_window` rigs a room with a rope, a rope ladder and a lift, slides the window off it (down, then sideways), saves and loads while they are away, and finds each back in its place (the ladder climbs); `a_window_slide_keeps_a_lift_on_its_way_to_its_stop` slides the window mid-ride. `a_found_places_ladders_come_back_once` stamps a mine, slides the window off it and back (down, then sideways) and finds each of its ladders once, where it hung (they came back doubled: the one that waited and the one the place hung anew). Helpers: `arena()` makes a cleared granite box around spawn, and `press(&mut w, n, |input| ...)` steps n ticks with a given input.

## 2. Scenario harness (`sim/examples/scenario.rs`)

```bash
cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/*.scn
cargo run --release -p molehill-sim --example scenario -- sim/tests/scenarios/run_jump.scn --json   # plus a final JSON state
```

Output is line-oriented and easy to grep:

* `PASS <file> (n passed, m failed, T ticks)` / `FAIL <file> ...`
* `  FAIL <file>:<line> tick T: <var> = <value>, expected <op> <n>`: one line per failed assertion.
* `DUMP <file> {json}`: a state snapshot from `dump T`.
* `ASCII <file> tick T` followed by the map region (`M` marks the mole body; legend below).
* The last line is `N scenarios, K failed`. The process exits with 1 if anything failed.

It is deterministic, so the same file always gives the same output. The sim runs at a fixed 60 ticks per second.

### File format (`.scn`)

The format is line-based. `//` starts a comment, and ticks start at 1.

```
// what this checks
seed 11                 // optional world seed (default 11); the world is generated and the map stamped over it
map                     // ASCII map, ends at `end`. '@' = the mole's lowest body cell (feet sit on the cell below)
##########
#........#
#.@......#
##########
end
give Tnt 5              // set inventory count (item names as in craft::Item, case-insensitive)
hold 1-30 right         // hold keys for ticks A-B: left right jump up down (several allowed)
hold 31-40 right jump
tool 41 3               // from tick 41 dig with tool tier 0 claws, 1 pickaxe, 2 drill, 3 bore
mine 41-60 3 0          // hold mine on cell (dx,dy) relative to '@' during ticks A-B
place 12 6 0 WoodPillar // at tick T place a round brush of that block centred at (dx,dy) (WoodPillar, Brick, IronBeam)
brush 10 2.5            // from tick T the brush radius in cells for digging and placing (0 = item default; clamped per item)
lamp 1 on               // headlamp on/off from tick T (off by default in scenarios; the game turns it on)
aim 1 20 -2             // from tick T the mole looks at tile (dx,dy) from @ (the headlamp beam points there); `aim T none`
wield 1 Torch           // from tick T the mole holds that item (a torch lights); `wield T none`
torch 1 6 0             // at tick T put a torch on tile (dx,dy)
clue 1 Note 8 0         // put a Lid clue (Carving, Note, Draft, Relic) on the floor of tile (dx,dy)
furnish 2 Table 6 -2    // set a piece of furniture down at tile (dx,dy): it finds the floor below (or the ceiling above, for a banner)
paint 2 Ochre 3 0       // paint a brush (`brush`) of standing blocks around tile (dx,dy) with a pigment, or dye the rug/banner/nest there
dig 41-60 3 -1          // same, but (dx,dy) is relative to the mole's current cell (feet row), so it follows the mole
explode 70 5 -2 0.25    // a blast of kg TNT-equivalent at (dx,dy) at tick T (a TNT charge is 0.5)
place 5 4 0 WoodPillar  // at tick T place a block item (WoodPillar, Brick, IronBeam) at (dx,dy), as a left click; a refusal prints NOTE
device 5 12 -1 Tnt      // place a device (logic::Dev name: Lever, Tnt, And, Tx, Rx, Lamp, Fuse, ...) in the 1 m block at (dx,dy)
use 10 12 -1            // right mouse on the device there: flip a lever, light TNT or a fuse, step a channel
pickup 40 12 -1         // E: take the device there (else a torch, else the copperline) back into the pack
wire 4 1 -1 13 -1       // lay copperline from (dx,dy) to (dx2,dy2), one per 1 m block (a single block if only one point)
saveload 60             // save the world to bytes and load it back (checks persistence mid-scenario)
hand 1 Pickaxe          // the brush item in hand (Pickaxe, Drill, Bore or a block); sizekey 2 ] / sizewheel 3 alt 1 resize it; brush = its radius in tiles
switch 5 creative       // change mode mid-run (`mode survival` at the top sets it from the start)
craft 5 Pickaxe         // craft as at a station ("_" for spaces); station 2 DX DY Workbench builds one on that tile; pour 3 DX DY Lava sets a tile's material; loosen 1 DX DY makes a tile's cells loose rubble, as a blast leaves it; set T weight N lets the first counterweight N tiles further down its rope
snap 4 on               // grid snap; line 5 DX1 DY1 DX2 DY2 Brick drags a snapped line; snapcells = the square side in tiles
slide 60                // slide the window away until only its bottom band is left, and back (things go into bands and return)
put 2 8 0 WoodPillar    // put exactly one block cell at (dx,dy) at tick T, no reach check (scripted building)
ruin 1 vault 2 -17      // stamp a ruin of the Ancients (hall | vault | tube, `left` for a tube running left) with its top-left on tile (dx,dy); a vault's chest is registered
wall 2 4 -2 timber      // set the back wall of tile (dx,dy) (none|rock|timber|masonry|beams|pipes|carved|damp|crack|root|gap|salt|crystal|fossil|mycel)
scrape 3 4 -2           // scrape lining with the current brush (never the rock in front)
site 2 mine 4 -24       // stamp a found place (mine|holt|pump|fissure|fossil|crystal|halt) with its top-left on tile (dx,dy)
mode survival           // start in a game mode (survival | creative): its rules and its pack; `give` applies after
nohunger                // the world's hunger option off (after `mode`): food never drains
set 40 hp -1            // set a vital at tick T: hp, hunger, breath, or oil (headlamp oil, seconds)
set 295 inv:Drill 1     // set the pack's count of an item at tick T (`give` is at the start); `set T quest N` jumps the quest chain (read back as `quest`)
undiscovered IronOre    // this material counts as not yet seen (all others are), so its first sight logs a discovery
placetorch 150 3 -2     // place a torch from the pack at (dx,dy) as a click (in Survival it needs oil); a refusal prints NOTE
grotto 2 6 -5 8 4       // at tick T a grotto's glowmite swarm centred on tile (dx,dy), radii rx ry in tiles
catch 40 6 -5           // at tick T right-mouse the tile (dx,dy): catch the glowmite there, if any
throw 2 Stone 1 -1 1    // throw one from the pack toward direction (dx,dy), power 0..1 of a full throw (TNT goes out lit)
drop 1 Wood 26 -12 0 0  // a loose item centred on tile (dx,dy), optionally moving at (vx,vy) tiles/s
place 2 6 0 Cart        // a transport machine from the pack goes down whole at (dx,dy): a cart finds the track (else the floor), a RopeLadder hangs from the rock above, a Lift's head under the shaft's ceiling
load 21 27 Stone        // (scripted) put that many items into the first cart's cargo
dump 50                 // print the JSON state after tick T
ascii 80                // print the map region after tick T
look 80                 // print 80 x 56 tiles round the mole (rope points '*', ladders '|', chests 'C'), whatever the map: finding a way through a site
fire 10 0 -30           // rope gun: throw the hook at tile (DX, DY) from '@' (needs RopeGun and a HookRope in the pack)
fireby 10 -1 -40        // the same, aimed (DX, DY) tiles from the mole's hand
expect 60 rise >= 4.5   // assertion after tick T: <var> <op> <number>, ops: < <= > >= == !=
```

**Scenarios are written in tiles of 0.25 m**, the original cell size. Each map character is stamped as a K × K block of cells (K = `mat::S`, 2 at 8 cells/m). Every offset and radius in a directive is in tiles, and positions, speeds, `count:`/`static:`/`crumbled` and the ASCII dump are reported back in tiles. A scenario therefore means the same thing in metres at any resolution. `is:` is true when most of the tile is that material; `util:`/`strain:` take the tile's maximum. `put` fills the whole tile.

The map is stamped with its top-left so that `@` lands on the world spawn (the mole stands centred on the `@` tile, feet on its bottom edge). A 20-tile granite frame is put around it, so the generated world can't leak in. Disasters are off. Rows shorter than the widest row are padded with `#`.

Every material counts as already discovered in a scenario (a discovery's torch reward would otherwise depend on what the generated world happened to show).

Map legend: `t` roots (a root tangle), `f` fungus, `.` empty, `#` granite, `B` bedrock, `r` rock, `d` dirt, `c` clay, `s` sand, `g` gravel, `w` water, `l` lava, `o` coal, `i` iron ore, `a` basalt, `b` wood beam, `I` iron beam, `k` stone brick, `T` root-tree, `R` root-tree roots, `m` moss, `y` mycelium, `p` planks, `q` polished stone, `x` tiles, `C` carved pillar, `=` rail, `%` pressure tube, `h` chute, `H` shaker screen, `|` bumper, `z` riffle bar, `j` iron shaft, `*` gear, `O` water wheel, `>` belt (runs right), `<` belt (runs left), `U` weight drum, `E` steam engine, `P` steam pipe, `Y` bellows, `V` ingot mould, `F` pull-rod, `e` tread plate, `A` float, `J` clutch (output only: `Z` sluice gate, `N` scale pan, `K` boiler, `M` stamp mill, `W` washer, `S` smelter shells, `X` a setting mould) (`~` steam, `?` other, only in output). The generated world's own grottos are left out of a scenario (`grottos` cleared).

**Assertion variables:**

| variable | meaning |
|---|---|
| `flash` `shake` | Hit-flash time left (s); view-shake strength (decays each tick) |
| `step` | Distance the body moved this tick (tiles); catches teleport pops that a velocity check misses |
| `hunger` `oil` `lamp` `mode` `hungeron` | Food (0-100); headlamp oil left (s); 1 if the headlamp shines; the mode (0 plain, 1 survival, 2 creative); 0 if hunger is turned off |
| `relics` `ruins_seen` | Relics found (bit mask: 1 Ancient bit, 2 Ground lens); ruins visited (their carving read) |
| `items:Name` | How many of that item lie loose |
| `ix:Name` `iy:Name` `ivx:Name` `ivy:Name` | The first loose item of that kind: centre in tiles from the `@` tile's centre (iy 0 = resting on the `@` floor), velocity in tiles/s |
| `irest:Name` `ilit:Name` | 1 if that item is asleep (lying still); 1 if it's a lit charge |
| `px` `py` `vx` `vy` | Player body in tiles from the map's top-left (py is the feet). Velocities are in tiles/s; +y is down |
| `dx` `dy` | Offset from the start position (dy < 0 means higher) |
| `rise` | Highest point reached so far, measured above the start |
| `grounded` `cling` | 0 or 1; cling is always 0 now (no wall climbing) |
| `hp` `breath` `deaths` | Mole vitals |
| `crumbled` | Tiles' worth of cells that have crumbled (world counter) |
| `awake` | CA chunks overlapping the map that are still awake (0 = everything there has settled) |
| `glowmites` | Glowmites drifting over the map rectangle |
| `p:DX,DY` | The last blast's peak overpressure (kPa) on that tile |
| `carts` / `cartx` / `carty` / `cartv` | Carts in the world; the first cart's position (tiles from the map corner) and speed along the track (tiles/s) |
| `cartpow` | Whether the track under the first cart is powered (live copperline in its block) |
| `bins` / `binload` / `binopen` / `tips` | Ore hoppers; the first one's load (items) and whether its gate is open; how many times a cart has tipped at a bumper |
| `cargoof:K` / `cargo:Item` | The K-th cart's cargo; the first cart's count of that item |
| `zone:Item,DX0,DX1[,DY0,DY1]` | Loose items of that kind whose centre lies in those tile columns (and rows) from the `@` tile |
| `shaking:DX,DY` | 1 if the tile is a shaker screen with current in its block |
| `flow:DX,DY` / `bedstress:DX,DY` | The water's current at the tile's centre (m/s, + = right) and its stress on the bed (depth·slope, m); 0 in still water or out of it |
| `turning:DX,DY` / `supply:DX,DY` / `demand:DX,DY` | How fast the tile's drive train turns (0..1); its power to give and to do (W) |
| `boilers` / `bsteam` / `bwater` / `bfuel` / `bburning` / `bchoked` | Boilers; the first one's steam (kJ), water (cells), fuel (lumps, the burning one too), whether it burns, whether its ash door is choked |
| `works` / `casts` / `stampload` / `washload` / `smeltload` / `smelttemp` / `smeltair` / `molten` | Stamp mills, washers and smelters; casts setting in moulds; the first stamp mill's, washer's or smelter's load waiting; the first smelter's heat (°C), air (0..1) and molten metal (ingots) |
| `gates` / `gateopen` / `scales` / `tipped` / `pulled:DX,DY` | Sluice gates and whether the first is lifted; tipping scales and whether the first has tipped; whether the tile's linkage is pulled |
| `borers` / `borerx` / `borery` / `borerang` / `borerv` / `borersteam` / `borerfuel` / `onborer` | Steam borers; the first one's head middle (tiles from the map's left, and from the map top), heading (degrees: 0 east, 90 down, 180 west, 270 up), speed (m/s), steam (kJ), fuel (lumps); whether the mole is in its cab |
| `wagons` / `train` / `wagonx` / `wagony` / `wagonload` / `inwagon` / `atbench` / `slept` / `spawnx` | Wagons; how many the first borer hauls; the first wagon's middle (tiles from the map's left, and from the map top), load (items or lumps); whether the mole is in one; whether it stands at a workbench (a workshop wagon's counts); whether it has just slept in a bunk wagon; the wake point (tiles from the map's left) |
| `weights` / `weighty` | Counterweights; the first one's top (tiles from the map top) |
| `cells:Mat,DX0,DX1,DY0,DY1` | Tiles' worth of that material in those tile columns and rows from `@` |
| `ladders` / `climbing` | Rope ladders hung; whether the mole is on one |
| `ropes` / `hook` / `onrope` / `ropex` / `ropey` | Rope-gun ropes fixed in the rock; a hook in flight (1); the mole on a rope (1); the first rope's hook (tiles from the map corner) |
| `lifts` / `lifty` / `liftv` / `ballast` | Lifts; the first one's cage floor (tiles from the map top), speed (tiles/s, + = down) and stones of ballast |
| `stops` / `liftstop` / `liftw` / `liftx` / `liftpow` / `liftover` | The first lift's stops and the one it stands at (0 = top, -1 between), cage width and middle (tiles), current at its head, kg the cage is over its counterweight |
| `ladderx` / `laddertop` / `ladderlen` | The first rope ladder's column, top row (tiles from the map corner) and length (tiles) |
| `hoppers` / `capsules` / `capat` / `intube` / `pressure` | Tube hoppers; capsules in the pipe and how far along its pipe the first is (0..1); whether the mole rides one; whether the first hopper's line holds pressure |
| `cargo` / `cartrail` / `cartgap` / `riding` | The first cart's cargo (items), whether it is on the rails, how far it is ahead of the mole (tiles), whether the mole rides a cart |
| `debris` / `debris:Mat` | Cells a blast threw that are still in flight (tiles), all or of one material |
| `wave:DX,DY` | The path distance (m) the last blast's front ran to reach that tile (-1: it never got there) |
| `front` / `frontcells` | The last blast's drawn front: its radius now (path m, 22 m/s), and the tiles in it |
| `count:<Mat>` | Tiles' worth of that material inside the map rectangle (cells / K²), e.g. `count:Water` |
| `is:DX,DY,<Mat>` | 1 if the cell at an offset from `@` is that material |
| `inv:<Item>` | Inventory count |
| `static:<Mat>` | Like `count:` but only cells that are fixed in place (not loose debris) |
| `util:DX,DY` | Stress utilisation of the cell at an offset from `@` (1.0 = at capacity; above that it cracks) |
| `journal` `clues` | Lid fragments read; unread clues inside the map |
| `clues_bands:A,B` | How many clues the world places in 32 m bands A..B (absolute, negative = higher), from the real generator |
| `furn:Item` `tint:Item` | How many of that furniture stand in the world; the first one's dye (0 undyed, 1–6 the pigments) |
| `paint:DX,DY` | The pigment painted on the tile's centre cell (0 unpainted; 1 ochre, 2 rust, 3 chalk, 4 verdigris, 5 woad, 6 soot) |
| `light:DX,DY` | Brightness (luma) at the tile's centre; light is recomputed over the map every tick. About 1 is fully lit, under 0.05 is dark |
| `log:WORD` | Lines in the message log containing WORD (case-insensitive; `_` stands for a space), e.g. `log:too_hard` |
| `strain:DX,DY` | The hover tooltip's strain level for that cell (`tips::strain_at`): 0 idle, 1 bearing, 2 straining, 3 overloaded, 4 cracking |
| `dev:DX,DY,<Dev>` | 1 if the 1 m block at that offset holds that device |
| `lit:DX,DY` / `on:DX,DY` / `wire:DX,DY` | The block's fuse is burning / its device is powered / it has copperline |
| `ev:<Event>` | How many of that event (Explode, Click, Place, ...) happened so far |
| `sites` `sites_seen` | Found places touching the window; how many have been visited |
| `walls:Kind` | Cells of that back wall in the window (rock, timber, masonry, …) |
| `wall:DX,DY,Kind` | 1 if the tile's centre cell has that lining |

Devices sit on a 1 m (4×4-cell) block grid. The spawn cell is block-aligned, so blocks start at offsets that are multiples of 4.

The JSON dump includes the tick (the world's absolute tick, a few ticks ahead of the scenario tick), player state, non-zero inventory, material counts in the map rectangle, the last blast's wave (`blast`: `age_s`, `front_m`, `reach_m`, `wave_tiles`, `front_tiles`, up to 48 `front` tiles relative to `@`, and `debris` in flight), and events as `"tick:Event"`.

### Included scenarios

| file | checks |
|---|---|
| `air_turn.scn` | Reversing in mid-air takes under 5 ticks; letting go in the air coasts instead of braking |
| `fall_hurt.scn` | A 30-cell fall hurts, flashes the mole and jolts the view; a normal jump landing doesn't |
| `step_up.scn` | Running into a 1-cell then a 2-cell step hops up them at full speed |
| `mantle.scn` | A jump at a 1.75 m ledge, pressing toward it, rolls over its top onto it (a jump rises about 1.25 m; `TUNE.ledge` reaches 0.75 m above the feet) |
| `run_jump.scn` | Top speed in under 8 ticks; full-jump height 4.5-9 cells; lands; the 11 m granite ceiling doesn't crumble |
| `coyote.scn` / `coyote_late.scn` | A jump 4 ticks after leaving a ledge works; 12 ticks after, it doesn't |
| `no_wall_climb.scn` | No wall climbing: in a 6 m chimney 2 m wide, W and Space held into a wall, and jumps from wall to wall, never rise above a jump (6.5 tiles), never cling; back on the floor unhurt |
| `rope_gun_anchor_stone.scn` | The rope gun's hook glances off a heap of sand and sinks in a pool (no rope; the hook rope lies where it stopped), bites a granite ceiling 7.5 m up (the rope stays through a save and load), and when that rock is blown away the rope comes down as a hook rope |
| `rope_gun_catch.scn` | Up 5 m of rope, `Space` lets go, `W` held while falling past grabs it again at once (no damage); `S` down onto the floor lets go by itself |
| `rope_gun_slide.scn` | A hook still in the air when the world window slides goes back into the pack as a hook rope |
| `ropeways_come_back.scn` | A rope ladder on a cliff, a lift and a rope-gun rope: the window slides away and back (`slide`), then again across a save and load, and each is still in its place (the ladder's column, top and length, the lift's 2 stops, 1 rope) |
| `rope_gun_climb.scn` | At a bare 4.5 m cliff's foot: the hook up into the ceiling, W grabs, W climbs at 2 m/s, D at the top steps off onto the ledge; full health |
| `first_hour_without_wallclimb.scn` | Survival, under a 4 m granite shaft: no way up by hand; from first-hour stuff a workbench, the rope gun and a hook rope; the hook up the shaft, climb, step off onto the upper floor unhurt |
| `no_ceiling_climb.scn` | Jumping into a low ceiling while holding sideways and `W` never clings; the mole falls back down |
| `survival_kit.scn` | Survival's start kit; only owned tools dig (granite holds against a pickaxe); a save keeps the mode |
| `survival_food.scn` | Hunger drains; the Delver eats grubs then roots below 40; starving hurts; root tangles give roots; the log says each meal with what's left, and an empty pack once |
| `survival_lamp_oil.scn` | The headlamp burns oil, refills from the pack, then presses fungus, then goes dark; a torch needs oil; the log says the pressing, the last measure and going dark, each once |
| `survival_light_fuel_messages.scn` | With no oil spare the headlamp warns once ~30 s before it goes dark, then says it's out and what to gather; a torch refused for lack of oil says "it needs fungus oil to burn" (not "something is in the way"); fungus found relights it with a message; with oil spare it never warns |
| `survival_death_drop.scn` | Death leaves the whole pack where you fell; you wake in the burrow and walk back for it |
| `survival_no_hunger.scn` | Hunger off: food never drains or gets eaten, the rest of Survival holds, and the option survives a save |
| `creative_harmless.scn` | Creative: 999 of everything, a long fall and an empty belly don't hurt, the lamp needs no oil |
| `ruin_hall_stands.scn` | A pumping hall (brick roof on iron pillars, one fallen with its bay on an old girder) stands 20 s under an open cave |
| `ruin_hall_caves_in.scn` | The same hall under 3 m of sand: its brick bays crack and fall in as rubble; the iron holds |
| `ruin_vault_loot.scn` | Tunnel into three vaults with a Pickaxe: the Ancient bit, then the Ground lens, then only old materials; a save keeps the relics and ruins seen |
| `throw_arc.scn` | A full throw flies a ~6 m arc, bounces, skids to rest; a weaker one falls shorter; walking over them picks them up |
| `item_bounces_rests.scn` | A stone bounces off granite but not off dirt; both fall asleep on the floor without drifting |
| `item_sinks_floats.scn` | Wood floats in water, an iron ingot sinks straight to the bottom, a grub sinks slowly |
| `thrown_tnt_explodes_at_landing.scn` | A thrown lit charge flies, rolls and blows a crater where it stops, far from the thrower; one lobbed into water is doused |
| `sprint_speed.scn` | `Shift` sprint reaches 1.6x run speed in 2 ticks, stops at once, and a sprint jump carries further |
| `dig_tiers.scn` | Claws clear sandstone within 55 ticks (a bite is 49); the bore within 15 (a bite is 13) |
| `water_flood.scn` | Mining a dirt plug floods the room below |
| `tnt_collapse.scn` | A blast removes rock, and the sand cap falls |
| `cave_stability.scn` | A small granite cave shows no crumbling over 600 ticks |
| `hall_sag_arrests.scn` | A 24 m granite hall (past failing) sags out of midspan and the cave-in stops (< 60 cells) |
| `strained_roof_creaks_and_holds.scn` | A 15 m granite hall is strained (past capacity, under `FAIL_U`): it creaks at once and at most every 20 s after, and nothing crumbles; harness variable `strain` counts the strained cells |
| `pillar_holds.scn` / `pillar_on_sand.scn` | Placed wood pillars stay rigid, also when standing on a sand floor |
| `pillar_wide_hall.scn` | A lone pillar in a wide, sound hall isn't crushed |
| `roof_unpropped.scn` / `roof_propped.scn` | A wide clay roof caves in; four pillar columns hold it |
| `water_levels.scn` / `lava_flows.scn` | A heap levels to the right depth (water by tick 120, lava much later; its thin sheet crusts to basalt), then sleeps |
| `sand_sinks.scn` / `sand_floats_lava.scn` | Sand sinks through water and piles on the bottom; it floats on lava |
| `player_lava_sink.scn` | Running onto a lava pool: you sink in, wade slowly, burn and die |
| `chunk_sinks_in_water.scn` / `wood_floats.scn` | An unsupported rock/ore clump in a pool gives way fast and its debris sinks; wood debris rises and floats |
| `lava_cools.scn` | A lava splash on the floor sets into basalt; a deep pool stays molten |
| `lava_meets_water.scn` | Lava on water quenches to basalt with steam; the steam clears and the chamber sleeps |
| `beam_span.scn` / `beam_overload_sand.scn` | An iron beam bridges an 8-cell gap; a sand-loaded wood beam breaks into loose debris |
| `pillar_holds_roof.scn` | Placed pillars (full height, a short stub, a ledge off a wall) stay rigid |
| `pillar_on_rubble.scn` | Pillars, a ledge and bricks built on a sand floor stay rigid; no new sand appears |
| `beam_spans_gap.scn` | Iron beams placed across a pit stay put, and the mole runs across them |
| `beam_overload.scn` | A long wood span under deep sand cracks and falls as wooden debris |
| `dirt_cavein_chokes.scn` | A void under a deep dirt bank caves in but bulking rubble chokes it off (< 600 cells, then sleeps) |
| `buried_under_rock.scn` / `buried_in_sand.scn` | A mole buried under a rock slab stays put (no popping through rock); under loose sand it wriggles out |
| `blast_flash_lights_a_dark_drift.scn` | A charge in an unlit drift lights its far end for a moment (the blast flash), not the corridor behind a wall, and it's dark again half a second later |
| `breath_gasp_underwater.scn` | Held under water, the mole gasps and the log says to surface when breath first drops below 35, and again ("drowning") at 0, once per crossing; then it's hurt |
| `tree_regrow.scn` / `tree_no_seep.scn` | A root-tree cut to its roots regrows beside its seep pool (and nothing crumbles); with the seep drained it never does |
| `glowmite_catch.scn` | A grotto's glowmite swarm drifts over the map |
| `brush_place.scn` | A radius-2 brick brush fills the open part of the disc and uses items 4 cells to one |
| `light_torch.scn` | A torch lights a pool with a bright core and a soft edge; a wall hides the far side |
| `lore_density.scn` | Lid clues by height: dense far above the start, a few around it, almost none deep down |
| `lore_journal.scn` | Walking past clues reads them into the journal; an unreached one stays unread; the journal survives save/load |
| `furnish_room.scn` | Furniture finds its floor or ceiling; a rug lies under a table; overlaps and a blocked spot are refused; `E` takes a piece back |
| `furniture_falls.scn` | Blast the floor from under a table and it falls loose as an item |
| `decor_blocks.scn` | Carved pillars save the 12 m rock roof like wood pillars do; planks, tiles and polished stone lay with the brush |
| `paint_and_dye.scn` | A pigment paints a brush of bricks (one pigment per 16 tiles), repaints, dyes a rug; paint and dye survive save/load |
| `lava_shadow.scn` | A lava pool lights the floor beside it through the same rays as torches; a full-height wall shadows the far side |
| `headlamp.scn` | The headlamp beam follows the aim, leaves behind the mole dark, keeps a small glow; out, only the faint mole glow |
| `headlamp_wall.scn` | Rock stops the beam: the wall face is lit, the room behind it dark |
| `held_torch.scn` | A torch in the paws lights around the mole; put away, it goes out |
| `hover_strain.scn` | The hover warns of an overloaded wood span (cracking) before it falls; floor and loose debris read idle |
| `brush_clamp.scn` | Brush radius is clamped per item (iron beam max 2, pillar min 0.5, pickaxe max 3) |
| `tnt_chain.scn` | A blast sets off a second charge 3.16 m away (130 kPa on its block sets it off) |
| `blast_tunnel_vs_cave.scn` | The same charge keeps ~370 kPa 5 m down a tunnel, ~80 kPa 5 m across a cave |
| `blast_shadow.scn` | A mole behind a wall (the wave can only get round its top) is barely hurt |
| `blast_granite_vs_dirt.scn` | Between a dirt wall and a granite wall: the dirt is blown out and cracked, the granite takes it |
| `blast_pushes_water.scn` | A blast in a flooded trough throws most of the pool up as spray, which all comes down again |
| `blast_soft_rock_tunnel.scn` | One charge on a soft-rock tunnel floor: crater through the 1 m floor and 1.25 m into the roof, rock 3.5 m off whole, 114+ tiles broken (~155), 100+ tiles of rubble thrown 4.5 m and more along the tunnel, all landed |
| `blast_pocket.scn` / `blast_cave_soaks.scn` | Confinement: tamped into a pocket a round crater ~5 m across (196+ tiles, ~250), on a cave floor ~38 |
| `blast_granite_chips.scn` / `blast_granite_pocket.scn` | Granite on a tunnel floor: crater 0.75 m down and 0.5 m into the roof, 35 to 80 tiles broken, rubble lying as gravel; tamped in a pocket 104+ tiles, round crater |
| `blast_basalt_holds.scn` | Basalt loses only the face under the charge |
| `cart_rolls_downhill.scn` | A cart on a 1-in-4 slope rolls off on its own, speeds up, runs out on the level and stops at the end of the track |
| `cart_push_level.scn` / `cart_load_heavier.scn` | Walking into a cart pushes it to walking pace and it coasts to rest; 30 stones of cargo make the same push about three times slower |
| `cart_ride.scn` | Right mouse gets in; the mole rides down the slope with the cart; Space jumps out |
| `cart_cargo.scn` | Items thrown or dropped in are cargo (saved); E unloads into the pack, then picks up the empty cart |
| `cart_off_rail.scn` | With the track gone under it a cart drops off the rails and won't roll when pushed |
| `powered_rail_climbs.scn` | Copperline on the track drives a cart up a 1-in-3 climb while a lever powers it; the line shuttles between its bumpers; lever off and it coasts to the foot |
| `rail_too_steep_to_push.scn` | Plain track: the mole can't push even an empty cart up a 1-in-3 (needs ~330 N, it has 320 N) |
| `falling_loose_no_climb.scn` / `falling_loose_walk_under.scn` | A stream of sand pouring from a hopper's slot (found in a playtest): jumping into it pressing into it and `W` gives no cling and no footing (it used to climb the stream and stick in mid-air), the mole drops through into a pit unhurt and climbs out its rock wall; walking along the floor under a pour, the mole runs on through it (it used to stop dead against it for good). Both fail with the rule off |
| `rope_ladder_climb.scn` / `rope_ladder_falls.scn` | A ladder tied on a cliff's lip: climbed at 3 m/s, hangs still without keys, climbed over onto the top; dig out its rock and it falls |
| `rope_ladder_top_out.scn` | At the foot of a 4.5 m cliff's ladder `W` grabs (harness var `ladder_up`: the game's W climbs, not jumps) and, held alone, climbs off the top onto the ledge and stays there (no bobbing on the knot) |
| `rope_ladder_down.scn` | On a 7.5 m cliff beside the ladder's head, `S` steps onto the rope (`ladder_down`: the game's S doesn't dig) and climbs down at 3.5 m/s; no fall damage at the foot (30 hp before the fix) |
| `rope_ladder_shaft.scn` | A rope hung from a shaft's ceiling: grabbed at the foot, climbed to the top where the mole hangs still under the ceiling, then `W`+`D` carries it off sideways at 2 m/s into the side passage |
| `rope_ladder_let_go.scn` | Hanging on a ladder, `D` alone lets go at once and runs off at full speed; `W` catches the rope again mid-fall; a jump lets go as a real jump |
| `lift_ride_stops.scn` | Cursor anywhere in a 2 m shaft with three passages: the cage snaps to the shaft (1.875 m, centred) with 4 stops. From the foot a tap on `W` stops level at the next passage, held runs through the next stop without slowing to the top, a tap on `S` goes one down, the mole walks off unhurt; width and stops saved |
| `lift_from_under.scn` | Placed with the cursor level with a side passage (it still finds the shaft); the mole drops into the shaft under the cage (no exit at the foot), calls it, the cage stops over its head, `W` climbs up through the floor hatch, and it rides up a stop |
| `lift_call.scn` | In a side passage a fresh `W` calls the cage up to that stop (no jump); on it `S` rides it down |
| `lift_balance.scn` | Four stones on the cage with the mole (70 kg over) are too heavy to haul (said; the brake holds); right mouse balances it with 7 stones from the pack and it rides |
| `lift_powered.scn` | The same overload rides on current at the head (1.5 m/s, the copperline in a block of the head frame beside its middle); lever off, too heavy again |
| `ladder_place_snap.scn` | A rope ladder aimed into rock is refused; aimed 1 m out from a cliff's face it ties on at the lip, hangs against the face and unrolls 4.5 m to the floor |
| `ladder_over_rope_slack.scn` | A rope ladder on a 5.5 m cliff and a rope-gun rope fired up beside it, its slack lying at the ladder's foot: `W` there takes the ladder (never the rope) and climbs straight up it |
| `tube_sends_items.scn` | A stone dropped in a hopper sits there until its blower gets current; then it is blown along the floor and up a 3 m riser in a capsule and comes out of the far hopper; hoppers are saved |
| `tube_breach.scn` | Cut the riser while a capsule is in it: it stops at the cut and spills there |
| `tube_rides_mole.scn` | Right mouse with an empty paw on a blowing hopper: the mole rides the capsule and climbs out up on the ledge, unhurt |
| `chute_slides.scn` | A stone slides down a 1-in-2 chute and out past its foot; on bare granite steps or a level chute it stays put |
| `cart_tips_at_bumper.scn` | A loaded cart rolls into a bumper and tips: its 20 stones come out over the front into the pit beyond; it stays on the rails |
| `ore_hopper_feeds_cart.scn` | An ore hopper takes in dropped stones and poured sand; its gate stays shut until it's held open or a cart stands under it, then it fills the cart; its load is saved |
| `cart_tip_resumes.scn` | A cart saved while tipping at a bumper finishes tipping after loading (a loaded cart standing against a bumper tips) |
| `production_line.scn` | The whole line: an ore hopper fills a cart, a lever drives it off, it tips at a bumper, the load slides down a chute onto a steep shaker screen, gravel and ore end in separate pits; the mole beside the hopper picks up nothing |
| `water_through_screen.scn` | A pool on a shaker screen drains through its mesh into the basin under it; an ore lump stays on top, sand falls through |
| `blast_ore_leaves_ore.scn` | A blast in an iron-ore seam leaves loose iron ore (over half the seam), not gravel |
| `sluice_sorts.scn` | A reservoir runs down a stepped 1-in-4 channel with riffle bars: gravel, coal and sand reach the tailings pond, ore and stone settle in the riffle traps |
| `sluice_washes_rubble.scn` | After a blast: running water washes every grain of loose sand and gravel off two rubble heaps into the pond; the iron ore stays |
| `belt_counterweight.scn` | A falling counterweight drives a belt through shafts and gears; stones ride at 1 m/s and drop off its end; with the weight taken down the belt is still; saved |
| `belt_backs_up.scn` | Fourteen stones on a 3 m belt running into its drive gear: twelve queue one to a tile, two are pushed off the back |
| `water_wheel.scn` | A stream under a water wheel gives several hundred watts, shakes a screen and winds a counterweight up; when it runs dry the weight carries on |
| `belt_carries_mole.scn` | A mole standing on a turning belt rides it at the belt's 1 m/s and stays on its feet |
| `counterweight_stops_on_build.scn` | A pillar built under a counterweight stops it on the pillar's top, not at the floor it had when hung |
| `boiler_drives_belt.scn` | Coal on a boiler by a sump: water drawn, steam up, an engine on the boiler drives a belt; ash and slag come out the ash door; saved |
| `boiler_ash_chokes.scn` | Ash piled at the ash door chokes the grate (logged); the mole picks it up and the fire catches |
| `boiler_ash_door_side.scn` | Rock against a boiler's right side: its ash comes out on the left, not into the rock |
| `boiler_walled_in.scn` | Rock against both sides: the grate counts as choked (logged) and the coal waits unlit |
| `boiler_dry.scn` | No water: the coal burns, no steam, the belt stays still |
| `steam_pipe_feeds_engine.scn` | Steam goes along a pipe to an engine 3.5 m off; cut the pipe and the belt stops |
| `stamp_mill_crushes.scn` | Two counterweights drive a stamp mill: copper and iron ore come out as concentrate and sand, stone as gravel; the weights share the work |
| `washer_sorts.scn` | A driven washer by a pool sorts concentrate and ore out left, sand, gravel and ash out right into the pool, using its water |
| `smelter_bellows_mould.scn` | A smelter tops out short of copper's heat with still bellows; driven, they take it past 1100 °C, concentrate melts and a mould casts a copper ingot |
| `stamp_mill_walled_in.scn` | A stamp mill with rock at both feet holds its ore unbroken and logs that its spout is walled in |
| `smelter_iron_slag.scn` | Raw iron ore waits until the smelter passes 1250 °C, then casts an iron ingot and lets out a lump of slag |
| `tread_plate_gate.scn` | The mole on a tread plate pulls a rod that lifts a sluice gate under a reservoir; stepping off drops it |
| `float_valve.scn` | A turned-round gate fills a tank until the water reaches a float on its rod, then shuts; no water lost; saved |
| `scale_clutch.scn` | A tipping scale set to 10 kg tips on the third stone, pulls its rod and engages a clutch: a belt starts |
| `cart_treadle.scn` | A cart stopping on rails laid over a tread plate presses it and pulls its rod |
| `ore_hopper_rod.scn` | The mole on a tread plate holds an ore hopper's gate open by a rod: it empties onto the floor; off the plate, it shuts |
| `borer_bores.scn` | A steam borer fed coal by hand (it runs on coal alone; no water) bores east into granite at about 2 m/s when ridden: a clean 1.5 m tunnel, roof and floor whole, no rubble left; Space climbs out into the tunnel; saved |
| `borer_steers.scn` | Steered through solid granite east, then down (a curve, not a snap), on the diagonal, up (turning back the short way) and west: it goes the way it is pointed |
| `borer_needs_steam.scn` | With no coal a ridden borer raises no steam, doesn't move and says so |
| `borer_bedrock.scn` | Against a bedrock face the borer stops, cutting none; steered down it slides off along the bedrock and bores down beside it |
| `borer_falls.scn` | Driven off a ledge over a pit, nothing holds it: it falls with the mole in its cab and lands on the pit floor, cutting nothing |
| `wagon_train.scn` | A store wagon and a coal wagon set down at a borer's segment places couple on, the coal wagon stokes it, the iron ore it bores goes into the store wagon, and turned down the wagons follow round the bend and down the shaft; saved coupled |
| `wagon_couple.scn` | Wagons set down where a borer's segments ride couple on; E on a middle one is refused, E on the last uncouples it (it doesn't hook straight back on) and E again takes it up; set down at the train's new tail it couples again |
| `wagon_stoker_waits.scn` | A coal wagon coupled to a parked borer stokes one lump into the cold firebox, then holds its chute shut once steam is over half full: two minutes on, nothing more has burnt |
| `wagon_kinds.scn` | A store wagon catches dropped stone and stows the paw's stack; a workshop wagon is a workbench to craft at; a bunk wagon moves the wake point, so after a death the mole wakes by it |
| `shaker_sorts.scn` | Gravel drops through a 1-in-4 shaker screen; ore lies on it until current shakes it, then walks off the end into its own pit |
| `rail_needs_support.scn` | A rail span over a drop breaks and falls; its ends on rock hold |
| `blast_moves_sand.scn` | A charge by a sand bed throws a third of it into the air at once; the near end is scoured, and it all lands (none lost) |
| `blast_wave_front.scn` | The front reaches a shaft round a corner by the tunnel (path longer than the straight line), never the rock between; the drawn front sweeps out; `dump` of its state |
| `grain_in_body.scn` | A sand grain lying inside the mole's body (low tunnel, no wriggle room) is shoved aside instead of wedging it |
| `pillar_built_in_room.scn` | A wood pillar built cell by cell in a granite room stays rigid |
| `pillars_rock_roof.scn` / `roof_needs_pillar.scn` | Two pillars at the third points save a 16 m rock roof that caves in (bending) without them |
| `pillar_sand_floor.scn` | A pillar built on a sand floor rests on it |
| `beam_bridge_ledges.scn` | A wood beam bridging a 3.5 m gap between ledges high in a cave stays put |
| `tnt_inert.scn` | Placed TNT is inert; right mouse lights a 1.5 s fuse, then it blows |
| `tnt_live_wire_refused.scn` | TNT set down on or beside a live copperline is refused with one line and nothing goes off; on the idle line it goes down inert, and powering the line then sets it off |
| `tnt_lever.scn` / `tnt_thrum.scn` | Remote detonation: a lever on copperline, or a thrummer knocking through 2 m of granite to a listener |
| `pickup_tnt.scn` / `tnt_douse.scn` | E picks an unlit charge back up, not a lit one; water puts a fuse out |
| `pickup_wire.scn` / `pickup_device.scn` | E takes up copperline one segment at a time (a device on it first), and levers, lamps, knots and torches |
| `brush_size_keys.scn` | `[` / `]` (as typed) and Option+wheel resize every brush within its range; Shift and Ctrl/Cmd + wheel don't; a bigger bite digs more |
| `place_grid_snap.scn` | Grid snap squares (0.25 / 0.5 / 1 m); a drag lays a straight row or column; nothing in mid-air |
| `craft_unlock_by_touch.scn` | Survival: a recipe appears once any input is picked up; Creative lists all |
| `craft_oil_press.scn` | Survival: at a workbench 3 fungus press into 2 measures of oil and 2 mushrooms into 1; away from a bench nothing is pressed |
| `craft_stations.scn` | Survival crafting at a workbench; a forge refuses cold and works with lava in reach; a workshop refuses unpowered and makes the drill once a lever powers its line |
| `place_refused_once.scn` | A refused place says why in one line (brick in mid-air or into rock, table or cart with none left, torch into rock or with no oil), once per 5 s for the same words; a different reason speaks at once |
| `dig_hollow_floor.scn` | Digging down on 1 m of rock over a 16 m drop is held back at once ("The floor rings hollow"), said once; digging beside the mole still works and it never falls |
| `dig_down_lava.scn` | Digging down on a thin floor over lava is held back ("The floor is hot") and the mole never sinks in |
| `dig_down_shallow.scn` / `dig_down_water.scn` | A drop that only hurts (2.5 m), or a deep drop into water, isn't held back: the floor goes and the mole drops, silently |
| `dig_refused_once.scn` | A dig the Pickaxe can't do says so once per press and material (not per cell swept), repeats the words only after 5 s, and soft rock beside it still digs |
| `dig_refusals.scn` | Every dig refusal says why at once: out of reach ("Too far to dig"), water with nothing solid, granite too hard for the pickaxe; each once per press, the same words not again within 5 s |
| `death_cause.scn` | A death names its cause in the log: starving with nothing to eat, then (respawned) a blast close by; never the bare "has died" when the cause is known |
| `death_pack_dry_ground.scn` | Survival: a mole drowned in a flooded hole leaves its pack on the dry floor by the hole, not under the water; the log says it drowned and where the pack is; the burrow's stores refill food and headlamp oil (no items); walking back over the pack takes it |
| `death_pack_nearest_dry.scn` | A mole that drowns without having stood on dry ground nearby (sealed in a flooded chamber) still leaves its pack on the nearest dry floor within 16 m, the room above the lid |
| `grotto_discovery.scn` | Coming within 8 m of a grotto's hollow logs one discovery line (water seep, root-trees, glowmites, wood); away and back, and after a save and load, it isn't said again |
| `dig_refused_mixed_face.scn` | One held dig swept back and forth over a face of granite with iron ore in it says each refusal once in 5 s (the two lines used to take turns about four times a second) |
| `explore_clues.scn` | A seam the tools can't cut, first seen, logs a discovery naming what to come back with; digging it refuses with the same advice; seeing it completes "Find iron ore" and names the next quest; with a drill it digs and "Go back for the iron" completes |
| `holt_home.scn` | Wooden furniture is listed from the kit; one piece by the workbench isn't enough, two complete "Make it a holt", which pays 16 timber wall and says how to panel; a cart's cargo comes into the pack with E and the log names it |
| `tool_tiers_survival.scn` | Survival digs with the best tool owned: claws (dirt, soft stone), pickaxe, drill (granite), bore (basalt) |
| `forge_coal_fuel.scn` | With no lava in reach the forge burns a coal per job; with no fuel it's cold |
| `feel_feedback.scn` | Each action answers on its tick: dig strike, refused dig (clink), throw, a 0.5 m landing bump |
| `tnt_place_vs_throw.scn` | TNT set down lies unlit as a device; thrown it goes off where it lands; beams and copperline aren't thrown |
| `knots.scn` | Truth tables of the both/either/turn/odd/catch knots, swapped in place |
| `door.scn` | A lever opens a door; it waits for the mole to clear the doorway before shutting; no door on yourself |
| `plate.scn` | A pressure plate above the floor powers a lamp while the mole stands on it |
| `saveload_devices.scn` | A thrown lever, a lit lamp and a burning TNT fuse survive save/load |
| `band_keeps_devices.scn` | The same build survives its rows leaving the window and coming back, also across a save taken while they are away |
| `beam_overload_iron.scn` | A wood beam under a sand heap breaks into wood splinters; the same iron span holds |
| `beams_placed_across_pit.scn` | Wood beams placed one by one across an 8-wide pit, and an iron stub, stay put |
| `pillar_in_cave.scn` | A wood pillar stands while half-built and once it reaches the roof of a 40-wide cave |
| `clay_roof_falls.scn` / `pillars_hold_clay_roof.scn` | A 20-wide clay roof falls alone; four placed pillars hold it |
| `beam_overload_wide.scn` | A 30-wide wood beam under a sand heap breaks and the sand pours through |
| `shaft_jump.scn` | Jumping half a cell off-centre under a 2-wide shaft slides round the corner and goes up |
| `tunnel_mouth.scn` | Jumping at a tunnel mouth 3 cells up a wall slots the mole in on the way up |
| `dig_tunnel.scn` / `dig_tunnel_high.scn` | Walking right while pointing at the rock ahead (foot / head height) tunnels through |
| `wall_place_scrape.scn` | A timber wall panels the back; scrape takes it off; neither edits the granite in front |
| `wall_saveload.scn` | Lining and a scrape survive save/load and a window slide |
| `wall_no_support.scn` | A 20-wide clay roof still falls when the back is timbered |
| `wall_walk.scn` | The mole walks through a timber-lined corridor, unhurt |
| `site_holt_traverse.scn` / `site_pumpworks_traverse.scn` / `site_mine_traverse.scn` / `site_fissure_traverse.scn` / `site_fossil_traverse.scn` / `site_crystal_traverse.scn` / `site_halt_traverse.scn` | Walk each found place from a way in (PR #8 layouts): holt, salt hall, halt and mine to the chest, opened (`chests`; the halt's 4.25 m crate stacks by rope gun, a throw-walk-climb-jump cycle every 200 ticks, since there's no wall climbing; mine and halt start inside: 72 m and 80 m don't fit the 128 m harness world beside the start); pumpworks up over the wheel (by rope gun: the hook 15 m up into the gallery under the roof, a swing west over the wheel), fossil into the bone hall, fissure to the east mouth's edge, each stopping short of a layout hazard its header describes. `tests/found_places.rs` checks every site by flood fill |
| `site_pumpworks_rope_to_chest.scn` / `site_fissure_east_bridge.scn` / `site_fossil_over_the_beast.scn` | Site-trap fixes: from the fossil cave's east mouth under the skull, up the mound's 2.2 m east face by rope gun, over the ribs of the beast lying on the floor, `S` at the rope off the mound's west end, down to the chest at 100 hp with nothing crumbling or falling (`crumbled` 0, no debris); from the pumpworks' east door over the wheel by rope gun, down onto the window-sill landing, `S` at the rope's head, 7 m down and the chest opened at 100 hp; from the fissure's east mouth level onto the 24 m bridge to its broken middle, then 10 s standing there with nothing falling (100 hp, no debris) |
| `site_mine.scn` (with the copper face and the ballast; soot lining) / `site_holt.scn` (table and stools on the open floor) / `site_pumpworks.scn` / `site_fissure.scn` / `site_fossil.scn` / `site_crystal.scn` / `site_halt.scn` | Each found-place family stamps with its materials, lining and interaction (rails, furniture, a held sump, a rope, carved bone, quartz pools, a wrecked cart and no waygate). Each map is solid rock sized to its reworked site (up to 80 × 22 m, or 18 × 40 m for the fissure), with a pocket for the mole under it or beside it; the site carves itself out |

### Writing a new scenario

1. Copy the closest `.scn`, then draw the map. Leave 3 empty cells above `@` for the mole's body.
2. Add `dump`/`ascii` lines at the ticks you care about and run the file alone. Read the JSON to pick thresholds; for example, find the tick where `grounded` flips.
3. Turn what you saw into `expect` lines, then remove the noisy dumps.
4. Run the whole folder. Every scenario must PASS before you commit.

Tuning constants for movement live in one place: `TUNE` in `sim/src/world.rs`. After changing them, rerun both the tests and the scenarios.

Zoom limits and brush ranges also have unit tests in `sim/tests/sim.rs` (`zoom_is_clamped_both_ways`, `brush_sizes_clamp_per_item`), as do tooltip texts (`every_thing_has_a_lore_safe_tooltip`) and world hover text from `sim/src/tips.rs` (`world_hover_explains_cells`).

## 2b. Settling / performance check

`cargo run --release -p molehill-sim --example wakediag [seed] [ticks]` generates a world, runs 1200 ticks (or `ticks`) and reports how many CA chunks are still awake and what is still changing. A settled world should be close to 0 awake chunks (at 8 cells/m a fresh world takes about 4800 ticks to settle).

`cargo run --release -p molehill-sim --no-default-features --example digbench -- [seed] [route…] [--sliced] [--no-prefetch] [--realtime] [--max-ms N]` times every frame (a tick plus one `prefetch_step`) on the game-sized window along routes: `down` (falling down a cleared shaft at the 15 m/s terminal speed, the longest real fall), `up`, `diag` (carried at 6 m/s, 200 m), `side`, `west` (carried 300 m east or west at 6 m/s: the window slides sideways a tile at a time), `fastdown`, `fastup` (40 m/s, faster than any real fall: a stress test), `respawn` (dies 200 m down) and `load` (saved 200 m down, loaded, carried deeper). Times are CPU ms of the stepping thread, so other work on the machine doesn't skew them. It prints mean, p99 and worst per route, and the worst slide tick (down or sideways), seam tick and other tick apart. `--sliced` builds tiles on the stepping thread as the web does (slices for up to 6 ms a frame); `--realtime` paces frames at 60 Hz of wall time, as the game runs them; `--max-ms N` fails if any frame is over N ms. On seed 4242, `side` also runs through a long cave-in (about 44k cells crumble along the pocket). That is the stress pass's worst case: p99 25 ms and worst 71 ms since the anchor memo lasts a whole pass, down from 624 ms and 1401 ms. (Since cooling lava no longer flags re-solves, `side` measures p99 16 ms and worst 33 ms.) Tests in `sim/tests/stream.rs`: a tile is the same however it is built (at once, in slices, on the thread, after other tiles, east or west of the home columns); building ahead never changes the world; and the perf gate `a_slide_onto_ground_built_ahead_is_cheap` (a slide or a seam tick under 150 ms of CPU on the game window, against 0.4–1.3 s before).

`cargo run --release -p molehill-sim --no-default-features --example borebench -- [seed] [route] [seconds]` rides a fuelled steam borer from beside the burrow on the game-sized window and times every frame (CPU ms of the stepping thread, disasters off): routes `east`, `west`, `down`, `up`, `diag`, `wave` (east, swinging down and up every 3 s) and `idle` (nobody steers: what the world costs ticking by itself). It prints mean, p99 and worst, the ticks over 16.7 ms and over 8 ms, how far the borer got, and what the ground did: cells crumbled, split into cut free and overloaded and by material, and the ticks the rider was hurt. On seed 4242 for 60 s, `diag` went from p99 68 ms and worst 107 ms (160 ticks over 16.7 ms) to p99 17 ms and worst 35 ms (34 ticks) when cooling lava stopped flagging stress re-solves, `down` from p99 78 ms and worst 105 ms to 14 ms and 38 ms, and `idle` no longer has a tick over 16.7 ms (113 in 3 minutes before). `east` still has a 57 ms tick where the tunnel cuts a 3000 m² mass free at once.

`cargo run --release -p molehill-sim --example stepbench [seed] [ticks]` times `World::step` on the game-sized window from a fresh holt: mean/p50/p95/p99/max per tick, the average per tick class (stress passes every 6 ticks, air every 15), a full stress solve and a light pass. Add `--no-default-features` for single-threaded numbers, which are closer to the web build.

`cargo run --release -p molehill-sim --example grottos [seed]` lists the grottos in a fresh holt's window and draws one; `--example slidecheck [seed]` counts cells that would fail a full stress solve after generation and after a slide each way (should be 0). `--example grottoseed -- [SEEDS] [--show SEED]` scores seeds by how close the new holt spawns to a grotto with trees (generator only, fast); `--show SEED` generates that world and prints an ASCII map around the spawn. `--example sitewalk -- [KIND ...]` stamps each found place alone in granite and flood-fills where the mole's body fits (`World::body_regions`, gravity ignored): it maps the regions at 0.5 m a character and says which reach a way in, so a sealed room or a cut layout shows at once (`tests/found_places.rs` asserts the same, with the sites cut today listed). `--example firsthour -- [SEEDS] [RADIUS_M] [--show SEED] [--col SEED]` lists, per seed, the worst drop off a ledge and through a thin floor (under 1 m) within the radius of the burrow and how many would kill (first-hour traversal traps); `--show` maps the spawn, `--col` the cells under it. `--example progression -- [SEEDS] [MINUTES] [ONE_SEED]` is first-to-mid pacing evidence in Survival at the game's window: routes from the burrow to each first-hour find (distance, a straight-line dig time with the kit's pickaxe, and how many cells on the way it can't cut) and a naive bot (dig down, then tunnel sideways, with the sim's own pickup, eating, oil, discoveries and quests) whose log prints as a timeline with depths; a third argument runs the bot on that seed alone. It also prints the **ladder**: the raw, cumulative bill of each rung (bench, forge and 8 copperline, drill, thrummer, bore; `craft::raw_bill`), and per bot the first minute its pack could have paid each rung (the bot never crafts), and when the light first went out and hunger first hit zero. Headless only: it says nothing about what the player sees. `--example moodbench [kind…]` stamps each found place (or the named families) in plain host rock and reports its mood: the light from its own torches and glowing materials (mean, 90th percentile and max brightness, the share left dark under 0.05 or blown out over 1.2), one TNT charge in a mine, pumpworks or halt (cells changed, crumble and debris over 10 s, the furthest changed cell, the events heard), and the pumpworks' water over 30 s after its lever is flipped (conservation). `--example loosesurvey -- [SEEDS] [RADIUS_M] [--nodig]` measures loose ground round a fresh holt's burrow: per seed, the share of sand, gravel, fungus and knocked-loose cells within the radius (and dirt and soaked ground), then a dig bot (a 20 m shaft and 1 m tunnels 30 m out each way at three depths, carved at a steady pace with the mole at the face) reports cells crumbled, loose fill left in the tunnels and generator-loose ground displaced, per metre of tunnel (`WHY=1` breaks the displaced ground down by material). `--example digbot -- [minutes] [seeds…]` runs a Survival mole that tunnels and digs staircases near home (turning back 40 m out or 25 m down) and reports, per minute of digging, creaks, cave-in episodes (and how many big ones were warned by a creak 0.5–10 s before), crumbled area, burials and loose ground drifting near the mole; `WHY=1` adds what crumbled and the episode sizes, `DRIFT=1` what drifted. `--example survey -- SEEDS` reports what the ground offers around the burrow per seed (item counts within reach and the nearest of each, in metres, plus the nearest root tangle, grotto with trees and abandoned mine, and `find`: the nearest found place with a chest, as the trek from the burrow to its nearest cell, and its family); Survival's kit and costs are balanced from it.

`cargo run --release -p molehill-sim --example digtempo` prints the dig tempo: per material and tool, the ticks a bite takes and the seconds to clear a 4 × 3 m room with the default bite, and the steam borer's time for the same room as its 1 m tall tunnel with the metres a lump of coal bores (the table in [Tools and Items](docs/wiki/Tools-and-Items.md#tool-tiers)). The test `dig_tempo_is_earned_but_the_first_hour_is_open` holds the rules: a pickaxe bite of any pickaxe rock is at most 30 ticks and of soft soil at most 6, each tool clears about twice the room a second of the one before, and nearly all the rock within 25 m of a new burrow is the pickaxe's.

`cargo run --release -p molehill-sim --example genbench [seed]` times world generation: fill, one full stress solve, stabilisation, and `World::new` in total. `--example loadbench [seed] [slides]` times a new holt both at once and in steps (`world::NewHolt`, as the loading line runs it: step count, and every step over 40 ms with its stage). It then builds a long-running save by sliding the window `slides` × 32 m deep and halfway back, and times saving and loading it. Test `staged_new_holt_matches_and_climbs` (sim.rs): a staged build gives the same holt as `World::new`, its progress only climbs, and it passes through every stage. `loading_prefetch_finishes_the_ground_ahead`: after a new holt, after a load, and in slices on the calling thread as the web builds (`stream.sliced`), `prefetch_step` ends with `stream_progress` at 1, and progress only climbs on the way. Sliced natively (seeds 4242, 7, 99): 970–1020 steps, 9–13 s in all, about 11 ms a step, the longest 75–170 ms, all behind the loading line.

**Web frame time:** serve `dist/` (`python3 -m http.server`) and measure `requestAnimationFrame` intervals in Chrome after starting a holt. At 8 cells/m on an M4 with the active region: 58 fps (p95 16.8 ms) from a fresh holt's first seconds, 60 fps steady state.

Blast tests in `sim.rs`: `blast_overpressure_curve` (Kinney-Graham values, monotone, deterministic, bounded), `blast_is_heard_by_listeners`, `blast_debris_deterministic_and_conserved`. Blast sound tests in `blast_events.rs`: `one_charge_is_heard_once_*` (one charge in a generated world: one blast, no later Explode or Quake, no collapse rumble for a cell or two trickling off the crater). Transport tests in `transport.rs`: `transport_is_reachable_in_survival`, `creative_has_transport_freely`, `transport_hover_tips_say_what_to_do` (each machine's hover tip names its action), `production_is_reachable_in_survival`, `production_hover_tips_say_what_to_do`, `hoppers_do_not_stack` (a tube hopper and an ore hopper refuse to be set over each other, and say which one is there). Sluicing tests in `sluice.rs`: `fines_wash_and_lumps_stay_by_mobility`, `still_water_has_no_current`. Power tests in `power.rs`: `a_straight_shaft_needs_a_gear_to_turn_a_corner`, `right_mouse_turns_a_belt_run_round`, `every_block_item_can_be_built`, `every_material_has_its_own_template_char`. `--example blastbench` times one charge.

Sideways tests in `sim/tests/roam.rs` (the window sliding sideways, `World::shift_x`): `no_side_walls_and_the_window_slides_sideways` (no bedrock in a fresh window or in the columns that come in; a mole near the east side makes the window slide a tile, and it stays on its rock), `things_survive_leaving_the_window_sideways` (devices, copperline, torches, drops, a station, a plot and rock go with their tiles and come back, also from west of the home columns), `fresh_ground_is_the_same_from_any_side` (a tile that slides in sideways is the seed's, and is the same reached from above), `a_save_away_from_home_loads_back` (a save 96 m east and 32 m down keeps the window's column, its tiles and the burrow's world column), `a_fall_carries_through_a_slide_down` (a mole falling down a shaft keeps its speed through a slide down, and its fall is still measured from where it began), `a_stamped_site_survives_a_slide` (a site stamped by a script stays listed with its chest through slides east and down, drops out when the window leaves it and comes back with it; before, the first slide forgot it, which is what made `--dump-art` panic on the pump and halt scenes), `v11_save_still_loads` (`data/v11_cpm8.bin`, written by the v11 build: its full-width far bands become tiles, with things from both halves of a band, and it saves again as v12), `a_found_place_is_furnished_once` (a mine and a holt in the first window: a reload, a second reload and a short slide set out nothing the mole took; left alone, a mine that leaves the window and comes back is as it was, its carts where they stood with their stone, nothing lost and nothing twice; taken bare, it stays bare when it is back), `an_old_saves_places_stay_as_they_were_left` (a v13 save, made by cutting the list of furnished sites out of today's, loads with the places in its window counted as furnished; a v14 one, with its second list, loads too), `a_cart_waits_with_its_cargo` (a rolling cart with stone and coal that the window leaves, down or sideways, waits in `tr.away`, through a save too, and is back where it stood, standing still, with its cargo), `a_v15_transport_blob_still_loads` (a transport blob from before carts waited).

Region tests in `sim/tests/biomes.rs` (sampled over world columns a home width west to two east of home): `regions_follow_the_layers` (every region turns up; no Ashlands above 30 m and all of the deep below 110 m; Fungal hollows only in the middle layers and only as pockets; deterministic), `each_region_has_its_own_ground` (compared at the same depths: soil leads in the loam, limestone in the karst, gravel and granite in the crags, growth in the fungal hollows; the karst floods more than the crags; the ashlands are basalt with lava pockets and no growth), `each_region_lines_its_caves` (mycelium lining only in the fungal hollows, split basalt in the ashlands, the most roots in the loam, salt and damp in the karst, cracks in the crags), `hard_ground_lies_just_past_the_start_zone` (35-60 m under the burrow, over 40 seeds, still holds tier-2 rock), `sideways_region_edges_wander_with_depth` (where the region changes from one column to the next, under half the changes run straight on 2 m down; 0.10 now, 0.89 with straight seams), `linings_have_no_block_grid` (lining changes fall on 1 m block edges about as often as on any column: under 1.6/8; 0.14 now, 0.31 with the 1 m quilt), `regions_line_the_tiles_the_window_brings_in` (tiles slid in from the west and from below are lined by their own cells' regions, exactly as `natural_at` says, and mycelium only where the region is the fungal hollows). `--example biomediag -- SEED` prints the regions round a fresh holt.

Scenario `crumbled` counts the map and its frame only (`World::crumble_box` / `crumbled_in`): the generated world round the frame may settle a cell or two far off, whatever the seed put there, and that failed scenarios whenever the generator changed what lies round the default seed's burrow (18 of them, `tree_regrow` and the beam and pillar ones among them, on the biomes branch: a moss cell and a basalt cell 42 m off).

Settled ground tests in `sim/tests/settled_ground.rs`: `loose_lenses_are_rare_round_the_burrow_but_not_beyond` (lenses under 1.2 % of the ground within 40 m of the burrow and none soaked; over 3 % beyond 80 m at the same depths and over 2 % at 15-35 m down, 24 seeds), `a_new_holt_starts_on_settled_ground` (under 2 % of the ground within 30 m of a fresh holt's burrow is loose, and hardly any sand faces open cave, 5 seeds).

Item registry (`tests/items.rs`): the rows of `craft::ITEM_DEFS` are hand-written, so these check what each row must keep: rows in `Item` order with one name each; blocks, pigments, furniture and walls can be set down, furniture picked back up, a pigment's swatch is its paint colour; every density is sane and every usable item has a controls line.

Seeded variation (`tests/variation.rs`): `below_vary_from_a_site_is_its_template`, `mirroring_flips_cells_chest_and_things_and_about_half_are_flipped`, `depth_remaps_follow_the_layers`, `random_zones_fire_at_their_chance`, `generated_sites_vary`. `found_places_entrances_reach_every_room_and_the_chest` also walks seeded variants of each family (flipped and not, each zone on and off).

Site templates (`tests/templates.rs`): `templates_match_the_procedural_sites` is the golden check that the found-place data in `assets/sites/` stamps exactly what the procedural reference in `sites.rs` does, cell for cell, for every variant (var 0–95 plus large values, both halt sides): materials, linings, chest and things. After a layout change in `sites.rs`, run `cargo run --release -p molehill-sim --example export_sites` (`-- --check` only reports SAME/STALE per file). `site_chest_comes_from_the_template`, `template_files_round_trip_and_explain_errors`, `template_files_are_canonical_and_parse_fast` (files on disk are exactly what the writer makes; all seven parse in about 8 ms), `stamp_template_matches_the_game_stamp` (the cave editor's stamp equals the game's site stamp). A file the cave editor saved (`"source": "editor"`) is skipped by the golden check and kept by the exporter.

The dev-only cave editor has its own tests: `cargo test --release -p molehill-cave-editor` (model, checks against the real sim, `panel`: every panel row clicked and every key pressed through the window's own code, and `packaging`: not a default member, not a dependency of the game, not in `build-web.sh` or `site/`). See `docs/wiki/Cave-Editor.md`. See `docs/wiki/Site-Templates.md`.

Worldgen tests in `sim.rs`: `ore_sits_in_its_depth_bands` (copper just below the holt, iron in the mid granite, basalt below 90 m), `grottos_follow_the_layers` (frequency by depth band over 6 seeds, none in the deep, deterministic), `generated_grottos_hold` (fresh holts' grottos stay open with their trees up), `slides_leave_nothing_failing`, `glowmites_drift_glow_and_get_caught`.

Performance-related tests in `sim.rs`: `local_stress_resolve_matches_full_solve` (a region re-solve equals a full one), `region_solve_sees_a_block_cut_free` (a block cut free inside a region re-solve loses its load path even though its far edge held last time; tied to a wall past the region it holds), `cut_free_mass_resting_on_rubble_stays` (a cut-free block whose countdown is running stays if rubble fills in under it first), `slide_keeps_stress_state` (a slide's carried stress state matches a full solve, both ways), `ca_runs_only_near_the_mole` (the CA runs in the active region, and far chunks resume when it reaches them).

## 3. Visual check (optional)

`cargo build --release -p molehill`. On a headless Linux box, run it under `Xvfb :99` with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, wait about 8 s, and grab one frame with `ffmpeg -f x11grab`. The game opens on the main menu; press Enter to start a New Holt. One screenshot is enough; don't record videos unless asked.

Native builds also take a frame by themselves: `molehill --fresh --seed=4242 --shot=out.bmp [--shot-at=150] [--shot-ui=pause|inv|help|dev|loading] [--shot-walk] [--shot-blast=K] [--shot-zoom=0.6]` starts a holt, writes one BMP and quits (`--shot-ui=loading` instead stills the loading line while that seed's holt is built). `--scene=mine|holt|pump|fissure|fossil|crystal|halt|walls` stamps that found place (or a mixed lining room) in host rock next to the burrow so a still can inspect it without hunting a seed; `--scene=production` stamps a small works (chute, shaker screen, riffle bars, ore hopper over track with a tube hopper beside it, bumper); `--scene=rope` a hall with a hook rope bitten into the ceiling and a hook in flight paying out its rope; `--scene=power` a counterweight driving a shaft, two gears and a belt, a water wheel standing by; `--scene=steam` a lit boiler piping steam to an engine that drives a belt; `--scene=refine` a counterweight driving a stamp mill, a washer and bellows on a lit smelter, an empty and a setting mould; `--scene=logic` every linkage part (a tread plate under the mole, rodded to a gate holding a pool, which the mole's weight lifts, a turned-round gate standing lifted, a tipping scale with three stones, a float in a tank rodded to a clutch in a drive train). `--scene=borer` puts a lit steam borer, water and steam up, in a tunnel facing a dirt face with the mole riding it (add `--shot-walk` and the running game drives it into the face), and a second one parked behind it, lit, unridden and facing west. `--scene=climb` hangs the mole halfway up a rope ladder in a tall hall that also has a hook rope; `--scene=climb-rope` hangs it on the rope swinging (add movement's `--shot-up` or `--shot-down` and it climbs or slides; `--shot-at=20` and `23` catch two frames of the hand-over-hand cycle). `--scene=wagons` has a ridden borer near a dirt face hauling all four wagon kinds coupled behind it, the store and coal wagons part full (`--shot-walk` drives the train), and a second borer facing west near the burrow with a coal wagon coupled behind it. The `cells-*` swatches' second frame shows a linkage part pulled. `--dump-art` also writes `cells-<Mat>-0.bmp`/`-1.bmp`, the production and machinery cells' own art unlit, at rest and 1.5 cells of travel in. `view.bmp` composites rotated sprites too (ropes, the held item). Add `--dump-art=DIR` to also write every sprite (8×) and the composited world around the mole: `view.bmp` has every visible world sprite (furniture, found-place furnishings, chests, torches, carts, the mole) drawn over the cells, and `view_cells.bmp` is the cells alone (`--dump-wide`: both cover the whole window, every cell shown and evenly lit, for comparing the regions' wash, parallax and back walls; seeds 4242 and 7 hold all five regions), with the mole's column marked magenta (kept out of `view.bmp`, where it used to show a pixel or two past the sprite). These are CPU-side and cover the whole window, so they work even where the window frame doesn't. On macOS the window must be visible on screen: launched from a background shell the surface reads back empty.
