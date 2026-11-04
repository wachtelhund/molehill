// Molehill simulation core. Pure TS, no DOM. Side-view grid, gravity = +y.
export const W = 96, H = 64, N = W * H;
export const SIM_HZ = 20;

export const enum T { AIR, DIRT, GRAVEL, CLAY, ROCK, GRANITE, BEDROCK, DEBRIS, STONE, BEAM, COPPER }
export const TILE_NAMES = ["Air", "Dirt", "Gravel", "Clay", "Rock", "Granite", "Bedrock", "Debris", "Stone", "Beam", "Copper ore"];
//                        AIR DIRT GRAV CLAY ROCK GRAN BED DEB STONE BEAM COP
const SPAN     = [0,   3,   2,   4,   0,   0,   0,  2,  0,    255, 3];
const DIG_TICKS= [0,  40,  30,  60, 100, 200,   0, 20, 100,  20,  80];
const HARD     = [0,   2,   2,   3,   5,   8,  99,  1,  5,    1,   4];
export const isSolid = (t: number) => t !== T.AIR && t !== T.BEAM;
export const isAnchor = (t: number) => t === T.ROCK || t === T.GRANITE || t === T.BEDROCK || t === T.STONE;
export const isStruct = (t: number) => t !== T.AIR;
export const diggable = (t: number) => t !== T.AIR && t !== T.BEDROCK;

// Fluid constants (compressible CA)
export const MAX_MASS = 1.0, MAX_COMPRESS = 0.02, MIN_MASS = 0.0001, MIN_FLOW = 0.005, MAX_SPEED = 1.0;
export const LAVA_N = 4, COLLAPSE_DELAY = 20, FUSE_TICKS = 10, TNT_R = 3, BLAST = 10;
export const CRUSH_DMG = 60, LAVA_DMG = 3, MOVE_TICKS = 4;

export function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

export type JobType = "dig" | "beam" | "haul" | "light";
export interface Job { type: JobType; cell: number; phase: number }
export interface Mole { id: number; x: number; y: number; px: number; py: number; hp: number; o2: number; job: Job | null; path: number[]; moveT: number; workT: number; carry: number; dead: boolean; hurtT: number; facing: number }
export interface Debris { x: number; y: number; mat: number; fallT: number; fy: number }
export interface Fx { type: "boom" | "dust" | "steam" | "spark" | "crush"; x: number; y: number; t: number }
export interface Ev { tick: number; text: string }

export class World {
  tick = 0; rng: () => number;
  tile = new Uint8Array(N); hp = new Uint16Array(N);
  water = new Float32Array(N); lava = new Float32Array(N);
  private next = new Float32Array(N);
  support = new Uint8Array(N); pending = new Int16Array(N);
  dig = new Uint8Array(N); beamDes = new Uint8Array(N); lightReq = new Uint8Array(N);
  fuse = new Uint8Array(N); burn = new Int16Array(N); tnt = new Uint8Array(N); fire = new Int16Array(N);
  items = new Uint8Array(N);
  moles: Mole[] = []; debris: Debris[] = []; fx: Fx[] = []; log: Ev[] = [];
  res = { wood: 30, tnt: 10, rubble: 0, ore: 0 };
  stock = 0; structDirty = true; boomQueue: number[] = []; shake = 0;
  reserved = new Set<string>(); lastCave = -999;

  constructor(seed = 1, gen = true) {
    this.rng = mulberry32(seed);
    if (gen) generate(this, seed);
  }
  static empty(fill: number, seed = 1) { const w = new World(seed, false); w.tile.fill(fill); return w; }
  idx(x: number, y: number) { return y * W + x; }
  inb(x: number, y: number) { return x >= 0 && y >= 0 && x < W && y < H; }
  say(text: string) { this.log.push({ tick: this.tick, text }); if (this.log.length > 40) this.log.shift(); }
  addMole(x: number, y: number) { this.moles.push({ id: this.moles.length, x, y, px: x, py: y, hp: 100, o2: 100, job: null, path: [], moveT: 0, workT: 0, carry: 0, dead: false, hurtT: 0, facing: 0 }); }
  walkable(i: number) { return !isSolid(this.tile[i]) && this.lava[i] < 0.3; }
  setTile(i: number, t: number) { this.tile[i] = t; this.structDirty = true; }

