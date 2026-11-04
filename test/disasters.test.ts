import { DisasterDirector, DisasterWorld, DisasterKind, FluidKind } from "../src/disasters";
import { sandDisasterWorld, createDisasters } from "../src/disasters-sand";
import { SandWorld, NC, mulberry32 } from "../src/sand/world";
import { M, K, KIND, ANCHOR } from "../src/sand/materials";
let fail = 0;
const ok = (c: boolean, msg: string) => { console.log((c ? "PASS " : "FAIL ") + msg); if (!c) fail++; };

// Engine-free fake: any resolution, 0 = open, 1 = loose, 2 = anchored.
function fakeWorld(width: number, height: number) {
  const cell = new Uint8Array(width * height), water = new Float32Array(width * height), lava = new Float32Array(width * height);
  const log: string[] = [], dislodged: number[] = []; let shakes = 0;
  const api: DisasterWorld = {
    width, height,
    isOpen: (x, y) => cell[y * width + x] === 0,
    isLoose: (x, y) => cell[y * width + x] === 1,
    fluidAt: (x, y, k: FluidKind) => (k === "water" ? water : lava)[y * width + x],
    addFluid: (x, y, k: FluidKind, a) => { (k === "water" ? water : lava)[y * width + x] += a; },
    dislodge: (x, y) => { cell[y * width + x] = 0; dislodged.push(y * width + x); },
    shake: () => { shakes++; },
    say: (t) => { log.push(t); },
  };
  return { api, cell, water, lava, log, dislodged, get shakes() { return shakes; } };
}
// Solid loose ground with an open tunnel band in the middle and a sky strip on top.
function tunnelWorld(width: number, height: number) {
  const f = fakeWorld(width, height); f.cell.fill(1);
  for (let x = 0; x < width; x++) { for (let y = 0; y < Math.floor(height * 0.1); y++) f.cell[y * width + x] = 0; f.cell[(height - 1) * width + x] = 2; }
  for (let y = Math.floor(height * 0.4); y < Math.floor(height * 0.75); y++) for (let x = 1; x < width - 1; x++) f.cell[y * width + x] = 0;
  return f;
}
const sum = (a: Float32Array) => a.reduce((s, v) => s + v, 0);
const runFor = (d: DisasterDirector, ticks: number) => { for (let i = 0; i < ticks; i++) d.step(); };

