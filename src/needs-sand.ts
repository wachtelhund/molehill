// Adapter: plugs the engine-agnostic NeedsSystem (needs.ts) into the v2 falling-sand SandWorld.
// The needs grid is the 4x4-cell block layer moles live on (mole.bx/by), so cellScale = 1.
// SandWorld has no plugin hooks, so attachSandNeeds wraps step()/designate() on the instance:
//  - before step: snapshot cells of blocks being dug (to detect dug material -> grubs)
//  - after step: run needs, mirror oxygen, slow weak workers, hold busy moles off the job board
// Materials are only queried through KIND / MAT_NAMES / DIG and named M members, never raw ids.
import { SandWorld, BW, BH, B, CW, SIM_HZ, type Mole, type Job } from "./sand/world";
import { M, K, KIND, MAT_NAMES, DIG } from "./sand/materials";
import { NeedsSystem, NEEDS_DEFAULTS, type NeedsConfig, type NeedsHost } from "./needs";
import type { Pt } from "./needs-grid";

const CELLS = B * B;
const SUBMERGED = 10, AIR_OPEN = 8, SOLID = 10; // cells per block, matching the sim's drown/nav thresholds
/** Inert placeholder job: keeps a busy mole out of assignJobs; stepMole drops it harmlessly (no block -1). */
const HOLD: Job = { type: "dig", b: -1 };

/** v2 tuning on top of NEEDS_DEFAULTS. Grub chances are per dug cell (a block is CELLS cells). */
export const SAND_NEEDS: Partial<NeedsConfig> = {
  drownDmg: 0,  // the sim already drowns moles once the (mirrored) o2 hits 0
  o2Stale: 0.5, // sealed chambers go stale slowly: time to dig a shaft
  grubChance: { [MAT_NAMES[M.DIRT]]: 0.12 / CELLS, [MAT_NAMES[M.CLAY]]: 0.08 / CELLS, [MAT_NAMES[M.GRAVEL]]: 0.04 / CELLS, [MAT_NAMES[M.SAND]]: 0.03 / CELLS },
};

const isOpen = (m: number) => KIND[m] === K.EMPTY || KIND[m] === K.GAS;

export function sandHost(w: SandWorld, seed: number, home: () => Pt | null): NeedsHost {
  const inb = (x: number, y: number) => x >= 0 && y >= 0 && x < BW && y < BH;
  const count = (x: number, y: number, pred: (m: number) => boolean) => w.blockCount(y * BW + x, i => pred(w.mat[i]));
  const airOpen = (x: number, y: number) => inb(x, y) && count(x, y, isOpen) >= AIR_OPEN;
  const mole = (id: number) => w.moles.find(m => m.id === id);
  return {
    width: BW, height: BH, tickHz: SIM_HZ, cellScale: 1, seed,
    isWalkable: (x, y) => inb(x, y) && w.nav[y * BW + x] === 1,
    isSolid: (x, y) => !inb(x, y) || w.blockCount(y * BW + x, i => w.isSolid(i)) >= SOLID,
    isSubmerged: (x, y) => inb(x, y) && count(x, y, m => KIND[m] === K.LIQUID && m !== M.LAVA) >= SUBMERGED,
    isHazard: (x, y) => inb(x, y) && count(x, y, m => m === M.LAVA) >= 1,
    // sky: an air block whose centre column is open all the way to the top of the world
    isSky: (x, y) => {
      if (!airOpen(x, y)) return false;
      for (let cy = y * B - 1, cx = x * B + (B >> 1); cy >= 0; cy--) if (!isOpen(w.mat[cy * CW + cx])) return false;
      return true;
    },
    airCost: (x, y) => airOpen(x, y) ? 1 : -1,
    // fleeing moles are reported inactive so the needs system yields control to the sim's flee
    moles: () => w.moles.map(m => ({ id: m.id, x: m.bx, y: m.by, dead: m.dead || m.fleeing > 0, hasJob: !!m.job && m.job !== HOLD })),
    moveMole: (id, x, y) => { const m = mole(id); if (!m) return; if (x !== m.bx) m.facing = x > m.bx ? 1 : -1; m.bx = x; m.by = y; },
    hurtMole: (id, dmg, why) => { const m = mole(id); if (m) w.hurt(m, dmg, why); },
    healMole: (id, amt) => { const m = mole(id); if (m && !m.dead) m.hp = Math.min(100, m.hp + amt); },
    dropJob: id => { const m = mole(id); if (m) w.dropJob(m); },
    setOxygen: (id, v) => { const m = mole(id); if (m) m.o2 = v; },
    spend: (r, n) => { const res = w.res as Record<string, number>; if ((res[r] ?? 0) < n) return false; res[r] -= n; return true; },
    refund: (r, n) => { const res = w.res as Record<string, number>; res[r] = (res[r] ?? 0) + n; },
    stockpile: home,
    say: text => w.say(text),
  };
}

