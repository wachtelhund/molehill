// Procedural WebAudio: every sound is synthesised from noise buffers and
// oscillators at runtime (no audio files). The AudioContext is created lazily
// by unlock() — browsers only allow audio after a user gesture.
import { AmbientLevels, Listener, RateLimiter, SoundEvent, spatialize } from "./events";

type Ctx = AudioContext;
export type ContextFactory = () => Ctx | null;

export const defaultContextFactory: ContextFactory = () => {
  const C = (globalThis as any).AudioContext || (globalThis as any).webkitAudioContext;
  return C ? new C() : null;
};

const MASTER = 0.55;

export class SoundEngine {
  ctx: Ctx | null = null;
  private master!: GainNode;
  private white!: AudioBuffer; private brown!: AudioBuffer;
  private rumble!: GainNode; private sizzle!: GainNode; private bubble!: GainNode;
  private limiter = new RateLimiter();
  private levels: AmbientLevels = { water: 0, lava: 0, burning: 0, depth: 0 };
  private nextDrip = 0; private nextBubble = 0;
  private _muted: boolean;

  constructor(private factory: ContextFactory = defaultContextFactory, muted = false, private rand: () => number = Math.random) { this._muted = muted; }

  get muted() { return this._muted; }
  get ready() { return !!this.ctx; }

  /** Create/resume the audio graph. Call from a user-gesture handler. */
  unlock() {
    if (!this.ctx) { const c = this.factory(); if (!c) return; this.ctx = c; this.build(); }
    if (!this._muted && this.ctx.state === "suspended") this.ctx.resume();
  }

  setMuted(m: boolean) {
    this._muted = m;
    if (!this.ctx) return;
    const t = this.ctx.currentTime;
    this.master.gain.cancelScheduledValues(t);
    this.master.gain.setTargetAtTime(m ? 0 : MASTER, t, 0.05);
    if (m) setTimeout(() => { if (this._muted && this.ctx) this.ctx.suspend(); }, 200);
    else if (this.ctx.state === "suspended") this.ctx.resume();
  }
  toggleMute() { this.setMuted(!this._muted); return this._muted; }

  private build() {
    const c = this.ctx!;
    this.master = c.createGain(); this.master.gain.value = this._muted ? 0 : MASTER;
    const comp = c.createDynamicsCompressor();
    this.master.connect(comp); comp.connect(c.destination);
    this.white = noiseBuffer(c, 2, "white", this.rand); this.brown = noiseBuffer(c, 4, "brown", this.rand);
    // underground drone: brown noise through a low lowpass, slowly breathing
    this.rumble = this.loop(this.brown, "lowpass", 110, 0.7);
    const lfo = c.createOscillator(), lfoGain = c.createGain();
    lfo.frequency.value = 0.07; lfoGain.gain.value = 0.04; lfo.connect(lfoGain); lfoGain.connect(this.rumble.gain); lfo.start();
    // fuse/fire sizzle and lava churn, faded in by levels
    this.sizzle = this.loop(this.white, "highpass", 4500, 0.5);
    this.bubble = this.loop(this.brown, "lowpass", 260, 2);
    if (this._muted) c.suspend();
  }

  private loop(buf: AudioBuffer, type: BiquadFilterType, freq: number, q: number) {
    const c = this.ctx!, src = c.createBufferSource(), f = c.createBiquadFilter(), g = c.createGain();
    src.buffer = buf; src.loop = true; f.type = type; f.frequency.value = freq; f.Q.value = q; g.gain.value = 0;
    src.connect(f); f.connect(g); g.connect(this.master); src.start();
    return g;
  }

  /** Per-frame: steer continuous layers and schedule random drips/bubbles. */
  update(levels: AmbientLevels) {
    this.levels = levels;
    if (!this.ctx || this._muted) return;
    const t = this.ctx.currentTime, L = levels;
    this.rumble.gain.setTargetAtTime(0.12 + 0.2 * L.depth, t, 0.5);
    this.sizzle.gain.setTargetAtTime(0.05 * Math.min(1, L.burning), t, 0.1);
    this.bubble.gain.setTargetAtTime(0.18 * Math.min(1, L.lava), t, 0.4);
    if (t >= this.nextDrip) {
      if (this.nextDrip) this.drip(0.3 + 0.7 * this.rand());
      this.nextDrip = t + (0.6 + this.rand() * 3) / (0.25 + 1.5 * L.water + 0.5 * L.depth);
    }
    if (L.lava > 0.02 && t >= this.nextBubble) {
      this.blob(0.4 + 0.6 * this.rand());
      this.nextBubble = t + (0.15 + this.rand() * 0.8) / (0.3 + L.lava);
    }
  }

