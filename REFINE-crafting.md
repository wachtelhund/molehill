# Refinement log: crafting, recipes and balance (p7)

Scope (from Hampus): audit recipes, fill progression gaps, sensible costs and yields; hunger and lamp oil as
interesting choices without blocking early discovery; headless progression checks (`examples/progression.rs`).
New items need a unique icon from p3, so this slice reuses existing items.

## Slice 1 (branch `refine/crafting-balance`)

**Audit (all 50 recipes before this slice, 52 after, `craft::RECIPES`).** The ladder holds up: every chain is at most two steps, every
metal has a use, and the drill needs no iron. The drill's whole bill from the ground, with its three stations
and a coal per cold-forge job, is 6 wood (exactly the kit's), 20 stone, 4 clay, 8 copper ore, 2 tin ore and 4
coal. Findings:
* **Gap: no oil recipe.** Fungus oil came only from the pack's automatic press (2 fungus a measure). There
  was no way to stock up at home, and a mushroom farm fed only the belly.
* Bronze from ingots (3 copper + 1 tin ingot → 4) costs the same ore as co-smelting (3 + 1 ore → 2, twice),
  but takes five forge jobs instead of two, so on a cold forge it burns more coal. Left as is: it uses up the
  ingots that the "Smelt 4 copper ingots" quest leaves in the pack.
* The torch is cheap to make (1 wood + 1 coal → 4) but soaks up a measure of oil each time one is set down
  in Survival, so light is paid for in oil, not wood. That is fine.
* Design items still missing: the glowmite lantern (a placeable light with no fuel). It would be a new item,
  which needs a p3 icon. Left for a later slice.

**Changes.**
1. **Oil press** (workbench): `Fungus oil x2` = 3 fungus (better than the pack's 2:1, so carrying fungus home
   pays), and `Fungus oil (mushroom)` = 2 mushrooms → 1 measure. Two mushrooms are 70 food (about 3.5 min) or
   3 minutes of light: a farm feeds the lamp or the belly. Both reuse the Fungus oil item, so no new icon.
2. **`craft::raw_bill` / `bill_of_station`**: what the ground has to give for anything, the cheapest way,
   counting a coal per forge job.
3. **`examples/progression.rs` ladder**: prints the raw, cumulative bill of each rung (bench, forge + 8
   copperline, drill, thrummer, bore), and for each bot the first minute its pack could have paid each rung,
   and when the light first went out and hunger first hit zero.

**Progression check** (`progression -- 6 15`, the bot never crafts; "could pay" = its pack held the rung's
whole raw bill at once):

| seed | bench | forge + line | drill | light out first | hunger at zero first |
|---|---|---|---|---|---|
| 7919 | 0.2 min | 0.8 min | never | never | never |
| 15838 | 0.2 min | never | never | 3.0 min | never |
| 23757 | 0.2 min | never | never | 3.0 min | 4.1 min |
| 31676 | 0.2 min | 0.2 min | never | 3.1 min | 8.7 min |
| 39595 | 0.2 min | never | never | 3.0 min | never |
| 47514 | 0.2 min | 0.3 min | 5.0 min | 3.1 min | never |

The bench is always affordable at once, from the kit. The drill rung is short of coal, clay or tin for a
digger that only goes down and sideways (one seed in six), and the quests' coal rewards never come because
the bot builds nothing. **The light goes out at about 3 minutes on 5 of 6 seeds, where the kit holds about 9.**
The cause is death, not the kit: the bot drowns early, its oil goes with the dropped pack, and it wakes in
the burrow with only what is left in the lamp. That wakes a new player in the dark with nothing to press,
which works against early discovery. Next slice candidate.

Tests: `the_drill_costs_a_first_hour_haul`, `the_bench_presses_oil_better_than_the_pack`.
Scenario: `craft_oil_press`.

## Dig tempo (branch `refine/dig-tempo`, Hampus playtest)

Feel: digging was too fast; clearing a base didn't feel like an accomplishment. The rule is unchanged (a
bite takes the slowest cell in it: `dig × 4 / (1 + tier) + 1` ticks, at least 4, and clears the whole round
bite at once). Only the `dig` column of the material table changes. Tiers are unchanged, so what the kit's
pickaxe can cut around the burrow is the same: all of the rock within 25 m on the seeds tested.

`dig`: sandstone, copper and tin ore 8→12, limestone 9→14, coal 5→7, clay 3→4 (pickaxe rock ×1.5); granite
14→22, iron ore 11→17, quartz 12→18, crystal 10→15, basalt 12→19 (drill and bore rock about ×1.6); stone
brick and polished stone 8→12, carved pillar 9→14 (laid stone with its rock). Dirt, sand, gravel, roots,
fungus, moss and mycelium are unchanged.

Ticks a bite / seconds to clear a 4 × 3 m room with the default bite, before → after (`--example digtempo`):

| material | claws | Pickaxe | Drill | Bore |
|---|---|---|---|---|
| dirt, gravel, roots | 9 / 2.8 s (same) | 5 / 0.8 s (same) | 4 / 0.4 s (same) | 4 / 0.3 s (same) |
| sand, fungus, moss | 5 / 1.6 s (same) | 4 / 0.7 s (same) | 4 / 0.4 s (same) | 4 / 0.3 s (same) |
| clay | 13→17 / 4.1→5.4 s | 7→9 / 1.2→1.5 s | 5→6 / 0.5→0.6 s | 4→5 / 0.3 s |
| coal | 21→29 / 6.7→9.2 s | 11→15 / 1.8→2.5 s | 7→10 / 0.7→1.0 s | 6→8 / 0.4→0.5 s |
| copper or tin ore, sandstone | 33→49 / 10.4→15.5 s | 17→25 / 2.8→4.2 s | 11→17 / 1.1→1.7 s | 9→13 / 0.6→0.9 s |
| limestone | 37→57 / 11.7→18.0 s | 19→29 / 3.2→4.8 s | 13→19 / 1.3→1.9 s | 10→15 / 0.7→1.0 s |
| iron ore | – | – | 15→23 / 1.5→2.3 s | 12→18 / 0.8→1.2 s |
| quartz | – | – | 17→25 / 1.7→2.5 s | 13→19 / 0.9→1.3 s |
| crystal | – | – | 14→21 / 1.4→2.1 s | 11→16 / 0.7→1.1 s |
| granite | – | – | 19→30 / 1.9→3.0 s | 15→23 / 1.0→1.5 s |
| basalt | – | – | – | 13→20 / 0.9→1.3 s |

Tool contrast (room per second, limestone): the pickaxe clears about 3.8× what claws do, and the drill about
2.5× the pickaxe; on granite the bore clears about 2× the drill. The first hour stays open: a pickaxe bite
of any pickaxe rock is at most 30 ticks (half a second) and of soft soil at most 6. Test
`dig_tempo_is_earned_but_the_first_hour_is_open`.

## Open
* Waking after death: the lamp holds whatever is left, with no oil in the pack (above). Option: the burrow
  keeps a measure for whoever crawls back in (a stash, not magic), once per wake.
* Glowmite lantern (new item: needs p3 icon).
* The bot never builds or crafts. A crafting bot that walks the quest line would test pacing directly.
