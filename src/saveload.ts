// Save/load core. Engine-agnostic: knows nothing about tiles, materials or grid size.
// A world is described to this module only through a WorldAdapter, which exposes it as
// a set of named per-cell layers (typed arrays of width*height) plus a JSON-able state blob.
// Coarser/other grids (e.g. a block layer) are declared in `grids`; a layer keyed "grid:name"
// must have that grid's width*height elements, an unprefixed layer the main width*height.
// Porting to a new engine = writing a new adapter (see saveload-sim.ts for the current one).

export const SAVE_FORMAT = "molehill-save";
export const SAVE_VERSION = 1;

export type LayerArray = Uint8Array | Uint16Array | Int16Array | Int32Array | Uint32Array | Float32Array;
type LayerKind = "u8" | "u16" | "i16" | "i32" | "u32" | "f32";
const CTORS: Record<LayerKind, { new (n: number): LayerArray; BYTES_PER_ELEMENT: number }> = {
  u8: Uint8Array, u16: Uint16Array, i16: Int16Array, i32: Int32Array, u32: Uint32Array, f32: Float32Array,
};
function kindOf(a: LayerArray): LayerKind {
  if (a instanceof Uint8Array) return "u8"; if (a instanceof Uint16Array) return "u16"; if (a instanceof Int16Array) return "i16";
  if (a instanceof Int32Array) return "i32"; if (a instanceof Uint32Array) return "u32"; return "f32";
}

/** What an engine captures for saving. */
export interface GridDims { width: number; height: number }
export interface Capture { width: number; height: number; grids?: Record<string, GridDims>; layers: Record<string, LayerArray>; state: unknown }

/** Bridge between a concrete engine's world and the save format. */
export interface WorldAdapter<World> {
  /** Identifies the engine/data model; saves from another engine are rejected. */
  engine: string;
  /** Layers that must be present for a save to be loadable. */
  requiredLayers: readonly string[];
  capture(w: World): Capture;
  /** Build a world from decoded data. May throw; the error becomes a load failure. */
  restore(data: Capture): World;
}

export interface SaveLayer { kind: LayerKind; len: number; data: string }
export interface SaveFile {
  format: typeof SAVE_FORMAT; version: number; engine: string; savedAt: number;
  width: number; height: number; grids?: Record<string, GridDims>; layers: Record<string, SaveLayer>; state: unknown;
}

export type LoadResult<World> = { ok: true; world: World; save: SaveFile } | { ok: false; error: string };

/** Upgrades a save of version v to v+1. Register here when SAVE_VERSION is bumped. */
export const MIGRATIONS: Record<number, (s: any) => any> = {};

