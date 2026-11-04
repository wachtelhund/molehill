// Adapter: exposes the v2 falling-sand engine (SandWorld + its Logic) to the engine-agnostic save core.
// Fields are enumerated generically, so typed arrays / plain fields added to SandWorld or Logic later
// are saved automatically:
//   typed array of NC elements  -> cell layer   "mat", "loose", ...  (CW x CH)
//   typed array of NB elements  -> block layer  "block:digDes", "block:logic.wire", ...  (BW x BH)
//   any other typed array       -> its own 1-D grid "n<len>:..." (chunk rects, netOn, ...)
//   numbers/strings/booleans/plain arrays/plain objects/Sets -> JSON state
// Material ids are stored with the material name table and remapped on load, so the enum may grow.
import { SandWorld, CW, CH, BW, BH, NC, NB, NCH } from "./sand/world";
import { MAT_NAMES, KIND } from "./sand/materials";
import type { WorldAdapter, Capture, LayerArray, GridDims } from "./saveload";

/** Closures / back-references that cannot be serialized (scenario `scripts` are demo-only). */
const SKIP = new Set(["rng", "logic", "scripts", "w"]);
/** Sub-objects enumerated like the world itself; their fields are saved under "<name>.<field>". */
const SUBS = ["logic"] as const;

const isTA = (v: unknown): v is LayerArray => ArrayBuffer.isView(v) && !(v instanceof DataView);
const isPlain = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && Object.getPrototypeOf(v) === Object.prototype;
const isIdx = (n: number) => (v: unknown) => Number.isInteger(v) && (v as number) >= 0 && (v as number) < n;

/** mulberry32 (same stream as sand/world.ts) whose position can be read back for saving. */
export function trackableRng(seed: number) {
  let a = seed >>> 0;
  const r = () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
  return Object.assign(r, { getState: () => a });
}

function gridKey(len: number): string | null { return len === NC ? "" : len === NB ? "block" : len > 0 ? `n${len}` : null; }

function captureFields(obj: object, prefix: string, layers: Record<string, LayerArray>, grids: Record<string, GridDims>) {
  const state: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(obj)) {
    if (SKIP.has(k) || typeof v === "function") continue;
    if (isTA(v)) {
      const g = gridKey(v.length); if (g === null) continue;
      if (g && !grids[g]) grids[g] = g === "block" ? { width: BW, height: BH } : { width: v.length, height: 1 };
      layers[(g ? g + ":" : "") + prefix + k] = v;
    } else if (v instanceof Set) state[k] = { $set: [...v] };
    else if (v === null || typeof v !== "object" || Array.isArray(v) || isPlain(v)) state[k] = v;
  }
  return state;
}

/** Validators for fields whose bad values could crash the sim (indices into the grids). */
const CHECK: Record<string, (v: unknown[]) => unknown[]> = {
  pendingList: a => a.filter(isIdx(NC)), boomQueue: a => a.filter(isIdx(NB)), devList: a => a.filter(isIdx(NB)),
  links: a => a.filter(l => isPlain(l) && isIdx(NB)(l.a) && isIdx(NB)(l.b)),
};

function restoreMole(raw: unknown, i: number, tpl: Record<string, unknown>) {
  if (!isPlain(raw)) throw new Error(`mole ${i} is not an object`);
  if (!isIdx(BW)(raw.bx) || !isIdx(BH)(raw.by)) throw new Error(`mole ${i} out of bounds`);
  const m: Record<string, unknown> = { ...tpl };
  for (const k of Object.keys(tpl)) if (typeof raw[k] === typeof tpl[k] && raw[k] !== null) m[k] = raw[k];
  const j = raw.job;
  m.job = isPlain(j) && typeof j.type === "string" && isIdx(NB)(j.b) ? { type: j.type, b: j.b } : null;
  m.path = Array.isArray(raw.path) ? raw.path.filter(isIdx(NB)) : [];
  return m;
}

