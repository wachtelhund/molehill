// Material table for the fine-grained (Noita-style) world. One cell = 4 px.
export const enum M { EMPTY, BEDROCK, ROCK, GRANITE, STONE, DIRT, GRAVEL, CLAY, ORE, WOOD, DOOR, SAND, WATER, LAVA, STEAM, FIRE }
export const MAT_NAMES = ["Empty", "Bedrock", "Rock", "Granite", "Stone", "Dirt", "Gravel", "Clay", "Copper ore", "Wood", "Door", "Rubble", "Water", "Lava", "Steam", "Fire"];
export const enum K { EMPTY, SOLID, POWDER, LIQUID, GAS }
//                        EMP BED ROCK GRAN STON DIRT GRAV CLAY ORE WOOD DOOR SAND WATR LAVA STEM FIRE
export const KIND =      [0,  1,  1,   1,   1,   1,   1,   1,   1,  1,   1,   2,   3,   3,   4,   1];
export const ANCHOR =    [0,  1,  1,   1,   1,   0,   0,   0,   0,  0,   0,   0,   0,   0,   0,   0];
/** Max unsupported span in cells (BFS cost). 0 = not structural. */
export const SPAN =      [0,  0,  0,   0,   0,   10,  6,   14,  10, 48,  0,   0,   0,   0,   0,   0];
/** Ticks for a mole to dig one cell. 0 = undiggable. */
export const DIG =       [0,  0,  6,   12,  6,   2,   2,   3,   4,  2,   2,   1,   0,   0,   0,   0];
/** Blast hardness (power*10 must reach it). */
export const HARD =      [0,  99, 5,   8,   5,   2,   2,   3,   4,  2,   3,   1,   0,   0,   0,   0];
/** LoRa attenuation per cell (per-block value / 4). */
export const ATTEN =     [0,  99, 0.6, 1,   0.6, 0.3, 0.3, 0.45,0.5, 0.2, 0.4, 0.3, 0.6, 2.5, 0,   0];
export const COLOR: [number, number, number][] = [
  [27, 23, 20], [34, 38, 45], [94, 100, 112], [63, 70, 82], [74, 69, 82], [122, 90, 58], [154, 143, 126], [168, 107, 76],
  [224, 123, 57], [200, 155, 91], [140, 110, 70], [128, 110, 92], [58, 143, 217], [255, 120, 40], [220, 226, 232], [255, 170, 60],
];
export const isStructMat = (m: number) => SPAN[m] > 0 || ANCHOR[m] === 1;