// ---------- encoding: byte-level RLE + base64 (sparse layers shrink a lot) ----------
// RLE stream: [count-1, byte] pairs, count in 1..256.
export function rle(bytes: Uint8Array): Uint8Array {
  const out: number[] = [];
  for (let i = 0; i < bytes.length;) {
    const b = bytes[i]; let n = 1;
    while (n < 256 && i + n < bytes.length && bytes[i + n] === b) n++;
    out.push(n - 1, b); i += n;
  }
  return Uint8Array.from(out);
}
export function unrle(src: Uint8Array, expected: number): Uint8Array {
  if (src.length % 2) throw new Error("bad RLE stream");
  const out = new Uint8Array(expected); let o = 0;
  for (let i = 0; i < src.length; i += 2) {
    const n = src[i] + 1; if (o + n > expected) throw new Error("RLE overflow");
    out.fill(src[i + 1], o, o + n); o += n;
  }
  if (o !== expected) throw new Error("RLE length mismatch");
  return out;
}
const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64I = new Int16Array(128).fill(-1); for (let i = 0; i < 64; i++) B64I[B64.charCodeAt(i)] = i;
export function toB64(b: Uint8Array): string {
  let s = "";
  for (let i = 0; i < b.length; i += 3) {
    const n = (b[i] << 16) | ((b[i + 1] ?? 0) << 8) | (b[i + 2] ?? 0);
    s += B64[n >> 18] + B64[(n >> 12) & 63] + (i + 1 < b.length ? B64[(n >> 6) & 63] : "=") + (i + 2 < b.length ? B64[n & 63] : "=");
  }
  return s;
}
export function fromB64(s: string): Uint8Array {
  if (typeof s !== "string" || s.length % 4) throw new Error("bad base64");
  const pad = s.endsWith("==") ? 2 : s.endsWith("=") ? 1 : 0;
  const out = new Uint8Array(s.length / 4 * 3 - pad);
  for (let i = 0, o = 0; i < s.length; i += 4) {
    let n = 0;
    for (let k = 0; k < 4; k++) {
      const c = s.charCodeAt(i + k), v = c === 61 && i + k >= s.length - pad ? 0 : c < 128 ? B64I[c] : -1;
      if (v < 0) throw new Error("bad base64");
      n = (n << 6) | v;
    }
    if (o < out.length) out[o++] = n >> 16; if (o < out.length) out[o++] = (n >> 8) & 255; if (o < out.length) out[o++] = n & 255;
  }
  return out;
}
export function encodeLayer(a: LayerArray): SaveLayer {
  const bytes = new Uint8Array(a.buffer, a.byteOffset, a.byteLength); // platform endianness (little on all browsers)
  return { kind: kindOf(a), len: a.length, data: toB64(rle(bytes)) };
}
export function decodeLayer(l: SaveLayer): LayerArray {
  const C = CTORS[l?.kind]; if (!C) throw new Error(`unknown layer kind ${l?.kind}`);
  if (!Number.isInteger(l.len) || l.len < 0) throw new Error("bad layer length");
  const bytes = unrle(fromB64(l.data), l.len * C.BYTES_PER_ELEMENT);
  const out = new C(l.len); new Uint8Array(out.buffer).set(bytes); return out;
}

// ---------- (de)serialize ----------
export function serialize<World>(adapter: WorldAdapter<World>, w: World, now = Date.now()): string {
  const c = adapter.capture(w);
  const layers: Record<string, SaveLayer> = {};
  for (const [k, a] of Object.entries(c.layers)) layers[k] = encodeLayer(a);
  const f: SaveFile = { format: SAVE_FORMAT, version: SAVE_VERSION, engine: adapter.engine, savedAt: now, width: c.width, height: c.height, layers, state: c.state };
  if (c.grids) f.grids = c.grids;
  return JSON.stringify(f);
}

