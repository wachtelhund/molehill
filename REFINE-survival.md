# Refinement log: Survival mode (branch `refine/survival`, p1)

Scope (from Hampus): New Holt chooses Survival or Creative; Survival's small kit, food, fungus-oil fuel,
stale air as is, and death leaving the pack where you fell. Minimal UI. See PROGRESSION.md.

1. **Modes (`sim/src/survival.rs`).** `World.mode` is `Plain` (the bare rules, the default, so every existing
   test and scenario is unchanged), `Survival` or `Creative`; `Mode::rules()` switches on `Rules`:
   `tools` (dig only with owned tools: `World::tool()` clamps the hotbar's tool to `tool_tier`), `fuel`
   (headlamp and torches burn fungus oil), `drop_pack` (death spills the pack), `harmless` (Creative).
   `World::set_mode` applies the rules and the pack (the kit, or 999 of everything). Saved (save v5: mode
   and headlamp oil; older saves load as Plain, which the game treats as Creative, the old behaviour).
2. **Kit:** stone tools (Pickaxe), 4 torches, 3 grubs, 2 roots, 2 fungus oil.
3. **Food:** hunger already drained and auto-ate; new item **Root** (food 30) comes from digging root
   tangles (about one per 8 tiles).
4. **Fuel:** new item **Fungus oil**. A lit headlamp burns 180 s a measure; when a measure runs out it takes
   the next, else presses 2 fungus, else goes dark (one message). Placing a torch takes a measure.
   `World::lamp_lit()` replaces `gear.lamp` for the light source, the reveal cone and the lens halo.
5. **Death:** in Survival the whole pack spills as loose items (physics bodies) where you fell; the
   respawn is unchanged. Burnable items dropped in lava or fire burn.
6. **Creative:** vitals held full and nothing kills (also no hurt flash).
7. **Game:** New Holt opens a small screen: Survival, Creative, Seed, Back. Restart keeps the mode. The
   background world at start is Survival.
Scenarios: `survival_kit`, `survival_food`, `survival_lamp_oil`, `survival_death_drop`, `creative_harmless`.
Harness: `mode`, `set`, `placetorch`; vars `hunger`, `oil`, `lamp`, `mode`; map `R` roots, `f` fungus.

## Open
* The hotbar still shows the Drill and Bore in Survival (they act as your best owned tool) and items at 0;
  a Survival hotbar layout belongs with the pack UI (p3/p4's files), so I left it.
* No way to craft yet in Survival (crafting is hidden), so tools beyond the kit can't be made: that's the
  next slice (workbench, recipes unlocked by touch).
* No oil gauge on the HUD: the pack count and the "out of oil" message carry it, to keep the UI minimal.