  play(ev: SoundEvent, l: Listener) {
    if (!this.ctx || this._muted) return;
    const t = this.ctx.currentTime;
    if (!this.limiter.allow(ev.kind, t)) return;
    const { pan, gain } = spatialize(ev, l), k = gain * (ev.intensity ?? 1);
    const out = this.out(pan, k), r = this.rand;
    switch (ev.kind) {
      case "explosion":
        this.noise(out, t, 1.6, "lowpass", 2400, 60, 1.0, 0.005);
        this.tone(out, t, "sine", 90, 28, 0.9, 1.0);
        this.noise(out, t + 0.05, 2.5, "lowpass", 300, 80, 0.5, 0.2, this.brown);
        break;
      case "cavein":
        this.noise(out, t, 2.8, "lowpass", 500, 90, 0.7, 0.25, this.brown);
        for (let i = 0; i < 6; i++) this.noise(out, t + r() * 1.5, 0.12, "bandpass", 400 + r() * 600, 200, 0.35, 0.002);
        break;
      case "thud": this.noise(out, t, 0.18, "lowpass", 380, 120, 0.45, 0.003); this.tone(out, t, "sine", 120, 55, 0.15, 0.3); break;
      case "dig": this.noise(out, t, 0.07, "bandpass", 900 + r() * 900, 600, 0.22, 0.002); break;
      case "dug": this.noise(out, t, 0.25, "lowpass", 900, 200, 0.4, 0.003); this.noise(out, t + 0.04, 0.1, "bandpass", 1600, 800, 0.15, 0.002); break;
      case "beam": this.tone(out, t, "triangle", 220, 180, 0.12, 0.45); this.tone(out, t + 0.11, "triangle", 260, 200, 0.1, 0.35); this.noise(out, t, 0.05, "bandpass", 2000, 1500, 0.2, 0.001); break;
      case "hurt": this.tone(out, t, "square", 900, 600, 0.09, 0.08); break;
      case "death": this.tone(out, t, "triangle", 880, 180, 0.7, 0.25); break;
      case "steam": this.noise(out, t, 0.9, "highpass", 2500, 5000, 0.25, 0.05); break;
      case "ignite": this.noise(out, t, 0.35, "highpass", 1500, 6000, 0.3, 0.01); this.tone(out, t, "sine", 1400, 2200, 0.08, 0.06); break;
    }
  }

  // ---------- primitives ----------
  private out(pan: number, gain: number): AudioNode {
    const c = this.ctx!, g = c.createGain(); g.gain.value = gain; g.connect(this.master);
    if (!c.createStereoPanner) return g;
    const p = c.createStereoPanner(); p.pan.value = pan; p.connect(g); return p;
  }

  /** Filtered noise burst with an exponential decay and a filter sweep f0 -> f1. */
  private noise(dst: AudioNode, t: number, dur: number, type: BiquadFilterType, f0: number, f1: number, peak: number, attack: number, buf = this.white) {
    const c = this.ctx!, src = c.createBufferSource(), f = c.createBiquadFilter(), g = c.createGain();
    src.buffer = buf; src.loop = true; f.type = type;
    f.frequency.setValueAtTime(f0, t); f.frequency.exponentialRampToValueAtTime(Math.max(20, f1), t + dur);
    env(g.gain, t, attack, dur, peak);
    src.connect(f); f.connect(g); g.connect(dst);
    src.start(t, this.rand() * (buf.duration - dur > 0 ? buf.duration - dur : 0)); src.stop(t + dur + 0.05);
  }

  /** Oscillator with an exponential pitch glide f0 -> f1. */
  private tone(dst: AudioNode, t: number, type: OscillatorType, f0: number, f1: number, dur: number, peak: number) {
    const c = this.ctx!, o = c.createOscillator(), g = c.createGain();
    o.type = type; o.frequency.setValueAtTime(f0, t); o.frequency.exponentialRampToValueAtTime(Math.max(20, f1), t + dur);
    env(g.gain, t, 0.004, dur, peak);
    o.connect(g); g.connect(dst); o.start(t); o.stop(t + dur + 0.05);
  }

  private drip(k: number) {
    const t = this.ctx!.currentTime, out = this.out((this.rand() - 0.5) * 1.4, 0.12 * k), f = 1400 + this.rand() * 1600;
    this.tone(out, t, "sine", f, f * 0.45, 0.12, 0.8);
    if (this.rand() < 0.5) this.tone(out, t + 0.09, "sine", f * 1.3, f * 0.7, 0.06, 0.3); // tiny echo
  }

  private blob(k: number) {
    const t = this.ctx!.currentTime, out = this.out((this.rand() - 0.5) * 1.2, 0.2 * k * Math.min(1, this.levels.lava * 2)), f = 60 + this.rand() * 80;
    this.tone(out, t, "sine", f, f * 2.2, 0.14, 0.6);
  }
}

function env(p: AudioParam, t: number, attack: number, dur: number, peak: number) {
  p.setValueAtTime(0.0001, t);
  p.exponentialRampToValueAtTime(Math.max(0.0002, peak), t + attack);
  p.exponentialRampToValueAtTime(0.0001, t + dur);
}

function noiseBuffer(c: BaseAudioContext, seconds: number, color: "white" | "brown", rand: () => number) {
  const n = Math.floor(c.sampleRate * seconds), buf = c.createBuffer(1, n, c.sampleRate), d = buf.getChannelData(0);
  let last = 0;
  for (let i = 0; i < n; i++) {
    const w = rand() * 2 - 1;
    if (color === "white") d[i] = w;
    else { last = (last + 0.02 * w) / 1.02; d[i] = last * 3.5; }
  }
  return buf;
}
