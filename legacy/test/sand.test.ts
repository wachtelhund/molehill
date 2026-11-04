import { SandWorld, CW, CH, B, BW, NC } from "../src/sand/world";
import { M } from "../src/sand/materials";
import { D } from "../src/sand/logic";
let fail = 0;
const ok = (c: boolean, msg: string) => { console.log((c ? "PASS " : "FAIL ") + msg); if (!c) fail++; };
const run = (w: SandWorld, n: number) => { for (let i = 0; i < n; i++) w.step(); };
const box = (w: SandWorld, x0: number, y0: number, x1: number, y1: number, m: number) => { for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) w.setCell(w.ci(x, y), m); };
const colHeight = (w: SandWorld, x: number, y0: number, y1: number, m: number) => { let h = 0; for (let y = y0; y <= y1; y++) if (w.mat[w.ci(x, y)] === m) h++; return h; };
const blk = (bx: number, by: number) => by * BW + bx;

{ // 1. powder piles at angle of repose
  const w = SandWorld.empty(M.ROCK); box(w, 40, 20, 200, 120, M.EMPTY);
  box(w, 118, 21, 122, 100, M.SAND); // tall 5-wide column
  run(w, 400);
  const peak = colHeight(w, 120, 20, 120, M.SAND), side = colHeight(w, 132, 20, 120, M.SAND);
  ok(w.count(M.SAND) === 5 * 80, `sand conserved (${w.count(M.SAND)})`);
  ok(peak < 40 && peak > 10, `column slumped into a pile (peak height ${peak})`);
  ok(side > 0 && side < peak, `pile slopes outward (h@+12 = ${side})`);
}
{ // 2. water conserves and levels
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 140, 80, M.EMPTY); box(w, 41, 41, 60, 70, M.WATER);
  const n0 = w.count(M.WATER); run(w, 600);
  ok(w.count(M.WATER) === n0, `water cells conserved (${n0})`);
  const hs = [45, 90, 135].map(x => colHeight(w, x, 40, 80, M.WATER));
  ok(Math.max(...hs) - Math.min(...hs) <= 2, `water level flat (${hs.join(",")})`);
}
{ // 3. lava + water -> stone + steam
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 140, 80, M.EMPTY); box(w, 41, 70, 139, 80, M.LAVA); box(w, 41, 41, 70, 50, M.WATER);
  run(w, 300);
  ok(w.count(M.STONE) > 30, `stone crust formed (${w.count(M.STONE)} cells)`);
}
{ // 4. unsupported dirt crumbles into powder and crushes a mole
  const w = SandWorld.empty(M.ROCK); box(w, 40, 60, 160, 80, M.EMPTY); box(w, 40, 36, 160, 59, M.DIRT);
  w.updateNav(); w.addMole(25, 19); run(w, 300);
  let loose = 0; for (let i = 0; i < NC; i++) if (w.loose[i]) loose++;
  const floor = colHeight(w, 100, 60, 80, M.DIRT);
  ok(loose > 1500, `dirt ceiling collapsed into ${loose} loose cells`);
  ok(floor > 5, `rubble piled on the tunnel floor (${floor} cells deep at x=100)`);
  ok(w.moles[0].hp < 100, `mole under the collapse was hurt (hp ${w.moles[0].hp})`);
}
{ // 5. beams hold the same ceiling
  const w = SandWorld.empty(M.ROCK); box(w, 40, 60, 160, 80, M.EMPTY); box(w, 40, 36, 160, 59, M.DIRT);
  for (let x = 48; x < 160; x += 16) box(w, x, 60, x + 1, 80, M.WOOD);
  run(w, 200); let loose = 0; for (let i = 0; i < NC; i++) if (w.loose[i]) loose++;
  ok(loose === 0, `beamed ceiling holds (${loose} loose)`);
}
{ // 6. rubble pile blocks water
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 140, 80, M.EMPTY); box(w, 88, 41, 92, 80, M.SAND); run(w, 300); box(w, 41, 72, 70, 80, M.WATER);
  run(w, 400); let right = 0; for (let y = 40; y <= 80; y++) for (let x = 100; x <= 140; x++) if (w.mat[w.ci(x, y)] === M.WATER) right++;
  ok(right < 20, `debris dam holds back water (${right} cells leaked)`);
}
{ // 7. fuse -> TNT chain blasts rock and loosens soil
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 160, 44, M.EMPTY); w.updateNav();
  for (let bx = 10; bx < 18; bx++) w.logic.place("fuse", blk(bx, 10));
  w.logic.place("tnt", blk(18, 10)); w.logic.place("tnt", blk(22, 10));
  const r0 = w.count(M.ROCK); w.logic.ignite(blk(10, 10)); run(w, 120);
  ok(r0 - w.count(M.ROCK) > 100, `blast cleared ${r0 - w.count(M.ROCK)} rock cells`);
  ok(w.logic.dev[blk(22, 10)] === 0, "second TNT chain-detonated");
}
{ // 8. lever -> wire -> door; NOT; AND; latch; timer
  const w = SandWorld.empty(M.ROCK); box(w, 40, 36, 200, 44, M.EMPTY); w.updateNav();
  const L = w.logic; L.place("lever", blk(10, 10)); for (let x = 11; x <= 14; x++) L.place("wire", blk(x, 10)); L.place("door", blk(15, 10));
  run(w, 3); ok(w.mat[w.ci(62, 42)] === M.DOOR, "door closed while lever off");
  L.interact(blk(10, 10)); run(w, 3); ok(w.mat[w.ci(62, 42)] === M.EMPTY, "lever on opens door");
  // NOT gate: lever2 -> wire -> NOT -> wire -> lamp
  L.place("lever", blk(20, 10)); L.place("wire", blk(21, 10)); L.place("not", blk(22, 10)); L.place("wire", blk(23, 10)); L.place("lamp", blk(24, 10));
  run(w, 4); ok(L.devS[blk(24, 10)] === 1, "NOT of off = lamp on");
  L.interact(blk(20, 10)); run(w, 4); ok(L.devS[blk(24, 10)] === 0, "NOT of on = lamp off");
  // AND: a=west, b=north
  L.place("lever", blk(30, 10)); L.place("wire", blk(31, 10)); L.place("and", blk(32, 10)); L.place("wire", blk(33, 10)); L.place("lamp", blk(34, 10));
  L.place("lever", blk(31, 9)); L.place("wire", blk(32, 9));
  L.interact(blk(30, 10)); run(w, 4); ok(L.devS[blk(34, 10)] === 0, "AND with one input off");
  L.interact(blk(31, 9)); run(w, 4); ok(L.devS[blk(34, 10)] === 1, "AND with both inputs on");
  // LATCH: set pulse holds
  L.place("lever", blk(40, 10)); L.place("wire", blk(41, 10)); L.place("latch", blk(42, 10)); L.place("wire", blk(43, 10)); L.place("lamp", blk(44, 10));
  L.interact(blk(40, 10)); run(w, 3); L.interact(blk(40, 10)); run(w, 3); ok(L.devS[blk(44, 10)] === 1, "latch holds after set released");
  // TIMER blinks
  L.place("timer", blk(26, 9)); L.place("wire", blk(27, 9)); L.place("lamp", blk(28, 9));
  let seen = new Set<number>(); for (let i = 0; i < 60; i++) { w.step(); seen.add(L.devS[blk(28, 9)]); } ok(seen.size === 2, "timer toggles lamp");
}
{ // 9. lava severs a wire -> door closes; LoRa is immune
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 300, 44, M.EMPTY); w.updateNav();
  const L = w.logic; L.place("lever", blk(10, 10)); for (let x = 11; x <= 19; x++) L.place("wire", blk(x, 10)); L.place("door", blk(20, 10));
  L.interact(blk(10, 10)); run(w, 3); ok(w.mat[w.ci(82, 42)] === M.EMPTY, "door open via wire");
  box(w, 60, 40, 63, 43, M.LAVA); run(w, 12); ok(L.wire[blk(15, 10)] === 0, "lava destroyed the wire");
  ok(w.mat[w.ci(82, 42)] === M.DOOR, "severed network -> door closed");
}
{ // 10. LoRa through rock: range, attenuation, repeater, channels
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 52, 47, M.EMPTY); box(w, 88, 40, 100, 47, M.EMPTY); box(w, 140, 40, 152, 47, M.EMPTY); w.updateNav();
  const L = w.logic; L.place("lever", blk(10, 10)); L.place("wire", blk(11, 10)); L.place("tx", blk(12, 10));
  L.place("rx", blk(23, 10)); L.place("wire", blk(24, 10)); L.place("lamp", blk(24, 11));
  L.interact(blk(10, 10)); run(w, 20);
  const bud = L.linkBudget(blk(12, 10), blk(23, 10));
  ok(bud >= 0 && L.devS[blk(24, 11)] === 1, `TX->RX through 9 blocks of rock (budget ${bud.toFixed(1)}) lights lamp`);
  L.place("rx", blk(36, 10)); L.place("wire", blk(37, 10)); L.place("lamp", blk(37, 11)); run(w, 20);
  ok(L.devS[blk(37, 11)] === 0, `far RX out of range (budget ${L.linkBudget(blk(12, 10), blk(36, 10)).toFixed(1)})`);
  L.place("rep", blk(22, 11)); run(w, 20);
  ok(L.devS[blk(37, 11)] === 1, "repeater relays to far RX");
  L.interact(blk(36, 10)); run(w, 20); ok(L.devS[blk(37, 11)] === 0, "channel mismatch blocks signal");
}
{ // 11. mole flees rising water and digs designated blocks
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 160, 47, M.EMPTY); box(w, 120, 44, 160, 47, M.ROCK); w.updateNav(); w.addMole(12, 11);
  box(w, 40, 40, 70, 47, M.WATER); run(w, 80);
  const m = w.moles[0];
  ok(w.log.some(e => e.text.includes("flees")) && !w.danger(m.by * BW + m.bx) && m.hp === 100, `mole fled the water to ${m.bx},${m.by}`);
  const w2 = SandWorld.empty(M.ROCK); box(w2, 40, 40, 60, 43, M.EMPTY); box(w2, 61, 40, 64, 43, M.DIRT); w2.updateNav(); w2.addMole(11, 10);
  w2.designate("dig", blk(15, 10)); run(w2, 200); ok(w2.mat[w2.ci(62, 41)] === M.EMPTY && w2.res.rubble >= 12, "mole dug a block");
}
{ // 12. determinism + generated world stable
  const a = new SandWorld(42), b = new SandWorld(42);
  for (const w of [a, b]) { for (let x = 30; x < 42; x++) w.designate("dig", blk(x, 11)); run(w, 300); }
  ok(a.hash() === b.hash(), `deterministic (${a.hash()})`);
  const g = new SandWorld(7); run(g, 60); let loose = 0; for (let i = 0; i < NC; i++) if (g.loose[i]) loose++;
  ok(loose === 0 && g.moles.every(m => !m.dead), `fresh world stable (${loose} loose)`);
}
{ // perf
  const w = new SandWorld(3); const t0 = Date.now(); run(w, 300); const ms = (Date.now() - t0) / 300;
  ok(ms < 12, `avg tick ${ms.toFixed(2)} ms, active chunks ${w.activeChunks()}`);
}
console.log(fail ? `${fail} FAILED` : "ALL PASSED"); process.exit(fail ? 1 : 0);
