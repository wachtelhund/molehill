# Refinement log: Phase 2, p1 (branch `refine/materials`)

Goals: one material definition table used everywhere (no behaviour change), then loose items and thrown things as physics bodies.

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

2. **Items are physics bodies (`items.rs`).** A `Drop` now has a float centre, velocity, a 0.25 m box,
   rest/sleep, a pick-up hold and a fuse. Each tick it falls (`TUNE.gravity`), less buoyancy by the share
   of its box under liquid (`Item::density` vs the liquid's), with drag (thicker in lava); moves swept and
   axis-separated; bounces off what it hits by that material's `bounce` and loses speed along the surface
   by its `friction` (on impact, in proportion to the knock, and while sliding); snaps flush on contact;
   and sleeps after 6 still ticks until its support goes. Heat burns burnable items (`Item::burns`, from
   the table). Dug items pop out of the bite. Blasts fling loose items (`fling_items`).
   **Throwing:** `World::throw_item(item, aim, power)`; in the game, hold `Q` to charge (0.8 s) and release.
   A thrown TNT is lit (`TNT_FUSE`), goes off where it is when the fuse runs out (`World::blast_tnt`, the
   same blast as placed TNT), is doused by water and lit by lava or fire; a lit charge and a just-thrown item
   can't be picked up. New table columns `bounce`, `friction`. Scenarios: `throw_arc` (arc, landing, rest,
   pick-up), `item_bounces_rests`, `item_sinks_floats`, `thrown_tnt_explodes_at_landing`. Harness:
   `throw`, `drop`, vars `items:`, `ix:` `iy:` `ivx:` `ivy:` `irest:` `ilit:`.
   **Not saved:** a drop's velocity, fuse and hold (it reloads at rest in its cell; a lit charge reloads
   as a dud). Floating items never sleep (cheap: there are few).

## Left alone
* `light.rs` still special-cases glass transmittance (0.88); p3 owns the light system, so that stays theirs to
  fold into the table.
* `craft::drop_of` (material → item) and the spore/crystal yields stay with the items in `craft.rs`.
* Steam's 1-in-7 condensation and the doused-flame puff (60 ticks) are still literals in `grid.rs`.