  // ---------- commands (from UI or scripts) ----------
  designate(tool: string, i: number) {
    const t = this.tile[i];
    switch (tool) {
      case "dig": if (diggable(t)) this.dig[i] = 1; break;
      case "beam": if (t === T.AIR) this.beamDes[i] = 1; break;
      case "fuse": if (!isSolid(t)) this.fuse[i] = 1; break;
      case "tnt": if (!isSolid(t) && !this.tnt[i] && this.res.tnt > 0) { this.tnt[i] = 1; this.res.tnt--; } break;
      case "ignite": if (this.fuse[i] || this.tnt[i]) this.lightReq[i] = 1; break;
      case "cancel": this.dig[i] = 0; this.beamDes[i] = 0; this.lightReq[i] = 0; if (this.tnt[i]) { this.tnt[i] = 0; this.res.tnt++; } this.fuse[i] = 0; break;
      case "water": if (!isSolid(t)) this.water[i] = Math.min(3, this.water[i] + 1); break;
      case "lava": if (!isSolid(t)) this.lava[i] = Math.min(3, this.lava[i] + 1); break;
    }
  }

  // ---------- tick (update order from GDD section 6) ----------
  step() {
    this.tick++;
    this.stepFuses();          // 5
    this.resolveExplosions();  // 6
    if (this.structDirty) this.resolveStructure(); // 7
    this.stepPending();
    this.stepDebris();         // 8
    stepFluid(this, this.water, 1);           // 9
    if (this.tick % LAVA_N === 0) stepFluid(this, this.lava, 0.5);
    this.reactions();          // 10
    if (this.tick % 10 === 0) this.assignJobs(); // 11
    for (const m of this.moles) this.stepMole(m); // 12
    for (const f of this.fx) f.t++;
    this.fx = this.fx.filter(f => f.t < 40);
    if (this.shake > 0) this.shake--;
  }

  // ---------- fuses & explosives ----------
  stepFuses() {
    const ignite: number[] = [];
    for (let i = 0; i < N; i++) {
      if (this.burn[i] <= 0) continue;
      if (this.water[i] > 0.3) { this.burn[i] = 0; continue; }
      if (--this.burn[i] === 0) {
        this.fuse[i] = 0;
        if (this.tnt[i]) this.boomQueue.push(i);
        for (const n of nb4(i)) { if (n < 0) continue; if (this.fuse[n] && this.burn[n] === 0) ignite.push(n); if (this.tnt[n]) this.boomQueue.push(n); }
      } else if (this.tick % 3 === 0) this.fx.push({ type: "spark", x: i % W, y: (i / W) | 0, t: 0 });
    }
    for (const n of ignite) this.burn[n] = FUSE_TICKS;
  }
  igniteCell(i: number) { if (this.fuse[i] && this.burn[i] === 0) this.burn[i] = FUSE_TICKS; else if (this.tnt[i]) this.boomQueue.push(i); }
  resolveExplosions() {
    const q = [...new Set(this.boomQueue)].sort((a, b) => a - b); this.boomQueue = [];
    for (const c of q) {
      if (!this.tnt[c]) continue;
      this.tnt[c] = 0; const cx = c % W, cy = (c / W) | 0;
      this.fx.push({ type: "boom", x: cx, y: cy, t: 0 }); this.shake = 12;
      for (let dy = -TNT_R; dy <= TNT_R; dy++) for (let dx = -TNT_R; dx <= TNT_R; dx++) {
        const x = cx + dx, y = cy + dy; if (!this.inb(x, y)) continue;
        const d = Math.hypot(dx, dy); if (d > TNT_R + 0.5) continue;
        const p = 1 - d / (TNT_R + 1), n = this.idx(x, y);
        const t = this.tile[n];
        if (t !== T.AIR && p * BLAST >= HARD[t]) { this.setTile(n, T.AIR); this.dig[n] = 0; if (this.rng() < 0.3) this.items[n]++; }
        this.fuse[n] = 0; this.burn[n] = 0;
        if (this.tnt[n] && n !== c && p > 0.2) this.boomQueue.push(n); // chain next tick
        for (const m of this.moles) if (!m.dead && m.x === x && m.y === y) this.hurt(m, Math.round(80 * p), "blast");
      }
      this.say(`💥 Explosion at ${cx},${cy}`);
    }
  }

