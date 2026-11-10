# Transport

The deeper the holt goes, the further its Delvers haul, so getting about is a ladder of machines you build. They run on weight, slope, friction and current, like everything else (see [Lore](Lore.md): nothing supernatural, nothing modern). The code is `engine/sim/src/transport.rs` (carts), `lifts.rs` (ladders and lifts), `ropes.rs` (the rope gun) and `tubes.rs` (pressure tubes).

| tier | machine | state |
|---|---|---|
| 2 | **Rails and mine carts** | **built** |
| 4 | **Powered rails** (copperline along the track) | **built** |
| 1 | **Rope ladders** | **built** |
| 1 | **Rope gun** (the way up: there is no wall climbing) | **built** |
| 3–4 | **Lifts** (hauled on the walking keys, balanced with stone, or wound by current) | **built** |
| 3+ | **Pressure tubes** (the top of the ladder for now) | **built** |

**Hover tips.** Rest the cursor on a machine and its tip says what it's doing and the one or two inputs that matter (`tips::transport_tip`, test `transport_hover_tips_say_what_to_do`):
* **Cart:** get in or push it; how much cargo it carries (`E` unloads it into the pack, and the log says what came: "Unloaded the cart: 6 stone, 3 copper ore.", scenario `holt_home`), or that it's empty (throw things in, `E` picks it up); off the rails. While riding, how to jump out.
* **Rope ladder:** grab on and climb, or how to let go while on it; `E` takes it down.
* **Lift:** how to ride and call it; which stop it's at (top, a passage or the shaft floor) or that it's on its way; balanced, light enough to haul, or too heavy and by how much (or that current hauls any load); right mouse balances it, `E` on the head takes it down. With a lift or rope ladder **in hand**, the tip says at once what it would do at the cursor (a 7 m shaft, a cage 1.9 m wide, 4 stops; a ladder tied on the ledge hanging 4.5 m) or why it can't go there (`tips::place_tip`).
* **Tube hopper:** blowing (drop items in to send, ride with an empty paw) or receiving only; which hopper it sends to.
* **Track and pipe:** whether the track is powered; that a line needs hoppers.

Carts also tip their load at a **bumper**, and ore hoppers fill them: see [Production](Production.md). The **steam borer**, the deep's machine, is steered through the rock like a worm in every direction, boring its own round tunnel, with a train of wagons following its path as body segments (store, workshop, bunk, coal; the borer runs on coal alone): see [Production: steam borer](Production.md#steam-borer).

The Ancients' tube stations and waygates in the ruins stay broken: they are endgame finds, not working lines (see [World and Generation](World-and-Generation.md#ruins-of-the-ancients)).

## Rails

**Rail** is a built block: copper straps on wooden sleepers (`Mat::Rail`, flag `TRACK` in the material table).
* Lay it like any block, with left mouse and the block brush. `X` snaps it to straight runs.
* Lay it on a floor. A slope is a stair of rail: a cart's wheels ride over a step of up to two cells.
* It needs support like any other built block. A span laid over a drop with nothing under it breaks and falls as loose rail, while the ends resting on rock hold (`rail_needs_support`).
* One rail item lays 2 m of single-cell track.

## Mine carts

A **mine cart** is its own body, 1 m long and 0.5 m tall, that sits on the track under its middle and follows the track's height.

| | value |
|---|---|
| gravity along the track | g·sinθ, with the real g = 9.81 m/s² |
| rolling friction | 0.1 × g·cosθ: a cart let go at walking pace on the level rolls a few metres |
| mass | 80 kg empty, 5 kg per item of cargo, 30 kg with a mole aboard |
| the mole's push | 320 N: an empty cart reaches walking pace in about a metre; a full one takes about three times as long |
| top speed | 10 m/s |
| cargo | up to 40 items |

