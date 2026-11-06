# Refinement log: production (p9, branch `refine/production-minecarts`)

Goal: a physics-based mining production line where everything is a real loose item. Staged: 1 minecarts, hoppers and the
shaker; 2 sluicing; 3 mechanical power; 4 coal power; 5 refinement; 6 non-electric logic. Wiki: `docs/wiki/Production.md`.

## Stage 1: minecarts, chutes, ore hoppers, shaker screen

* **Chutes** (`Mat::Chute`, flag `CHUTE`). A loose item lying on one slides down its slope with g·sinθ against μ·g·cosθ.
  The slope is read over ±1 m of chute top, stepping inward to the nearest chute column, so a stair-built chute reads as its
  run, and neither the rock under it nor the drop past its end counts. On a chute, the tick's own settling contact no longer
  takes impact friction. Elsewhere it does: other item scenarios are unchanged.
* **Shaker screen** (`Mat::Screen`, `CHUTE | SCREEN`). Items finer than `MESH_M` (5 cm, `Item::size_m`) pass through it,
  so `item_hits` is now per item (`World::item_passes`). Copperline current in its block cuts its grip to 15 %, and asleep
  items on it wake up.
* **Bumper** (`Mat::Bumper`). A loaded cart that hits one tips (`Cart::tip`): one item every 3 ticks goes out over the front
  as a loose `Drop`. If nothing fits, the load jams. When the cart is empty it rights itself and its running direction flips.
* **Ore hopper** (`Item::OreHopper`, `Transport::bins`). A bin on legs over the track. It catches loose items and poured
  loose cells (`CELLS_PER_ITEM` cells per item) and holds up to 60. Its gate is tripped by a cart under it with room, or held
  open by right mouse. It falls if both legs lose their floor. It is saved (transport encoding v4) and shifts with the window.
* Harness: map `h` `H` `|`; vars `bins` `binload` `binopen` `tips` `cargoof:K` `cargo:Item` `zone:Item,..` `shaking:DX,DY`.
* Scenarios: `chute_slides`, `cart_tips_at_bumper`, `ore_hopper_feeds_cart`, `shaker_sorts`.

### For p3 (art): new kinds needing a unique icon and in-world sprite
* Pack icons: one each for Chute, Shaker screen, Bumper and Ore hopper in `itemart.rs`, after the unique-art merge, so
  `every_item_has_its_own_icon` holds. Please restyle them as you like.
* Blocks: **Chute** (timber trough), **Shaker screen** (iron grating on a timber frame; a shaking state is nice to have),
  **Bumper** (timber buffer stop). In the world they draw with their table colour.
* Machine: **Ore hopper**: bin, two legs, spout and gate (open or shut), with its heap filling by `load()/BIN_LOAD`.
  Today it draws as plain boxes.
* Cart: `Cart::tip > 0` while tipping (tilt toward `dir`).

### For p7 (crafting) / p8 (registry)
Additive rows only: items `Chute, Screen, Bumper, OreHopper` were appended to the end of `Item` (save indices unchanged),
plus four recipes (Chute x8, Bumper x2, Shaker screen x4, Ore hopper). Costs are provisional for p7's balance pass.
`Item::size_m` is a new per-item column, ready to move into p8's ItemDef.

### Left alone
* Item-on-item stacking (piles, back-ups) isn't modelled yet. Stage 3 belts need it.
* `sites.rs`: only `tip: 0` on its cart literal. No layout edits.

## Stage 1 polish (branch `refine/production-polish`, after PR #9)

I playtested the machines together in headless scenarios and probed the edges:
* **Saved mid-tip (fixed):** a cart saved while tipping came back with 17 of its 20 stones and never tipped again. The tip
  state isn't saved, and only a cart arriving with speed started a tip. Now a loaded cart standing against a bumper tips.
  Scenario `cart_tip_resumes`.
* **Mole beside the hopper (no change needed):** with the mole standing by an ore hopper, all 6 lumps went into the cart and
  none into the pack, because released lumps carry a 20-tick pick-up hold that outlasts the 0.75 m fall. It is asserted in
  `production_line`.
* **The whole line (new scenario):** hopper → cart → powered stretch → slope → bumper tip → chute → steep shaker → two pits.
  All 8 lumps sort correctly. Scenario `production_line`.
* **Survival (new tests):** all four machines can be made in Survival, and their hover tips say what to do. Tests
  `production_is_reachable_in_survival` and `production_hover_tips_say_what_to_do`.
* **Noted for stage 2:** a shaker screen is a solid cell, so water doesn't run through it. Sluicing will need liquids to pass
  screens and riffles.

## Stage 2: water sluicing (branch `refine/production-sluice`)

