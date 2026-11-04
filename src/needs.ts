// Mole needs: hunger, sleep and oxygen meters, plus food (grubs, mushroom farms).
// Engine-agnostic: talks to the world only through NeedsHost (see needs-adapter.ts for
// the current tile sim). All rates are per second and all distances/budgets are in
// "mole lengths", scaled by host.cellScale, so nothing depends on grid resolution.
import { bfsPath, costTo, mulberry32, type Pt } from "./needs-grid";

export type Activity = "none" | "toFood" | "eating" | "sleeping" | "toAir" | "toBuild" | "building";

export interface HostMole { id: number; x: number; y: number; dead: boolean; hasJob: boolean }

export interface NeedsHost {
  readonly width: number; readonly height: number;
  readonly tickHz: number;      // sim ticks per second
  readonly cellScale: number;   // cells per mole body length (1 on the current grid)
  seed: number;
  isWalkable(x: number, y: number): boolean;
  isSolid(x: number, y: number): boolean;
  isSubmerged(x: number, y: number): boolean; // fluid deep enough to drown in
  isHazard(x: number, y: number): boolean;    // lava etc.
  isSky(x: number, y: number): boolean;       // open air at the surface (fresh-air source)
  airCost(x: number, y: number): number;      // per-cell cost for air to seep through; <0 = airtight
  moles(): HostMole[];
  moveMole(id: number, x: number, y: number): void;
  hurtMole(id: number, dmg: number, why: string): void;
  healMole(id: number, amt: number): void;
  dropJob(id: number): void;
  setOxygen(id: number, v: number): void;     // mirror into the host's own o2 field, if any
  spend(resource: string, n: number): boolean;
  refund(resource: string, n: number): void;
  stockpile(): Pt | null;
  say(text: string): void;
}

export const NEEDS_DEFAULTS = {
  hungerDecay: 100 / 300, sleepDecay: 100 / 420, // per second, idle
  workMul: 1.5,                                  // decay multiplier while on a job
  o2Drown: 10, o2Stale: 2, o2Recover: 40,        // per second
  hungry: 40, starving: 15, tired: 30, lowO2: 35, fullHunger: 85, rested: 95,
  slowBelow: 20,                                 // hunger/sleep below this halves work speed
  starveDmg: 1, drownDmg: 20, staleDmg: 3, regen: 0.5, // hp per second
  sleepRegen: 100 / 40,                          // sleep restored per second while asleep
  eatSec: 2, grubFood: 45, mushroomFood: 35,
  moveSec: 0.2,                                  // seconds per mole length moved
  ventReach: 160, ventEverySec: 1,               // max air-path cost to the sky (mole lengths)
  searchBudget: 6000,                            // path search budget (mole lengths²)
  grubChance: { Dirt: 0.12, Clay: 0.08, Gravel: 0.04, Debris: 0.03 } as Record<string, number>,
  farmCost: { wood: 4 } as Record<string, number>,
  farmBuildSec: 6, farmGrowSec: 25, farmCap: 5,
};
export type NeedsConfig = typeof NEEDS_DEFAULTS;

export interface MoleNeeds {
  hunger: number; sleep: number; oxygen: number; activity: Activity;
  path: Pt[]; target: Pt | null; timer: number; moveT: number;
  stale: number; ventT: number; workAcc: number; dmgAcc: number; healAcc: number;
}
export interface Farm { x: number; y: number; built: boolean; paid: boolean; progress: number; mushrooms: number; growT: number; builder: number }

const clamp = (v: number, a: number, b: number) => v < a ? a : v > b ? b : v;

export class NeedsSystem {
  readonly cfg: NeedsConfig;
  needs = new Map<number, MoleNeeds>();
  farms: Farm[] = [];
  grubs = 0;
  private rng: () => number;
  private dt: number;

  constructor(readonly host: NeedsHost, cfg: Partial<NeedsConfig> = {}) {
    this.cfg = { ...NEEDS_DEFAULTS, ...cfg };
    this.rng = mulberry32(host.seed * 9301 + 49297);
    this.dt = 1 / host.tickHz;
  }

