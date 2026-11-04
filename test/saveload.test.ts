import { SandWorld, CW, CH, BW, BH, NC, NB } from "../src/sand/world";
import { M, MAT_NAMES } from "../src/sand/materials";
import { D } from "../src/sand/logic";
import { wiringScenario } from "../src/sand/scenarios";
import { serialize, deserialize, rle, unrle, toB64, fromB64, encodeLayer, decodeLayer, SaveStore, memoryKV, Autosaver, SAVE_VERSION, MIGRATIONS, WorldAdapter, KV } from "../src/saveload";
import { sandAdapter } from "../src/saveload-sand";
let fail = 0;
const ok = (c: boolean, msg: string) => { console.log((c ? "PASS " : "FAIL ") + msg); if (!c) fail++; };
const run = (w: SandWorld, n: number) => { for (let i = 0; i < n; i++) w.step(); };
const busy = () => { const w = new SandWorld(42); for (let bx = 30; bx < 60; bx++) w.designate("dig", 14 * BW + bx); run(w, 120); return w; };
const demo = () => { const w = new SandWorld(7); wiringScenario(w); run(w, 220); return w; }; // past all scripted demo events
const eqTA = (a: ArrayLike<number>, b: ArrayLike<number>) => a.length === b.length && Array.prototype.every.call(a, (v: number, i: number) => v === b[i]);
const reload = (w: SandWorld) => { const r = deserialize(sandAdapter, serialize(sandAdapter, w)); if (!r.ok) throw new Error(r.error); return r; };

