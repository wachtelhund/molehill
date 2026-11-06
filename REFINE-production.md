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
* Blocks: **Chute** (timber trough), **Shaker screen** (iron grating on a timber frame; a shaking state is nice to have),
  **Bumper** (timber buffer stop). Today they draw with their table colour.
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