  // ---------- structure: 0-1 BFS support ----------
  resolveStructure() {
    this.structDirty = false;
    const s = this.support; s.fill(255);
    const dq = new Int32Array(N * 2); let head = N, tail = N; // deque in middle
    for (let i = 0; i < N; i++) if (isAnchor(this.tile[i])) { s[i] = 0; dq[tail++] = i; }
    while (head < tail) {
      const i = dq[head++]; const x = i % W;
      const ns = [i - W, i + W, x > 0 ? i - 1 : -1, x < W - 1 ? i + 1 : -1];
      for (let k = 0; k < 4; k++) {
        const n = ns[k]; if (n < 0 || n >= N) continue;
        const t = this.tile[n]; if (!isStruct(t) || isAnchor(t)) continue;
        if (t === T.BEAM && k !== 0) continue;          // beams only rest on what is below
        const cost = k === 0 ? 0 : 1;                   // upward support is free
        const c = s[i] + cost;
        if (c < s[n] && c <= SPAN[t]) { s[n] = c; if (cost === 0) dq[--head] = n; else dq[tail++] = n; }
      }
    }
    for (let i = 0; i < N; i++) {
      const t = this.tile[i];
      if (isStruct(t) && s[i] === 255) { if (this.pending[i] === 0) this.pending[i] = COLLAPSE_DELAY; }
      else this.pending[i] = 0;
    }
  }
  stepPending() {
    let first = -1;
    for (let i = N - 1; i >= 0; i--) {
      if (this.pending[i] <= 0) continue;
      if (--this.pending[i] === 0 && this.support[i] === 255 && isStruct(this.tile[i])) {
        const mat = this.tile[i]; this.tile[i] = T.AIR; this.dig[i] = 0; this.structDirty = true;
        this.debris.push({ x: i % W, y: (i / W) | 0, mat: mat === T.BEAM ? T.DEBRIS : mat, fallT: 0, fy: (i / W) | 0 });
        if (first < 0) first = i;
      }
    }
    if (first >= 0) { if (this.tick - this.lastCave > 40) this.say(`⚠ Cave-in at ${first % W},${(first / W) | 0}`); this.shake = Math.max(this.shake, 6); this.lastCave = this.tick; }
  }
  stepDebris() {
    this.debris.sort((a, b) => b.y - a.y || a.x - b.x);
    const keep: Debris[] = [];
    for (const d of this.debris) {
      d.fallT++;
      const below = d.y + 1;
      const bi = below < H ? this.idx(d.x, below) : -1;
      const blocked = bi < 0 || isSolid(this.tile[bi]) || this.tile[bi] === T.BEAM;
      if (blocked) {
        let y = d.y; while (y > 0 && isSolid(this.tile[this.idx(d.x, y)])) y--;
        const i = this.idx(d.x, y);
        // displace fluids upward (conserve mass)
        if (y > 0) { this.water[i - W] += this.water[i]; this.lava[i - W] += this.lava[i]; }
        this.water[i] = 0; this.lava[i] = 0; this.fuse[i] = 0;
        if (this.tnt[i]) this.boomQueue.push(i);
        this.setTile(i, T.DEBRIS);
        this.fx.push({ type: "dust", x: d.x, y, t: 0 });
        for (const m of this.moles) if (!m.dead && m.x === d.x && m.y === y) { this.hurt(m, CRUSH_DMG, "crushed"); if (!m.dead && y > 0 && this.walkable(i - W)) { m.y--; m.path = []; } }
        continue;
      }
      const rate = d.fallT < 4 ? 3 : 1; // accelerate
      if (d.fallT % rate === 0) {
        d.y = below;
        for (const m of this.moles) if (!m.dead && m.x === d.x && m.y === d.y) { this.hurt(m, Math.round(CRUSH_DMG * 0.5), "crushed"); this.fx.push({ type: "crush", x: d.x, y: d.y, t: 0 }); }
      }
      keep.push(d);
    }
    this.debris = keep;
  }

