# Transport

The deeper the holt goes, the further its Delvers haul, so getting about is a ladder of machines you build. They run on weight, slope, friction and current, like everything else (see [Lore](Lore.md): nothing supernatural, nothing modern). The code is `engine/sim/src/transport.rs` (carts), `lifts.rs` (ladders and lifts) and `tubes.rs` (pressure tubes).

| tier | machine | state |
|---|---|---|
| 2 | **Rails and mine carts** | **built** |
| 4 | **Powered rails** (copperline along the track) | **built** |
| 1 | **Rope ladders** | **built** |
| 3–4 | **Lifts** (counterweight, or wound by current) | **built** |
| 3+ | **Pressure tubes** (the top of the ladder for now) | **built** |

**Hover tips.** Rest the cursor on a machine and its tip says what it's doing and the one or two inputs that matter (`tips::transport_tip`, test `transport_hover_tips_say_what_to_do`):
* **Cart:** get in or push it; how much cargo it carries (`E` unloads it into the pack, and the log says what came: "Unloaded the cart: 6 stone, 3 copper ore.", scenario `holt_home`), or that it's empty (throw things in, `E` picks it up); off the rails. While riding, how to jump out.
* **Rope ladder:** climb, or let go while on it; `E` takes it down.
* **Lift:** the counterweight and cage in kg and whether it rises or sinks, or that current winds it; stone on or off.
* **Tube hopper:** blowing (drop items in to send, ride with an empty paw) or receiving only; which hopper it sends to.
* **Track and pipe:** whether the track is powered; that a line needs hoppers.

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
* **Hanging one.** Left mouse in a drop hangs a ladder from the nearest rock above: a ceiling over it, or the lip of a ledge beside it (the rope tied on the ledge's top, hanging down its face). It unrolls straight down until it meets the floor, at most 8 m.
* **Climbing.** Over a ladder, `W` grabs on at once (it climbs instead of jumping) and `W` / `S` climb: 3 m/s up and 3.5 m/s down, a little faster than a bare wall (2.6 m/s). Let go of the keys and you hang still: there's no slide as there is on a wall. Grabbing pulls you onto the rope, clear of the rock face it hangs over, and a grab while falling past it catches at once. Climbing down is never a fall: a long ladder costs no fall damage at the foot (`rope_ladder_down`).
* **Getting on and off.** At the head of a ledge's ladder, standing on the ledge, `S` steps onto the rope and climbs down (it doesn't dig the ledge it's tied to). Hold `W` to the top and you climb off onto the ledge by yourself, like rolling over a wall's top; with nowhere to step off, you hang at the rope's top end rather than bob off it (`rope_ladder_top_out`, `site_mine_traverse`). `A` / `D` alone lets go at once and you drop or walk off at your own speed; a jump lets go and counts as a jump from the ground (`rope_ladder_let_go`, `rope_ladder_climb`).
* **It hangs from its rock.** Dig out the rock it's tied to and the ladder falls as a loose item (`rope_ladder_falls`). A floor that rises under it (sand, a block) shortens it. `E` takes it down.

## Lifts

A **lift** is a cage on a rope over a head frame, balanced by a counterweight. Left mouse in a shaft hangs the head under the shaft's ceiling, as far up as its 32 m rope reaches. The 1.5 m cage starts at the shaft's floor. The cage floor is a platform: the mole stands on it and rides with it, and loose items lie on it.

| | value |
|---|---|
| cage | 60 kg, plus 30 kg with a mole on it and 5 kg per item lying on it |
| counterweight | 40 kg frame plus 10 kg per stone of ballast |
| balance | a = g·(counterweight − cage) / (counterweight + cage), held to 2 m/s by the head's brake |
| winch | wound by current: up at 1.5 m/s whatever it carries |

* **Balanced by hand.** Right mouse on the head or the cage with stone in the paw adds a stone of ballast. Right mouse with anything else takes one back into the pack. The cage rises while the counterweight is the heavier, and sinks while it isn't. A mole on the cage (90 kg together) needs six stones (100 kg) to go up. Take them back off at the top and it comes down again (`lift_counterweight`).
* **Driven by power.** Copperline at the head carrying current turns the winch and winds the cage up, with no ballast needed. Cut the current and the balance takes over: an unballasted cage sinks back down (`lift_powered`).
* **The shaft must be clear.** Anything in the way (a cave-in, a block) stops the cage there. If the ceiling the head hangs from goes, the head comes down: the lift and its ballast fall as loose items. `E` on the head takes the lift down, ballast and all.
* Ladders, lifts and their ballast are saved.

## Pressure tubes

A sealed pipe network that blows capsules along it with air.
* **Pipe.** Pressure tube is a built block: iron-banded glass pipe (`Mat::Tube`, flag `TUBE`). Lay it with the block brush (`X` snaps straight runs). Tube cells that touch edge to edge are one line, and like any built block it needs support.
* **Hoppers.** A tube hopper is a 1 m mouth with a blower in it. Left mouse sets it on the floor or pipe under the cursor, and it must touch a tube within a tile of its mouth. Its blower runs while copperline in its block carries current.
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
| Forge | 8 rail | 1 copper ingot + 1 wood | 2 |
| Forge | mine cart | 2 copper ingots + 4 wood | 2 |
| Workshop | lift | 2 iron ingots + 4 wood + 1 copper ingot | 3 |
| Forge | 8 pressure tube | 1 iron ingot + 2 glass | 3+ |
| Workshop | tube hopper | 2 iron ingots + 2 copper ingots + 1 glass | 3+ |

Rails and carts need no iron, so they come before the drill. The lift and the tubes need iron and the workshop, so they come after the drill (test `transport_is_reachable_in_survival`). Creative has 999 of each (`creative_has_transport_freely`).
