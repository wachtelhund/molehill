// Molehill v2: Noita-style falling-sand world at 4 px per cell, with a coarse
// 4x4-cell "block" layer for moles, jobs, wiring and devices.
import { M, K, KIND, ANCHOR, SPAN, DIG, HARD, isStructMat } from "./materials";
import { Logic } from "./logic";

export const CW = 384, CH = 240, NC = CW * CH;     // cells
export const B = 4, BW = CW / B, BH = CH / B, NB = BW * BH; // blocks (16 px)
export const CHUNK = 32, CX = CW / CHUNK, CY = CH / CHUNK, NCH = CX * CY;
export const SIM_HZ = 30, COLLAPSE_DELAY = 24, TNT_R = 12, FUSE_TICKS = 8, MOVE_TICKS = 4;
export const SURFACE = 24; // cell row where ground starts

export function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

export type JobType = "dig" | "beam" | "light";
export interface Job { type: JobType; b: number }
export interface Mole { id: number; bx: number; by: number; px: number; py: number; hp: number; o2: number; job: Job | null; path: number[]; moveT: number; workT: number; dead: boolean; hurtT: number; facing: number; fleeing: number }
export interface Fx { type: "boom" | "dust" | "steam" | "spark"; x: number; y: number; t: number } // x,y in px
export interface Ev { tick: number; text: string }

export class SandWorld {
  tick = 0; rng: () => number;
  // ---- cell layer (struct of arrays) ----
  mat = new Uint8Array(NC); loose = new Uint8Array(NC); shade = new Uint8Array(NC);
  life = new Uint8Array(NC); moved = new Uint8Array(NC); pending = new Uint8Array(NC);
  support = new Uint8Array(NC);
  // ---- chunk dirty rects (current / next), Noita-style ----
  rx0 = new Int16Array(NCH); ry0 = new Int16Array(NCH); rx1 = new Int16Array(NCH); ry1 = new Int16Array(NCH);
  nx0 = new Int16Array(NCH); ny0 = new Int16Array(NCH); nx1 = new Int16Array(NCH); ny1 = new Int16Array(NCH);
  // ---- block layer ----
  nav = new Uint8Array(NB);          // 1 walkable
  digDes = new Uint8Array(NB); beamDes = new Uint8Array(NB); lightReq = new Uint8Array(NB);
  moles: Mole[] = []; fx: Fx[] = []; log: Ev[] = [];
  res = { wood: 30, tnt: 12, rubble: 0, ore: 0 };
  structDirty = true; lastStruct = -99; pendingList: number[] = []; shake = 0; lastCave = -999;
  reserved = new Set<string>(); boomQueue: number[] = [];
  scripts: { tick: number; fn: (w: SandWorld) => void }[] = [];
  moleDmg = new Float32Array(16);
  logic: Logic;

  constructor(seed = 1, gen = true) {
    this.rng = mulberry32(seed);
    for (let i = 0; i < NC; i++) this.shade[i] = (this.rng() * 255) | 0;
    this.clearRects(true);
    this.logic = new Logic(this);
    if (gen) generate(this, seed);
    this.wakeAll();
  }
  static empty(fill: number, seed = 1) { const w = new SandWorld(seed, false); w.mat.fill(fill); w.wakeAll(); return w; }
  say(text: string) { this.log.push({ tick: this.tick, text }); if (this.log.length > 40) this.log.shift(); }