const attached = new WeakMap<SandWorld, NeedsSystem>();
export const sandNeedsOf = (w: SandWorld) => attached.get(w);

export interface SandNeedsOpts { home?: Pt; cfg?: Partial<NeedsConfig> }

/** Attach needs to a SandWorld (idempotent). Home (where grubs are stored and eaten) defaults to
 *  the first mole's position at attach time. Tool "farm" places a mushroom farm site. */
export function attachSandNeeds(w: SandWorld, seed = 1, opts: SandNeedsOpts = {}): NeedsSystem {
  const have = attached.get(w); if (have) return have;
  const m0 = w.moles[0], home: Pt | null = opts.home ?? (m0 ? { x: m0.bx, y: m0.by } : null);
  const nd = new NeedsSystem(sandHost(w, seed, () => home), { ...SAND_NEEDS, ...opts.cfg, grubChance: { ...SAND_NEEDS.grubChance, ...opts.cfg?.grubChance } });
  const step = w.step.bind(w), designate = w.designate.bind(w);
  const staleSaid = new Set<number>();

  w.step = () => {
    // snapshot blocks under active digging so we know which materials got dug this tick
    const digs: { m: Mole; cells: number[]; mats: number[] }[] = [];
    for (const m of w.moles) {
      const j = m.job; if (m.dead || !j || j === HOLD || j.type !== "dig" || m.path.length) continue;
      const x0 = (j.b % BW) * B, y0 = ((j.b / BW) | 0) * B, cells: number[] = [];
      for (let y = y0; y < y0 + B; y++) for (let x = x0; x < x0 + B; x++) cells.push(y * CW + x);
      digs.push({ m, cells, mats: cells.map(i => w.mat[i]) });
    }
    const rubble = w.res.rubble;
    step();
    let dug = w.res.rubble - rubble;
    for (const d of digs) d.cells.forEach((c, k) => { if (dug > 0 && DIG[d.mats[k]] > 0 && w.mat[c] === M.EMPTY) { dug--; nd.onDug(MAT_NAMES[d.mats[k]], d.m.id); } });

    nd.step();
    for (const m of w.moles) {
      if (m.dead || m.fleeing) continue;
      const busy = nd.busy(m.id);
      if (busy && !m.job) m.job = HOLD; else if (!busy && m.job === HOLD) m.job = null;
      if (m.job && m.job !== HOLD && !m.path.length && m.workT >= 1 && !nd.workTick(m.id)) m.workT--; // weak: half speed
      const n = nd.get(m.id);
      if (n.stale && n.oxygen < 60 && !staleSaid.has(m.id)) { staleSaid.add(m.id); w.say(`😮‍💨 Mole #${m.id} is short of air — dig a shaft to the surface`); }
      else if (!n.stale) staleSaid.delete(m.id);
    }
  };
  w.designate = (tool: string, b: number) => {
    if (b < 0 || b >= BW * BH) return;
    if (tool === "farm") { nd.designate(tool, b % BW, (b / BW) | 0); return; }
    designate(tool, b);
    if (tool === "cancel") nd.designate(tool, b % BW, (b / BW) | 0);
  };
  attached.set(w, nd);
  return nd;
}
export { NEEDS_DEFAULTS };
