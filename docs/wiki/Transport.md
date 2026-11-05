# Transport

The deeper the holt goes, the further its Delvers haul, so getting about is a ladder of machines you build. They run on weight, slope, friction and current, like everything else (see [Lore](Lore.md): no magic, nothing modern). The code is `engine/sim/src/transport.rs`.

| tier | machine | state |
|---|---|---|
| 2 | **Rails and mine carts** | **built** |
| 4 | Powered rails (copperline along the track) | next |
| 2–4 | Rope ladders and lifts | to come |
| 5 | Pressure tubes | to come |

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
* Carts and their cargo are saved. A cart that scrolls out of the live window with the rows around it is lost, as loose items are.

## Making them

| station | makes | from |
|---|---|---|
| Forge | 8 rail | 1 copper ingot + 1 wood |
| Forge | mine cart | 2 copper ingots + 4 wood |

Both are tier 2 (copper): they need no iron, so they come before the drill (test `transport_is_reachable_in_survival`). Creative has 999 of each (`creative_has_transport_freely`).