  // ---------- cell helpers ----------
  ci(x: number, y: number) { return y * CW + x; }
  isSolid(i: number) { const k = KIND[this.mat[i]]; return k === K.SOLID || k === K.POWDER; }
  isStatic(i: number) { return KIND[this.mat[i]] === K.SOLID && !this.loose[i]; }
  setCell(i: number, m: number, loose = 0) {
    const was = this.mat[i];
    this.mat[i] = m; this.loose[i] = loose; this.life[i] = m === M.STEAM ? 60 : m === M.FIRE ? 80 : 0;
    if (isStructMat(was) || isStructMat(m)) this.structDirty = true;
    this.wake(i % CW, (i / CW) | 0);
  }
  fillRect(x0: number, y0: number, x1: number, y1: number, m: number) { for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) this.setCell(this.ci(x, y), m); }
  wake(x: number, y: number) {
    for (const [dx, dy] of [[0, 0], [-1, 0], [1, 0], [0, -1], [0, 1]]) {
      const xx = x + dx, yy = y + dy; if (xx < 0 || yy < 0 || xx >= CW || yy >= CH) continue;
      const c = ((yy / CHUNK) | 0) * CX + ((xx / CHUNK) | 0);
      if (xx < this.nx0[c]) this.nx0[c] = xx; if (xx > this.nx1[c]) this.nx1[c] = xx;
      if (yy < this.ny0[c]) this.ny0[c] = yy; if (yy > this.ny1[c]) this.ny1[c] = yy;
    }
  }
  wakeRect(x0: number, y0: number, x1: number, y1: number) { for (let y = Math.max(0, y0); y <= Math.min(CH - 1, y1); y += 8) for (let x = Math.max(0, x0); x <= Math.min(CW - 1, x1); x += 8) this.wake(x, y); this.wake(Math.min(CW - 1, x1), Math.min(CH - 1, y1)); }
  wakeAll() { for (let c = 0; c < NCH; c++) { const cx = c % CX, cy = (c / CX) | 0; this.nx0[c] = cx * CHUNK; this.ny0[c] = cy * CHUNK; this.nx1[c] = cx * CHUNK + CHUNK - 1; this.ny1[c] = cy * CHUNK + CHUNK - 1; } }
  clearRects(both = false) { if (both) { this.rx0.fill(32767); this.ry0.fill(32767); this.rx1.fill(-1); this.ry1.fill(-1); } this.nx0.fill(32767); this.ny0.fill(32767); this.nx1.fill(-1); this.ny1.fill(-1); }
  activeChunks() { let n = 0; for (let c = 0; c < NCH; c++) if (this.rx1[c] >= 0) n++; return n; }

  // ---------- tick (update order) ----------
  step() {
    this.tick++;
    for (const s of this.scripts) if (s.tick === this.tick) s.fn(this);
    this.logic.step();                           // 1-4 inputs, nets, LoRa, devices, fuses
    this.resolveExplosions();                    // 6
    if (this.structDirty && this.tick - this.lastStruct >= 3) this.resolveSupport(); // 7
    this.stepPending();
    this.stepCA();                               // 8-10 powders, liquids, gases, reactions
    if (this.tick % 3 === 0) this.updateNav();
    if (this.tick % 10 === 0) this.assignJobs(); // 11
    for (const m of this.moles) this.stepMole(m); // 12
    for (const f of this.fx) f.t++;
    this.fx = this.fx.filter(f => f.t < 40);
    if (this.shake > 0) this.shake--;
  }

  // ---------- falling-sand CA over dirty rects, bottom-up, alternating x ----------
  stepCA() {
    // promote next rects to current
    this.rx0.set(this.nx0); this.ry0.set(this.ny0); this.rx1.set(this.nx1); this.ry1.set(this.ny1); this.clearRects();
    const stamp = (this.tick & 255) || 1, odd = this.tick & 1;
    this.moleDmg.fill(0);
    for (let y = CH - 1; y >= 0; y--) {
      const cy = (y / CHUNK) | 0;
      for (let k = 0; k < CX; k++) {
        const cx = odd ? CX - 1 - k : k, c = cy * CX + cx;
        if (this.rx1[c] < 0 || y < this.ry0[c] || y > this.ry1[c]) continue;
        const a = this.rx0[c], b = this.rx1[c];
        if (odd) { for (let x = b; x >= a; x--) this.updateCell(x, y, stamp); }
        else { for (let x = a; x <= b; x++) this.updateCell(x, y, stamp); }
      }
    }
    for (let i = 0; i < this.moles.length && i < 16; i++) if (this.moleDmg[i] > 0) this.hurt(this.moles[i], Math.min(25, this.moleDmg[i]), "crushed by a cave-in");
  }
  private move(i: number, j: number, stamp: number) {
    const m = this.mat, l = this.loose, s = this.shade, f = this.life;
    let t = m[i]; m[i] = m[j]; m[j] = t; t = l[i]; l[i] = l[j]; l[j] = t; t = s[i]; s[i] = s[j]; s[j] = t; t = f[i]; f[i] = f[j]; f[j] = t;
    this.moved[j] = stamp; this.moved[i] = stamp;
    this.wake(i % CW, (i / CW) | 0); this.wake(j % CW, (j / CW) | 0);
  }
  private canSink(j: number) { const k = KIND[this.mat[j]]; return k === K.EMPTY || k === K.LIQUID || k === K.GAS; }
  updateCell(x: number, y: number, stamp: number) {
    const i = y * CW + x; if (this.moved[i] === stamp) return;
    const m = this.mat[i]; if (m === M.EMPTY) return;
    const kind = KIND[m];
    if (kind === K.POWDER || (kind === K.SOLID && this.loose[i])) {
      if (y >= CH - 1) return;
      const d = i + CW;
      if (this.canSink(d) && (KIND[this.mat[d]] !== K.LIQUID || (this.rng() < 0.5 && (y === 0 || KIND[this.mat[i - CW]] !== K.POWDER && !this.loose[i - CW])))) { this.crushCheck(x, y + 1, 1); this.move(i, d, stamp); return; }
      const steep = m === M.GRAVEL || m === M.CLAY; // higher angle of repose
      if (steep && this.rng() < 0.4) return;
      const dir = this.rng() < 0.5 ? -1 : 1;
      for (const s of [dir, -dir]) {
        const nx = x + s; if (nx < 0 || nx >= CW) continue;
        if (this.mat[d + s] === M.EMPTY && this.mat[i + s] === M.EMPTY) { this.move(i, d + s, stamp); return; } // piles don't slump into liquid: dams hold
      }
      // settled powder: compact back into static soil after a long rest (keeps piles stable)
      return;
    }
    if (kind === K.LIQUID) {
      if (m === M.LAVA) {
        for (const n of [i - CW, i + CW, x > 0 ? i - 1 : -1, x < CW - 1 ? i + 1 : -1]) {
          if (n < 0 || n >= NC) continue;
          const nm = this.mat[n];
          if (nm === M.WATER) { this.setCell(i, M.STONE); this.setCell(n, M.STEAM); if (this.rng() < 0.2) this.fx.push({ type: "steam", x: (n % CW) * 4, y: ((n / CW) | 0) * 4, t: 0 }); return; }
          if (nm === M.WOOD && this.rng() < 0.05) this.setCell(n, M.FIRE);
        }
        if (this.rng() > 0.3) { this.wake(x, y); return; } // viscosity
      }
      if (y < CH - 1) {
        const d = i + CW;
        if (this.mat[d] === M.EMPTY || this.mat[d] === M.STEAM) { this.move(i, d, stamp); return; }
        const dir = this.rng() < 0.5 ? -1 : 1;
        for (const s of [dir, -dir]) { const nx = x + s; if (nx >= 0 && nx < CW && this.mat[d + s] === M.EMPTY) { this.move(i, d + s, stamp); return; } }
      }
      const disp = m === M.WATER ? 5 : 2, dir = (this.tick + x) & 1 ? 1 : -1;
      for (const s of [dir, -dir]) {
        let best = -1;
        for (let k = 1; k <= disp; k++) { const nx = x + s * k; if (nx < 0 || nx >= CW || this.mat[i + s * k] !== M.EMPTY) break; best = i + s * k; if (y < CH - 1 && this.mat[best + CW] === M.EMPTY) break; }
        if (best >= 0) { this.move(i, best, stamp); return; }
      }
      return;
    }
    if (m === M.STEAM) {
      if (--this.life[i] === 0) { this.setCell(i, this.rng() < 0.15 ? M.WATER : M.EMPTY); return; }
      this.wake(x, y);
      const dx = ((this.rng() * 3) | 0) - 1, j = i - CW + dx;
      if (y > 0 && x + dx >= 0 && x + dx < CW && this.mat[j] === M.EMPTY) this.move(i, j, stamp);
      else if (x + dx >= 0 && x + dx < CW && this.mat[i + dx] === M.EMPTY) this.move(i, i + dx, stamp);
      return;
    }
    if (m === M.FIRE) {
      this.wake(x, y);
      for (const n of [i - CW, i + CW, i - 1, i + 1]) {
        if (n < 0 || n >= NC) continue;
        if (this.mat[n] === M.WATER) { this.setCell(i, M.STEAM); return; }
        if (this.mat[n] === M.WOOD && this.rng() < 0.03) this.setCell(n, M.FIRE);
      }
      if (--this.life[i] === 0) this.setCell(i, M.EMPTY);
    }
  }
  /** Falling material entering a block occupied by a mole hurts it. */
  crushCheck(x: number, y: number, dmg: number) {
    const bx = (x / B) | 0, by = (y / B) | 0;
    for (let k = 0; k < this.moles.length && k < 16; k++) { const mo = this.moles[k]; if (!mo.dead && mo.bx === bx && mo.by === by) this.moleDmg[k] += dmg; }
  }

  // ---------- structural support: 0-1 BFS (up free, sideways 1, hanging 3) ----------
  resolveSupport() {
    this.structDirty = false; this.lastStruct = this.tick;
    const s = this.support, mat = this.mat, loose = this.loose; s.fill(255);
    const dq = new Int32Array(NC * 2); let head = NC, tail = NC;
    for (let i = 0; i < NC; i++) if (ANCHOR[mat[i]]) { s[i] = 0; dq[tail++] = i; }
    while (head < tail) {
      const i = dq[head++], x = i % CW, si = s[i];
      // k: 0 up, 1 down, 2 left, 3 right
      for (let k = 0; k < 4; k++) {
        const n = k === 0 ? i - CW : k === 1 ? i + CW : k === 2 ? (x > 0 ? i - 1 : -1) : (x < CW - 1 ? i + 1 : -1);
        if (n < 0 || n >= NC) continue;
        const t = mat[n], span = SPAN[t]; if (span === 0 || loose[n]) continue;
        const cost = k === 0 ? 0 : k === 1 ? 3 : 1;
        const c = si + cost;
        if (c < s[n] && c <= span) { s[n] = c; if (cost === 0) dq[--head] = n; else dq[tail++] = n; }
      }
    }
    this.pendingList = [];
    for (let i = 0; i < NC; i++) {
      const t = mat[i];
      if (SPAN[t] > 0 && !loose[i] && s[i] === 255) { if (this.pending[i] === 0) this.pending[i] = COLLAPSE_DELAY; this.pendingList.push(i); }
      else this.pending[i] = 0;
    }
  }
  stepPending() {
    if (!this.pendingList.length) return;
    let fell = 0, fx = 0, fy = 0; const keep: number[] = [];
    for (const i of this.pendingList) {
      if (this.pending[i] === 0) continue;
      if (--this.pending[i] === 0) {
        if (this.support[i] === 255 && SPAN[this.mat[i]] > 0 && !this.loose[i]) {
          if (this.mat[i] === M.WOOD) this.mat[i] = M.SAND; // broken beam -> splinters
          this.loose[i] = 1; this.structDirty = true; this.wake(i % CW, (i / CW) | 0); fell++; fx = i % CW; fy = (i / CW) | 0;
        }
      } else keep.push(i);
    }
    this.pendingList = keep;
    if (fell) { this.shake = Math.max(this.shake, Math.min(10, 2 + fell / 40)); if (this.tick - this.lastCave > 60) { this.say(`⚠ Cave-in near ${(fx / B) | 0},${(fy / B) | 0}`); this.lastCave = this.tick; } }
  }

  // ---------- explosions ----------
  explode(cx: number, cy: number, R = TNT_R) {
    this.fx.push({ type: "boom", x: cx * 4, y: cy * 4, t: 0 }); this.shake = 14;
    const R2 = R + 8;
    for (let dy = -R2; dy <= R2; dy++) for (let dx = -R2; dx <= R2; dx++) {
      const x = cx + dx, y = cy + dy; if (x < 0 || y < 0 || x >= CW || y >= CH) continue;
      const d = Math.hypot(dx, dy); if (d > R2) continue;
      const i = this.ci(x, y), m = this.mat[i], p = 1 - d / (R + 1);
      if (d <= R && m !== M.EMPTY && KIND[m] !== K.LIQUID && p * 10 >= HARD[m]) {
        if (m !== M.BEDROCK) this.setCell(i, this.rng() < 0.15 && d > R * 0.5 ? M.SAND : (this.rng() < 0.05 ? M.FIRE : M.EMPTY), 0);
      } else if (SPAN[m] > 0 && m !== M.WOOD) { this.loose[i] = 1; this.structDirty = true; this.wake(x, y); } // shock loosens soil
    }
    this.wakeRect(cx - R2, cy - R2, cx + R2, cy + R2);
    for (const mo of this.moles) { const d = Math.hypot(mo.bx * B + 2 - cx, mo.by * B + 2 - cy); if (d < R * 1.3) this.hurt(mo, Math.round(90 * (1 - d / (R * 1.3))), "blown up"); }
    this.logic.onExplosion(cx, cy, R);
    this.say(`💥 Explosion at ${(cx / B) | 0},${(cy / B) | 0}`);
  }
  resolveExplosions() {
    const q = [...new Set(this.boomQueue)].sort((a, b) => a - b); this.boomQueue = [];
    for (const b of q) { if (!this.logic.consumeTnt(b)) continue; this.explode((b % BW) * B + 2, ((b / BW) | 0) * B + 2); }
  }

  // ---------- block layer ----------
  blockCount(b: number, pred: (i: number) => boolean) {
    const x0 = (b % BW) * B, y0 = ((b / BW) | 0) * B; let n = 0;
    for (let y = y0; y < y0 + B; y++) for (let x = x0; x < x0 + B; x++) if (pred(y * CW + x)) n++;
    return n;
  }
  liquidIn(b: number, m: number) { return this.blockCount(b, i => this.mat[i] === m); }
  updateNav() {
    for (let b = 0; b < NB; b++) {
      const x0 = (b % BW) * B, y0 = ((b / BW) | 0) * B; let solid = 0, lava = 0;
      for (let y = y0; y < y0 + B; y++) for (let x = x0; x < x0 + B; x++) { const m = this.mat[y * CW + x], k = KIND[m]; if (k === K.SOLID || k === K.POWDER) solid++; if (m === M.LAVA) lava++; }
      this.nav[b] = solid <= 6 && lava < 3 ? 1 : 0;
    }
  }
  nb4(b: number) { const x = b % BW; return [b >= BW ? b - BW : -1, b < NB - BW ? b + BW : -1, x > 0 ? b - 1 : -1, x < BW - 1 ? b + 1 : -1]; }
  bfs(from: number, ok: (b: number) => boolean = b => this.nav[b] === 1) {
    const dist = new Int16Array(NB).fill(-1), par = new Int32Array(NB).fill(-1), q = new Int32Array(NB);
    let h = 0, t = 0; dist[from] = 0; q[t++] = from;
    while (h < t) { const i = q[h++]; for (const n of this.nb4(i)) if (n >= 0 && dist[n] < 0 && ok(n)) { dist[n] = dist[i] + 1; par[n] = i; q[t++] = n; } }
    return { dist, par, order: q.subarray(0, t) };
  }
  pathTo(par: Int32Array, from: number, to: number) { const p: number[] = []; let c = to; while (c !== from && c >= 0) { p.push(c); c = par[c]; } return c === from ? p.reverse() : null; }
  diggableCells(b: number) { return this.blockCount(b, i => DIG[this.mat[i]] > 0); }

  designate(tool: string, b: number) {
    if (b < 0 || b >= NB) return;
    switch (tool) {
      case "dig": if (this.diggableCells(b)) this.digDes[b] = 1; break;
      case "beam": if (this.nav[b]) this.beamDes[b] = 1; break;
      case "ignite": if (this.logic.isIgnitable(b)) this.lightReq[b] = 1; break;
      case "cancel": this.digDes[b] = 0; this.beamDes[b] = 0; this.lightReq[b] = 0; this.logic.remove(b); break;
      case "water": case "lava": case "sand": {
        const m = tool === "water" ? M.WATER : tool === "lava" ? M.LAVA : M.SAND;
        const x0 = (b % BW) * B, y0 = ((b / BW) | 0) * B;
        for (let y = y0; y < y0 + B; y++) for (let x = x0; x < x0 + B; x++) { const i = this.ci(x, y); if (this.mat[i] === M.EMPTY) this.setCell(i, m); }
        break;
      }
      default: this.logic.place(tool, b);
    }
  }

  // ---------- moles ----------
  addMole(bx: number, by: number) { this.moles.push({ id: this.moles.length, bx, by, px: bx, py: by, hp: 100, o2: 100, job: null, path: [], moveT: 0, workT: 0, dead: false, hurtT: 0, facing: 1, fleeing: 0 }); }
  hurt(m: { hp: number; hurtT: number; dead: boolean; id: number }, dmg: number, why: string) {
    if (m.dead) return; m.hp -= dmg; m.hurtT = 8;
    if (m.hp <= 0) { m.dead = true; m.hp = 0; this.dropJob(m as Mole); this.say(`☠ Mole #${m.id} ${why}`); }
  }
  dropJob(m: Mole) { if (m.job) this.reserved.delete(m.job.type + m.job.b); m.job = null; m.path = []; m.workT = 0; }
  /** Danger score of a block: liquid inside, lava near, cracking ceiling above. */
  danger(b: number) {
    if (this.liquidIn(b, M.WATER) >= 6 || this.liquidIn(b, M.LAVA) >= 1) return true;
    for (const n of this.nb4(b)) if (n >= 0 && this.liquidIn(n, M.LAVA) >= 2) return true;
    const x0 = (b % BW) * B, y0 = ((b / BW) | 0) * B;
    for (let y = Math.max(0, y0 - 12); y < y0; y++) for (let x = x0; x < x0 + B; x++) { const i = y * CW + x; if (this.pending[i] > 0 || (this.loose[i] && KIND[this.mat[i]] === K.SOLID && this.moved[i] === ((this.tick & 255) || 1))) return true; }
    return false;
  }
  assignJobs() {
    for (const m of this.moles) {
      if (m.dead || m.job || m.fleeing) continue;
      const start = m.by * BW + m.bx; const { dist, par } = this.bfs(start);
      let best: Job | null = null, bestStand = -1, bestScore = 1e9;
      for (let b = 0; b < NB; b++) {
        let type: JobType | null = null, self = false, pri = 0;
        if (this.lightReq[b]) { type = "light"; self = true; pri = 60; }
        else if (this.digDes[b]) { type = "dig"; pri = 20; }
        else if (this.beamDes[b] && this.res.wood > 0) { type = "beam"; self = true; pri = 40; }
        if (!type || this.reserved.has(type + b)) continue;
        let s = -1;
        if (self && dist[b] >= 0) s = b;
        else { let bd = 1e9; for (const n of this.nb4(b)) if (n >= 0 && dist[n] >= 0 && dist[n] < bd) { bd = dist[n]; s = n; } }
        if (s < 0) continue;
        const score = dist[s] - pri; if (score < bestScore) { bestScore = score; best = { type, b }; bestStand = s; }
      }
      if (best) { m.job = best; this.reserved.add(best.type + best.b); m.path = this.pathTo(par, start, bestStand) || []; m.workT = 0; }
    }
  }
  stepMole(m: Mole) {
    m.px += (m.bx - m.px) * 0.3; m.py += (m.by - m.py) * 0.3; if (m.hurtT > 0) m.hurtT--;
    if (m.dead) return;
    const here = m.by * BW + m.bx;
    // environment damage
    const lava = this.liquidIn(here, M.LAVA), water = this.liquidIn(here, M.WATER);
    if (lava) this.hurt(m, 2 + lava, "melted in lava");
    if (water >= 10) { m.o2 -= 0.6; if (m.o2 <= 0) { m.o2 = 0; this.hurt(m, 1, "drowned"); } } else m.o2 = Math.min(100, m.o2 + 2);
    if (!this.nav[here] && this.blockCount(here, i => this.isSolid(i)) >= 14) { // buried: try to climb out
      const up = here - BW; if (up >= 0 && this.nav[up]) { m.by--; } else this.hurt(m, 2, "buried alive");
    }
    if (m.dead) return;
    // flee
    if (this.tick % 5 === 0 && !m.fleeing && this.danger(here)) {
      this.dropJob(m);
      const { par, order } = this.bfs(here);
      for (const b of order) if (b !== here && !this.danger(b) && !this.danger(b - BW >= 0 ? b - BW : b)) { m.path = this.pathTo(par, here, b) || []; m.fleeing = 60; break; }
      if (m.fleeing) this.say(`🏃 Mole #${m.id} flees`);
    }
    if (m.fleeing) { m.fleeing--; if (!m.path.length) m.fleeing = 0; }
    if (m.path.length) {
      const slow = water >= 8 ? 2 : 1, speed = m.fleeing ? MOVE_TICKS - 1 : MOVE_TICKS;
      if (++m.moveT >= speed * slow) {
        m.moveT = 0; const n = m.path[0];
        if (!this.nav[n]) { this.dropJob(m); m.fleeing = 0; return; }
        m.path.shift(); const nx = n % BW; if (nx !== m.bx) m.facing = nx > m.bx ? 1 : -1;
        m.bx = nx; m.by = (n / BW) | 0;
      }
      return;
    }
    const j = m.job; if (!j) return;
    m.workT++;
    switch (j.type) {
      case "dig": {
        if (!this.digDes[j.b]) { this.dropJob(m); break; }
        // remove one cell when enough work accumulated; nearest-to-mole first
        const x0 = (j.b % BW) * B, y0 = ((j.b / BW) | 0) * B;
        let target = -1;
        for (let y = y0; y < y0 + B && target < 0; y++) for (let x = x0; x < x0 + B; x++) { const i = this.ci(x, y); if (DIG[this.mat[i]] > 0) { target = i; break; } }
        if (target < 0) { this.digDes[j.b] = 0; this.dropJob(m); break; }
        if (m.workT >= DIG[this.mat[target]]) {
          m.workT = 0; const mt = this.mat[target];
          this.setCell(target, M.EMPTY); this.res.rubble++; if (mt === M.ORE) this.res.ore++;
          if (this.res.rubble % 40 === 0) this.res.wood++;
          if (this.rng() < 0.1) this.fx.push({ type: "dust", x: (target % CW) * 4, y: ((target / CW) | 0) * 4, t: 0 });
        }
        break;
      }
      case "beam":
        if (m.workT >= 30) { if (this.res.wood > 0) { this.res.wood--; this.buildPillar(j.b); } this.beamDes[j.b] = 0; this.dropJob(m); }
        break;
      case "light":
        if (m.workT >= 15) { this.lightReq[j.b] = 0; this.logic.ignite(j.b); this.say(`🧨 Mole #${m.id} lit a fuse`); this.dropJob(m); }
        break;
    }
  }
  /** Wooden pillar, 2 cells wide, from the block's floor up to the ceiling. */
  buildPillar(b: number) {
    const x = (b % BW) * B + 1, yc = ((b / BW) | 0) * B + 2;
    let yb = yc; while (yb < CH - 1 && !this.isSolid(this.ci(x, yb + 1))) yb++;
    let yt = yc; while (yt > 0 && !this.isSolid(this.ci(x, yt - 1)) && yc - yt < 60) yt--;
    for (let y = yt; y <= yb; y++) for (const xx of [x, x + 1]) { const i = this.ci(xx, y); if (!this.isSolid(i)) this.setCell(i, M.WOOD); }
  }
  hash() { let h = 2166136261; const mix = (v: number) => { h ^= v; h = Math.imul(h, 16777619); };
    for (let i = 0; i < NC; i++) { mix(this.mat[i]); mix(this.loose[i]); } for (const m of this.moles) { mix(m.bx); mix(m.by); mix(m.hp | 0); } return h >>> 0; }
  count(m: number) { let n = 0; for (let i = 0; i < NC; i++) if (this.mat[i] === m) n++; return n; }
}

