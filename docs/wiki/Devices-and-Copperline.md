# Devices and Copperline

> Code: `engine/sim/src/logic.rs` (nets, devices, fuses, thrumming), `place_device`, `use_device`, `lay_wire` and `pick_up` in `world.rs`.

Delvers have no aerials and no chips. They have **copper drawn into wire**, **knots** that click over when current reaches them, and **thrummers** that knock on the rock. This page covers how those pieces behave and how to build with them.

## The block layer

Copperline and devices don't live in cells. They live on a coarser grid of **1 m blocks (8 × 8 cells, `logic::B = CPM`)** laid over the world. Cell-count thresholds below are in tiles (2 × 2 cells each) and scale with the block's area. Each block holds at most one copperline and one device. The spawn cell is block-aligned.

* **Laying copperline:** hold left mouse with copperline in hand; it is laid one block at a time as you drag.
* **Placing a device:** left mouse (or right mouse on an empty spot) puts it in the block under the cursor (it costs its pack item; levers, plates, sensors, doors, floodgates, pumps and fuses are free). Placing a knot on copperline gives the copperline back.
* **Interacting:** right mouse flips a lever, steps a thrum pattern (8 patterns) or a ticker's beat (20, 40 or 80 ticks), and lights a fuse cord or TNT.
* **Picking up:** `E` takes the hovered device back into the pack (with its cost), then, on the next press or pass, the copperline under it, one segment at a time; hold `E` and sweep to take up a run. A lit charge or a burning fuse cord can't be picked up. A device's state is reset when it is placed or removed (scenarios `pickup_device`, `pickup_wire`, `pickup_tnt`).
* **Damage:** lava in a block (2 or more cells) or burial under 10 or more loose cells destroys its copperline and device, with a message in the log. Doors, floodgates and plates are exempt; a buried charge stays put, and lava lights it.

## Nets

Copperline blocks that touch along an edge form a **net**, found with union-find whenever the wiring changes. A net is **live** if any source next to it drives it. A device is **powered** if a live net runs through its own block or any of its four neighbours.

```
 [lever]─═══─═══─[both]═══─[lamp]
                   ║
 [plate]─═══─═══───╝
```

## Sources

| device | sends current when… |
|---|---|
| Lever | flipped on (right mouse) |
| Pressure plate | a mole's body overlaps its block, or 3 or more loose grains rest in it |
| Fluid sensor | 4 or more cells of water or lava are in its block |
| Listener | it hears a thrummer knocking the same pattern (see below) |
| Heatstone | lava touches its block or the blocks around it (2 or more cells): a thermoelement |
| Water wheel | 2 or more water cells moved through its block this tick |

Heatstones and water wheels are the only power plants, as the lore says: heat from lava and flowing water.

## Knots

A knot reads two inputs, the copperline block to its **west** (`a`) and the one to its **north** (`b`), and drives the copperline block to its **east**. It switches **one tick** after its inputs change, so loops and clocks behave predictably.

| knot | out | |
|---|---|---|
| Both-knot | `a AND b` | |
| Either-knot | `a OR b` | |
| Turn-knot | `NOT a` | |
| Odd-knot | `a XOR b` | exactly one |
| Catch-knot | latch: `a` sets it, `b` lets it go | memory |
| Ticker | pulses on for half its beat, off for the other half | while `a` is live, or always if nothing is wired to its west |

The truth tables are checked by the `knots.scn` scenario, which swaps each knot into the same circuit.

## Sinks

| device | while powered |
|---|---|
| Door | **Opens.** Unpowered, its block fills with door cells (loose material in the way is pushed up). It never shuts on a mole: it waits until the doorway is clear. You can't place a door on yourself |
| Floodgate | **Shuts** its block with gate cells, and waits for the doorway to clear before shutting. Unpowered, it stands open |
| Pump | Lifts up to 3 liquid cells a tick from its block, up through as much as 16 blocks of rock, to the first open block above |
| Lamp | Lights |
| TNT | Goes off at once |
| Thrummer | Knocks its pattern into the rock |

A **vent** isn't wired: it draws stale air out of the tunnels around it (see [Physics](Physics.md#stale-air)).

## TNT and fuse cords

TNT has two uses. **Set down** (left mouse, or right mouse on an empty spot) it goes down **inert**: nothing explodes where you click, and `E` picks it back up while it is unlit. It isn't set down on or beside a copperline that is already **live**, since it would go off on the next tick: the place is refused with "Can't place TNT: the copperline is live (cut the power first)." Cut the power, set the charge down, then power the line to set it off (scenario `tnt_live_wire_refused`). **Thrown** (`Q`) it goes out lit (see [Tools and Items](Tools-and-Items.md#throwing)); scenario `tnt_place_vs_throw`. A charge set down can be set off in several ways:

