// Wiring, devices, fuses and LoRa on the 4x4-cell block layer.
// Interface used by SandWorld: step(), place(), remove(), ignite(), isIgnitable(), consumeTnt(), onExplosion().
import { M, K, KIND, ATTEN } from "./materials";
import type { SandWorld } from "./world";
import { B, BW, BH, NB, CW, FUSE_TICKS } from "./world";

export const enum D { NONE, LEVER, PLATE, FSENSOR, DOOR, FLOODGATE, PUMP, LAMP, TNT, FUSE, AND, OR, NOT, XOR, TIMER, LATCH, TX, RX, REP }
export const DEV_NAMES = ["", "Lever", "Pressure plate", "Fluid sensor", "Door", "Floodgate", "Pump", "Lamp", "TNT", "Fuse", "AND", "OR", "NOT", "XOR", "Timer", "Latch", "LoRa TX", "LoRa RX", "LoRa repeater"];
export const TOOL_DEV: Record<string, number> = { lever: D.LEVER, plate: D.PLATE, sensor: D.FSENSOR, door: D.DOOR, floodgate: D.FLOODGATE, pump: D.PUMP, lamp: D.LAMP, tnt: D.TNT, fuse: D.FUSE, and: D.AND, or: D.OR, not: D.NOT, xor: D.XOR, timer: D.TIMER, latch: D.LATCH, tx: D.TX, rx: D.RX, rep: D.REP };
export const isGate = (d: number) => d >= D.AND && d <= D.LATCH;
export const isLora = (d: number) => d >= D.TX && d <= D.REP;
export const LORA_POWER = 40;

class UnionFind { p: Int32Array; constructor(n: number) { this.p = new Int32Array(n); for (let i = 0; i < n; i++) this.p[i] = i; }
  find(a: number): number { while (this.p[a] !== a) { this.p[a] = this.p[this.p[a]]; a = this.p[a]; } return a; }
  union(a: number, b: number) { a = this.find(a); b = this.find(b); if (a !== b) this.p[b] = a; } }

export interface LoraLink { a: number; b: number; budget: number }

export class Logic {
  wire = new Uint8Array(NB); dev = new Uint8Array(NB); devP = new Int16Array(NB); devS = new Uint8Array(NB); burn = new Int16Array(NB);
  netId = new Int32Array(NB).fill(-1); netOn = new Uint8Array(1); nets = 0; dirty = true;
  devList: number[] = []; links: LoraLink[] = []; rxOn = new Uint8Array(NB); txOn = new Uint8Array(NB); loraDirty = true;
  powered = new Uint8Array(NB);
  constructor(private w: SandWorld) {}

  // ---------- editing ----------
  place(tool: string, b: number) {
    const w = this.w;
    if (tool === "wire") { if (!isGate(this.dev[b])) { this.wire[b] = 1; this.dirty = true; } return; }
    if (tool === "interact") { this.interact(b); return; }
    const d = TOOL_DEV[tool]; if (!d || this.dev[b]) return;
    if (w.blockCount(b, i => w.isSolid(i)) > 8) return; // needs open space
    if (d === D.TNT) { if (w.res.tnt <= 0) return; w.res.tnt--; }
    if (isLora(d)) { if (w.res.ore <= 0 && !(w as any).sandbox) { /* free in prototype */ } }
    this.dev[b] = d; this.devS[b] = 0; this.devP[b] = d === D.TIMER ? 40 : isLora(d) ? 1 : 0;
    if (isGate(d)) this.wire[b] = 0;
    if (d === D.DOOR) this.setDoor(b, false);
    this.dirty = true; this.loraDirty = true;
  }
  remove(b: number) {
    if (this.dev[b] === D.DOOR || this.dev[b] === D.FLOODGATE) this.setDoor(b, true);
    if (this.dev[b] === D.TNT) this.w.res.tnt++;
    if (this.wire[b] || this.dev[b]) { this.wire[b] = 0; this.dev[b] = 0; this.dirty = true; this.loraDirty = true; }
  }
  interact(b: number) {
    const d = this.dev[b];
    if (d === D.LEVER) this.devS[b] ^= 1;
    else if (isLora(d)) this.devP[b] = (this.devP[b] + 1) % 8;
    else if (d === D.TIMER) this.devP[b] = this.devP[b] === 20 ? 40 : this.devP[b] === 40 ? 80 : 20;
    this.loraDirty = true;
  }
  isIgnitable(b: number) { return this.dev[b] === D.FUSE || this.dev[b] === D.TNT; }
  ignite(b: number) { if (this.dev[b] === D.FUSE && this.burn[b] === 0) this.burn[b] = FUSE_TICKS; else if (this.dev[b] === D.TNT) this.w.boomQueue.push(b); }
  consumeTnt(b: number) { if (this.dev[b] !== D.TNT) return false; this.dev[b] = 0; this.dirty = true; return true; }
  onExplosion(cx: number, cy: number, R: number) {
    for (let b = 0; b < NB; b++) {
      if (!this.wire[b] && !this.dev[b]) continue;
      const d = Math.hypot((b % BW) * B + 2 - cx, ((b / BW) | 0) * B + 2 - cy);
      if (this.dev[b] === D.TNT && d <= R + 6) { this.w.boomQueue.push(b); continue; } // chain reaction
      if (this.dev[b] === D.FUSE && d <= R + 4) { this.ignite(b); continue; }
      if (d <= R * 0.8) { if (this.dev[b] === D.DOOR || this.dev[b] === D.FLOODGATE) this.setDoor(b, true); this.wire[b] = 0; this.dev[b] = 0; this.dirty = true; this.loraDirty = true; }
    }
  }

