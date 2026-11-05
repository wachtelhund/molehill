# Refine run: bugs and logic

Short log of what this run changed, and what is still open.

## Changed
* **Placed pillars and beams no longer collapse and flow away** (`stress.rs`). Overburden used to add up through air gaps, so a free-standing post or a beam bridging a gap carried the whole roof above the gap, cracked and crumbled. Overburden now counts only material resting contiguously on a cell. A run only takes tributary load from a void beside it if it reaches that void's ceiling. Natural cantilevers and ledges benefit the same way.
* **Broken wood and iron become their own debris** (`mat.rs`): wood beams and doors crumble into loose wood splinters, and iron beams into loose iron scrap, instead of sand or gravel. Digging the debris gives the wood or iron back.
* Scenario harness: `place` directive and the `I` (iron beam) map char. New scenarios: `pillar_holds`, `pillar_holds_roof` + `roof_needs_pillar` (control), `beam_spans_gap`, `beam_overload`.
* **TNT no longer explodes where you click** (`logic.rs`, `world.rs`, game `main.rs`). Placing TNT (item or device) puts down an inert charge. Right mouse lights a 1.5 s fuse (it blinks); fire or lava lights it too; water puts it out. Current on an adjacent copperline sets it off at once, which gives remote detonation by lever, knot or thrummer→listener. A blast nearby or a fuse cord burning into it also sets it off at once. `Ctrl`+right mouse on a lit charge defuses it back into the pack. Lava no longer blows TNT instantly (it lights the fuse), and a charge buried under rubble stays put instead of being "destroyed by cave-in".
* **Device state is reset on place/remove**: a fuse cord picked up while burning used to come back already burning; channels, latches and listener/thrummer flags no longer leak to the next device on that block.
* **Pack accounting**: dragging copperline over a knot used to eat a copperline every frame. `Ctrl`+right mouse now refunds the copperline under a device too, and a knot placed on copperline gives the copperline back. Placement goes through `World::place_device / use_device / lay_wire / pick_up`, which also check reach.
* **Save/load**: burning fuses and lit TNT keep burning after a load, and the death count is kept (optional trailing section, so older v3 saves still load). Loading a save where the mole is far below the burrow no longer counts as a fall: it used to kill the mole on load.
* Harness: `device`, `use`, `pickup`, `wire`, `saveload` directives; `dev:`, `lit:`, `on:`, `wire:`, `ev:` variables. Action events (clicks, places) are no longer dropped. New scenarios: `tnt_inert`, `tnt_lever`, `tnt_thrum`, `tnt_defuse`, `tnt_douse`, `saveload_devices`. New tests: `load_deep_does_not_fall_damage`, `wire_and_device_pack_accounting`, `replaced_fuse_is_not_burning`.
* **Doors never shut on the mole**: a door (or floodgate) told to close waits while a mole stands in its block, then closes once it is clear. A door can't be placed on yourself. Scenario `door`.
* Knots audited with a truth-table scenario (`knots`: both, either, turn, odd, catch). They were correct.
* **Pressure plates see the whole mole**: a plate used to check only the mole's feet cell, so a plate just above the floor never triggered (depending on block alignment). It now uses the same body overlap as doors. Scenario `plate`.
* **Lamp and Vent pack items place their device** on left click (they used to do nothing in hand).
* **Blocks built on sand or rubble stand** (`stress.rs`): placement allows resting on powder, but the load path only ran through static solids, so a pillar on a sand floor cracked and crumbled. The path may now climb straight up through granular ground (never sideways). Scenario `pillar_on_sand`.

## Open
* Placing TNT onto a copperline that is already live sets it off on the next tick. That is consistent (it is powered), but easy to do by accident.