function applyState(target: any, saved: unknown, ctx: { moleTpl: Record<string, unknown> }) {
  if (!isPlain(saved)) return;
  for (const [k, v] of Object.entries(saved)) {
    if (SKIP.has(k) || !(k in target)) continue; // unknown/removed fields are ignored
    const cur = target[k];
    if (cur instanceof Set) { if (isPlain(v) && Array.isArray(v.$set)) target[k] = new Set(v.$set); }
    else if (Array.isArray(cur)) {
      if (!Array.isArray(v)) continue;
      target[k] = k === "moles" ? v.map((m, i) => restoreMole(m, i, ctx.moleTpl)) : CHECK[k] ? CHECK[k](v) : v;
    }
    else if (isPlain(cur)) { if (isPlain(v)) for (const f of Object.keys(cur)) if (typeof v[f] === typeof cur[f]) cur[f] = v[f]; }
    else if (cur !== null && typeof cur !== "object" && typeof v === typeof cur) target[k] = v;
  }
}

/** old material id -> current id, matched by name; a renamed material keeps its slot if that slot is free. */
function materialRemap(saved: unknown): Uint8Array | null {
  if (!Array.isArray(saved) || (saved.length === MAT_NAMES.length && saved.every((n, i) => n === MAT_NAMES[i]))) return null;
  const map = new Uint8Array(256).fill(255), claimed = new Set<number>();
  saved.forEach((n, i) => { const j = MAT_NAMES.indexOf(n); if (j >= 0) { map[i] = j; claimed.add(j); } });
  saved.forEach((n, i) => { if (map[i] === 255) { if (i < MAT_NAMES.length && !claimed.has(i)) { map[i] = i; claimed.add(i); } else throw new Error(`unknown material "${n}"`); } });
  return map;
}

export const sandAdapter: WorldAdapter<SandWorld> = {
  engine: "sand-v2",
  requiredLayers: ["mat"],
  capture(w): Capture {
    // SandWorld's own rng cannot report its position: swap in an equivalent trackable stream
    // seeded from it, so the live world and the save continue identically from here on.
    if (!(w.rng as any).getState) w.rng = trackableRng((w.rng() * 4294967296) >>> 0);
    const layers: Record<string, LayerArray> = {}, grids: Record<string, GridDims> = {};
    const state: Record<string, unknown> = { world: captureFields(w, "", layers, grids), rng: (w.rng as any).getState(), materials: MAT_NAMES };
    for (const s of SUBS) state[s] = captureFields(w[s], s + ".", layers, grids);
    return { width: CW, height: CH, grids, layers, state: JSON.parse(JSON.stringify(state)) };
  },
  restore({ width, height, grids, layers, state }) {
    if (width !== CW || height !== CH) throw new Error(`world size ${width}x${height} does not match engine ${CW}x${CH}`);
    const b = grids?.block; if (b && (b.width !== BW || b.height !== BH)) throw new Error(`block grid ${b.width}x${b.height} does not match engine ${BW}x${BH}`);
    const s: any = isPlain(state) ? state : {};
    const w = new SandWorld(1, false);
    w.addMole(0, 0); const moleTpl = w.moles.pop() as unknown as Record<string, unknown>;
    for (const [key, a] of Object.entries(layers)) {
      const path = key.slice(key.indexOf(":") + 1), dot = path.indexOf(".");
      const owner: any = dot < 0 ? w : (SUBS as readonly string[]).includes(path.slice(0, dot)) ? (w as any)[path.slice(0, dot)] : null;
      const field = path.slice(dot + 1), cur = owner?.[field];
      if (!isTA(cur) || cur.constructor !== a.constructor) continue; // unknown or retyped field: keep default
      if (cur.length === a.length) cur.set(a); else if (!key.startsWith("block:") && key.includes(":")) owner[field] = a; // variable-length (e.g. netOn)
    }
    const remap = materialRemap(s.materials);
    for (let i = 0; i < NC; i++) { const m = remap ? remap[w.mat[i]] : w.mat[i]; if (m >= KIND.length) throw new Error(`invalid material id ${w.mat[i]} at cell ${i}`); w.mat[i] = m; }
    applyState(w, s.world, { moleTpl });
    for (const sub of SUBS) applyState(w[sub], s[sub], { moleTpl });
    w.rng = trackableRng(typeof s.rng === "number" ? s.rng : (w.tick * 2654435761) >>> 0);
    if (!(`${gridKey(NCH)}:nx0` in layers)) w.wakeAll(); // no dirty rects saved: simulate everything once
    return w;
  },
};