  // ---------- door cells ----------
  setDoor(b: number, open: boolean) {
    const w = this.w, x0 = (b % BW) * B, y0 = ((b / BW) | 0) * B;
    for (let y = y0; y < y0 + B; y++) for (let x = x0; x < x0 + B; x++) {
      const i = w.ci(x, y), m = w.mat[i];
      if (open) { if (m === M.DOOR) w.setCell(i, M.EMPTY); }
      else if (KIND[m] !== K.SOLID || w.loose[i]) {
        if (m !== M.EMPTY) { // displace liquid/powder upward to keep mass
          for (let yy = y0 - 1; yy >= Math.max(0, y0 - 12); yy--) { const j = w.ci(x, yy); if (w.mat[j] === M.EMPTY) { w.setCell(j, m, w.loose[i]); break; } }
        }
        w.setCell(i, M.DOOR);
      }
    }
  }

  // ---------- networks: union-find over wire blocks ----------
  rebuild() {
    this.dirty = false;
    const uf = new UnionFind(NB);
    for (let b = 0; b < NB; b++) {
      if (!this.wire[b]) continue;
      const x = b % BW;
      if (x < BW - 1 && this.wire[b + 1]) uf.union(b, b + 1);
      if (b + BW < NB && this.wire[b + BW]) uf.union(b, b + BW);
    }
    const map = new Map<number, number>(); this.netId.fill(-1);
    for (let b = 0; b < NB; b++) if (this.wire[b]) { const r = uf.find(b); let id = map.get(r); if (id === undefined) { id = map.size; map.set(r, id); } this.netId[b] = id; }
    this.nets = map.size; this.netOn = new Uint8Array(Math.max(1, this.nets));
    this.devList = []; for (let b = 0; b < NB; b++) if (this.dev[b]) this.devList.push(b);
  }
  private nb(b: number) { const x = b % BW; return [b, b >= BW ? b - BW : -1, b < NB - BW ? b + BW : -1, x > 0 ? b - 1 : -1, x < BW - 1 ? b + 1 : -1]; }
  private drive(on: Uint8Array, b: number) { for (const c of this.nb(b)) if (c >= 0 && this.wire[c]) on[this.netId[c]] = 1; }
  private isPowered(b: number) { for (const c of this.nb(b)) if (c >= 0 && this.wire[c] && this.netOn[this.netId[c]]) return true; return false; }
  private wireOn(b: number) { return b >= 0 && b < NB && this.wire[b] === 1 && this.netOn[this.netId[b]] === 1; }