* **Water through screens** (the one change in p2's CA, `grid.rs::liquid_target`): a liquid cell with a screen under it
  drains through up to a tile of screen into the open under it. Scenario `water_through_screen`.
* **The current** (`sluice::current`) is read from the water's surface slope over ±1 m (the CA keeps no velocities):
  speed 4·√|s| m/s, bed stress depth·|s|. A level pool is still, so p2's pools and their scenarios are untouched.
* **Mobility** = (ρ/ρw − 1)·size. Water carries a lump past `WASH_K`·h·|s| (1.0) and holds it up at twice that.
  Fines (sand to coal, ≤ 0.03) wash; ore and stone (0.26 to 0.34) stay. At `WASH_K` 1.5 deep water at the reservoir mouth
  rolled iron ore 1.5 m; at 1.0 it moves a tile at most.
* **Items** (`items.rs`): drag toward the current. In a current, bed friction scales with submerged weight; at first a lump
  on the bed kept full dry friction, which cancelled the pull every tick. A held-up lump is lifted over what it's driven
  into (riffles).
* **Grains** (`sluice::step_wash`, every 4 ticks over the awake chunks; about 0.005 ms a pass, measured beside a 0.3–0.5 ms CA step): loose cells move a cell downstream. Riffles trap
  washed sand: a grain can't climb a 2-cell bar because the CA keeps sinking it. That's real sluice behaviour, and it's
  documented.
* **Riffle bar** (`Mat::Riffle` / `Item::Riffle`, workbench: 2 wood → 8). A sluice floor must not be chute: a 1-in-4 chute
  slides ore down dry.
* **Regression found by p2's `death_pack_dry_ground` (fixed):** a flooded passage under a bedrock roof beside an open
  hole read as a downhill surface, because its "top" was capped a tile lower. That fake current dragged a dead mole's
  grubs under the roof. Only a free surface (open above) now counts. Test `water_under_a_roof_is_not_a_current`.
* Harness: `loosen T DX DY`; vars `flow:`, `bedstress:`, `cells:`; legend `z`. Scenarios `water_through_screen`,
  `sluice_sorts`, `sluice_washes_rubble`; tests `sluice.rs`. A hover line on running water gives direction and speed.
* For p3: **Riffle bar** needs an in-world look (it draws with its table colour) and has a pack icon in `itemart.rs`.

## Stage 2 polish (branch `refine/production-sluice-polish`, after PR #33)

I playtested sluicing in context and probed the edges:
* **A blast destroyed the ore it broke (fixed).** A 0.5 kg charge in a 26-tile iron-ore seam left no loose ore at all:
  ores and coal had `crumbles_to: Gravel`, so sluicing after a blast had nothing to recover. Ores and coal now crumble into
  loose cells of themselves; the same charge leaves about 14 tiles of loose iron ore. Scenario `blast_ore_leaves_ore`.
  Progression note for p7: loose iron ore still needs a drill to dig by hand, but an ore hopper takes poured rubble, so
  blasting plus a hopper is an alternative to drilling iron. That's legitimate mining, kept on purpose.
* **Washing and world settling (no change needed):** `wakediag` over 1200 ticks with washing on vs off: seed 11
  5.4 vs 7.8 awake chunks, seeds 42 and 4242 0 vs 0. Washing doesn't keep generated worlds awake.
* **Fungus under running water (no change needed):** it's lighter than water, so p2's CA floats it up, and floating
  debris drifts with the current. That's correct.
* `site/` is rebuilt on main after #33.

## Stage 3: mechanical power (branch `refine/production-power`)

* **Trains** (`power.rs`): `MECH` cells flood-filled over (cell, axis) states. A plain shaft passes along its own axis only;
  a `HUB` (gear, wheel, belt, drum) passes every way, so corners need a gear. Solved every 6 ticks over the active region.
* **Power balance** per train: wheel supply against belt, item and screen demand; speed is supply/demand, capped at 1.
  - **Water wheel:** undershot, ½ρAv³η with the current read beside the wheel (the water under it is capped). It first gave
    nothing, because a one-cell sheet never touched the paddles; they now reach 2 cells down.
  - **Counterweight:** 200 kg on a drum; falls to cover a shortfall and winds up on surplus, at up to 0.25 m/s. Saved
    (transport encoding v6).
* **Belts:** `Belt` runs right and `BeltL` left (laid facing; right mouse turns a run round). Items are gripped toward
  belt speed; the per-tick settling friction is exempt on belts, as on chutes (at first it halved the speed). `belt_queue`
  keeps lumps from passing on a belt row, never pushing one into rock (a jam).
* **Screens** are shaken by a turning train within a tile.
* **Fixed (stages 1 and 2):** chutes, screens, bumpers and riffle bars had no build code, so the game's brush couldn't
  place them. Scenarios stamp maps and never caught it. Block codes now come from one table, `world::BUILDS`; codes 2..=10
  are unchanged. Test `every_block_item_can_be_built`.
* **Lore:** belts are woven fungus-fibre, not leather (there's no hide in the Deepholt).
* Recipes (for p7): gear x2 (2 wood), water wheel x8 (4 wood), belt x8 (2 wood + 2 fungus) at the workbench; iron shaft
  x8 (1 iron ingot) and counterweight (4 iron ingots + 2 wood) at the forge.
* For p3: Iron shaft, Gear, Water wheel, Belt (two directions; a moving look would help), Counterweight (drum, rope and
  weight; a placeholder is drawn in `main.rs`). All five have placeholder pack icons in `itemart.rs`.

## Stage 3 polish (branch `refine/production-power-polish`, after PR #45)

I playtested the trains in context and probed the edges:
* **The mole didn't ride belts (fixed).** A belt turning at full speed ran under a standing mole, which stayed put. A
  grounded mole on a turning belt is now carried at its speed if its body fits (`carry_on_belt`); walking adds to it.
  Scenario `belt_carries_mole`.
* **A counterweight passed through what was built under it (fixed).** Its floor was measured once, when it was hung, so a
  pillar built under it didn't stop it. The floor is now re-measured down its own line each solve. Scenario
  `counterweight_stops_on_build`. (The first try measured a quarter metre either side and caught the shaft beside the rope
  in `water_wheel`; the rope runs plumb, so only its own line counts.)
* **Solve cost (improved).** Every solve rescanned the active region for machinery: 0.26 ms every 6 ticks even with none.
  Now it rescans once a second, cells join as they're built, and the cached list is re-checked between: about 0 ms a
  solve between rescans.
* `site/` is rebuilt on main after #45.
