# Production

A mine's output moves as **real loose items**: every lump is a loose item in the world, in a cart, or in an ore hopper's bin (drawn as its heap). Nothing teleports and nothing is stored out of sight. Lumps get from the face to where they are wanted by falling, sliding and rolling, so a line can jam, spill and pile up. The code is `engine/sim/src/production.rs`; carts are in [Transport](Transport.md).

| stage | what | state |
|---|---|---|
| 1 | Chutes, shaker screens, bumpers (carts tip), ore hoppers that feed carts | **built** |
| 2 | Water sluicing: channels carry silt off, riffles catch heavy ore | design |
| 3 | Mechanical power: water wheels, shafts, gears, belts | design |
| 4 | Coal power: boiler and steam engine; ash and slag to haul away | design |
| 5 | Refinement: stamp mill, washer, bellows smelter, foundry | design |
| 6 | Non-electric logic: plates, tipping scales, float valves, gates | design |

## Chutes

**Chute** (`Mat::Chute`, flag `CHUTE`): a smooth timber trough, laid with the block brush like rail. A loose item lying on a chute slides down its slope: gravity along the slope (g·sinθ) against the chute's low friction (0.12 × g·cosθ). A 1-in-2 chute runs lumps off fast; a level one holds them. The slope is read like a cart reads its track, from the chute's top up to 1 m either side, so a chute built in steps reads as its overall run. The rock a chute is laid on, and the drop past its end, don't count. A chute is a built block and needs support like any other.

Scenario `chute_slides`: a stone slides down a 1-in-2 chute and out past its foot; on bare granite steps of the same slope, or on a level chute, it stays put.

## Shaker screens

**Shaker screen** (`Mat::Screen`, flags `CHUTE | SCREEN`): an iron grating on a timber frame. Anything finer than its mesh (5 cm, `MESH_M`, from `Item::size_m`) falls straight through it: sand, dirt, clay, gravel and pigment powder. Lumps ride over it: ore, coal, stone. The iron grips (friction 0.45), so lumps lie still on a gentle screen. **Copperline current** laid along it (in the same 1 m blocks) works its knocker. While it shakes, the grip drops to 15 % (`SHAKE_GRIP`), and the lumps walk down the slope and off the end. Lumps lying asleep on a screen wake up when it starts shaking.

Scenario `shaker_sorts`: gravel and copper ore dropped on a 1-in-4 screen. The gravel falls into the pit under it; the ore stays on top until a lever powers the line, then walks off into the far pit. Nothing ends up in the wrong pit.

## Bumpers and tipping

**Bumper** (`Mat::Bumper`, flag `BUMPER`): a timber buffer built at the end of a track. A **loaded** cart that runs into one **tips**. It is held against the buffer while its load comes out over the front, one item every 3 ticks, thrown forward at 3 m/s and up at 1 m/s. Then it rights itself, empty and still on the rails, and its running direction flips, so a powered line drives it back. If there is no room for a lump to come out (the buffer stands above the rim, or the spill has piled up), the load **jams** and stays in the cart ("The cart's load is jammed against the buffer."). An empty cart, or a plain track end with no bumper, just stops as before. A loaded cart standing against a bumper tips too: one eased up to it, or one loaded back from a save taken mid-tip (the tip itself isn't saved, so it simply starts again with what is left).

Scenario `cart_tips_at_bumper`: a cart with 20 stones rolls down a 1-in-4 slope into a bumper, tips, and all 20 land in the pit beyond. Scenario `cart_tip_resumes`: saved mid-tip, it finishes after loading.

## Ore hoppers

**Ore hopper** (`Item::OreHopper`): a 1.5 m wide, 1 m tall timber bin on legs. Its spout is 0.75 m over the floor, so a cart fits under it. Set it down over the track: both legs need a floor.
* **Filling:** loose items that fall into its mouth are its load, and so is loose sand and rubble poured in as cells (24 cells make one item, as when digging). It holds up to 60 items.
* **The gate:** a cart standing under the spout with room left trips the gate (items still falling count against that room). The load comes down into the cart one item every 5 ticks. With no cart there, the gate stays shut. **Right mouse** holds the gate open by hand: it then empties onto whatever is under it. Right mouse again lets it go.
* `E` empties its load into the pack, and `E` on an empty hopper picks it up.
* **Footing:** dig away the floor under both legs and it falls, spilling its load as loose items.
* Ore hoppers and their loads are saved (transport save v4) and travel with the window.

Scenario `ore_hopper_feeds_cart`: ten stones and a pour of sand go in. The gate stays shut until it's held open, and two lumps drop onto the track. A cart rolled down under the spout scoops those up and is filled until the hopper is empty. The load survives a save.

## A whole line

Scenario `production_line` runs the line end to end with nothing carried by hand:
1. Gravel and copper ore go into an ore hopper, which fills a cart under its spout.
2. A lever powers copperline on the level stretch, which drives the cart off. It rolls down to a bumper and tips.
3. The load slides down a chute onto a steep shaker screen. The gravel drops through into the pit under it, and the ore slides off its foot into the far pit.

The mole stands by the hopper throughout and picks up nothing, because lumps let down from a hopper or tipped from a cart can't be picked up for their first third of a second.

## How they look

Chutes, screens and bumpers are cells with their own patterns (`art::mat_color`). A chute has a pale worn lip and boards along it. A screen is iron bars over a dark mesh under a timber rail. A bumper is red-painted timber with a bright head and iron bands. The ore hopper is drawn: its heap is tinted with what it holds, it shows through the open front of the bin and rises with the load, and the gate under the spout swings open when it lets a lump down. `--scene=production` stamps a small works (chute onto screen, a hopper of copper ore over track, a bumper) for stills. See [Item and Place Art](Item-and-Place-Art.md).

## Hover tips

The chute, screen (shaking or not) and bumper say what moves things along them. The ore hopper's tip gives its load and capacity, and says whether the gate is held open or opens for a cart (test `production_hover_tips_say_what_to_do`).

## Making them

| station | makes | from |
|---|---|---|
| Workbench | 8 chute | 2 wood |
| Workbench | 2 bumper | 2 wood + 1 stone |
| Forge | 4 shaker screen | 1 iron ingot + 1 wood |
| Forge | ore hopper | 6 wood + 1 copper ingot |

Chutes and bumpers come from the workbench (8 chute from 2 wood; 2 bumper from 2 wood and 1 stone), the hopper needs copper but no iron (forge: 6 wood and 1 copper ingot), and the screen needs iron (forge: 4 from 1 iron ingot and 1 wood) (test `production_is_reachable_in_survival`; all recipes are on [Tools and Items](Tools-and-Items.md#recipes)). Digging a chute or bumper gives wood back.