  // ---------- per tick ----------
  step() {
    const w = this.w;
    if (this.tick5()) this.checkSevering();
    if (this.dirty) this.rebuild();
    this.stepFuses();
    // 1. sample inputs -> drive nets
    const on = new Uint8Array(Math.max(1, this.nets));
    for (const b of this.devList) {
      const d = this.dev[b];
      let out = false;
      switch (d) {
        case D.LEVER: out = this.devS[b] === 1; break;
        case D.PLATE: out = w.moles.some(m => !m.dead && m.by * BW + m.bx === b) || w.blockCount(b, i => KIND[w.mat[i]] === K.POWDER || (w.loose[i] === 1 && KIND[w.mat[i]] === K.SOLID)) >= 3; this.devS[b] = out ? 1 : 0; break;
        case D.FSENSOR: out = w.blockCount(b, i => KIND[w.mat[i]] === K.LIQUID) >= 4; this.devS[b] = out ? 1 : 0; break;
        case D.RX: out = this.rxOn[b] === 1; this.devS[b] = out ? 1 : 0; break;
      }
      if (out) this.drive(on, b);
      if (isGate(d) && this.devS[b]) { const e = b % BW < BW - 1 ? b + 1 : -1; if (e >= 0 && this.wire[e]) on[this.netId[e]] = 1; }
    }
    this.netOn = on;
    // 2. gates: read west (a) / north (b) this tick, output next tick (1-tick gate delay)
    const nextS: [number, number][] = [];
    for (const b of this.devList) {
      const d = this.dev[b]; if (!isGate(d)) continue;
      const a = this.wireOn(b % BW > 0 ? b - 1 : -1), c = this.wireOn(b - BW);
      let s = this.devS[b];
      switch (d) {
        case D.AND: s = a && c ? 1 : 0; break;
        case D.OR: s = a || c ? 1 : 0; break;
        case D.NOT: s = a ? 0 : 1; break;
        case D.XOR: s = a !== c ? 1 : 0; break;
        case D.TIMER: { const P = this.devP[b] || 40; s = (w.tick % P) < P / 2 && (!this.wire[b - 1] || a) ? 1 : 0; break; }
        case D.LATCH: if (a) s = 1; else if (c) s = 0; break;
      }
      nextS.push([b, s]);
    }
    for (const [b, s] of nextS) this.devS[b] = s;
    // 3. LoRa
    if (this.loraDirty || w.tick % 15 === 0) this.rebuildLinks();
    this.propagateLora();
    // 4. outputs
    for (const b of this.devList) {
      const d = this.dev[b]; if (!d) continue;
      const p = this.isPowered(b); this.powered[b] = p ? 1 : 0;
      switch (d) {
        case D.DOOR: if ((this.devS[b] === 1) !== p) { this.devS[b] = p ? 1 : 0; this.setDoor(b, p); } break;
        case D.FLOODGATE: { const closed = p; if ((this.devS[b] === 1) !== closed) { this.devS[b] = closed ? 1 : 0; this.setDoor(b, !closed); } break; }
        case D.PUMP: if (p) this.pump(b); break;
        case D.LAMP: this.devS[b] = p ? 1 : 0; break;
        case D.TNT: if (p) w.boomQueue.push(b); break;
        case D.TX: this.txOn[b] = p ? 1 : 0; break;
      }
    }
  }
  private t5 = 0; private tick5() { return ++this.t5 % 5 === 0; }