  // ---------- reactions ----------
  reactions() {
    for (let i = 0; i < N; i++) {
      if (this.lava[i] > 0.01) {
        for (const n of [i, ...nb4(i)]) {
          if (n < 0 || this.water[n] <= 0.01 || isSolid(this.tile[n])) continue;
          const k = Math.min(this.lava[i], this.water[n]);
          this.lava[i] -= k; this.water[n] -= k;
          if (this.rng() < 0.3) this.fx.push({ type: "steam", x: n % W, y: (n / W) | 0, t: 0 });
          if (this.lava[i] < 0.5) {
            if (i >= W) this.water[i - W] += this.water[i];
            this.water[i] = 0; this.lava[i] = 0; this.setTile(i, T.STONE);
            for (const m of this.moles) if (!m.dead && this.idx(m.x, m.y) === i) this.hurt(m, 100, "entombed");
            break;
          }
        }
        if (this.lava[i] > 0.1) for (const n of nb4(i)) {
          if (n < 0) continue;
          if (this.tile[n] === T.BEAM && this.fire[n] === 0) this.fire[n] = 100;
          if (this.fuse[n] && this.burn[n] === 0) this.burn[n] = FUSE_TICKS;
          if (this.tnt[n]) this.boomQueue.push(n);
        }
      }
      if (this.fire[i] > 0) {
        if (this.water[i] > 0.3 || this.tile[i] !== T.BEAM) { this.fire[i] = 0; continue; }
        if (--this.fire[i] === 0) { this.setTile(i, T.AIR); this.say("🔥 A beam burned away"); }
        else if (this.rng() < 0.02) for (const n of nb4(i)) if (n >= 0 && this.tile[n] === T.BEAM && !this.fire[n]) this.fire[n] = 100;
      }
    }
  }

  // ---------- moles & jobs ----------
  hurt(m: Mole, dmg: number, why: string) {
    m.hp -= dmg; m.hurtT = 8;
    if (m.hp <= 0 && !m.dead) { m.dead = true; m.hp = 0; this.dropJob(m); this.say(`☠ Mole #${m.id} ${why}`); }
  }
  dropJob(m: Mole) { if (m.job) this.reserved.delete(key(m.job)); m.job = null; m.path = []; m.workT = 0; }