// ---------- world generation ----------
function vnoise(seed: number) {
  const r = mulberry32(seed); const g = new Float32Array(256 * 256); for (let i = 0; i < g.length; i++) g[i] = r();
  const at = (x: number, y: number) => g[((y & 255) << 8) | (x & 255)];
  return (x: number, y: number, s: number) => {
    const fx = x / s, fy = y / s, x0 = Math.floor(fx), y0 = Math.floor(fy), tx = fx - x0, ty = fy - y0;
    const sx = tx * tx * (3 - 2 * tx), sy = ty * ty * (3 - 2 * ty);
    const a = at(x0, y0) + (at(x0 + 1, y0) - at(x0, y0)) * sx, b = at(x0, y0 + 1) + (at(x0 + 1, y0 + 1) - at(x0, y0 + 1)) * sx;
    return a + (b - a) * sy;
  };
}
export function generate(w: SandWorld, seed: number) {
  const n1 = vnoise(seed * 7 + 1), n2 = vnoise(seed * 13 + 5), n3 = vnoise(seed * 31 + 9);
  for (let y = 0; y < CH; y++) for (let x = 0; x < CW; x++) {
    const i = w.ci(x, y); let m: number = M.EMPTY;
    const surf = SURFACE + Math.round((n1(x, 0, 40) - 0.5) * 8);
    if (y >= surf) {
      const depth = y - SURFACE, r = n1(x, y, 28) * 0.6 + n1(x + 999, y, 10) * 0.4;
      m = M.DIRT;
      if (n2(x, y, 18) > 0.68) m = M.GRAVEL;
      if (n2(x + 500, y, 22) > 0.72 && depth > 30) m = M.CLAY;
      if (r > 0.64 - depth * 0.0016) m = depth > 130 && r > 0.74 ? M.GRANITE : M.ROCK;
      if (n3(x, y, 6) > 0.86 && depth > 20 && m !== M.ROCK) m = M.ORE;
      const cave = n3(x + 700, y * 1.6, 22);
      if (depth > 45 && cave > 0.7) { m = depth < 130 ? M.WATER : y > CH - 50 ? M.LAVA : M.EMPTY; }
    }
    if (y >= CH - 6 || x < 3 || x >= CW - 3) m = M.BEDROCK;
    w.mat[i] = m;
  }
  // starting chamber (blocks 42..54 x, rows 10..12) on a rock floor
  const bx0 = 42, bx1 = 54, by0 = 10, by1 = 12;
  for (let y = by0 * B; y < (by1 + 1) * B; y++) for (let x = bx0 * B; x < (bx1 + 1) * B; x++) w.mat[w.ci(x, y)] = M.EMPTY;
  for (let y = (by1 + 1) * B; y < (by1 + 1) * B + 3; y++) for (let x = (bx0 - 2) * B; x < (bx1 + 3) * B; x++) w.mat[w.ci(x, y)] = M.ROCK;
  // stabilise: anything unsupported at gen becomes rock (natural arches)
  w.resolveSupport();
  for (const i of w.pendingList) { w.mat[i] = M.ROCK; w.pending[i] = 0; }
  w.pendingList = []; w.structDirty = true; w.resolveSupport();
  w.updateNav();
  for (let k = 0; k < 4; k++) w.addMole(bx0 + 2 + k * 3, by1);
  w.say("Colony founded. Drag with Dig to tunnel.");
}