  /** Lava or burial destroys wires and devices (LoRa nodes too: counterplay). */
  checkSevering() {
    const w = this.w;
    for (let b = 0; b < NB; b++) {
      if (!this.wire[b] && !this.dev[b]) continue;
      const d = this.dev[b];
      if (d === D.DOOR || d === D.FLOODGATE) continue;
      const lava = w.liquidIn(b, M.LAVA), buried = w.blockCount(b, i => w.isSolid(i) && w.mat[i] !== M.DOOR && w.mat[i] !== M.WOOD) >= 12;
      if (lava >= 2 || buried) {
        if (d === D.TNT && lava) { w.boomQueue.push(b); continue; }
        if (d === D.FUSE && lava) { this.ignite(b); continue; }
        this.wire[b] = 0; this.dev[b] = 0; this.dirty = true; this.loraDirty = true;
        w.say(`⚡ ${d ? DEV_NAMES[d] : "Wire"} destroyed by ${lava ? "lava" : "cave-in"}`);
      }
    }
  }
  stepFuses() {
    const w = this.w, ign: number[] = [];
    for (const b of this.devList) {
      if (this.dev[b] !== D.FUSE) continue;
      if (this.burn[b] === 0 && w.liquidIn(b, M.LAVA) > 0) this.burn[b] = FUSE_TICKS;
      if (this.burn[b] <= 0) continue;
      if (w.liquidIn(b, M.WATER) >= 4) { this.burn[b] = 0; continue; }
      if (w.tick % 2 === 0) w.fx.push({ type: "spark", x: (b % BW) * B * 4 + 8, y: ((b / BW) | 0) * B * 4 + 12, t: 0 });
      if (--this.burn[b] === 0) {
        this.dev[b] = 0; this.dirty = true;
        for (const n of this.nb(b)) { if (n < 0 || n === b) continue; if (this.dev[n] === D.FUSE && this.burn[n] === 0) ign.push(n); if (this.dev[n] === D.TNT) w.boomQueue.push(n); }
      }
    }
    for (const n of ign) this.burn[n] = FUSE_TICKS;
  }
  pump(b: number) {
    const w = this.w, bx = b % BW; let by = ((b / BW) | 0) - 1, crossed = false, out = -1;
    for (let k = 0; k < 16 && by >= 0; k++, by--) {
      const ob = by * BW + bx, solid = w.blockCount(ob, i => w.isSolid(i));
      if (solid >= 12) { crossed = true; continue; }
      if (crossed && w.blockCount(ob, i => w.mat[i] === M.EMPTY) > 0) { out = ob; break; }
    }
    if (out < 0) return;
    let moved = 0; const x0 = bx * B, y0 = ((b / BW) | 0) * B, ox = (out % BW) * B, oy = ((out / BW) | 0) * B;
    for (let y = y0 + B - 1; y >= y0 && moved < 3; y--) for (let x = x0; x < x0 + B && moved < 3; x++) {
      const i = w.ci(x, y), m = w.mat[i]; if (KIND[m] !== K.LIQUID) continue;
      for (let yy = oy + B - 1; yy >= oy; yy--) { let done = false; for (let xx = ox; xx < ox + B; xx++) { const j = w.ci(xx, yy); if (w.mat[j] === M.EMPTY) { w.setCell(j, m); w.setCell(i, M.EMPTY); moved++; done = true; break; } } if (done) break; }
    }
  }

  // ---------- LoRa: links by attenuation budget, BFS through repeaters ----------
  linkBudget(a: number, b: number) {
    const w = this.w;
    const x0 = (a % BW) * B + 2, y0 = ((a / BW) | 0) * B + 2, x1 = (b % BW) * B + 2, y1 = ((b / BW) | 0) * B + 2;
    let budget = LORA_POWER - Math.hypot(x1 - x0, y1 - y0) / B;
    // Bresenham over cells
    let x = x0, y = y0; const dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0), sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1; let err = dx + dy;
    while (!(x === x1 && y === y1)) {
      budget -= ATTEN[w.mat[y * CW + x]]; if (budget < 0) return budget;
      const e2 = 2 * err; if (e2 >= dy) { err += dy; x += sx; } if (e2 <= dx) { err += dx; y += sy; }
    }
    return budget;
  }
  rebuildLinks() {
    this.loraDirty = false; this.links = [];
    const nodes = this.devList.filter(b => isLora(this.dev[b]));
    for (let i = 0; i < nodes.length; i++) for (let j = i + 1; j < nodes.length; j++) {
      const bud = this.linkBudget(nodes[i], nodes[j]); if (bud >= 0) this.links.push({ a: nodes[i], b: nodes[j], budget: bud });
    }
  }
  propagateLora() {
    this.rxOn.fill(0);
    const adj = new Map<number, number[]>();
    for (const l of this.links) { (adj.get(l.a) || adj.set(l.a, []).get(l.a)!).push(l.b); (adj.get(l.b) || adj.set(l.b, []).get(l.b)!).push(l.a); }
    for (const tx of this.devList) {
      if (this.dev[tx] !== D.TX || !this.txOn[tx]) continue;
      const ch = this.devP[tx], seen = new Set([tx]), q = [tx];
      while (q.length) {
        const n = q.shift()!;
        for (const m of adj.get(n) || []) {
          if (seen.has(m)) continue; seen.add(m);
          const d = this.dev[m];
          if (d === D.RX && this.devP[m] === ch) this.rxOn[m] = 1;
          if (d === D.REP) q.push(m); // repeaters forward every channel
        }
      }
    }
  }
}
export { BH };
