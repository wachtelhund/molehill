// Small grid search helpers for the needs system. Pure, host-agnostic, bounded by a
// budget so they stay cheap on very large (high-res) grids. Visited sets are Maps,
// not width*height arrays, for the same reason.
export interface Pt { x: number; y: number }

const DIRS = [[0, -1], [0, 1], [-1, 0], [1, 0]];

export function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

/** BFS from start to the nearest cell satisfying goal. Returns steps (excluding start), [] if
 *  already at a goal, or null if none found within `budget` visited cells. */
export function bfsPath(w: number, h: number, start: Pt, walkable: (x: number, y: number) => boolean,
  goal: (x: number, y: number) => boolean, budget: number): Pt[] | null {
  if (goal(start.x, start.y)) return [];
  const s = start.y * w + start.x, par = new Map<number, number>([[s, -1]]), q = [s];
  for (let head = 0; head < q.length && par.size <= budget; head++) {
    const i = q[head], x = i % w, y = (i / w) | 0;
    for (const [dx, dy] of DIRS) {
      const nx = x + dx, ny = y + dy; if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
      const n = ny * w + nx; if (par.has(n) || !walkable(nx, ny)) continue;
      par.set(n, i);
      if (goal(nx, ny)) {
        const out: Pt[] = []; for (let c = n; c !== s; c = par.get(c)!) out.push({ x: c % w, y: (c / w) | 0 });
        return out.reverse();
      }
      q.push(n);
    }
  }
  return null;
}

/** Dijkstra over per-cell costs (cost < 0 = impassable). Returns the cheapest cost to reach
 *  any goal cell, or Infinity if none within maxCost. */
export function costTo(w: number, h: number, start: Pt, cost: (x: number, y: number) => number,
  goal: (x: number, y: number) => boolean, maxCost: number): number {
  if (goal(start.x, start.y)) return 0;
  const best = new Map<number, number>(), heap = new MinHeap();
  const s = start.y * w + start.x; best.set(s, 0); heap.push(0, s);
  while (heap.size) {
    const [d, i] = heap.pop(); if (d > (best.get(i) ?? Infinity)) continue;
    const x = i % w, y = (i / w) | 0;
    if (goal(x, y)) return d;
    for (const [dx, dy] of DIRS) {
      const nx = x + dx, ny = y + dy; if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
      const c = cost(nx, ny); if (c < 0) continue;
      const nd = d + c, n = ny * w + nx; if (nd > maxCost || nd >= (best.get(n) ?? Infinity)) continue;
      best.set(n, nd); heap.push(nd, n);
    }
  }
  return Infinity;
}

class MinHeap {
  private k: number[] = []; private v: number[] = [];
  get size() { return this.k.length; }
  push(k: number, v: number) {
    const K = this.k, V = this.v; let i = K.length; K.push(k); V.push(v);
    while (i > 0) { const p = (i - 1) >> 1; if (K[p] <= k) break; K[i] = K[p]; V[i] = V[p]; i = p; }
    K[i] = k; V[i] = v;
  }
  pop(): [number, number] {
    const K = this.k, V = this.v, rk = K[0], rv = V[0], lk = K.pop()!, lv = V.pop()!, n = K.length;
    if (n) {
      let i = 0;
      for (;;) { let c = 2 * i + 1; if (c >= n) break; if (c + 1 < n && K[c + 1] < K[c]) c++; if (K[c] >= lk) break; K[i] = K[c]; V[i] = V[c]; i = c; }
      K[i] = lk; V[i] = lv;
    }
    return [rk, rv];
  }
}