{ // 1. codec round trips
  const bytes = Uint8Array.from([0, 0, 0, 5, 5, 255, 1, ...new Array(600).fill(7)]);
  ok(eqTA(unrle(rle(bytes), bytes.length), bytes), "RLE round trip (incl. runs > 256)");
  let b64ok = true;
  for (let n = 0; n < 8; n++) { const b = Uint8Array.from({ length: n }, (_, i) => (i * 97 + 13) & 255); if (!eqTA(fromB64(toB64(b)), b)) b64ok = false; }
  ok(b64ok, "base64 round trip for all padding lengths");
  const f = Float32Array.from([0, 1.5, -2.25, 1e-7, 3]), g = decodeLayer(encodeLayer(f));
  ok(g instanceof Float32Array && eqTA(g, f), "Float32 layer round trip is bit-exact");
  const i16 = Int16Array.from([-5, 0, 32767, -32768]), j = decodeLayer(encodeLayer(i16));
  ok(j instanceof Int16Array && eqTA(j, i16), "Int16 layer round trip keeps sign");
}
{ // 2. layers are enumerated generically from SandWorld + Logic
  const f = JSON.parse(serialize(sandAdapter, new SandWorld(3)));
  const want = ["mat", "loose", "shade", "life", "pending", "block:nav", "block:digDes", "block:beamDes", "block:lightReq", "block:logic.wire", "block:logic.dev", "block:logic.devP", "block:logic.devS", "block:logic.burn"];
  const missing = want.filter(k => !(k in f.layers));
  ok(missing.length === 0, `expected layers present${missing.length ? " (missing " + missing + ")" : ""}`);
  ok(f.engine === "sand-v2" && f.width === CW && f.height === CH && f.grids.block.width === BW && f.grids.block.height === BH, "engine id + cell/block grid dims recorded");
  ok(Array.isArray(f.state.world.moles) && typeof f.state.world.tick === "number" && typeof f.state.world.res.wood === "number", "moles / tick / res in JSON state");
  ok(!("scripts" in f.state.world) && !("rng" in f.state.world), "closures (scripts, rng fn) not serialized");
  // a field added later is picked up without touching the adapter
  const w = new SandWorld(3) as any; w.futureLayer = new Uint16Array(NC).fill(9); w.logic.futureBlock = new Uint8Array(NB).fill(2); w.futureCounter = 5;
  const g = JSON.parse(serialize(sandAdapter, w));
  ok("futureLayer" in g.layers && "block:logic.futureBlock" in g.layers && g.state.world.futureCounter === 5, "new typed arrays / fields captured automatically");
}
{ // 3. round trip preserves state
  const w = busy(), text = serialize(sandAdapter, w), r = deserialize(sandAdapter, text);
  ok(r.ok, "busy world deserializes");
  if (r.ok) {
    const v = r.world;
    ok(v.hash() === w.hash() && v.tick === w.tick, `hash/tick preserved (${v.hash()} @ ${v.tick})`);
    ok(JSON.stringify(v.moles) === JSON.stringify(w.moles), "moles (jobs, paths) preserved");
    ok(JSON.stringify(v.res) === JSON.stringify(w.res), "resources preserved");
    ok([...v.reserved].sort().join() === [...w.reserved].sort().join(), "job reservations preserved");
    ok(eqTA(v.digDes, w.digDes) && eqTA(v.shade, w.shade) && eqTA(v.support, w.support), "designations, shade, support preserved");
    ok(r.save.version === SAVE_VERSION, "save carries version");
  }
  console.log(`  save size: ${(text.length / 1024).toFixed(1)} KB for ${NC} cells`);
}
{ // 4. determinism continues across save/load (digging world and wiring/LoRa demo)
  for (const [name, mk] of [["dig colony", busy], ["wiring+LoRa demo", demo]] as const) {
    const a = mk(), r = reload(a);
    run(a, 300); run(r.world, 300);
    ok(a.hash() === r.world.hash() && eqTA(a.logic.devS, r.world.logic.devS) && eqTA(a.logic.wire, r.world.logic.wire), `${name}: loaded world evolves identically (${a.hash()})`);
  }
  const a = demo(); a.logic.place("lever", 5 * BW + 50); a.logic.interact(5 * BW + 50);
  const v = reload(a).world; ok(v.logic.dev[5 * BW + 50] === D.LEVER && v.logic.devS[5 * BW + 50] === 1, "placed device + its state survive");
}
{ // 5. collapse mid-flight survives (pending, loose, dirty rects, rng)
  const w = SandWorld.empty(M.ROCK); w.fillRect(40, 40, 120, 80, M.EMPTY); w.fillRect(40, 30, 120, 39, M.DIRT); w.addMole(20, 19);
  run(w, 30); const r = reload(w); run(w, 200); run(r.world, 200);
  ok(w.hash() === r.world.hash(), "mid-collapse save resumes identically");
}
{ // 6. material table: growing / reordered enum is remapped by name
  const w = new SandWorld(3), o = JSON.parse(serialize(sandAdapter, w));
  const water = MAT_NAMES.indexOf("Water"), rock = MAT_NAMES.indexOf("Rock");
  // pretend the save came from an engine where Water and Rock had swapped ids
  const names = [...MAT_NAMES]; [names[water], names[rock]] = [names[rock], names[water]]; o.state.materials = names;
  const mat = decodeLayer(o.layers.mat) as Uint8Array; for (let i = 0; i < mat.length; i++) mat[i] = mat[i] === water ? rock : mat[i] === rock ? water : mat[i];
  o.layers.mat = encodeLayer(mat);
  const r = deserialize(sandAdapter, JSON.stringify(o));
  ok(r.ok && eqTA(r.world.mat, w.mat), "material ids remapped by name");
  o.state.materials = [...names, "Unobtainium"]; mat[0] = names.length; o.layers.mat = encodeLayer(mat);
  const r2 = deserialize(sandAdapter, JSON.stringify(o)); ok(!r2.ok && /Unobtainium/.test(r2.error), "unknown material rejected");
}
{ // 7. robustness: missing / corrupt / wrong data never throws
  const good = serialize(sandAdapter, new SandWorld(3));
  const bad = (text: any, why: string) => { let r: any; try { r = deserialize(sandAdapter, text); } catch (e) { ok(false, `${why}: threw ${e}`); return; } ok(!r.ok && typeof r.error === "string", `${why} -> rejected ("${r.ok ? "" : r.error}")`); };
  bad(null, "missing save"); bad("", "empty string"); bad("{not json", "garbage text"); bad(good.slice(0, good.length / 2), "truncated JSON");
  bad(JSON.stringify({ hello: 1 }), "foreign JSON"); bad("null", "JSON null"); bad("[1,2]", "JSON array");
  const mut = (f: (o: any) => void) => { const o = JSON.parse(good); f(o); return JSON.stringify(o); };
  bad(mut(o => delete o.version), "no version"); bad(mut(o => o.version = SAVE_VERSION + 1), "future version");
  bad(mut(o => o.engine = "molehill-grid-v1"), "v1 grid save"); bad(mut(o => { o.width = 10; }), "wrong dimensions");
  bad(mut(o => { o.width = CW * 2; o.height = CH / 2; }), "same cell count, different shape");
  bad(mut(o => { o.grids.block = { width: BW * 2, height: BH / 2 }; }), "block grid reshaped");
  bad(mut(o => { delete o.grids.block; }), "undeclared grid"); bad(mut(o => { o.grids.block = { width: -1, height: 2 }; }), "invalid grid dims");
  bad(mut(o => delete o.layers.mat), "missing mat layer"); bad(mut(o => o.layers.mat.data = "@@@@"), "bad base64 layer");
  bad(mut(o => o.layers.mat.data = o.layers.mat.data.slice(4)), "short layer data"); bad(mut(o => o.layers.loose.kind = "f64"), "unknown layer kind");
  bad(mut(o => { const m = new Uint8Array(NC).fill(250); o.layers.mat = encodeLayer(m); }), "out-of-range material ids");
  bad(mut(o => o.state.world.moles = [{ bx: -4, by: 9999 }]), "mole out of bounds");
  const o = JSON.parse(good); delete o.layers.loose; delete o.layers["block:logic.wire"]; o.state = { world: { moles: "nope", tick: "x", pendingList: [-1, 5, 1e9], res: { wood: "lots" } } };
  const r = deserialize(sandAdapter, JSON.stringify(o));
  ok(r.ok && r.world.moles.length === 0 && r.world.tick === 0 && r.world.pendingList.join() === "5" && r.world.res.wood === 30, "optional layers/state missing or mistyped -> safe defaults");
  if (r.ok) { let threw = false; try { run(r.world, 30); } catch { threw = true; } ok(!threw, "sanitized world steps without error"); }
}
{ // 8. migrations hook
  const o = JSON.parse(serialize(sandAdapter, new SandWorld(5))); o.version = 0; o.state.world.res.lumber = 77; delete o.state.world.res.wood;
  ok(!deserialize(sandAdapter, JSON.stringify(o)).ok, "old version rejected when no migration is registered");
  MIGRATIONS[0] = (f: any) => { f.state.world.res.wood = f.state.world.res.lumber; f.version = 1; return f; };
  const r = deserialize(sandAdapter, JSON.stringify(o)); delete MIGRATIONS[0];
  ok(r.ok && r.world.res.wood === 77, "registered migration upgrades old save before restore");
}
{ // 9. core stays engine-agnostic: a toy engine of any size
  type Toy = { w: number; h: number; cells: Uint8Array };
  const toy: WorldAdapter<Toy> = { engine: "toy", requiredLayers: ["cells"],
    capture: t => ({ width: t.w, height: t.h, layers: { cells: t.cells }, state: null }),
    restore: d => ({ w: d.width, h: d.height, cells: d.layers.cells as Uint8Array }) };
  const t: Toy = { w: 1000, h: 700, cells: new Uint8Array(700000) }; t.cells.fill(3, 1000, 5000);
  const r = deserialize(toy, serialize(toy, t));
  ok(r.ok && r.world.w === 1000 && r.world.cells[4000] === 3 && r.world.cells[6000] === 0, "toy 1000x700 engine round trips");
  ok(!deserialize(sandAdapter, serialize(toy, t)).ok, "toy save rejected by sand adapter");
}
{ // 10. SaveStore slots + storage failures
  const kv = memoryKV(), st = new SaveStore(sandAdapter, kv), w = busy();
  ok(!st.has("manual") && !st.load("manual").ok, "empty store reports no save");
  ok(st.save("manual", w).ok && st.has("manual"), "manual save stored");
  const r = st.load("manual"); ok(r.ok && r.world.hash() === w.hash(), "manual load restores world");
  st.save("auto", w); st.clear("manual"); ok(!st.has("manual") && st.has("auto"), "clear(slot) only removes that slot");
  st.clear(); ok(!st.has("auto") && kv.map.size === 0, "clear() removes all saves");
  kv.setItem(st.key("auto"), "corrupt!!"); ok(!st.load("auto").ok, "corrupt stored data -> load fails gracefully");
  const full: KV = { getItem: () => { throw new Error("denied"); }, setItem: () => { throw new Error("QuotaExceededError"); }, removeItem: () => { throw new Error("denied"); } };
  const fs = new SaveStore(sandAdapter, full); let threw = false;
  try { const a = fs.save("auto", w), b = fs.load("auto"); ok(!a.ok && !b.ok && !fs.has("auto"), "throwing storage -> errors returned, not thrown"); fs.clear(); } catch { threw = true; }
  ok(!threw, "throwing storage never throws out of SaveStore");
}
{ // 11. autosaver cadence
  let n = 0; const a = new Autosaver(1000, () => n++);
  for (let i = 0; i < 60; i++) a.tick(16.7); ok(n === 1, `autosave fires once per interval (${n})`);
  a.reset(); a.tick(900); ok(n === 1, "reset restarts the interval");
}
console.log(fail ? `${fail} FAILED` : "ALL PASSED"); process.exit(fail ? 1 : 0);