{ // 1. warning countdown precedes the event, then returns to calm
  const f = tunnelWorld(40, 30);
  const d = new DisasterDirector(f.api, mulberry32(1), { hz: 10, warning: 3, duration: { flood: 2, lava: 2, quake: 2 }, minGap: 5, maxGap: 5 });
  ok(d.state.phase === "calm" && d.state.ticksLeft === 50, "starts calm with gap in sim ticks");
  ok(d.schedule("flood"), "flood can be scheduled");
  ok(d.state.phase === "warning" && Math.abs(d.secondsLeft - 3) < 1e-9, `warning countdown at 3s (${d.secondsLeft})`);
  ok(/warning/i.test(f.log[0] ?? ""), `warning announced: "${f.log[0]}"`);
  runFor(d, 29);
  ok(d.state.phase === "warning" && sum(f.water) === 0, "nothing happens during the countdown");
  runFor(d, 1);
  ok(d.state.phase === "active" && sum(f.water) > 0, "flood starts when countdown hits zero");
  runFor(d, 19);
  ok(d.state.phase === "calm", "returns to calm after duration");
}
{ // 2. random scheduling produces every kind, always with a warning first
  const f = tunnelWorld(48, 32);
  const d = new DisasterDirector(f.api, mulberry32(3), { hz: 10, minGap: 1, maxGap: 2, warning: 1, duration: { flood: 1, lava: 1, quake: 1 } });
  const seen = new Set<DisasterKind>(); let prev = "calm", badOrder = 0;
  for (let i = 0; i < 3000; i++) { d.step(); const p = d.state.phase; if (p === "active" && prev === "calm") badOrder++; if (p === "active") seen.add(d.state.kind!); prev = p; }
  ok(seen.size === 3, `all disaster kinds occurred (${[...seen].join(",")})`);
  ok(badOrder === 0, "no disaster ever skipped its warning");
}
{ // 3. disabled director never fires on its own
  const f = tunnelWorld(32, 32);
  const d = new DisasterDirector(f.api, mulberry32(4), { minGap: 0.1, maxGap: 0.1 }); d.enabled = false;
  runFor(d, 500); ok(d.state.phase === "calm" && f.log.length === 0, "disabled director stays calm");
}
{ // 4. fluid volume and quake damage scale with world area, not cell count constants
  for (const [w, h] of [[40, 30], [160, 120]]) {
    const f = tunnelWorld(w, h);
    const d = new DisasterDirector(f.api, mulberry32(5), { hz: 10, warning: 0.1, quakeSamples: 0.02, duration: { flood: 2, lava: 2, quake: 2 } });
    d.schedule("flood"); runFor(d, 21);
    const frac = sum(f.water) / (w * h);
    ok(Math.abs(frac - d.cfg.floodVolume) < 1e-4, `${w}x${h}: flood volume = ${frac.toFixed(4)} of area`);
    d.schedule("lava"); runFor(d, 21);
    ok(Math.abs(sum(f.lava) / (w * h) - d.cfg.lavaVolume) < 1e-4, `${w}x${h}: lava volume scales with area`);
    d.schedule("quake"); runFor(d, 21);
    const cap = Math.round(d.cfg.quakeLoose * w * h);
    ok(f.dislodged.length > 0 && f.dislodged.length <= cap, `${w}x${h}: quake loosened ${f.dislodged.length} (cap ${cap})`);
    ok(f.dislodged.every(i => f.cell[i + w] === 0), `${w}x${h}: quake only loosens overhangs`);
    ok(f.shakes > 0, `${w}x${h}: quake shakes the screen`);
  }
}
{ // 5. site selection: flood high, lava deep, both underground
  const f = tunnelWorld(60, 40); const d = new DisasterDirector(f.api, mulberry32(9));
  for (let k = 0; k < 20; k++) {
    const fl = d.pickSite("flood")!, lv = d.pickSite("lava")!;
    if (!(fl.y < 20 && f.api.isOpen(fl.x, fl.y))) { ok(false, `flood site ${fl.x},${fl.y} in upper half`); break; }
    if (!(lv.y >= 24 && f.api.isOpen(lv.x, lv.y))) { ok(false, `lava site ${lv.x},${lv.y} deep`); break; }
    if (k === 19) ok(true, "flood sites upper half, lava sites deep, all open");
  }
  const solid = fakeWorld(20, 20); solid.cell.fill(1);
  ok(!new DisasterDirector(solid.api, mulberry32(1)).schedule("flood"), "no site in solid world -> not scheduled");
}
{ // 6. deterministic for a given seed
  const run = () => { const f = tunnelWorld(50, 40); const d = new DisasterDirector(f.api, mulberry32(77), { minGap: 1, maxGap: 3, warning: 1 });
    runFor(d, 2000); return f.log.join("|") + sum(f.water) + sum(f.lava) + f.dislodged.join(","); };
  ok(run() === run(), "same seed -> same disasters");
}
// ---------- integration with the v2 falling-sand engine ----------
const box = (w: SandWorld, x0: number, y0: number, x1: number, y1: number, m: number) => { for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) w.setCell(w.ci(x, y), m); };
const countIn = (w: SandWorld, x0: number, y0: number, x1: number, y1: number, pred: (i: number) => boolean) => { let n = 0; for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) if (pred(w.ci(x, y))) n++; return n; };
const runBoth = (w: SandWorld, d: DisasterDirector, n: number) => { for (let i = 0; i < n; i++) { w.step(); d.step(); } };
{ // 7. earthquake loosens soft ceiling cells; the sand sim crumbles them and the support pass brings more down
  const w = SandWorld.empty(M.ROCK); box(w, 100, 100, 115, 110, M.EMPTY); box(w, 100, 95, 115, 99, M.DIRT);
  for (let i = 0; i < 60; i++) w.step();
  const fallen = () => countIn(w, 100, 100, 115, 110, i => w.mat[i] === M.DIRT);
  ok(fallen() === 0 && w.loose.every(v => v === 0), "16-wide dirt ceiling is stable before the quake");
  const api = sandDisasterWorld(w); let loosened = 0;
  const d = new DisasterDirector({ ...api, dislodge: (x, y) => { loosened++; api.dislodge(x, y); } }, mulberry32(2), { warning: 0.05, quakeSamples: 0.05 });
  ok(d.schedule("quake"), "quake finds an epicentre in a mostly solid world");
  let touched = true;
  // Every loosened cell is soft material: no anchors, nothing outside the soft set.
  for (let i = 0; i < 200; i++) { w.step(); d.step(); for (let c = 0; c < NC; c++) if (w.loose[c] && (ANCHOR[w.mat[c]] || KIND[w.mat[c]] === K.LIQUID)) touched = false; }
  ok(touched, "quake only loosens soft cells");
  ok(d.state.phase === "calm" && loosened > 0, `quake loosened ${loosened} cells and ended`);
  ok(fallen() >= 10, `dirt crumbled into the cavern (${fallen()} cells)`);
  ok(!api.isLoose(50, 50), "rock is never a quake target");
}
{ // 8. flood and lava spawn whole fluid cells matching the configured volume
  const w = SandWorld.empty(M.ROCK); box(w, 60, 40, 300, 200, M.EMPTY);
  const d = new DisasterDirector(sandDisasterWorld(w), mulberry32(3), { warning: 0.05, duration: { flood: 2, lava: 2, quake: 2 } });
  const area = w.mat.length;
  d.schedule("flood"); runBoth(w, d, 80);
  const water = w.count(M.WATER), want = d.cfg.floodVolume * area;
  ok(water > want * 0.9 && water <= want + 1, `flood spawned ${water} water cells (target ${want.toFixed(0)})`);
  d.schedule("lava"); runBoth(w, d, 80);
  ok(w.count(M.LAVA) + w.count(M.STONE) > d.cfg.lavaVolume * area * 0.5, `lava surge spawned lava (${w.count(M.LAVA)} lava, ${w.count(M.STONE)} stone)`);
}
{ // 9. spring pushes up through a full water pocket instead of losing the fluid
  const w = SandWorld.empty(M.ROCK); box(w, 100, 100, 120, 140, M.EMPTY); box(w, 100, 130, 120, 140, M.WATER);
  const api = sandDisasterWorld(w), n0 = w.count(M.WATER);
  for (let k = 0; k < 10; k++) api.addFluid(110, 135, "water", 1);
  ok(w.count(M.WATER) === n0 + 10, `fluid added into a full pocket (${w.count(M.WATER) - n0} of 10)`);
}
{ // 10. URL-style options on a generated world
  const w = new SandWorld(7);
  ok(!createDisasters(w, 7, new URLSearchParams("disasters=0")).enabled, "?disasters=0 disables random events");
  const now = createDisasters(w, 7, new URLSearchParams("disaster=quake"));
  ok(now.state.phase === "warning" && now.state.kind === "quake", "?disaster=quake starts a quake warning");
}
console.log(fail ? `${fail} FAILED` : "ALL PASSED"); process.exit(fail ? 1 : 0);
