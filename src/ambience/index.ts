// Procedural sound for the v2 renderer. main.ts only needs:
//   const ambience = new Ambience();                          // once
//   ambience.frame(world, { x0, y0, x1, y1 });                // per draw, visible cells
// Key N or the top-bar button mutes/unmutes (persisted per browser).
import type { SandWorld } from "../sand/world";
import { SoundEngine } from "./audio";
import { CellRegion, SandEventTracker, ambientLevels } from "./sandAdapter";

const store = {
  get(k: string) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k: string, v: string) { try { localStorage.setItem(k, v); } catch { /* private mode */ } },
};

export class Ambience {
  readonly sound = new SoundEngine(undefined, store.get("molehill.muted") === "1");
  readonly tracker = new SandEventTracker();
  private btn: HTMLButtonElement | null = null;

  constructor(mount: HTMLElement | null = document.getElementById("top")) {
    if (mount) {
      const b = this.btn = document.createElement("button");
      b.id = "mute"; b.title = "Sound (N)";
      b.style.cssText = "background:#2a2420;color:#eee;border:1px solid #4a3f37;border-radius:6px;padding:5px 9px;cursor:pointer;font:inherit";
      b.onclick = () => this.toggleMute();
      mount.appendChild(b); this.syncButton();
    }
    const unlock = () => this.sound.unlock();
    addEventListener("pointerdown", unlock, true);
    addEventListener("keydown", e => {
      unlock();
      if (!e.repeat && !e.ctrlKey && !e.metaKey && !e.altKey && e.key.toLowerCase() === "n") this.toggleMute();
    }, true);
  }

  toggleMute() {
    this.sound.unlock();
    const m = this.sound.toggleMute(); store.set("molehill.muted", m ? "1" : "0"); this.syncButton();
  }
  private syncButton() { if (this.btn) this.btn.textContent = this.sound.muted ? "🔇 Sound off" : "🔊 Sound on"; }

  /** Call once per rendered frame with the visible region in cells. */
  frame(world: SandWorld, view: CellRegion) {
    const events = this.tracker.update(world);
    if (!this.sound.ready || this.sound.muted) return;
    const listener = { x: (view.x0 + view.x1) / 2, y: (view.y0 + view.y1) / 2, range: Math.max(32, (view.x1 - view.x0) / 2) };
    for (const e of events) this.sound.play(e, listener);
    this.sound.update(ambientLevels(world, view));
  }
}
