# Refinement log: material table (branch `refine/materials`, Phase 2, p1)

Goal: one material definition table used everywhere. No behaviour change.

1. **`Props` is the whole material.** The table is now named struct literals (`..BASE` for defaults) and holds
   everything that used to be spread across the code as hard-coded lists:
   * heat (`heat`, `ignite`, `cools_to`): what burns and from what (coal only from lava), lava quench and
     cooling to basalt, a flame doused into steam, a mole burnt by lava or fire, the air's fire load, the
     Heat sensor (`heat >= 2`, molten), fuses lit by heat;
   * flow (`spread`, `flow_in_10`): water runs 6 tiles a tick, lava 2 and moves on 3 ticks in 10;
   * `life`: how long steam and fire last;
   * flags `SOIL`, `POROUS`, `GRUBS`, `BUILT`, `NOTABLE`, `OPEN` replace eight `matches!` lists in
     `world.rs`, `air.rs` and `disasters.rs`;
   * the tooltip line (`desc`), which was a parallel array;
   * the unused legacy `atten` field is gone (thrumming derives from density and compressive strength).
   Helpers: `Mat::has(flag)`, `Mat::passable()`, `Mat::lit_by(heat)`.
   **Bit-identical:** three generated holts (seeds 3, 11, 42; 512×256 tiles) stepped 1200 ticks hash the
   same before and after (cells, flags, life, air, mole hp, crumble count). Test:
   `material_table_is_consistent` (row invariants and the heat rules).

## Left alone
* `light.rs` still special-cases glass transmittance (0.88); p3 owns the light system, so that stays theirs to
  fold into the table.
* `craft::drop_of` (material → item) and the spore/crystal yields stay with the items in `craft.rs`.
* Steam's 1-in-7 condensation and the doused-flame puff (60 ticks) are still literals in `grid.rs`.
