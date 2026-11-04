// Disaster events: floods, lava surges and earthquakes, each announced by a warning countdown.
// Engine-agnostic: talks to the world only through DisasterWorld. Sizes are fractions of the
// world, times are in seconds (converted with the sim rate), so it ports to any grid resolution.

export type DisasterKind = "flood" | "lava" | "quake";
export type FluidKind = "water" | "lava";

/** Minimal world view a disaster needs. Coordinates are cell coordinates of the host engine. */
export interface DisasterWorld {
  readonly width: number;
  readonly height: number;
  /** Cell can hold fluid / be walked through. */
  isOpen(x: number, y: number): boolean;
  /** Solid, non-anchored material that an earthquake can shake loose. */
  isLoose(x: number, y: number): boolean;
  fluidAt(x: number, y: number, kind: FluidKind): number;
  addFluid(x: number, y: number, kind: FluidKind, amount: number): void;
  /** Knock a cell loose; the host's own cave-in mechanism handles what follows. */
  dislodge(x: number, y: number): void;
  shake(intensity: number): void;
  say(text: string): void;
}

export interface DisasterConfig {
  hz: number;             // sim ticks per second
  minGap: number;         // seconds of calm between disasters
  maxGap: number;
  warning: number;        // seconds of countdown before a disaster hits
  duration: Record<DisasterKind, number>; // seconds a disaster stays active
  kinds: DisasterKind[];  // which disasters may be rolled
  floodVolume: number;    // total fluid mass, as fraction of world cell count
  lavaVolume: number;
  sourceWidth: number;    // fluid source width, as fraction of world width
  quakeLoose: number;     // max cells knocked loose, as fraction of world cell count
  quakeSamples: number;   // random probes per tick, as fraction of world cell count
  quakeRadius: number;    // half-size of the shaken area around the epicentre, as fraction of world width
}

export const DEFAULT_CONFIG: DisasterConfig = {
  hz: 20, minGap: 90, maxGap: 180, warning: 10,
  duration: { flood: 8, lava: 8, quake: 4 },
  kinds: ["flood", "lava", "quake"],
  floodVolume: 0.012, lavaVolume: 0.006, sourceWidth: 0.03,
  quakeLoose: 0.0015, quakeSamples: 0.003, quakeRadius: 0.3,
};

export const DISASTER_INFO: Record<DisasterKind, { name: string; icon: string; color: string }> = {
  flood: { name: "Flood", icon: "🌊", color: "#3A8FD9" },
  lava: { name: "Lava surge", icon: "🌋", color: "#FF6A1F" },
  quake: { name: "Earthquake", icon: "🌐", color: "#C89B5B" },
};

export type Phase = "calm" | "warning" | "active";
export interface DisasterState {
  phase: Phase;
  kind: DisasterKind | null;
  x: number; y: number;   // epicentre / source cell
  ticksLeft: number;      // until the next phase change
  loosened: number;       // quake: cells knocked loose so far
}

export class DisasterDirector {
  readonly cfg: DisasterConfig;
  state: DisasterState;
  enabled = true;

  constructor(private world: DisasterWorld, private rng: () => number, cfg: Partial<DisasterConfig> = {}) {
    this.cfg = { ...DEFAULT_CONFIG, ...cfg, duration: { ...DEFAULT_CONFIG.duration, ...cfg.duration } };
    this.state = { phase: "calm", kind: null, x: 0, y: 0, ticksLeft: this.rollGap(), loosened: 0 };
  }

  /** Seconds until the current phase ends (countdown shown to the player). */
  get secondsLeft() { return this.state.ticksLeft / this.cfg.hz; }

  /** Start the warning for a disaster now (debug / scripted events). Returns false if no site found. */
  schedule(kind: DisasterKind): boolean {
    const site = this.pickSite(kind);
    if (!site) return false;
    const w = this.cfg.warning * this.cfg.hz;
    this.state = { phase: "warning", kind, x: site.x, y: site.y, ticksLeft: Math.max(1, Math.round(w)), loosened: 0 };
    const info = DISASTER_INFO[kind];
    this.world.say(`${info.icon} ${info.name} warning! ${Math.round(this.cfg.warning)}s until it hits near ${site.x},${site.y}`);
    return true;
  }

  step() {
    const s = this.state;
    if (s.phase === "calm") {
      if (!this.enabled || --s.ticksLeft > 0) return;
      const kinds = this.cfg.kinds;
      if (!kinds.length || !this.schedule(kinds[Math.floor(this.rng() * kinds.length)])) s.ticksLeft = this.rollGap();
      return;
    }
    if (s.phase === "warning") {
      if (--s.ticksLeft > 0) return;
      s.phase = "active"; s.ticksLeft = Math.max(1, Math.round(this.cfg.duration[s.kind!] * this.cfg.hz));
      const info = DISASTER_INFO[s.kind!];
      this.world.say(`${info.icon} ${info.name}!`);
    }
    this.act(s.kind!);
    if (--s.ticksLeft <= 0) this.state = { phase: "calm", kind: null, x: 0, y: 0, ticksLeft: this.rollGap(), loosened: 0 };
  }

