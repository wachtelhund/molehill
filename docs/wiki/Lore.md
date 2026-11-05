# Lore

> This page mirrors [`LORE.md`](../../LORE.md), which is binding. Every feature in the game must follow from it. If a feature contradicts the lore, the feature changes, not the lore. If this page and `LORE.md` disagree, `LORE.md` wins.

## The world: the Deepholt

* There is no surface. The moles live in the Deepholt, an endless body of earth in every direction. Nobody has ever found a top or a bottom.
* "The Lid" is an old mole myth about an open sky far above. There is no proof it exists, and the game never shows it.
* The layers follow a simple rule: higher up it is cooler and wetter, so there are more water pockets. Further down it is hotter and drier, so there is more lava and harder stone.
* Nothing is supernatural. Everything follows physics: weight, heat, water, pressure and electric current.

## The moles

* The moles are **Delvers**, practical people who dig, build and tinker. They live in colonies called **holts**.
* They see poorly and rely on light, touch and vibration.
* Their dangers are cave-ins, flooding, lava, stale air and hunger.

## Technology: simple and handmade

* Energy comes from nature. Heat from lava is turned into current with **heatstones**, which are thermoelements. Water flow can drive **wheels**.
* Copper is drawn into **copperline** (wire) that carries current.
* Logic is built from mechanical-electric **knots**: the both-knot (AND), the either-knot (OR) and the turn-knot (NOT), plus levers, switches and pressure plates.
* Signalling without a line is called **thrumming**. A thrummer knocks a pattern into the rock, and a listener picks it up somewhere else. The knock weakens with distance, and materials carry it differently: stone carries well, loose dirt muffles and water dampens. There are no aerials and no modern words for any of it.

## Naming rules

* Use simple, earthy, descriptive words: copperline, thrummer, heatstone.
* No brand names and no modern technical terms. Nothing supernatural, and no fantasy races other than moles and underground animals.

## Grottos
- Grottos are rare hollows where water seeps through the rock. Cave trees, called root-trees, grow there. They drink the seep and the minerals in it, and they need no light.
- Glowing mycelium, damp moss and small pools fill a grotto. Glowmites drift in the air: small insects that make their own glow, like glow-worms. Delvers catch them in jars for lanterns.
- Grottos follow the layers: they are more common in the wet upper layers and rarer deeper down.
- A root-tree cut down but left with its roots grows back slowly. A grotto that is clear-cut or drained is gone for good, so a holt looks after the one it lives by.

## The Ancients
- Long ago there were Ancient Delvers. They were moles like any other, who dug deeper and built finer than today's holts.
- They left facilities, ruins and relics. Their relics are better-made versions of things Delvers know. Today's Delvers can't make them again, because the know-how is lost.
- Everything the Ancients built follows the same physics as everything else: weight, heat, water, pressure and current.
- Their ruins hold carvings and knocked patterns that seem to point toward the Lid. Nothing proves the Lid exists, and the game never shows it.

## How the lore shapes the game

The lore is a design constraint as much as a story. Here is where it shows up in the code:

| lore | in the game |
|---|---|
| No sky, no surface | The world is a sliding window over an infinite column (see [World and Generation](World-and-Generation.md)). There is no daylight: `World::step` pins `day` to 0, and all light comes from emitters and lamps. |
| Cooler and wetter up, hotter and drier down | Fluid pockets choose water or lava by depth: the lava share rises from 0 at 10 m to 92 % at 70 m. Stone hardens from dirt and clay to limestone, granite and basalt. |
| Everything follows physics | Roofs and pillars are checked with beam bending and compression from a material table in kg/m³ and kPa (see [Physics](Physics.md)). |
| Delvers see poorly | Fog of war, a dark palette, and tunnels that stay black until you light them. Remembered rock is drawn deliberately dim. A Delver carries its own handmade light: a fungus-oil lamp on its hat, whose beam reaches 12 m and is stopped by rock. |
| Dangers | Cave-ins (structural failure), flooding, lava, stale air (CO2 on a block grid) and hunger are all simulated. |
| Handmade tech | Every device has a plain name: lever, both-knot, ticker, catch-knot, floodgate, listener, relay thrummer. Tooltips are checked by a test (`every_thing_has_a_lore_safe_tooltip`). |
| Thrumming | Modelled as an elastic wave through the actual cells between thrummer and listener (see [Devices and Copperline](Devices-and-Copperline.md#thrumming)). |

## Glossary

| word | meaning |
|---|---|
| Deepholt | The endless earth the moles live in |
| Delver | A mole of the Deepholt |
| holt | A mole colony; a new game is a "New Holt" |
| the Lid | A myth about a sky above; never shown |
| copperline | Copper wire that carries current |
| knot | A mechanical-electric logic piece: both, either, turn, odd, catch |
| ticker | A knot that pulses on and off by itself |
| heatstone | A thermoelement that makes current while lava touches it |
| water wheel | Makes current while water flows through it |
| thrummer / listener / relay thrummer | Knock a pattern into the rock, hear it, and pass it on |
| pack | Your inventory |
| grotto | A rare underground oasis around seeping water: root-trees, mycelium, moss, pools |
| root-tree | A cave tree that drinks seeping water; it grows back from its roots |
| glowmite | A small insect that makes its own glow; caught in a jar, it lights a lantern |
| the Ancients | Delvers of long ago, who left facilities, ruins and relics |