  get(id: number): MoleNeeds {
    let n = this.needs.get(id);
    if (!n) { n = { hunger: 100, sleep: 100, oxygen: 100, activity: "none", path: [], target: null, timer: 0, moveT: 0, stale: 0, ventT: 0, workAcc: 0, dmgAcc: 0, healAcc: 0 }; this.needs.set(id, n); }
    return n;
  }
  busy(id: number) { return this.get(id).activity !== "none"; }
  mushrooms() { let s = 0; for (const f of this.farms) if (f.built) s += f.mushrooms; return s; }
  foodTotal() { return this.grubs + this.mushrooms(); }

  /** Work-rate gate: returns false on ticks a weak mole skips its job work. */
  workTick(id: number): boolean {
    const n = this.get(id), c = this.cfg;
    if (n.hunger >= c.slowBelow && n.sleep >= c.slowBelow) return true;
    n.workAcc += 0.5; if (n.workAcc >= 1) { n.workAcc -= 1; return true; }
    return false;
  }

  /** Called when mole `by` finishes digging out a cell of material `mat` (e.g. "Dirt"). */
  onDug(mat: string, by: number) {
    const p = (this.cfg.grubChance[mat] || 0) / (this.host.cellScale ** 2);
    if (p > 0 && this.rng() < p) { this.grubs++; this.host.say(`🐛 Mole #${by} found a grub`); }
  }

  /** Tool hook: "farm" places a mushroom-farm site, "cancel" removes an unbuilt one. */
  designate(tool: string, x: number, y: number) {
    const h = this.host;
    if (tool === "farm") {
      if (this.farms.some(f => f.x === x && f.y === y)) return;
      if (!h.isWalkable(x, y) || y + 1 >= h.height || !h.isSolid(x, y + 1) || h.isSubmerged(x, y)) return;
      this.farms.push({ x, y, built: false, paid: false, progress: 0, mushrooms: 0, growT: 0, builder: -1 });
    } else if (tool === "cancel") {
      const i = this.farms.findIndex(f => f.x === x && f.y === y && !f.built);
      if (i >= 0) { const f = this.farms[i]; if (f.paid) this.refundFarm(); this.release(f); this.farms.splice(i, 1); }
    }
  }

  step() {
    const h = this.host, c = this.cfg, dt = this.dt;
    this.stepFarms();
    for (const m of h.moles()) {
      const n = this.get(m.id);
      if (m.dead) { if (n.activity !== "none") this.setActivity(n, "none"); continue; }
      const sub = h.isSubmerged(m.x, m.y);
      // --- decay ---
      const work = m.hasJob ? c.workMul : 1;
      n.hunger = clamp(n.hunger - c.hungerDecay * work * dt, 0, 100);
      if (n.activity === "sleeping") n.sleep = clamp(n.sleep + c.sleepRegen * dt, 0, 100);
      else n.sleep = clamp(n.sleep - c.sleepDecay * work * dt, 0, 100);
      if (sub) n.oxygen -= c.o2Drown * dt;
      else {
        if ((n.ventT -= dt) <= 0) { n.ventT = c.ventEverySec; n.stale = this.staleness(m.x, m.y); }
        n.oxygen += n.stale > 0 ? -c.o2Stale * n.stale * dt : c.o2Recover * dt;
      }
      n.oxygen = clamp(n.oxygen, 0, 100);
      h.setOxygen(m.id, n.oxygen);
      // --- health ---
      let dmg = 0, why = "";
      if (n.oxygen <= 0) { dmg += (sub ? c.drownDmg : c.staleDmg) * dt; why = sub ? "drowned" : "suffocated"; }
      if (n.hunger <= 0) { dmg += c.starveDmg * dt; why ||= "starved"; }
      if (dmg > 0) { n.dmgAcc += dmg; if (n.dmgAcc >= 1) { const d = Math.floor(n.dmgAcc); n.dmgAcc -= d; h.hurtMole(m.id, d, why); } }
      else if (n.hunger > 60 && n.sleep > 50 && n.oxygen > 80) { n.healAcc += c.regen * dt; if (n.healAcc >= 1) { n.healAcc -= 1; h.healMole(m.id, 1); } }
      if (h.moles().find(o => o.id === m.id)?.dead) { this.setActivity(n, "none"); continue; }
      // --- behaviour ---
      this.decide(m, n, sub);
      this.act(m, n, sub);
    }
  }