  private rollGap() {
    const { minGap, maxGap, hz } = this.cfg;
    return Math.max(1, Math.round((minGap + (maxGap - minGap) * this.rng()) * hz));
  }

  private get area() { return this.world.width * this.world.height; }
  private activeTicks(kind: DisasterKind) { return Math.max(1, Math.round(this.cfg.duration[kind] * this.cfg.hz)); }

  private act(kind: DisasterKind) {
    if (kind === "quake") return this.quake();
    const fluid: FluidKind = kind === "flood" ? "water" : "lava";
    const volume = (kind === "flood" ? this.cfg.floodVolume : this.cfg.lavaVolume) * this.area;
    this.pour(fluid, volume / this.activeTicks(kind));
  }

  /** Inject `amount` of fluid spread over the open cells of a horizontal source around the site. */
  private pour(kind: FluidKind, amount: number) {
    const { x, y } = this.state, w = this.world;
    const half = Math.max(0, Math.round(this.cfg.sourceWidth * w.width / 2));
    const cells: number[] = [];
    for (let dx = -half; dx <= half; dx++) if (this.inb(x + dx, y) && w.isOpen(x + dx, y)) cells.push(x + dx);
    if (!cells.length) return;
    for (const cx of cells) w.addFluid(cx, y, kind, amount / cells.length);
  }

  private quake() {
    const w = this.world, s = this.state;
    w.shake(8);
    const cap = Math.max(1, Math.round(this.cfg.quakeLoose * this.area));
    // Spread the cap evenly over the active time so the quake rumbles rather than pops.
    const ticks = this.activeTicks("quake"), elapsed = ticks - s.ticksLeft + 1;
    const target = Math.min(cap, Math.ceil(cap * elapsed / ticks));
    const samples = Math.max(1, Math.round(this.cfg.quakeSamples * this.area));
    const r = Math.max(1, Math.round(this.cfg.quakeRadius * w.width));
    for (let k = 0; k < samples && s.loosened < target; k++) {
      const x = s.x + Math.floor((this.rng() * 2 - 1) * r), y = s.y + Math.floor((this.rng() * 2 - 1) * r);
      if (!this.inb(x, y)) continue;
      // Only overhangs: loose cells with open space below can actually fall.
      if (w.isLoose(x, y) && this.inb(x, y + 1) && w.isOpen(x, y + 1)) { w.dislodge(x, y); s.loosened++; }
    }
  }

  private inb(x: number, y: number) { return x >= 0 && y >= 0 && x < this.world.width && y < this.world.height; }

  /** Pick a random open cell in the band of depths where this disaster makes sense. */
  pickSite(kind: DisasterKind): { x: number; y: number } | null {
    const w = this.world;
    const [lo, hi] = kind === "flood" ? [0, 0.5] : kind === "lava" ? [0.6, 1] : [0, 1];
    const fluid: FluidKind | null = kind === "flood" ? "water" : kind === "lava" ? "lava" : null;
    const y0 = Math.floor(lo * w.height), y1 = Math.max(y0 + 1, Math.ceil(hi * w.height));
    const tries = Math.max(64, Math.round(this.area * 0.05));
    let fallback: { x: number; y: number } | null = null;
    for (let k = 0; k < tries; k++) {
      const x = Math.floor(this.rng() * w.width), y = y0 + Math.floor(this.rng() * (y1 - y0));
      if (!this.inb(x, y) || !w.isOpen(x, y)) continue;
      // Quakes and fluids prefer underground sites (solid above); fluids prefer existing pockets.
      if (!this.covered(x, y)) { fallback ??= { x, y }; continue; }
      if (!fluid || w.fluidAt(x, y, fluid) > 0.1) return { x, y };
      fallback = fallback && this.covered(fallback.x, fallback.y) ? fallback : { x, y };
    }
    if (fallback) return fallback;
    // Sparse open space: scan the band from a random offset so small caverns are still found.
    const span = w.width * (y1 - y0), start = Math.floor(this.rng() * span);
    for (let k = 0; k < span; k++) {
      const c = (start + k) % span, x = c % w.width, y = y0 + Math.floor(c / w.width);
      if (this.inb(x, y) && w.isOpen(x, y)) return { x, y };
    }
    return null;
  }

  private covered(x: number, y: number) {
    for (let yy = y - 1; yy >= 0; yy--) if (!this.world.isOpen(x, yy)) return true;
    return false;
  }
}
