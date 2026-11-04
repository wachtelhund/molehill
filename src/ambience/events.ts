// Engine-neutral sound vocabulary shared by the sim adapter (producer) and the
// SoundEngine (consumer). Positions are in grid cells.

export type SoundKind =
  | "explosion" | "cavein" | "thud" | "dig" | "dug" | "beam"
  | "hurt" | "death" | "steam" | "ignite";

export interface SoundEvent { kind: SoundKind; x: number; y: number; intensity?: number }

/** Continuous ambience drivers, each 0..1. */
export interface AmbientLevels { water: number; lava: number; burning: number; depth: number }

/** Where the "ear" is (camera centre) and how far it hears clearly, in cells. */
export interface Listener { x: number; y: number; range: number }

/** Minimum seconds between two sounds of the same kind; keeps bursts (50 debris landing) from clipping. */
export const MIN_GAP: Record<SoundKind, number> = {
  explosion: 0.08, cavein: 1.2, thud: 0.06, dig: 0.09, dug: 0.08, beam: 0.1,
  hurt: 0.25, death: 0.2, steam: 0.15, ignite: 0.2,
};

export class RateLimiter {
  private last = new Map<SoundKind, number>();
  constructor(private gaps: Record<SoundKind, number> = MIN_GAP) {}
  allow(kind: SoundKind, now: number) {
    const t = this.last.get(kind);
    if (t !== undefined && now - t < this.gaps[kind]) return false;
    this.last.set(kind, now); return true;
  }
}

/** Stereo pan (-1..1) and gain (0..1) for an event heard from `l`. Distant sounds stay faintly audible. */
export function spatialize(ev: { x: number; y: number }, l: Listener) {
  const dx = ev.x - l.x, dy = ev.y - l.y;
  const pan = Math.max(-1, Math.min(1, dx / l.range));
  const d = Math.hypot(dx, dy) / l.range;
  return { pan: pan * 0.8, gain: Math.max(0.08, 1 / (1 + d * d)) };
}