  /** 0 = fresh air reaches this cell from the sky (through tunnels or porous soil), 1 = stale. */
  private staleness(x: number, y: number) {
    const h = this.host;
    return costTo(h.width, h.height, { x, y }, (a, b) => h.airCost(a, b), (a, b) => h.isSky(a, b), this.cfg.ventReach * h.cellScale) === Infinity ? 1 : 0;
  }

  private decide(m: HostMole, n: MoleNeeds, sub: boolean) {
    const c = this.cfg, a = n.activity;
    if (a === "toAir" || a === "eating") return;
    if (sub && n.oxygen < c.lowO2) {
      const p = this.path(m, (x, y) => !this.host.isSubmerged(x, y) && !this.host.isHazard(x, y));
      if (p) { this.interrupt(m, n); n.path = p; this.setActivity(n, "toAir"); }
      return;
    }
    if (a === "sleeping") { if (n.sleep >= c.rested || sub) this.setActivity(n, "none"); return; }
    if (a === "toFood") return;
    const free = !m.hasJob && (a === "none");
    if (n.hunger < c.starving || (n.hunger < c.hungry && free)) {
      if (this.goEat(m, n)) return;
    }
    if (n.sleep <= 0 || (n.sleep < c.tired && free)) {
      if (!sub && !this.host.isHazard(m.x, m.y)) { this.interrupt(m, n); this.setActivity(n, "sleeping"); return; }
    }
    if (free) this.takeBuild(m, n);
  }

  private goEat(m: HostMole, n: MoleNeeds) {
    const h = this.host;
    const farmAt = (x: number, y: number) => this.farms.some(f => f.built && f.mushrooms > 0 && f.x === x && f.y === y);
    const st = this.grubs > 0 ? h.stockpile() : null;
    if (!this.mushrooms() && !st) return false;
    const p = this.path(m, (x, y) => farmAt(x, y) || (!!st && st.x === x && st.y === y));
    if (!p) return false;
    this.interrupt(m, n);
    n.path = p; n.target = p.length ? p[p.length - 1] : { x: m.x, y: m.y };
    this.setActivity(n, "toFood");
    return true;
  }

  private takeBuild(m: HostMole, n: MoleNeeds) {
    const sites = this.farms.filter(f => !f.built && f.builder < 0);
    if (!sites.length) return;
    const p = this.path(m, (x, y) => sites.some(f => f.x === x && f.y === y));
    if (!p) return;
    const t = p.length ? p[p.length - 1] : { x: m.x, y: m.y };
    const f = sites.find(s => s.x === t.x && s.y === t.y)!;
    if (!f.paid) { if (!this.payFarm()) return; f.paid = true; }
    f.builder = m.id; n.path = p; n.target = t; this.setActivity(n, "toBuild");
  }

  private act(m: HostMole, n: MoleNeeds, sub: boolean) {
    const c = this.cfg, dt = this.dt;
    switch (n.activity) {
      case "toAir": if (this.walk(m, n)) this.setActivity(n, "none"); break;
      case "toFood": if (this.walk(m, n)) { n.timer = c.eatSec; this.setActivity(n, "eating", true); } break;
      case "eating":
        if ((n.timer -= dt) > 0) break;
        if (!this.eatOne(m, n)) { this.setActivity(n, "none"); break; }
        if (n.hunger >= c.fullHunger) this.setActivity(n, "none"); else n.timer = c.eatSec;
        break;
      case "toBuild": {
        const f = this.farmFor(n);
        if (!f || f.built) { this.setActivity(n, "none"); break; }
        if (this.walk(m, n)) this.setActivity(n, "building", true);
        break;
      }
      case "building": {
        const f = this.farmFor(n);
        if (!f || f.built || sub) { if (f) f.builder = -1; this.setActivity(n, "none"); break; }
        if (this.workTick(m.id)) f.progress += dt / c.farmBuildSec;
        if (f.progress >= 1) { f.built = true; f.builder = -1; this.host.say(`🍄 Mushroom farm built at ${f.x},${f.y}`); this.setActivity(n, "none"); }
        break;
      }
    }
  }

