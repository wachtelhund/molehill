# Refinement log: Ruins of the Ancients (branch `refine/ruins`, p1, iteration 2)

Scope (Hampus): rare random ruins (pumping halls, sealed vaults, collapsed tube stations) with lore fragments,
unique loot, and broken waygates as the endgame transport hook. No magic, no modern names. gen.rs changes
isolated. Ruins take part in the physics.

* `sim/src/ruins.rs`: placement (one per 160 × 80 m cell by hash; none above 15 m, 5 % → 25 % by 60 m; never by
  the start, never in a grotto), the three layouts (`Ruin::cell`), `Ruin::new`, eight lore fragments, vault loot
  and relics, and `step_ruins` (first visit within 8 m reads the carving; a tube station describes its waygate).
* gen.rs hooks (two lines): `cell_with` stamps ruin cells first; `in_grotto_void` also keeps ruin interiors open.
* Relics (new items): Ancient bit (dig time × 3/4 while carried), Ground lens (headlamp oil burns at half).
  Each turns up once (`World::relics` bitmask). Saved with the ruins visited (save v8).
* World: `ruins` (refreshed with the chests), vault chests (`Chest::ancient`).
* Physics: ruins are plain brick/iron/door/gravel. Designed so they stand as built but are honestly fallible: the
  hall's roof is brick on iron pillars (an iron girder only under the fallen pillar's bay). Embedded in rock the
  solver's arching carries any roof, so collapse comes from an open cave above plus load (sand) or the like.
  Found on the way: the solver counts a static cell hanging under a roof (a cut pillar's upper half, a stub left
  by a bite) as a support, so cutting pillars from below doesn't drop a roof. Worth a look in stress.rs (p2).
* Harness: `ruin T kind dx dy [left]`; vars `relics`, `ruins_seen`; generated ruins are cleared from scenarios.
* Tests: `ruins_are_rare_deep_and_clear_of_the_start_and_grottos` (300 seeds, plus seed 66's generated tube
  station keeps its waygate), `relics_bite_faster_and_save_oil`. Scenarios: `ruin_hall_stands`,
  `ruin_hall_caves_in`, `ruin_vault_loot`. Example `ruinsurvey`.

## Open
* Waygates are a hook only: no mending or travel yet (PROGRESSION's open decision).
* No pixel art for ruins beyond the material textures (brick, iron, door); p3's area if wanted.
* Ruin carvings go to the log (toasts), not p3's Lid journal; they could join it.
