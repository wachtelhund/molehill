import { SandWorld, BW, CW, CH } from "../src/sand/world";
import { M } from "../src/sand/materials";
import { SandEventTracker, ambientLevels } from "../src/ambience/sandAdapter";
import { RateLimiter, spatialize, SoundEvent, SoundKind, MIN_GAP } from "../src/ambience/events";
import { SoundEngine } from "../src/ambience/audio";
let fail = 0;
const ok = (c: boolean, msg: string) => { console.log((c ? "PASS " : "FAIL ") + msg); if (!c) fail++; };
const box = (w: SandWorld, x0: number, y0: number, x1: number, y1: number, m: number) => { for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) w.setCell(w.ci(x, y), m); };
const blk = (bx: number, by: number) => by * BW + bx;
const record = (w: SandWorld, n: number, tr = new SandEventTracker()) => { tr.update(w); const ev: SoundEvent[] = []; for (let i = 0; i < n; i++) { w.step(); ev.push(...tr.update(w)); } return ev; };

{ // 1. rate limiter + spatialisation
  const r = new RateLimiter();
  ok(r.allow("thud", 0) && !r.allow("thud", MIN_GAP.thud / 2) && r.allow("thud", MIN_GAP.thud * 1.1) && r.allow("dig", 0), "rate limiter gates per kind");
  const L = { x: 50, y: 20, range: 20 };
  const near = spatialize({ x: 50, y: 20 }, L), right = spatialize({ x: 70, y: 20 }, L), far = spatialize({ x: 500, y: 20 }, L);
  ok(near.pan === 0 && near.gain === 1 && right.pan > 0.5 && spatialize({ x: 30, y: 20 }, L).pan < -0.5, "sounds pan toward their side");
  ok(far.gain > 0 && far.gain < right.gain, "distant sounds are quieter but audible");
}