  private eatOne(m: HostMole, n: MoleNeeds) {
    const st = this.host.stockpile();
    const f = this.farms.find(f => f.built && f.mushrooms > 0 && f.x === m.x && f.y === m.y);
    if (f) { f.mushrooms--; n.hunger = clamp(n.hunger + this.cfg.mushroomFood, 0, 100); return true; }
    if (st && st.x === m.x && st.y === m.y && this.grubs > 0) { this.grubs--; n.hunger = clamp(n.hunger + this.cfg.grubFood, 0, 100); return true; }
    return false;
  }

  /** Advance along n.path; returns true on arrival. Aborts the activity if blocked. */
  private walk(m: HostMole, n: MoleNeeds) {
    if (!n.path.length) return true;
    n.moveT += this.dt;
    const per = this.cfg.moveSec / this.host.cellScale * (this.host.isSubmerged(m.x, m.y) ? 2 : 1);
    if (n.moveT < per) return false;
    n.moveT = 0;
    const p = n.path[0];
    if (!this.host.isWalkable(p.x, p.y)) { this.setActivity(n, "none"); return false; }
    n.path.shift(); this.host.moveMole(m.id, p.x, p.y); m.x = p.x; m.y = p.y;
    return n.path.length === 0;
  }

  private stepFarms() {
    const h = this.host, dt = this.dt;
    for (let i = this.farms.length - 1; i >= 0; i--) {
      const f = this.farms[i];
      if (h.isSolid(f.x, f.y) || h.isHazard(f.x, f.y)) {
        if (f.built) h.say(`🍄 Mushroom farm at ${f.x},${f.y} destroyed`); else if (f.paid) this.refundFarm();
        this.release(f); this.farms.splice(i, 1); continue;
      }
      if (!f.built || h.isSubmerged(f.x, f.y) || f.mushrooms >= this.cfg.farmCap) continue;
      if ((f.growT += dt) >= this.cfg.farmGrowSec) { f.growT = 0; f.mushrooms++; }
    }
  }

  private path(m: HostMole, goal: (x: number, y: number) => boolean) {
    const h = this.host;
    return bfsPath(h.width, h.height, { x: m.x, y: m.y }, (x, y) => h.isWalkable(x, y), goal, Math.round(this.cfg.searchBudget * h.cellScale ** 2));
  }
  private interrupt(m: HostMole, n: MoleNeeds) { if (m.hasJob) { this.host.dropJob(m.id); m.hasJob = false; } if (n.activity !== "none") this.setActivity(n, "none"); }
  private farmFor(n: MoleNeeds) { const t = n.target; return t ? this.farms.find(f => f.x === t.x && f.y === t.y) : undefined; }
  private release(f: Farm) { if (f.builder >= 0) { const n = this.needs.get(f.builder); if (n && (n.activity === "toBuild" || n.activity === "building")) this.setActivity(n, "none"); f.builder = -1; } }
  private setActivity(n: MoleNeeds, a: Activity, keepTarget = false) {
    if ((n.activity === "toBuild" || n.activity === "building") && a !== "building") { const f = this.farmFor(n); if (f && !f.built) f.builder = -1; }
    n.activity = a; n.moveT = 0;
    if (!keepTarget && a === "none") { n.path = []; n.target = null; }
  }
  private payFarm() {
    const cost = Object.entries(this.cfg.farmCost), paid: [string, number][] = [];
    for (const [r, k] of cost) { if (!this.host.spend(r, k)) { for (const [pr, pk] of paid) this.host.refund(pr, pk); return false; } paid.push([r, k]); }
    return true;
  }
  private refundFarm() { for (const [r, k] of Object.entries(this.cfg.farmCost)) this.host.refund(r, k); }
}
