// Demo scenarios for the v2 (falling-sand) engine. Coordinates in blocks (4x4 cells).
import { SandWorld, B, BW } from "./world";
import { M } from "./materials";

const blk = (bx: number, by: number) => by * BW + bx;
/** fill a block-rect [bx0..bx1]x[by0..by1] with material */
function fb(w: SandWorld, bx0: number, by0: number, bx1: number, by1: number, m: number) { w.fillRect(bx0 * B, by0 * B, bx1 * B + B - 1, by1 * B + B - 1, m); }
function wire(w: SandWorld, path: [number, number][]) { for (const [x, y] of path) w.logic.place("wire", blk(x, y)); }
function hline(y: number, x0: number, x1: number): [number, number][] { const r: [number, number][] = []; for (let x = Math.min(x0, x1); x <= Math.max(x0, x1); x++) r.push([x, y]); return r; }
function vline(x: number, y0: number, y1: number): [number, number][] { const r: [number, number][] = []; for (let y = Math.min(y0, y1); y <= Math.max(y0, y1); y++) r.push([x, y]); return r; }

/** Wiring + wireless showcase: flood trap, sensor->TNT, gates, LoRa through rock, cave-in. */
export function wiringScenario(w: SandWorld) {
  const L = w.logic;
  // solid canvas below the start chamber
  fb(w, 4, 14, 92, 34, M.ROCK);

  // --- A. Flood trap: reservoir -> door -> tunnel; pressure plate opens it; sensor triggers TNT into lava
  fb(w, 8, 15, 20, 17, M.EMPTY); fb(w, 8, 15, 20, 17, M.WATER);          // reservoir
  fb(w, 6, 19, 30, 20, M.EMPTY);                                          // tunnel
  fb(w, 14, 18, 15, 18, M.EMPTY); L.place("door", blk(14, 18)); L.place("door", blk(15, 18));             // reservoir floor doors
  L.place("plate", blk(27, 20));
  wire(w, [...vline(28, 18, 20), ...hline(18, 16, 27)]);                  // plate -> doors (wire runs through rock)
  L.place("sensor", blk(7, 20));
  wire(w, vline(6, 20, 20)); fb(w, 6, 21, 6, 21, M.EMPTY); L.place("tnt", blk(6, 21));
  fb(w, 4, 23, 14, 27, M.EMPTY); fb(w, 4, 24, 14, 27, M.LAVA);            // lava chamber under the TNT
  // sand hopper above the plate: a scripted "mole drops rubble"
  w.scripts.push({ tick: 90, fn: w => { w.say("▶ Demo: rubble dropped on the pressure plate"); w.fillRect(27 * B, 19 * B + 2, 27 * B + 3, 19 * B + 3, M.SAND); } });

  // --- B. Cave-in: wide dirt ceiling, half beamed, beams removed by script
  fb(w, 33, 15, 47, 17, M.DIRT); fb(w, 33, 18, 47, 20, M.EMPTY);
  for (const bx of [36, 40, 44]) fb(w, bx, 18, bx, 20, M.WOOD);
  w.scripts.push({ tick: 150, fn: w => { w.say("▶ Demo: beams knocked out — watch the ceiling crumble"); for (const bx of [40, 44]) fb(w, bx, 18, bx, 20, M.EMPTY); w.structDirty = true; } });

  // --- C. Logic bench: timer AND lever -> lamp; lever -> NOT -> lamp
  fb(w, 50, 15, 62, 20, M.EMPTY);
  L.place("timer", blk(51, 16)); wire(w, [[52, 16]]); L.place("and", blk(53, 16)); wire(w, [[54, 16]]); L.place("lamp", blk(55, 16));
  L.place("lever", blk(52, 15)); wire(w, [[53, 15]]);                    // b input (north of AND)
  L.place("lever", blk(51, 18)); wire(w, [[52, 18]]); L.place("not", blk(53, 18)); wire(w, [[54, 18]]); L.place("lamp", blk(55, 18));
  L.place("lever", blk(57, 17)); wire(w, [[58, 17]]); L.place("latch", blk(59, 17)); wire(w, [[60, 17]]); L.place("lamp", blk(61, 17));
  w.scripts.push({ tick: 60, fn: w => { w.say("▶ Demo: lever → AND gate enabled"); w.logic.interact(blk(52, 15)); } });
  w.scripts.push({ tick: 120, fn: w => { w.say("▶ Demo: latch set by a pulse"); w.logic.interact(blk(57, 17)); } });
  w.scripts.push({ tick: 135, fn: w => { w.logic.interact(blk(57, 17)); } });

  // --- D. LoRa: lever -> TX, through rock to RX -> door; repeater -> far RX on ch1 vs mismatch ch2
  fb(w, 64, 15, 67, 17, M.EMPTY); L.place("lever", blk(64, 17)); wire(w, [[65, 17]]); L.place("tx", blk(66, 17));
  fb(w, 75, 15, 79, 17, M.EMPTY); L.place("rx", blk(75, 17)); wire(w, [...hline(16, 75, 77)]); L.place("door", blk(78, 16)); L.place("lamp", blk(76, 15));
  L.place("rep", blk(79, 17));
  fb(w, 86, 15, 91, 17, M.EMPTY); L.place("rx", blk(87, 17)); wire(w, [[87, 16]]); L.place("lamp", blk(88, 16));
  L.place("rx", blk(90, 17)); L.interact(blk(90, 17)); wire(w, [[90, 16]]); L.place("lamp", blk(91, 16)); // channel 1: mismatch
  // all TX/RX default to channel 0
  w.scripts.push({ tick: 200, fn: w => { w.say("▶ Demo: lever → LoRa TX (signal goes through rock)"); w.logic.interact(blk(64, 17)); } });
  // ladder-ish access shafts from the chamber so moles can visit
  fb(w, 49, 13, 49, 20, M.EMPTY);
  w.wakeAll(); w.structDirty = true;
  w.say("Demo: flood trap (A), cave-in (B), logic gates (C), LoRa (D)");
}