// ---------- mock WebAudio ----------
function mockAudio() {
  const created: string[] = []; let started = 0;
  const param = (v = 0) => ({ value: v, setValueAtTime() { return this; }, linearRampToValueAtTime() { return this; }, exponentialRampToValueAtTime(x: number) { if (!(x > 0)) throw new Error("exp ramp to " + x); return this; }, setTargetAtTime() { return this; }, cancelScheduledValues() { return this; } });
  const node = (kind: string, extra: any = {}) => { created.push(kind); return { connect(n: any) { return n; }, disconnect() {}, ...extra }; };
  const ctx: any = {
    currentTime: 0, sampleRate: 8000, state: "running", destination: node("dest"),
    createGain: () => node("gain", { gain: param(1) }),
    createBiquadFilter: () => node("filter", { type: "lowpass", frequency: param(350), Q: param(1) }),
    createOscillator: () => node("osc", { type: "sine", frequency: param(440), start() { started++; }, stop() {} }),
    createBufferSource: () => node("src", { buffer: null, loop: false, start() { started++; }, stop() {} }),
    createStereoPanner: () => node("pan", { pan: param(0) }),
    createDynamicsCompressor: () => node("comp"),
    createBuffer: (_c: number, n: number, sr: number) => { const d = new Float32Array(n); return { duration: n / sr, getChannelData: () => d }; },
    suspend() { this.state = "suspended"; return Promise.resolve(); }, resume() { this.state = "running"; return Promise.resolve(); },
  };
  return { ctx, created, started: () => started };
}
{ // 2. sound engine: lazy, mute-aware, every kind synthesises
  const m = mockAudio(); let calls = 0;
  const s = new SoundEngine(() => { calls++; return m.ctx; });
  const L = { x: 0, y: 0, range: 20 };
  s.play({ kind: "explosion", x: 0, y: 0 }, L);
  ok(calls === 0 && m.created.length === 1 /* mock destination */, "no AudioContext before unlock (user gesture)");
  s.unlock(); s.unlock();
  ok(calls === 1 && s.ready, "unlock creates exactly one context");
  const base = m.created.length;
  const kinds: SoundKind[] = ["explosion", "cavein", "thud", "dig", "dug", "beam", "hurt", "death", "steam", "ignite"];
  let threw = false;
  try { kinds.forEach((k, i) => { m.ctx.currentTime = i; s.play({ kind: k, x: i, y: 0 }, L); }); } catch (e) { threw = true; console.log(e); }
  ok(!threw && m.created.length > base + kinds.length * 2, `all ${kinds.length} sound kinds synthesise (${m.created.length - base} nodes)`);
  m.ctx.currentTime = 50; s.play({ kind: "dig", x: 0, y: 0 }, L); const n1 = m.created.length; s.play({ kind: "dig", x: 0, y: 0 }, L);
  ok(m.created.length === n1, "rate limiter suppresses same-time repeats");
  s.setMuted(true); m.ctx.currentTime = 100; const n2 = m.created.length;
  s.play({ kind: "explosion", x: 0, y: 0 }, L); s.update({ water: 1, lava: 1, burning: 1, depth: 1 });
  ok(m.created.length === n2 && s.muted, "muted engine makes no sound");
  ok(s.toggleMute() === false, "toggleMute unmutes");
  for (let t = 100; t < 160; t += 0.5) { m.ctx.currentTime = t; s.update({ water: 1, lava: 1, burning: 0.5, depth: 0.5 }); }
  ok(m.created.length > n2 + 20, "ambience schedules drips and lava bubbles over time");
  const quiet = new SoundEngine(() => null); quiet.unlock(); quiet.play({ kind: "thud", x: 0, y: 0 }, L);
  ok(!quiet.ready, "no WebAudio available: engine stays silent without throwing");
}
{ // 3. SandWorld: fuse -> TNT gives one explosion per TNT at its block; new world adopted silently
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 160, 44, M.EMPTY); w.updateNav();
  for (let bx = 10; bx < 18; bx++) w.logic.place("fuse", blk(bx, 10));
  w.logic.place("tnt", blk(18, 10)); w.logic.place("tnt", blk(22, 10));
  w.fx.push({ type: "boom", x: 0, y: 0, t: 0 }); w.say("⚠ Cave-in near 1,1");
  const tr = new SandEventTracker();
  ok(tr.update(w).length === 0, "first sight of a world emits nothing (pre-existing fx/log ignored)");
  w.logic.ignite(blk(10, 10));
  const ev: SoundEvent[] = []; for (let i = 0; i < 120; i++) { w.step(); ev.push(...tr.update(w)); }
  const booms = ev.filter(e => e.kind === "explosion");
  ok(booms.length === 2, `two TNT -> two explosion events (${booms.length})`);
  ok(booms.some(e => Math.abs(e.x - (18 * 4 + 2)) <= 1 && Math.abs(e.y - (10 * 4 + 2)) <= 1), "explosion positioned at the TNT block (cells)");
}
{ // 4. SandWorld: collapse -> cavein + hurt/death
  const w = SandWorld.empty(M.ROCK); box(w, 40, 60, 160, 80, M.EMPTY); box(w, 40, 36, 160, 59, M.DIRT);
  w.updateNav(); w.addMole(25, 19);
  const ev = record(w, 300);
  const cave = ev.find(e => e.kind === "cavein");
  ok(!!cave && cave.x >= 40 && cave.x <= 160 && cave.y >= 36 && cave.y <= 80, `cave-in event located in the collapse (${cave?.x},${cave?.y})`);
  ok(ev.some(e => e.kind === "hurt" || e.kind === "death"), "crushed mole -> hurt/death event");
}
{ // 5. SandWorld: digging -> scrape + dug; tracking never perturbs the sim
  const mk = () => { const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 60, 43, M.EMPTY); box(w, 61, 40, 64, 43, M.DIRT); w.updateNav(); w.addMole(11, 10); w.designate("dig", blk(15, 10)); return w; };
  const a = mk(), b = mk(), tr = new SandEventTracker(); tr.update(a); const ev: SoundEvent[] = [];
  for (let i = 0; i < 200; i++) { a.step(); b.step(); ev.push(...tr.update(a)); ambientLevels(a, { x0: 0, y0: 0, x1: CW - 1, y1: CH - 1 }); }
  ok(ev.some(e => e.kind === "dig"), "mole digging -> dig scrape events");
  ok(ev.some(e => e.kind === "dug" && e.x === 15 * 4 + 2 && e.y === 10 * 4 + 2), "block dug out -> dug event at that block");
  ok(a.hash() === b.hash(), "sim is unaffected by the sound adapter");
}
{ // 6. SandWorld: beam knock and fuse ignite from mole jobs
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 100, 47, M.EMPTY); w.updateNav(); w.addMole(11, 11);
  w.designate("beam", blk(14, 11)); w.logic.place("fuse", blk(18, 11)); w.designate("ignite", blk(18, 11));
  const ev = record(w, 300);
  ok(ev.some(e => e.kind === "beam"), "pillar built -> beam event");
  ok(ev.some(e => e.kind === "ignite" && e.x === 18 * 4 + 2), "mole lit fuse -> ignite event");
}
{ // 7. ambient levels from materials
  const w = SandWorld.empty(M.ROCK); box(w, 40, 40, 120, 80, M.EMPTY);
  const reg = { x0: 30, y0: 30, x1: 130, y1: 90 }, dry = ambientLevels(w, reg);
  box(w, 40, 70, 120, 80, M.WATER); box(w, 40, 66, 120, 69, M.LAVA);
  const wet = ambientLevels(w, reg);
  ok(dry.water === 0 && dry.lava === 0 && wet.water > 0.3 && wet.lava > 0.5, `water/lava levels reflect the view (${wet.water.toFixed(2)}, ${wet.lava.toFixed(2)})`);
  ok(ambientLevels(w, { x0: 0, y0: 180, x1: 100, y1: 230 }).depth > dry.depth, "depth grows going down");
  w.logic.place("fuse", blk(12, 12)); w.logic.ignite(blk(12, 12));
  ok(ambientLevels(w, reg).burning > 0, "lit fuse raises the burning level");
}
console.log(fail ? `${fail} FAILED` : "ALL PASSED"); process.exit(fail ? 1 : 0);