/** Parse + validate + migrate + restore. Never throws. */
export function deserialize<World>(adapter: WorldAdapter<World>, text: string | null | undefined): LoadResult<World> {
  if (text == null || text === "") return { ok: false, error: "no save data" };
  let f: any;
  try { f = JSON.parse(text); } catch { return { ok: false, error: "save is corrupt (not JSON)" }; }
  if (!f || typeof f !== "object" || f.format !== SAVE_FORMAT) return { ok: false, error: "not a molehill save" };
  if (!Number.isInteger(f.version) || f.version < 0) return { ok: false, error: "save has no valid version" };
  if (f.version > SAVE_VERSION) return { ok: false, error: `save version ${f.version} is newer than supported (${SAVE_VERSION})` };
  try { while (f.version < SAVE_VERSION) { const m = MIGRATIONS[f.version]; if (!m) throw new Error(`no migration from v${f.version}`); f = m(f); } }
  catch (e) { return { ok: false, error: `migration failed: ${(e as Error).message}` }; }
  if (f.engine !== adapter.engine) return { ok: false, error: `save is for engine "${f.engine}", expected "${adapter.engine}"` };
  const { width, height } = f;
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) return { ok: false, error: "save has invalid dimensions" };
  if (!f.layers || typeof f.layers !== "object") return { ok: false, error: "save has no layers" };
  for (const k of adapter.requiredLayers) if (!(k in f.layers)) return { ok: false, error: `save is missing layer "${k}"` };
  const grids: Record<string, GridDims> = {};
  if (f.grids != null) {
    if (typeof f.grids !== "object") return { ok: false, error: "save has invalid grids" };
    for (const [g, d] of Object.entries<any>(f.grids)) {
      if (!d || !Number.isInteger(d.width) || !Number.isInteger(d.height) || d.width <= 0 || d.height <= 0) return { ok: false, error: `save has invalid grid "${g}"` };
      grids[g] = { width: d.width, height: d.height };
    }
  }
  const layers: Record<string, LayerArray> = {};
  try {
    for (const [k, l] of Object.entries(f.layers as Record<string, SaveLayer>)) {
      const sep = k.indexOf(":"), g = sep < 0 ? null : grids[k.slice(0, sep)];
      if (sep >= 0 && !g) throw new Error(`layer "${k}" refers to an undeclared grid`);
      const cells = g ? g.width * g.height : width * height;
      const a = decodeLayer(l);
      if (a.length !== cells) throw new Error(`layer "${k}" has ${a.length} cells, expected ${cells}`);
      layers[k] = a;
    }
  } catch (e) { return { ok: false, error: `save is corrupt: ${(e as Error).message}` }; }
  try { return { ok: true, world: adapter.restore({ width, height, grids, layers, state: f.state }), save: f as SaveFile }; }
  catch (e) { return { ok: false, error: `cannot restore: ${(e as Error).message}` }; }
}

// ---------- storage ----------
/** Subset of the Web Storage API; localStorage satisfies it, tests pass a Map-backed fake. */
export interface KV { getItem(k: string): string | null; setItem(k: string, v: string): void; removeItem(k: string): void }
export function memoryKV(): KV & { map: Map<string, string> } {
  const map = new Map<string, string>();
  return { map, getItem: k => map.get(k) ?? null, setItem: (k, v) => void map.set(k, String(v)), removeItem: k => void map.delete(k) };
}
/** localStorage if usable, else in-memory (private mode, sandboxed iframes, node). */
export function defaultKV(): KV {
  try { const ls = (globalThis as any).localStorage as KV; const t = "__molehill_probe__"; ls.setItem(t, "1"); ls.removeItem(t); return ls; }
  catch { return memoryKV(); }
}

export type Slot = "auto" | "manual";
export class SaveStore<World> {
  constructor(private adapter: WorldAdapter<World>, private kv: KV = defaultKV(), private prefix = "molehill.save.") {}
  key(slot: Slot) { return this.prefix + slot; }
  save(slot: Slot, w: World): { ok: true; bytes: number } | { ok: false; error: string } {
    try { const s = serialize(this.adapter, w); this.kv.setItem(this.key(slot), s); return { ok: true, bytes: s.length }; }
    catch (e) { return { ok: false, error: `save failed: ${(e as Error).message}` }; }
  }
  load(slot: Slot): LoadResult<World> {
    let text: string | null;
    try { text = this.kv.getItem(this.key(slot)); } catch (e) { return { ok: false, error: `storage unavailable: ${(e as Error).message}` }; }
    return deserialize(this.adapter, text);
  }
  has(slot: Slot) { try { return this.kv.getItem(this.key(slot)) != null; } catch { return false; } }
  clear(slot?: Slot) { for (const s of slot ? [slot] : (["auto", "manual"] as Slot[])) try { this.kv.removeItem(this.key(s)); } catch { /* ignore */ } }
}

/** Calls save() every `everyMs` of game time fed through tick(dt). Decoupled from sim tick rate. */
export class Autosaver {
  private acc = 0;
  constructor(public everyMs: number, private save: () => void) {}
  tick(dtMs: number) { this.acc += dtMs; if (this.acc >= this.everyMs) { this.acc = 0; this.save(); } }
  reset() { this.acc = 0; }
}