* **Downhill it runs on its own.** Set on a 1-in-4 slope it picks up speed (about 1.4 m/s² less friction), runs out onto the level at about 4.7 m/s, and slows (`cart_rolls_downhill`). A slope too gentle to beat rolling friction holds it still.
* **The end of the track is a bumper.** A cart stops dead there with a little bounce back and stays on the rails.
* **Pushing.** Walk into a cart to lean on it. The push is a force on the cart's mass, and the mole is held at its side, so a loaded cart slows you to its pace (`cart_push_level`, `cart_load_heavier`). Let go and it rolls on until friction stops it.
* **Riding.** Right mouse on a cart climbs in, as does dropping into it from above. You're carried with it, `A`/`D` lean on it a little, and `Space` jumps out (`cart_ride`).
* **Cargo.** Loose items that fall or are thrown into a cart are its cargo. A lit charge isn't taken in: it rides along and goes off. `E` on a cart takes its cargo into the pack, and `E` on an empty cart nobody is in picks the cart up (`cart_cargo`).
* **Off the rails.** If the track under it goes, a cart drops and sits with its wheels in the ground. It won't roll until it's set back on track (`cart_off_rail`).
* **Carts meet.** Two carts that run into each other trade speeds by their masses, like billiard balls, losing a little to the knock.
* **A slope too steep to push.** The mole leans in with 320 N; a 1-in-3 climb needs about 330 N for an empty cart (gravity along it plus rolling friction). The run-up carries a cart a step or two up, and there it stalls (`rail_too_steep_to_push`).
* Carts and their cargo are saved. A cart that scrolls out of the live window with the rows around it is lost, as loose items are.

## Powered rails

There is no separate powered rail: **copperline laid along the track** (in the same 1 m blocks, see [Devices and Copperline](Devices-and-Copperline.md)) drives a cart while its line carries current.
* The motor pulls the cart toward 5 m/s the way it's running (or last ran), with up to 6 m/s² of pull. That climbs a 1-in-2 slope at a crawl and a 1-in-4 at speed, where plain track would hold the cart at the bottom.
* Whatever powers the line switches the stretch: a lever, a pressure plate, a knot, a listener, a heatstone or a water wheel. A powered line can be automated with the same logic as doors and pumps. Without current it's plain track.
* At the end of a powered stretch the cart bounces off the bumper and is driven back the other way, so a powered line between two bumpers shuttles on its own.
* Laying the copperline up a slope: a cart's block must carry a live segment, and segments join edge to edge, so a slope's copperline is laid as a staircase of 1 m blocks.

`powered_rail_climbs`: a cart at the foot of a 1-in-3 climb sits still on dead track. The lever drives it up, along the top, into the bumper and back down. Lever off while it runs down, and it coasts to the foot and stops.

## Rope ladders

Cheap vertical access that needs no power.
* **Hanging one.** With the ladder in hand, a ghost shows where it would hang: the whole rope to the floor with its rungs every 0.5 m, and the knot. It's pale where it fits, amber when out of reach, and a red cross with the tip saying why where it can't go. It snaps: a ledge's lip within 1 m beside the cursor ties it on there, hanging against the ledge's face (not out in the air where the cursor was), else it hangs from the rock right above. It unrolls to the floor, at most 8 m (`ladder_place_snap`). A ladder that falls drops where a mole would hang on it, clear of the face.
* **Climbing.** Over a ladder, `W` grabs on at once (it climbs instead of jumping) and `W` / `S` climb: 3 m/s up and 3.5 m/s down, faster than a rope-gun rope (2 m/s): it has rungs. Let go of the keys and you hang still. Grabbing pulls you onto the rope, clear of the rock face it hangs over, and a grab while falling past it catches at once. Climbing down is never a fall: a long ladder costs no fall damage at the foot (`rope_ladder_down`).
* **Getting on and off.** At the head of a ledge's ladder, standing on the ledge, `S` steps onto the rope and climbs down (it doesn't dig the ledge it's tied to). Hold `W` to the top and you climb off onto the ledge by yourself, like rolling over a ledge from a jump; with nowhere to step off, you hang at the rope's top end rather than bob off it (`rope_ladder_top_out`, `site_mine_traverse`). `A` / `D` with `W` / `S` held move you along the rope at 2 m/s (to step off into a side passage, say), `A` / `D` alone lets go at once and you drop or walk off at your own speed; a jump lets go and counts as a jump from the ground (`rope_ladder_let_go`, `rope_ladder_climb`).
* **It hangs from its rock.** Dig out the rock it's tied to and the ladder falls as a loose item (`rope_ladder_falls`). A floor that rises under it (sand, a block) shortens it. `E` takes it down.