* **Lit:** right mouse, fire or lava starts a **1.5 s fuse** (90 ticks; the charge blinks). Water (4 or more cells in the block) puts the fuse out. Once lit, it can't be picked up.
* **At once:** current on a neighbouring copperline (from a lever, a knot or a listener, which gives remote detonation by thrumming), a blast nearby, or a fuse cord burning into it.

**Fuse cords** burn for 24 ticks each, then light the next cord or set off the next charge, so a trail of cords is a slow, visible timer.

**Powered rail:** copperline laid along track drives a mine cart on while it carries current, so any source or knot switches a stretch of line (see [Transport](Transport.md#powered-rails)).

**The blast** is a pressure wave from a 0.5 kg charge (see [Physics](Physics.md#blasts)). It reaches much further down a tunnel than across a cave, a wall shadows what's behind it, and rock breaks where the wave beats its strength: dirt and clay go readily, and one charge on a tunnel floor in limestone or sandstone blows a crater through the floor and into the roof and throws its rubble metres along the tunnel. Tamped into a pocket it blows a round crater about 5 m across; in a big cave it does much less. Granite gives a smaller crater (about 0.75 m into the floor, more when tamped), and basalt and iron beams hardly mark. Loose ground and water are thrown, items fly, and a mole takes damage from the overpressure where it stands. Another charge in a block hit by 130 kPa or more goes off with it (`tnt_chain.scn`), a fuse cord at 60 kPa lights, and copperline and other devices are wrecked above 400 kPa. Listeners in reach through the rock hear the blast's knock on every channel for half a second.

## Thrumming

Thrumming carries a signal through solid rock with no line between the ends.

| device | role |
|---|---|
| Thrummer | knocks one of 8 patterns while its copperline is live |
| Listener | live while it hears a thrummer on the same pattern |
| Relay thrummer | hears any knock that reaches it and passes it on |

Right mouse steps a thrum device's pattern. All three cost a thrummer from the pack.

**Links.** Every 15 ticks (or whenever something changes) each pair of thrum devices gets a **link budget**: 60 dB minus spreading, absorption and boundary losses along the straight line between their centres. The cells of their own blocks are skipped, because the devices are braced against them. A pair is linked while the budget is 0 or more. A knock spreads from each live thrummer through the links, hopping on through relays, and lights every listener on its pattern. The wave model is on [Physics](Physics.md#thrumming).

In practice:

* **Solid granite and basalt** carry a knock tens of metres.
* **Dirt** muffles it within a few metres, and **loose rubble** is worse.
* **Water** dampens it.
* **An open tunnel** between the ends costs about 30 dB at each rock face, so a knock usually can't cross a cave. Go around it, or put a relay on each side.

`tnt_thrum.scn` checks the classic use: a lever drives a thrummer, the knock goes through 2 m of granite to a listener, and the listener's copperline sets off a charge.

## How it looks

Copperline is drawn per 1 m block as a 3-pixel copper wire with a bright top edge and a shaded underside. It bends to join the blocks around it, ends in a brass stud and gets a solder blob where it branches. Live copperline glows warm, and a glint runs outward along it. Devices have their own pixel art on wood or iron mounts, with on/off states: a lever thrown left or right, a plate that sinks when pressed, a lamp's glass lit, a pump and vent turning, a thrummer's mallet knocking, a listener's trumpet ringing, a water wheel turning, a fuse cord or TNT spark. Knots are brass plates with their mark (both: two rings, either: a fork, turn: a loop, odd: a cross, catch: a hook, ticker: a pendulum), copper stubs on the west and north inputs, and an output stub on the east that lights. Thrum devices show a small pip in their pattern's colour.

## Wiring view

`V` switches to a schematic of the logic layer, and `V` again switches back to the pixel art. The world dims, a thin pale-blue border and a small `WIRING` label frame the screen, and the cursor becomes a crosshair, so it's always clear which view you're in. Holding copperline, a knot, a thrummer, a lamp or TNT shows a small "V wiring view" hint.

* **Copperline:** a thick line per block. Steel blue when idle; amber when carrying current, with a pulse running along the net.
* **Devices:** a framed glyph coloured by role. Sources (lever, plate, sensor, listener, heatstone, water wheel) are green, knots violet, sinks (door, floodgate, pump, lamp, TNT, fuse, vent) orange, and thrummers take their pattern's colour. Bright when on, dim when off.
* **Knots:** arrows on the west and north edges show the inputs (amber when live), an arrow on the east the output, and the knot's name (BOTH, EITHER, TURN, ODD, CATCH, TICKER) sits above.
* **Thrumming:** each thrummer, relay and listener is labelled with its pattern (CH 1–8). A line joins every linked pair in the sender's colour. A faint ring around each thrummer shows its reach through the rock it's braced in. It counts spreading and absorption only, since boundaries cost more, so the link lines are the truth.

## Save and load

Copperline, devices and their state (thrown levers, lit lamps, burning fuses and lit charges, patterns and beats) survive save and load (`saveload_devices.scn`), and survive the window sliding away from them and back (`band_keeps_devices.scn`).
