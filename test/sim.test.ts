import { World, T, W, H, N } from "../src/sim";
let fail = 0;
const ok = (c: boolean, msg: string) => { console.log((c ? "PASS " : "FAIL ") + msg); if (!c) fail++; };
const box = (w: World, x0: number, y0: number, x1: number, y1: number, t: number) => { for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) w.tile[w.idx(x, y)] = t; };

{ // 1. water mass conservation + settles
  const w = World.empty(T.ROCK); box(w, 10, 10, 29, 29, T.AIR);
  for (let y = 10; y < 15; y++) for (let x = 10; x < 20; x++) w.water[w.idx(x, y)] = 1;
  const m0 = w.totalMass(w.water); for (let i = 0; i < 1500; i++) w.step();
  const m1 = w.totalMass(w.water);
  ok(Math.abs(m1 - m0) < 1e-2, `water mass conserved (${m0.toFixed(3)} -> ${m1.toFixed(3)})`);
  let bottom = 0; for (let x = 10; x < 30; x++) bottom += w.water[w.idx(x, 29)];
  ok(bottom > 19, `water settled to floor (bottom row mass ${bottom.toFixed(2)} of 20 cells)`);
  ok(w.water[w.idx(15, 10)] < 0.01, "top of box drained");
}
{ // 2. pressure: U-tube equalises
  const w = World.empty(T.ROCK); box(w, 10, 10, 12, 30, T.AIR); box(w, 20, 10, 22, 30, T.AIR); box(w, 10, 28, 22, 30, T.AIR);
  for (let y = 10; y < 28; y++) for (let x = 10; x <= 12; x++) w.water[w.idx(x, y)] = 1;
  for (let i = 0; i < 4000; i++) w.step();
  const lvl = (x: number) => { let s = 0; for (let y = 10; y < 28; y++) s += w.water[w.idx(x, y)]; return s; };
  ok(lvl(21) > 1.5 && Math.abs(lvl(11) - lvl(21)) < 1, `U-tube pressure equalises (left ${lvl(11).toFixed(1)}, right ${lvl(21).toFixed(1)})`);
}
{ // 3. lava + water = stone
  const w = World.empty(T.ROCK); box(w, 10, 10, 30, 20, T.AIR);
  for (let y = 18; y <= 20; y++) for (let x = 10; x <= 30; x++) w.lava[w.idx(x, y)] = 1;
  for (let y = 10; y <= 11; y++) for (let x = 10; x <= 15; x++) w.water[w.idx(x, y)] = 1;
  for (let i = 0; i < 400; i++) w.step();
  let stone = 0; for (let i = 0; i < N; i++) if (w.tile[i] === T.STONE) stone++;
  ok(stone > 3, `lava+water produced ${stone} stone tiles`);
}
{ // 4. cave-in crushes mole and leaves debris that blocks
  const w = World.empty(T.ROCK); box(w, 10, 10, 30, 14, T.AIR); box(w, 10, 6, 30, 9, T.DIRT);
  w.addMole(20, 14); w.step();
  for (let i = 0; i < 200; i++) w.step();
  let debris = 0; for (let i = 0; i < N; i++) if (w.tile[i] === T.DEBRIS) debris++;
  ok(debris > 10, `unsupported dirt collapsed into ${debris} debris tiles`);
  ok(w.moles[0].hp < 100, `mole under collapse took damage (hp ${w.moles[0].hp})`);
}
{ // 5. beams prevent collapse
  const w = World.empty(T.ROCK); box(w, 10, 10, 20, 12, T.AIR); box(w, 10, 7, 20, 9, T.DIRT);
  for (const x of [13, 17]) for (let y = 10; y <= 12; y++) w.tile[w.idx(x, y)] = T.BEAM;
  for (let i = 0; i < 100; i++) w.step();
  let debris = 0; for (let i = 0; i < N; i++) if (w.tile[i] === T.DEBRIS) debris++;
  ok(debris === 0, `beamed 11-wide dirt ceiling holds (debris ${debris})`);
}
{ // 6. fuse -> TNT chain
  const w = World.empty(T.ROCK); box(w, 10, 10, 40, 10, T.AIR);
  for (let x = 10; x < 20; x++) w.fuse[w.idx(x, 10)] = 1;
  w.tnt[w.idx(20, 10)] = 1; w.tnt[w.idx(23, 10)] = 1; w.tnt[w.idx(26, 10)] = 1;
  w.igniteCell(w.idx(10, 10));
  for (let i = 0; i < 200; i++) w.step();
  ok(w.tnt[w.idx(26, 10)] === 0, "fuse burned and TNT chain-detonated");
  ok(w.tile[w.idx(26, 12)] === T.AIR, "blast removed rock");
}
{ // 7. mole digs designated tile, hauls rubble
  const w = World.empty(T.DIRT); box(w, 0, 0, W - 1, 0, T.ROCK); box(w, 0, H - 1, W - 1, H - 1, T.BEDROCK);
  box(w, 10, 20, 14, 20, T.AIR); w.stock = w.idx(10, 20); w.addMole(11, 20);
  w.designate("dig", w.idx(15, 20)); w.designate("dig", w.idx(16, 20));
  for (let i = 0; i < 600; i++) w.step();
  ok(w.tile[w.idx(16, 20)] === T.AIR, "mole dug two designated tiles");
  ok(w.res.rubble >= 1, `rubble hauled to stockpile (${w.res.rubble})`);
}
{ // 8. determinism
  const run = () => { const w = new World(42); for (let x = 30; x < 60; x++) w.designate("dig", w.idx(x, 11)); for (let i = 0; i < 800; i++) w.step(); return w.hash(); };
  const a = run(), b = run(); ok(a === b, `deterministic hash ${a} == ${b}`);
}
{ // 9. generated world is stable at start
  const w = new World(7); for (let i = 0; i < 60; i++) w.step();
  ok(w.debris.length === 0 && w.moles.every(m => !m.dead), "fresh world has no spontaneous collapse");
}
console.log(fail ? `${fail} FAILED` : "ALL PASSED"); process.exit(fail ? 1 : 0);
