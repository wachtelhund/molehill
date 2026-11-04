// Browser wiring for save/load: Save / Load / Reset buttons, hotkeys (default K/L), periodic + on-hide autosave.
// Engine-agnostic: talks to the game only through the host callbacks and a WorldAdapter.
import { SaveStore, Autosaver, WorldAdapter, KV, defaultKV } from "./saveload";

export interface SaveHost<World> {
  getWorld(): World;
  setWorld(w: World): void;
  /** A fresh world, used by Reset. */
  newWorld(): World;
}
export interface SaveUIOptions { autosaveMs?: number; kv?: KV; mount?: HTMLElement | null; keys?: { save: string; load: string } }

export function initSaveLoad<World>(adapter: WorldAdapter<World>, host: SaveHost<World>, opts: SaveUIOptions = {}) {
  const store = new SaveStore(adapter, opts.kv ?? defaultKV());
  const auto = new Autosaver(opts.autosaveMs ?? 30_000, () => { const r = store.save("auto", host.getWorld()); if (!r.ok) status(r.error, true); });

  const box = document.createElement("div");
  box.id = "saveload"; box.style.cssText = "display:flex;gap:4px;align-items:center";
  const msg = document.createElement("span"); msg.style.cssText = "opacity:.8;margin-left:4px";
  let msgTimer: any = 0;
  function status(text: string, err = false) {
    msg.textContent = text; msg.style.color = err ? "#e5484d" : "#ccc";
    clearTimeout(msgTimer); msgTimer = setTimeout(() => (msg.textContent = ""), 3000);
  }
  const btn = (label: string, title: string, fn: () => void) => {
    const b = document.createElement("button"); b.textContent = label; b.title = title; b.onclick = fn;
    b.style.cssText = "background:#2a2420;color:#eee;border:1px solid #4a3f37;border-radius:6px;padding:5px 9px;cursor:pointer;font:inherit";
    box.appendChild(b); return b;
  };

  function save() { const r = store.save("manual", host.getWorld()); status(r.ok ? `Saved (${Math.round(r.bytes / 1024)} KB)` : r.error, !r.ok); }
  function load() {
    const slot = store.has("manual") ? "manual" : "auto";
    const r = store.load(slot);
    if (!r.ok) { status(`Load failed: ${r.error}`, true); return; }
    host.setWorld(r.world); auto.reset(); status(`Loaded ${slot} save`);
  }
  function reset() {
    if (typeof confirm === "function" && !confirm("Delete saves and start a new colony?")) return;
    store.clear(); host.setWorld(host.newWorld()); auto.reset(); status("Saves cleared, new colony");
  }
  /** Resume from autosave if one exists. Corrupt data is discarded, never fatal. */
  function resume() {
    if (!store.has("auto")) return false;
    const r = store.load("auto");
    if (r.ok) { host.setWorld(r.world); status("Resumed autosave"); return true; }
    store.clear("auto"); status(`Autosave discarded: ${r.error}`, true); return false;
  }

  const keys = opts.keys ?? { save: "k", load: "l" };
  btn("💾 Save", `Save game (${keys.save.toUpperCase()})`, save);
  btn("📂 Load", `Load saved game (${keys.load.toUpperCase()})`, load);
  btn("♻ Reset", "Delete saves and start over", reset);
  box.appendChild(msg);
  (opts.mount ?? document.getElementById("top") ?? document.body).appendChild(box);

  addEventListener("keydown", e => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const k = e.key.toLowerCase();
    if (k === keys.save) save(); else if (k === keys.load) load();
  });
  const flush = () => store.save("auto", host.getWorld());
  addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") flush(); });
  addEventListener("pagehide", flush);

  return { store, save, load, reset, resume, /** feed real frame dt (ms) while unpaused */ tick: (dtMs: number) => auto.tick(dtMs) };
}
