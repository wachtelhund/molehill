# Refine run: bugs and logic

Short log of what this run changed, and what is still open.

## Changed
* **Placed pillars and beams** (the user's bug report: they collapsed and rolled away like powder). I fixed this on this branch, and a parallel run landed an equivalent fix on main first (`6bbb493`). When merging I kept main's solver (`stress.rs`, `mat.rs`) and dropped my overlapping version. My scenarios run against main's solver and pass: `pillar_holds`, `pillar_on_sand`, `beam_bridge_ledges`, `beam_overload_iron` (wood breaks into splinters, iron holds), and `pillars_rock_roof` with its control `roof_needs_pillar` (pillars rescue a roof that fails in bending).
* Scenario harness: `place` directive (now main's brush placement; the default pillar brush is one cell).
* **TNT no longer explodes where you click** (`logic.rs`, `world.rs`, game `main.rs`). Placing TNT (item or device) puts down an inert charge. Right mouse lights a 1.5 s fuse (it blinks); fire or lava lights it too; water puts it out. Current on an adjacent copperline sets it off at once, which gives remote detonation by lever, knot or thrummer→listener. A blast nearby or a fuse cord burning into it also sets it off at once. `Ctrl`+right mouse on a lit charge defuses it back into the pack. Lava no longer blows TNT instantly (it lights the fuse), and a charge buried under rubble stays put instead of being "destroyed by cave-in".
* **Device state is reset on place/remove**: a fuse cord picked up while burning used to come back already burning; channels, latches and listener/thrummer flags no longer leak to the next device on that block.
* **Pack accounting**: dragging copperline over a knot used to eat a copperline every frame. `Ctrl`+right mouse now refunds the copperline under a device too, and a knot placed on copperline gives the copperline back. Placement goes through `World::place_device / use_device / lay_wire / pick_up`, which also check reach.
* **Save/load**: burning fuses and lit TNT keep burning after a load, and the death count is kept (optional trailing section, so older v3 saves still load). Loading a save where the mole is far below the burrow no longer counts as a fall: it used to kill the mole on load.
* Harness: `device`, `use`, `pickup`, `wire`, `saveload` directives; `dev:`, `lit:`, `on:`, `wire:`, `ev:` variables. Action events (clicks, places) are no longer dropped. New scenarios: `tnt_inert`, `tnt_lever`, `tnt_thrum`, `tnt_defuse`, `tnt_douse`, `saveload_devices`. New tests: `load_deep_does_not_fall_damage`, `wire_and_device_pack_accounting`, `replaced_fuse_is_not_burning`.
* **Doors never shut on the mole**: a door (or floodgate) told to close waits while a mole stands in its block, then closes once it is clear. A door can't be placed on yourself. Scenario `door`.
* Knots audited with a truth-table scenario (`knots`: both, either, turn, odd, catch). They were correct.
* **Pressure plates see the whole mole**: a plate used to check only the mole's feet cell, so a plate just above the floor never triggered (depending on block alignment). It now uses the same body overlap as doors. Scenario `plate`.
* **Lamp and Vent pack items place their device** on left click (they used to do nothing in hand).

## Open
* `site/` was not rebuilt: there is no wasm toolchain on this machine (`wasm-bindgen`, `wasm32-unknown-unknown`). The native game builds; run `cd engine && ./build-web.sh` where the toolchain is installed.
* Main's stress model still sums overburden through air gaps (a dirt layer under ~16 m of rock with caves in between is crushed even if the caves carry it). Not a placed-block problem; left as is.
* Placing TNT onto a copperline that is already live sets it off on the next tick. That is consistent (it is powered), but easy to do by accident.