  bfs(from: number) {
    const dist = new Int16Array(N).fill(-1), par = new Int32Array(N).fill(-1), q = new Int32Array(N);
    let h = 0, t = 0; dist[from] = 0; q[t++] = from;
    while (h < t) { const i = q[h++]; for (const n of nb4(i)) if (n >= 0 && dist[n] < 0 && this.walkable(n)) { dist[n] = dist[i] + 1; par[n] = i; q[t++] = n; } }
    return { dist, par };
  }
  pathTo(par: Int32Array, from: number, to: number) { const p: number[] = []; let c = to; while (c !== from && c >= 0) { p.push(c); c = par[c]; } return c === from ? p.reverse() : null; }
  standFor(dist: Int16Array, c: number, self: boolean) {
    let best = -1, bd = 1e9;
    if (self && dist[c] >= 0) return c;
    for (const n of nb4(c)) if (n >= 0 && dist[n] >= 0 && dist[n] < bd) { bd = dist[n]; best = n; }
    return best;
  }
  assignJobs() {
    for (const m of this.moles) {
      if (m.dead || m.job) continue;
      const start = this.idx(m.x, m.y);
      const { dist, par } = this.bfs(start);
      let best: Job | null = null, bestStand = -1, bestScore = 1e9;
      for (let c = 0; c < N; c++) {
        let type: JobType | null = null, self = false, pri = 0;
        if (this.lightReq[c]) { type = "light"; self = true; pri = 60; }
        else if (this.dig[c] && diggable(this.tile[c])) { type = "dig"; pri = 20; }
        else if (this.beamDes[c] && this.tile[c] === T.AIR && this.res.wood > 0) { type = "beam"; self = false; pri = 40; }
        else if (this.items[c] && c !== this.stock) { type = "haul"; self = true; pri = 0; }
        if (!type) continue;
        const j: Job = { type, cell: c, phase: 0 };
        if (this.reserved.has(key(j))) continue;
        const s = this.standFor(dist, c, self); if (s < 0) continue;
        const score = dist[s] - pri;
        if (score < bestScore) { bestScore = score; best = j; bestStand = s; }
      }
      if (best) { m.job = best; this.reserved.add(key(best)); m.path = this.pathTo(par, start, bestStand) || []; m.workT = 0; }
    }
  }
  stepMole(m: Mole) {
    m.px += (m.x - m.px) * 0.35; m.py += (m.y - m.py) * 0.35; if (m.hurtT > 0) m.hurtT--;
    if (m.dead) return;
    const here = this.idx(m.x, m.y);
    if (isSolid(this.tile[here])) { this.hurt(m, CRUSH_DMG, "buried"); if (m.dead) return; }
    if (this.lava[here] > 0.1) this.hurt(m, LAVA_DMG, "melted in lava");
    if (this.water[here] > 0.6) { m.o2 -= 0.5; if (m.o2 <= 0) { m.o2 = 0; this.hurt(m, 1, "drowned"); } } else m.o2 = Math.min(100, m.o2 + 2);
    if (m.dead) return;
    const j = m.job; if (!j) return;
    if (!this.jobValid(j)) { this.dropJob(m); return; }
    if (m.path.length) {
      const slow = this.water[here] > 0.5 ? 2 : 1;
      if (++m.moveT >= MOVE_TICKS * slow) {
        m.moveT = 0; const n = m.path[0];
        if (!this.walkable(n)) { this.dropJob(m); return; }
        m.path.shift(); const nx = n % W; if (nx !== m.x) m.facing = nx > m.x ? 1 : -1;
        m.x = nx; m.y = (n / W) | 0;
      }
      return;
    }
    m.workT++;
    const c = j.cell;
    switch (j.type) {
      case "dig": {
        const t = this.tile[c]; this.hp[c]++;
        if (this.hp[c] >= DIG_TICKS[t]) {
          this.hp[c] = 0; this.dig[c] = 0; this.setTile(c, T.AIR);
          this.items[c] = Math.min(9, this.items[c] + 1); if (t === T.COPPER) this.res.ore++;
          this.fx.push({ type: "dust", x: c % W, y: (c / W) | 0, t: 0 }); this.dropJob(m);
        }
        break;
      }
      case "beam": if (m.workT >= 30) { if (this.res.wood > 0 && this.tile[c] === T.AIR) { this.res.wood--; this.setTile(c, T.BEAM); } this.beamDes[c] = 0; this.dropJob(m); } break;
      case "light": if (m.workT >= 20) { this.lightReq[c] = 0; this.igniteCell(c); this.dropJob(m); this.say(`🧨 Mole #${m.id} lit a fuse`); } break;
      case "haul":
        if (j.phase === 0) {
          if (this.items[c] === 0) { this.dropJob(m); break; }
          this.items[c]--; m.carry = 1; j.phase = 1;
          const { par } = this.bfs(here); const p = this.pathTo(par, here, this.stock);
          if (!p) { this.items[c]++; m.carry = 0; this.dropJob(m); break; }
          m.path = p;
        } else { this.res.rubble += m.carry; m.carry = 0; if (this.res.rubble % 5 === 0) this.res.wood++; this.dropJob(m); }
        break;
    }
  }
  jobValid(j: Job) {
    const c = j.cell;
    switch (j.type) {
      case "dig": return this.dig[c] === 1 && diggable(this.tile[c]);
      case "beam": return this.beamDes[c] === 1 && this.tile[c] === T.AIR;
      case "light": return this.lightReq[c] === 1 && (this.fuse[c] === 1 || this.tnt[c] === 1);
      case "haul": return true;
    }
  }
  hash() { let h = 2166136261; const mix = (v: number) => { h ^= v; h = Math.imul(h, 16777619); };
    for (let i = 0; i < N; i++) { mix(this.tile[i]); mix((this.water[i] * 1000) | 0); mix((this.lava[i] * 1000) | 0); }
    for (const m of this.moles) { mix(m.x); mix(m.y); mix(m.hp); } return h >>> 0; }
  totalMass(a: Float32Array) { let s = 0; for (let i = 0; i < N; i++) s += a[i]; return s; }
}