## Rope gun

The way up a face: the mole **can't climb bare walls** (scenario `no_wall_climb`). A jump rolls over a ledge up to about 2 m (`TUNE.ledge`, scenario `mantle`); higher, hang a rope ladder from above, or throw a rope up with the rope gun. Code: `ropes.rs`.
* **Making it.** Workbench: the gun from 2 wood, 3 stone and 2 fungus (a crank-wound spring in a root-wood stock); a hook rope from 3 fungus and 1 stone (a stone hook on 16 m of fungus-fibre rope; reusable gear, priced like a rope ladder). It's a first-hour make: wood, stone and fungus are all to hand by the workbench quest (`first_hour_without_wallclimb`).
* **Throwing.** Left mouse with the gun in hand throws the hook at the cursor, however far, and uses one hook rope. It flies at 20 m/s and falls at 9.81 m/s², paying the rope out behind it.
* **Biting.** The hook bites **solid, settled rock or any built block** and holds there for good. It glances off sand, gravel and loose rubble (spent: it bites nothing after), and sinks in water or lava; then the hook rope falls where the hook stopped, to be picked up again. Past 16 m from the gun the rope runs out and the hook drops short (`rope_gun_anchor_stone`).
* **The rope** is a chain of 32 half-metre segments moved by verlet: it hangs, goes slack, lies over the rock it touches and swings with the mole on it. Grains falling past don't hold it up. It is saved with the world (transport save v5).
* **Climbing.** `W` with the body within 0.5 m of the rope grabs it (from the floor a fresh press, so `W` held as you step off at a top doesn't grab again). `W` / `S` climb at 2 m/s up and 3 m/s down. `A` / `D` swing: the mole is a pendulum under the hook, and a push (16 m/s²) only goes the way the swing already goes, so holding a key builds the swing up. Pressing toward a ledge whose top is within 0.75 m of the feet steps off onto it. `Space` lets go with a hop and the swing's speed (the pumpworks route swings over its 9 m wheel this way: `site_pumpworks_traverse`); climbing down onto the floor lets go. `W` held while falling past a rope catches it (`rope_gun_catch`). A rope is never a fall (`rope_gun_climb`).
* **Found places it gates (on purpose).** The buried cargo halt's crate stacks (4.25 m), the fossil cave's mound face (2.2 m) and the flooded pumpworks' wheel (9 m) are beyond a jump: the rope gun is the way over them, and no ladder or steps are added. The gun opens the gate in the first minutes (workbench, gun and a hook rope cost only wood, stone and fungus, the wood within the Survival kit), and the walk-throughs reach each chest with it (`site_halt_traverse`, `site_fossil_over_the_beast`, `site_pumpworks_rope_to_chest`; test `rope_gun_comes_before_the_faces_that_need_it`).
* **It hangs from its rock.** Blow or dig away the rock the hook bit and the rope comes down as a hook rope item (`rope_gun_anchor_stone`). `E` on any part of a rope takes it back into the pack as a hook rope. A hook still in the air when the world window slides goes back into the pack (`rope_gun_slide`); a fixed rope the window leaves waits for it and hangs there again when the window comes back, like a rope ladder or a lift (see Lifts, below). Climbing or swinging past the window's margin slides it mid-climb, up, down or sideways: the rope moves with the rock and the mole keeps its hold (tests `a_window_slide_keeps_the_mole_on_its_rope`, `a_sideways_window_slide_keeps_the_mole_on_its_rope`).
* **Riding lets go.** Getting into a cart, onto a borer or into a wagon from a rope lets go of it, so jumping out later is a plain hop, not a yank back toward the hook (test `riding_from_a_rope_lets_go_of_it`).

| | value |
|---|---|
| rope | 16 m (a big hall's ceiling from its floor), 32 segments of 0.5 m |
| hook | 20 m/s, falls at 9.81 m/s²; bites solid rock and blocks only |
| climb | 2 m/s up, 3 m/s down; swing push 16 m/s² |
| cost | gun: 2 wood + 3 stone + 2 fungus; hook rope: 3 fungus + 1 stone (workbench) |

## Lifts

A **lift** is a cage on a rope over a head frame, balanced by a counterweight. Code: `lifts.rs`.
* **Placing.** With the lift in hand, the cursor anywhere in a shaft shows a ghost of where it goes:
  * The cage is built to the shaft's width (1.5 to 3 m, an odd number of cells, centred). The width is the narrowest open run through the cursor's column within 2 m, so a cursor level with a side passage still finds the shaft. In a cave wider than 3 m, a 3 m cage is set under the cursor.
  * The head frame hangs under the ceiling above it, as far up as the 32 m rope reaches.
  * The ghost shows the cage at the foot, a mark at every stop, and the counterweight up top. It's pale where the lift fits and amber when it's out of reach. Where it can't go, a red cross shows and the tip says why ("the shaft is too narrow for the cage (1.5 m)", "needs a ceiling to hang from"). `tips::place_tip`, scenario `lift_ride_stops`.
* **Stops.** The cage stops at three kinds of place, marked in brass on the shaft's side:
  * the top, with room for a mole under the head;
  * every side opening whose floor meets the cage's side within 0.25 m (a passage off the shaft; stops closer than 1 m count as one);
  * the shaft's floor.

  The stops follow the rock: dig a passage off the shaft and it becomes one. Between rides the head's brake holds the cage still.
* **Riding.** The same keys as walking. On the cage, a tap on `W` / `S` hauls the hand rope to the next stop up or down, and the cage stops there, level with its floor. Held, it runs on through each stop without slowing (`lift_ride_stops`). On the cage, `W` doesn't jump and `S` doesn't dig.
* **Calling.** Standing at a stop beside the shaft (within 1 m of the cage's side) or on its floor, a fresh `W` or `S` hauls the cage to you (`lift_call`). A cage coming down stops on a whole row over the head of a mole standing under it. `W` there climbs up through the cage's floor hatch onto it, so a mole that drops into a shaft with no way out at the foot can still call the cage and ride (`lift_from_under`).
* **Balance.** The mole's arms haul up to 60 kg out of balance either way. That covers the bare 40 kg counterweight against the 60 kg cage and a 30 kg rider (50 kg), plus a couple of things besides.
  * A heavier load is refused: "Too heavy to haul … right mouse on the lift balances it", and the brake holds.
  * **Right mouse on the head or the cage balances it in one go.** It puts stone from the pack on the counterweight, or hands spare stone back, until the weight matches what's on the cage now, a rider counted (at the least the cage and a rider). It then says how it stands ("Balanced: counterweight 110 kg against the cage's 110 kg", or how many more stones it would need) (`lift_balance`).
  * A **needle** on the head frame leans toward the heavier side. It is green when balanced, amber while still haulable, and red when the mole can't haul it.
* **Driven by power.** Copperline carrying current anywhere on the head frame turns the winch. On the same keys and calls, the winch hauls any load at 1.5 m/s. Cut the current and the arms' limit applies again (`lift_powered`).

| | value |
|---|---|
| cage | 60 kg, plus 30 kg with a mole on it and 5 kg per item lying on it; the shaft's width, 1.5–3 m |
| counterweight | 40 kg frame plus 10 kg per stone of ballast |
| haul by hand | up to 60 kg out of balance; 2 m/s, speeding up and slowing at 4 m/s² to stop level |
| winch | current at the head: any load at 1.5 m/s |

* **The shaft must be clear.** Anything in the way (a cave-in, a block) stops the cage there. If the ceiling the head hangs from goes, the head comes down, and the lift and its ballast fall as loose items. `E` on the head takes the lift down, ballast and all.
* Ladders, lifts and their ballast are saved.
* **They stay where you built them.** A rope, a rope ladder or a lift that the world window leaves (a trip of 70 m or more down from it, a long way sideways, or dying deep and waking in the burrow) waits, and is back in place when the window covers the whole of it again: the home shaft keeps its ladders and its lift (`away.rs`; test `ropes_ladders_and_lifts_come_back_with_the_window`, scenario `ropeways_come_back`). They used to be lost with the rows they hung in. The ones waiting are saved too (transport blob 15). A lift comes back braked where its cage stood, with its ballast. A found place's own ladders wait too and come back once: a place hangs a ladder only where none hangs, so the one that waited is put back first (test `a_found_places_ladders_come_back_once`). A lift waits as a whole: with part of its shaft (up to 32 m) outside the window it isn't there, so a mole on a cage whose shaft the window leaves falls, as before. Carts, the borer and wagons that the window leaves are still lost.
* A window slide in the middle of a ride keeps the cage on its way to the stop it was hauled to (test `a_window_slide_keeps_a_lift_on_its_way_to_its_stop`; before, a slide up or down left the stop's row behind and the cage ran on to the end of the shaft).

## Pressure tubes

A sealed pipe network that blows capsules along it with air.
* **Pipe.** Pressure tube is a built block: iron-banded glass pipe (`Mat::Tube`, flag `TUBE`). Lay it with the block brush (`X` snaps straight runs). Tube cells that touch edge to edge are one line, and like any built block it needs support.
* **Hoppers.** A tube hopper is a 1 m mouth with a blower in it. Left mouse sets it on the floor or pipe under the cursor, and it must touch a tube within a tile of its mouth. It can't go inside an ore hopper's bin or legs (the two would hide each other from right mouse and `E`); its log lines say "tube hopper" so they aren't taken for the ore hopper's. Its blower runs while copperline in its block carries current.
* **Sending.** Loose items dropped or thrown into the mouth of a hopper whose blower runs are sealed in a capsule and blown along the pipe to that hopper's destination: the nearest other hopper on the line, and right mouse cycles through the others. They come out of the destination's mouth. A hopper without current only receives, so what arrives stays there.
* **Riding.** Right mouse with an empty paw on a hopper whose blower runs seals the mole in a capsule. It's carried along the pipe out of sight, unhurt by the rock the pipe runs through, and climbs out of the far hopper (`tube_rides_mole`).
* **Speed.** Capsules run at 12 m/s, up a riser as easily as along the flat (`tube_sends_items`).
* **Pressure.** A line holds pressure from any blowing hopper still joined to it. With none, capsules wait where they are. If the pipe ahead of a capsule is cut or caves in (a cell that's no longer standing tube), the capsule stops at the breach and spills there: its items fall out at the nearest open cell, and a riding mole crawls out (`tube_breach`).
* Hoppers are saved. A capsule in the pipe at a save comes out where it is: its items as loose items at the nearest open cell.
* The Ancients' tube stations in the ruins are not part of this. They stay broken.

## Making them

| station | makes | from | tier |
|---|---|---|---|
| Workbench | rope ladder | 2 wood + 2 fungus | 1 |
| Workbench | rope gun | 2 wood + 3 stone + 2 fungus | 1 |
| Workbench | hook rope | 3 fungus + 1 stone | 1 |
| Forge | 8 rail | 1 copper ingot + 1 wood | 2 |
| Forge | mine cart | 2 copper ingots + 4 wood | 2 |
| Workshop | lift | 2 iron ingots + 4 wood + 1 copper ingot | 3 |
| Forge | 8 pressure tube | 1 iron ingot + 2 glass | 3+ |
| Workshop | tube hopper | 2 iron ingots + 2 copper ingots + 1 glass | 3+ |

Rails and carts need no iron, so they come before the drill. The lift and the tubes need iron and the workshop, so they come after the drill (test `transport_is_reachable_in_survival`). Creative has 999 of each (`creative_has_transport_freely`).
