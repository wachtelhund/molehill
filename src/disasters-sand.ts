// Adapter: exposes the v2 falling-sand SandWorld as a DisasterWorld.
// The only disasters file that knows sand internals. Materials are classified via KIND/ANCHOR/SPAN,
// never numeric ids, so new materials are handled automatically.
import { SandWorld, CW, CH, SIM_HZ, mulberry32 } from "./sand/world";
import { M, K, KIND, ANCHOR, SPAN } from "./sand/materials";
import { DisasterWorld, FluidKind, DisasterDirector, DisasterKind, DISASTER_INFO } from "./disasters";

const fluidMat = (k: FluidKind) => k === "water" ? M.WATER : M.LAVA;
const isOpenMat = (m: number) => { const k = KIND[m]; return k === K.EMPTY || k === K.LIQUID || k === K.GAS; };
/** Soft = static structural soil that can crumble (dirt, gravel, clay, ore...), not anchors or wood beams. */
const isSoftMat = (m: number) => KIND[m] === K.SOLID && SPAN[m] > 0 && !ANCHOR[m] && m !== M.WOOD;

const isFree = (m: number) => KIND[m] === K.EMPTY || KIND[m] === K.GAS;
/** Spring pressure: nearest free cell reachable through liquid connected to `i0` (upward first), or -1. */
function freeCellNear(world: SandWorld, i0: number, limit = 1024) {
  if (isFree(world.mat[i0])) return i0;
  if (KIND[world.mat[i0]] !== K.LIQUID) return -1;
  const seen = new Set([i0]), q = [i0];
  for (let h = 0; h < q.length && q.length < limit; h++) {
    const i = q[h], x = i % CW;
    for (const n of [i - CW, x > 0 ? i - 1 : -1, x < CW - 1 ? i + 1 : -1, i + CW]) {
      if (n < 0 || n >= CW * CH || seen.has(n)) continue;
      seen.add(n);
      if (isFree(world.mat[n])) return n;
      if (KIND[world.mat[n]] === K.LIQUID) q.push(n);
    }
  }
  return -1;
}

export function sandDisasterWorld(world: SandWorld): DisasterWorld {
  // Fluid comes in fractional amounts per tick; whole cells are spawned as the per-cell balance reaches 1.
  const owed = new Map<number, number>();
  return {
    width: CW, height: CH,
    isOpen: (x, y) => isOpenMat(world.mat[world.ci(x, y)]),
    isLoose: (x, y) => { const i = world.ci(x, y); return isSoftMat(world.mat[i]) && !world.loose[i]; },
    fluidAt: (x, y, kind) => world.mat[world.ci(x, y)] === fluidMat(kind) ? 1 : 0,
    addFluid: (x, y, kind, amount) => {
      const i0 = world.ci(x, y), m = fluidMat(kind);
      let bal = (owed.get(i0) ?? 0) + amount;
      while (bal >= 1) {
        const i = freeCellNear(world, i0);
        if (i < 0) break;
        world.setCell(i, m); bal--;
      }
      owed.set(i0, Math.min(bal, 1));
    },
    dislodge: (x, y) => {
      // Loosen the soil and let the sand sim crumble it; structDirty makes the existing
      // support pass find what it was holding up and bring that down too.
      world.loose[world.ci(x, y)] = 1; world.structDirty = true; world.wake(x, y);
    },
    shake: (k) => { world.shake = Math.max(world.shake, k); },
    say: (text) => world.say(text),
  };
}

/** Build a director for a SandWorld. URL: ?disasters=0 disables random events, ?disaster=<kind> triggers one now. */
export function createDisasters(world: SandWorld, seed: number, params?: URLSearchParams) {
  const d = new DisasterDirector(sandDisasterWorld(world), mulberry32(seed ^ 0xD15A57E5), { hz: SIM_HZ });
  if (params?.get("disasters") === "0") d.enabled = false;
  const kind = params?.get("disaster");
  if (kind && kind in DISASTER_INFO) d.schedule(kind as DisasterKind);
  return d;
}