const key = (j: Job) => j.type + ":" + j.cell;
export function nb4(i: number): number[] {
  const x = i % W;
  return [i >= W ? i - W : -1, i < N - W ? i + W : -1, x > 0 ? i - 1 : -1, x < W - 1 ? i + 1 : -1];
}
const clamp = (v: number, a: number, b: number) => v < a ? a : v > b ? b : v;
function stableBelow(total: number) {
  if (total <= MAX_MASS) return MAX_MASS;
  if (total < 2 * MAX_MASS + MAX_COMPRESS) return (MAX_MASS * MAX_MASS + total * MAX_COMPRESS) / (MAX_MASS + MAX_COMPRESS);
  return (total + MAX_COMPRESS) / 2;
}

// Compressible-mass cellular automaton: down, sideways, up (pressure). Double-buffered.
export function stepFluid(w: World, m: Float32Array, flowK: number) {
  const next = (w as any).next as Float32Array; next.set(m);
  const tile = w.tile, odd = w.tick & 1;
  for (let y = H - 1; y >= 0; y--) for (let xx = 0; xx < W; xx++) {
    const x = odd ? W - 1 - xx : xx; const i = y * W + x;
    if (isSolid(tile[i])) continue;
    let rem = m[i]; if (rem <= 0) continue;
    if (y < H - 1 && !isSolid(tile[i + W])) {
      const d = i + W; let f = stableBelow(rem + m[d]) - m[d];
      if (f > MIN_FLOW) f *= 0.5 * flowK + (1 - flowK) * 0.25;
      f = clamp(f, 0, Math.min(MAX_SPEED, rem));
      next[i] -= f; next[d] += f; rem -= f;
    }
    if (rem <= 0) continue;
    if (x > 0 && !isSolid(tile[i - 1])) { let f = (m[i] - m[i - 1]) / 4; if (f > MIN_FLOW) f *= 0.5 * flowK + (1 - flowK) * 0.25; f = clamp(f, 0, rem); next[i] -= f; next[i - 1] += f; rem -= f; }
    if (rem <= 0) continue;
    if (x < W - 1 && !isSolid(tile[i + 1])) { let f = (m[i] - m[i + 1]) / 4; if (f > MIN_FLOW) f *= 0.5 * flowK + (1 - flowK) * 0.25; f = clamp(f, 0, rem); next[i] -= f; next[i + 1] += f; rem -= f; }
    if (rem <= 0) continue;
    if (y > 0 && !isSolid(tile[i - W])) {
      const u = i - W; let f = rem - stableBelow(rem + m[u]);
      if (f > MIN_FLOW) f *= 0.5;
      f = clamp(f, 0, Math.min(MAX_SPEED, rem));
      next[i] -= f; next[u] += f;
    }
  }
  for (let i = 0; i < N; i++) { const v = next[i]; m[i] = isSolid(tile[i]) ? 0 : v; }
}

