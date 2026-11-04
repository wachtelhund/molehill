// Thin adapter: the ONLY ambience file that knows SandWorld. It turns world
// state (fx, log, moles, materials) into the engine-neutral SoundEvent /
// AmbientLevels vocabulary of events.ts. Read-only: never mutates the world.
// Positions are in cells. Materials are classified via KIND[] and named M.*
// members only, so new materials in the enum don't break anything.
import { SandWorld, CW, CH, B, BW, SURFACE } from "../sand/world";
import type { Fx, Ev, Mole } from "../sand/world";
import { M, K, KIND } from "../sand/materials";
import { AmbientLevels, SoundEvent } from "./events";

export interface CellRegion { x0: number; y0: number; x1: number; y1: number } // inclusive, cells

/** Centre cell of a block. */
const blockXY = (b: number) => ({ x: (b % BW) * B + B / 2, y: Math.floor(b / BW) * B + B / 2 });

/** Continuous ambience drivers measured over the visible region (sampled every `step` cells). */
export function ambientLevels(w: SandWorld, r: CellRegion, step = 2): AmbientLevels {
  const x0 = Math.max(0, Math.floor(r.x0)), x1 = Math.min(CW - 1, Math.ceil(r.x1));
  const y0 = Math.max(0, Math.floor(r.y0)), y1 = Math.min(CH - 1, Math.ceil(r.y1));
  let water = 0, lava = 0, fire = 0, fluidish = 0, n = 0;
  for (let y = y0; y <= y1; y += step) for (let x = x0; x <= x1; x += step) {
    const m = w.mat[y * CW + x], k = KIND[m]; n++;
    if (k === K.EMPTY || k === K.LIQUID || k === K.GAS) fluidish++;
    if (m === M.WATER) water++; else if (m === M.LAVA) lava++; else if (m === M.FIRE) fire++;
  }
  // lit fuses live on the block layer
  let fuses = 0; const L = w.logic, bx0 = Math.floor(x0 / B), bx1 = Math.floor(x1 / B), by0 = Math.floor(y0 / B), by1 = Math.floor(y1 / B);
  for (let by = by0; by <= by1; by++) for (let bx = bx0; bx <= bx1; bx++) if (L.burn[by * BW + bx] > 0) fuses++;
  const s2 = step * step, cy = (y0 + y1) / 2;
  return {
    water: n ? Math.min(1, water / Math.max(16, fluidish * 0.3)) : 0,
    lava: Math.min(1, lava * s2 / 120),
    burning: Math.min(1, fuses / 3 + fire * s2 / 40),
    depth: Math.max(0, Math.min(1, (cy - SURFACE) / (CH - SURFACE))),
  };
}

interface MoleMemo { hp: number; dead: boolean; job: { type: string; b: number } | null }
const CAVE_RE = /Cave-in near (\d+),(\d+)/;

/** Diffs successive SandWorld snapshots into discrete sound events. */
export class SandEventTracker {
  private world: SandWorld | null = null;
  private seenFx = new WeakSet<Fx>(); private seenLog = new WeakSet<Ev>();
  private moles = new Map<Mole, MoleMemo>();
  private tick = 0;

  update(w: SandWorld): SoundEvent[] {
    const ev: SoundEvent[] = [];
    if (w !== this.world) { this.reset(w); return ev; } // new colony: adopt silently
    if (w.tick === this.tick) return ev;
    for (const f of w.fx) {
      if (this.seenFx.has(f)) continue; this.seenFx.add(f);
      const x = f.x / 4, y = f.y / 4; // fx are in px, 4 px per cell
      if (f.type === "boom") ev.push({ kind: "explosion", x, y });
      else if (f.type === "dust") ev.push({ kind: "thud", x, y, intensity: 0.5 });
      else if (f.type === "steam") ev.push({ kind: "steam", x, y, intensity: 0.6 });
    }
    for (const e of w.log) {
      if (this.seenLog.has(e)) continue; this.seenLog.add(e);
      const c = CAVE_RE.exec(e.text);
      if (c) ev.push({ kind: "cavein", x: +c[1] * B + B / 2, y: +c[2] * B + B / 2 });
    }
    for (const m of w.moles) {
      const prev = this.moles.get(m), job = m.job ? { type: m.job.type, b: m.job.b } : null;
      const at = { x: m.bx * B + B / 2, y: m.by * B + B / 2 };
      if (prev) {
        if (m.dead && !prev.dead) ev.push({ kind: "death", ...at });
        else if (!m.dead && m.hp < prev.hp) ev.push({ kind: "hurt", ...at, intensity: 0.7 });
        const pj = prev.job;
        if (pj && (!job || job.b !== pj.b || job.type !== pj.type)) {
          if (pj.type === "dig" && !w.digDes[pj.b]) ev.push({ kind: "dug", ...blockXY(pj.b) });
          if (pj.type === "beam" && !w.beamDes[pj.b] && !m.dead && !m.fleeing) ev.push({ kind: "beam", ...blockXY(pj.b) });
          if (pj.type === "light" && !w.lightReq[pj.b] && !m.dead && !m.fleeing) ev.push({ kind: "ignite", ...blockXY(pj.b) });
        }
      }
      // rhythmic claw scraping while digging
      if (!m.dead && m.job?.type === "dig" && m.path.length === 0 && Math.floor(w.tick / 6) !== Math.floor(this.tick / 6))
        ev.push({ kind: "dig", ...blockXY(m.job.b), intensity: 0.8 });
      this.moles.set(m, { hp: m.hp, dead: m.dead, job });
    }
    this.tick = w.tick;
    return ev;
  }

  private reset(w: SandWorld) {
    this.world = w; this.tick = w.tick; this.moles.clear();
    for (const f of w.fx) this.seenFx.add(f);
    for (const e of w.log) this.seenLog.add(e);
    for (const m of w.moles) this.moles.set(m, { hp: m.hp, dead: m.dead, job: m.job ? { type: m.job.type, b: m.job.b } : null });
  }
}