// ---------- world generation ----------
function vnoise(seed: number) {
  const r = mulberry32(seed); const g = new Float32Array(64 * 64); for (let i = 0; i < g.length; i++) g[i] = r();
  const at = (x: number, y: number) => g[((y & 63) << 6) | (x & 63)];
  return (x: number, y: number, s: number) => {
    const fx = x / s, fy = y / s, x0 = Math.floor(fx), y0 = Math.floor(fy), tx = fx - x0, ty = fy - y0;
    const sx = tx * tx * (3 - 2 * tx), sy = ty * ty * (3 - 2 * ty);
    const a = at(x0, y0) + (at(x0 + 1, y0) - at(x0, y0)) * sx, b = at(x0, y0 + 1) + (at(x0 + 1, y0 + 1) - at(x0, y0 + 1)) * sx;
    return a + (b - a) * sy;
  };
}
export const SURFACE = 6;
function generate(w: World, seed: number) {
  const n1 = vnoise(seed * 7 + 1), n2 = vnoise(seed * 13 + 5), n3 = vnoise(seed * 31 + 9);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const i = w.idx(x, y); let t: number = T.AIR;
    if (y >= SURFACE) {
      const depth = y - SURFACE, r = n1(x, y, 7) * 0.6 + n1(x + 99, y, 3) * 0.4;
      t = T.DIRT;
      if (n2(x, y, 5) > 0.68) t = T.GRAVEL;
      if (n2(x + 50, y, 6) > 0.72 && depth > 8) t = T.CLAY;
      if (r > 0.66 - depth * 0.006) t = depth > 34 && r > 0.75 ? T.GRANITE : T.ROCK;
      if (n3(x, y, 3) > 0.86 && depth > 6) t = T.COPPER;
      const cave = n3(x + 200, y * 1.6, 6);
      if (depth > 12 && cave > 0.7) { t = T.AIR; if (depth < 34) w.water[i] = 1; else if (y > H - 14) w.lava[i] = 1; }
    }
    if (y >= H - 2 || x === 0 || x === W - 1) t = T.BEDROCK;
    w.tile[i] = t; if (isSolid(t)) { w.water[i] = 0; w.lava[i] = 0; }
  }
  // starting chamber
  const cx = W >> 1, cy = SURFACE + 4;
  for (let y = cy - 1; y <= cy + 1; y++) for (let x = cx - 5; x <= cx + 5; x++) { const i = w.idx(x, y); w.tile[i] = T.AIR; w.water[i] = 0; w.lava[i] = 0; }
  for (let x = cx - 7; x <= cx + 7; x++) { w.tile[w.idx(x, cy + 2)] = T.ROCK; }
  w.stock = w.idx(cx, cy + 1);
  // stabilize: anything unsupported at gen becomes rock (natural arches)
  w.resolveStructure();
  for (let i = 0; i < N; i++) if (w.pending[i]) { w.tile[i] = T.ROCK; w.pending[i] = 0; }
  w.structDirty = true; w.resolveStructure();
  for (let k = 0; k < 4; k++) w.addMole(cx - 3 + k * 2, cy + 1);
  w.say("Colony founded. Drag with Dig to tunnel.");
}

// ---------- scripted demo scenario (used for screenshots & attract mode) ----------
export function demoScenario(w: World) {
  const cx = W >> 1, cy = SURFACE + 4;
  // carve a wide dirt-ceilinged hall to the left with no beams -> will need beams
  for (let x = cx - 20; x <= cx - 6; x++) w.designate("dig", w.idx(x, cy + 1));
  for (let x = cx - 18; x <= cx - 8; x += 5) w.designate("beam", w.idx(x, cy + 1));
  // TNT chain on the right, fuse from the chamber
  for (let x = cx + 6; x <= cx + 14; x++) w.designate("dig", w.idx(x, cy + 1));
  // sandbox: a test pit with water poured onto lava
  const px = 14, py = 30;
  for (let y = py; y < py + 8; y++) for (let x = px; x < px + 14; x++) w.setTile(w.idx(x, y), T.AIR);
  for (let y = py; y < py + 9; y++) { w.setTile(w.idx(px - 1, y), T.ROCK); w.setTile(w.idx(px + 14, y), T.ROCK); }
  for (let x = px - 1; x <= px + 14; x++) w.setTile(w.idx(x, py + 8), T.ROCK);
  for (let y = py + 5; y < py + 8; y++) for (let x = px; x < px + 14; x++) w.lava[w.idx(x, y)] = 1;
  for (let y = py; y < py + 2; y++) for (let x = px; x < px + 5; x++) w.water[w.idx(x, y)] = 1;
  // unsupported dirt bridge over a gap -> cave-in
  const gx = 60, gy = 26;
  for (let y = gy; y < gy + 5; y++) for (let x = gx; x < gx + 12; x++) w.setTile(w.idx(x, y), y === gy ? T.DIRT : T.AIR);
  for (let y = gy - 3; y < gy; y++) for (let x = gx; x < gx + 12; x++) w.setTile(w.idx(x, y), T.GRAVEL);
  for (let y = gy - 3; y < gy + 6; y++) { w.setTile(w.idx(gx - 1, y), T.ROCK); w.setTile(w.idx(gx + 12, y), T.ROCK); }
  for (let x = gx - 1; x <= gx + 12; x++) w.setTile(w.idx(x, gy + 5), T.ROCK);
  w.addMole(gx + 6, gy + 4);
  // TNT row with fuse inside the bridge chamber wall
  for (let x = gx + 1; x < gx + 11; x++) w.fuse[w.idx(x, gy + 4)] = 1;
  w.tnt[w.idx(gx + 11, gy + 4)] = 1;
  w.igniteCell(w.idx(gx + 1, gy + 4));
}
