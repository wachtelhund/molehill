// v2 renderer + input for the falling-sand engine (SandWorld). Legacy grid renderer lives in main_v1.ts.
import { SandWorld, CW, CH, B, BW, BH, NB, SIM_HZ } from "./sand/world";
import { M, COLOR, MAT_NAMES, KIND, K } from "./sand/materials";
import { D, DEV_NAMES, isGate, isLora } from "./sand/logic";
import { wiringScenario } from "./sand/scenarios";
import { createDisasters } from "./disasters-sand";
import { updateDisasterBanner, drawDisasterMarker } from "./disasters-ui";

const PX = 4; // screen px per cell at zoom 1
const params = new URLSearchParams(location.search);
const seed = Number(params.get("seed") || 7);
function makeWorld(s: number, scen: string | null) { const w = new SandWorld(s); if (scen === "demo" || scen === "wiring") wiringScenario(w); return w; }
let world = makeWorld(seed, params.get("scenario"));
(window as any).__world = () => world;
let disasters = createDisasters(world, seed, params), disastersFor = world;
function stepDisasters() { if (disastersFor !== world) { const en = disasters.enabled; disasters = createDisasters(world, seed); disasters.enabled = en; disastersFor = world; } disasters.step(); }
(window as any).__disaster = (k: any) => disasters.schedule(k);

const cv = document.getElementById("c") as HTMLCanvasElement, ctx = cv.getContext("2d")!;
const off = document.createElement("canvas"); off.width = CW; off.height = CH;
const octx = off.getContext("2d")!, img = octx.createImageData(CW, CH), buf = new Uint32Array(img.data.buffer);
const cam = { x: CW * PX / 2, y: 60 * PX, z: 1.6 };
let tool = "dig", paused = false, speed = 1, overlay = false, wiresVis = true;
const TOOLS: [string, string, string][] = [
  ["dig", "Dig", "1"], ["beam", "Beam", "2"], ["wire", "Wire", "3"], ["interact", "Use", "E"], ["cancel", "Cancel", "X"],
  ["lever", "Lever", "4"], ["plate", "Plate", "5"], ["sensor", "Fluid sensor", "6"], ["door", "Door", "7"], ["floodgate", "Floodgate", "8"], ["pump", "Pump", "9"], ["lamp", "Lamp", "0"],
  ["fuse", "Fuse", "F"], ["tnt", "TNT", "T"], ["ignite", "Ignite", "I"],
  ["and", "AND", "G"], ["or", "OR", ""], ["not", "NOT", ""], ["xor", "XOR", ""], ["timer", "Timer", ""], ["latch", "Latch", ""],
  ["tx", "LoRa TX", "L"], ["rx", "LoRa RX", ""], ["rep", "Repeater", ""],
  ["water", "Water", ""], ["lava", "Lava", ""], ["sand", "Sand", ""],
];
const bar = document.getElementById("tools")!;
for (const [id, label, key] of TOOLS) { const b = document.createElement("button"); b.textContent = (key ? key + " " : "") + label; b.dataset.t = id; b.onclick = () => setTool(id); bar.appendChild(b); }
function setTool(t: string) { tool = t; bar.querySelectorAll("button").forEach(b => b.classList.toggle("on", (b as HTMLElement).dataset.t === t)); }
setTool("dig");
function resize() { cv.width = innerWidth * devicePixelRatio; cv.height = innerHeight * devicePixelRatio; }
addEventListener("resize", resize); resize();

// ---------- input ----------
const keys = new Set<string>();
let painting = false, panning = false, lastX = 0, lastY = 0, mouseX = 0, mouseY = 0, lastPaint = -1;
const scale = () => cam.z * PX;
function toWorld(sx: number, sy: number) { return { x: (sx * devicePixelRatio - cv.width / 2) / scale() + cam.x / PX, y: (sy * devicePixelRatio - cv.height / 2) / scale() + cam.y / PX }; }
function blockAt(sx: number, sy: number) { const p = toWorld(sx, sy); const bx = Math.floor(p.x / B), by = Math.floor(p.y / B); return bx >= 0 && by >= 0 && bx < BW && by < BH ? by * BW + bx : -1; }
const ONE_SHOT = new Set(["interact", "ignite", "lever", "plate", "sensor", "door", "floodgate", "pump", "lamp", "tnt", "and", "or", "not", "xor", "timer", "latch", "tx", "rx", "rep"]);
function paint(e: MouseEvent, first: boolean) {
  const b = blockAt(e.clientX, e.clientY); if (b < 0 || b === lastPaint) return;
  if (!first && ONE_SHOT.has(tool)) return; lastPaint = b; world.designate(tool, b);
}
cv.addEventListener("contextmenu", e => e.preventDefault());
cv.addEventListener("mousedown", e => { if (e.button === 0) { painting = true; lastPaint = -1; paint(e, true); } else panning = true; lastX = e.clientX; lastY = e.clientY; });
addEventListener("mouseup", () => { painting = panning = false; });
cv.addEventListener("mousemove", e => {
  mouseX = e.clientX; mouseY = e.clientY;
  if (painting) paint(e, false);
  if (panning) { cam.x -= (e.clientX - lastX) * devicePixelRatio / cam.z; cam.y -= (e.clientY - lastY) * devicePixelRatio / cam.z; }
  lastX = e.clientX; lastY = e.clientY;
});
cv.addEventListener("wheel", e => { e.preventDefault(); cam.z = Math.max(0.5, Math.min(6, cam.z * (e.deltaY < 0 ? 1.12 : 1 / 1.12))); }, { passive: false });
const KEYMAP: Record<string, string> = {}; for (const [id, , k] of TOOLS) if (k) KEYMAP[k.toLowerCase()] = id;
addEventListener("keydown", e => {
  const k = e.key.toLowerCase(); keys.add(k);
  if (k === " ") { paused = !paused; e.preventDefault(); }
  else if (k === "[") speed = Math.max(1, speed / 2); else if (k === "]") speed = Math.min(8, speed * 2);
  else if (k === "o") overlay = !overlay; else if (k === "v") wiresVis = !wiresVis;
  else if (k === "r") world = makeWorld((Math.random() * 1e9) | 0, null);
  else if (k === "m") world = makeWorld(seed, "wiring");
  else if (KEYMAP[k] && !"wasd".includes(k)) setTool(KEYMAP[k]);
});
addEventListener("keyup", e => keys.delete(e.key.toLowerCase()));

// ---------- loop ----------
let acc = 0, lastT = performance.now();
function frame(t: number) {
  const dt = Math.min(0.1, (t - lastT) / 1000); lastT = t;
  const pan = 600 * dt / cam.z;
  if (keys.has("a") || keys.has("arrowleft")) cam.x -= pan; if (keys.has("d") || keys.has("arrowright")) cam.x += pan;
  if (keys.has("w") || keys.has("arrowup")) cam.y -= pan; if (keys.has("s") || keys.has("arrowdown")) cam.y += pan;
  if (!paused) { acc += dt * SIM_HZ * speed; let n = 0; while (acc >= 1 && n < 16) { world.step(); stepDisasters(); acc--; n++; } if (n >= 16) acc = 0; }
  draw(); requestAnimationFrame(frame);
}
(window as any).__cam = (x: number, y: number, z: number) => { cam.x = x; cam.y = y; cam.z = z; };
(window as any).__advance = (n: number) => { for (let i = 0; i < n; i++) { world.step(); stepDisasters(); } draw(); };
(window as any).__pause = (p: boolean) => { paused = p; };

// ---------- drawing ----------
const pack = (r: number, g: number, b: number) => 0xff000000 | (b << 16) | (g << 8) | r;
function renderCells(w: SandWorld) {
  const tick = w.tick;
  for (let y = 0; y < CH; y++) for (let x = 0; x < CW; x++) {
    const i = y * CW + x, m = w.mat[i];
    if (m === M.EMPTY) { const sky = y < 24 && w.mat[Math.min(i + CW * (24 - y), i)] === M.EMPTY; buf[i] = y < 22 ? pack(43 - y, 58 - y, 74 - y / 2) : pack(27, 23, 20); continue; void sky; }
    const c = COLOR[m]; let v = (w.shade[i] - 128) / 10;
    if (m === M.LAVA) v += 22 * Math.sin((x * 0.4 + y * 0.3 + tick * 0.15)) ;
    else if (m === M.WATER) v += 6 * Math.sin(x * 0.3 + tick * 0.2);
    else if (m === M.FIRE) v += (w.shade[i] & 31) - 10 + 15 * Math.sin(tick + x);
    let r = c[0] + v, g = c[1] + v, b = c[2] + v;
    if (w.pending[i] > 0 && ((tick >> 2) & 1)) { r += 40; g += 20; } // cracking
    if (w.loose[i] && KIND[m] === K.SOLID) { r -= 8; g -= 8; b -= 8; }
    if (overlay && w.support && KIND[m] === K.SOLID && w.support[i] < 255) { const s = Math.min(1, w.support[i] / 14); r = r * (1 - s) + 230 * s; g = g * (1 - s) + 60 * s; b *= 1 - s; }
    buf[i] = pack(Math.max(0, Math.min(255, r)) | 0, Math.max(0, Math.min(255, g)) | 0, Math.max(0, Math.min(255, b)) | 0);
  }
  octx.putImageData(img, 0, 0);
}
const DEV_COL: Record<number, string> = { [D.LEVER]: "#d9b34a", [D.PLATE]: "#a0a0a0", [D.FSENSOR]: "#4aa3d9", [D.DOOR]: "#8b5a2b", [D.FLOODGATE]: "#5b7b9b", [D.PUMP]: "#6a8fa8", [D.LAMP]: "#f5e27a", [D.TNT]: "#d33", [D.FUSE]: "#c96" };
const GATE_LABEL: Record<number, string> = { [D.AND]: "&", [D.OR]: "≥1", [D.NOT]: "¬", [D.XOR]: "⊕", [D.TIMER]: "⏱", [D.LATCH]: "SR", [D.TX]: "TX", [D.RX]: "RX", [D.REP]: "RP" };
function draw() {
  const w = world;
  renderCells(w);
  ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.fillStyle = "#1B1714"; ctx.fillRect(0, 0, cv.width, cv.height);
  const s = scale(), sh = w.shake > 0 ? (Math.random() - 0.5) * w.shake : 0;
  ctx.setTransform(s, 0, 0, s, cv.width / 2 - cam.x / PX * s + sh, cv.height / 2 - cam.y / PX * s);
  ctx.imageSmoothingEnabled = false; ctx.drawImage(off, 0, 0);
  const L = w.logic;
  // designations
  for (let b = 0; b < NB; b++) {
    const bx = (b % BW) * B, by = ((b / BW) | 0) * B;
    if (w.digDes[b]) { ctx.strokeStyle = "rgba(255,210,90,.8)"; ctx.lineWidth = 0.3; ctx.strokeRect(bx + .2, by + .2, B - .4, B - .4); }
    if (w.beamDes[b]) { ctx.strokeStyle = "rgba(200,150,80,.9)"; ctx.setLineDash([0.6, 0.6]); ctx.strokeRect(bx + .4, by + .4, B - .8, B - .8); ctx.setLineDash([]); }
    if (w.lightReq[b]) { ctx.fillStyle = "rgba(255,120,40,.7)"; ctx.beginPath(); ctx.arc(bx + 2, by + 2, 0.8, 0, 7); ctx.fill(); }
  }
  // wires
  if (wiresVis) {
    ctx.lineWidth = 0.6; ctx.lineCap = "round";
    for (let b = 0; b < NB; b++) {
      if (!L.wire[b] && !L.dev[b]) continue;
      const cx = (b % BW) * B + 2, cy = ((b / BW) | 0) * B + 2;
      if (L.wire[b]) {
        const on = L.netOn[L.netId[b]] === 1;
        ctx.strokeStyle = on ? "#ff9a3c" : "#9a5a2a";
        for (const n of [b + 1, b + BW]) if (n < NB && (L.wire[n] || L.dev[n]) && (n !== b + 1 || b % BW < BW - 1)) { ctx.beginPath(); ctx.moveTo(cx, cy); ctx.lineTo((n % BW) * B + 2, ((n / BW) | 0) * B + 2); ctx.stroke(); }
        ctx.fillStyle = on ? "#ffb35c" : "#7a4a22"; ctx.fillRect(cx - .5, cy - .5, 1, 1);
      }
      const d = L.dev[b]; if (!d) continue;
      const x0 = cx - 2, y0 = cy - 2, st = L.devS[b];
      if (d === D.DOOR || d === D.FLOODGATE) { ctx.strokeStyle = DEV_COL[d]; ctx.lineWidth = .5; ctx.strokeRect(x0 + .3, y0 + .3, 3.4, 3.4); ctx.lineWidth = .6; continue; }
      if (d === D.LAMP && st) { const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, 14); g.addColorStop(0, "rgba(255,230,120,.45)"); g.addColorStop(1, "rgba(255,230,120,0)"); ctx.fillStyle = g; ctx.fillRect(cx - 14, cy - 14, 28, 28); }
      ctx.fillStyle = isGate(d) || isLora(d) ? (st || L.powered[b] || L.txOn[b] ? "#3c8f5a" : "#2d3a33") : (DEV_COL[d] || "#888");
      if (d === D.LAMP) ctx.fillStyle = st ? "#fff3a0" : "#6b6440";
      if (d === D.LEVER) { ctx.fillStyle = "#555"; ctx.fillRect(x0 + .8, y0 + 2.6, 2.4, 1); ctx.strokeStyle = "#d9b34a"; ctx.lineWidth = .5; ctx.beginPath(); ctx.moveTo(cx, y0 + 2.8); ctx.lineTo(cx + (st ? 1.2 : -1.2), y0 + .8); ctx.stroke(); ctx.lineWidth = .6; continue; }
      if (d === D.PLATE) { ctx.fillStyle = st ? "#e0e0e0" : "#8a8a8a"; ctx.fillRect(x0 + .3, y0 + 3.2, 3.4, .6); continue; }
      if (d === D.FUSE) { ctx.fillStyle = L.burn[b] ? "#ff5" : "#a86"; ctx.fillRect(x0 + .5, cy - .3, 3, .6); continue; }
      ctx.fillRect(x0 + .6, y0 + .6, 2.8, 2.8);
      const lab = GATE_LABEL[d] || (d === D.TNT ? "T" : d === D.FSENSOR ? "~" : d === D.PUMP ? "P" : "");
      if (lab) { ctx.fillStyle = "#fff"; ctx.font = "1.4px sans-serif"; ctx.textAlign = "center"; ctx.textBaseline = "middle"; ctx.fillText(lab, cx, cy + .1); }
      if (isLora(d)) { ctx.fillStyle = "#9fd"; ctx.font = "1px sans-serif"; ctx.fillText(d === D.REP ? "*" : "ch" + L.devP[b], cx, y0 - .4); }
    }
    // LoRa links
    for (const l of L.links) {
      const ax = (l.a % BW) * B + 2, ay = ((l.a / BW) | 0) * B + 2, bx = (l.b % BW) * B + 2, by = ((l.b / BW) | 0) * B + 2;
      const live = L.txOn[l.a] || L.txOn[l.b] || L.rxOn[l.a] || L.rxOn[l.b];
      ctx.strokeStyle = live ? "rgba(120,255,200,.75)" : "rgba(120,200,255,.25)"; ctx.lineWidth = .35; ctx.setLineDash([1.2, 1]);
      ctx.lineDashOffset = -w.tick * 0.2; ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(bx, by); ctx.stroke(); ctx.setLineDash([]);
    }
  }
  // moles (≈ 5.5 x 4 cells — larger than a cell)
  for (const m of w.moles) {
    const x = m.px * B + 2, y = m.py * B + B; if (m.dead) { ctx.fillStyle = "#555"; ctx.fillRect(x - 2.5, y - 1.2, 5, 1.2); continue; }
    ctx.save(); ctx.translate(x, y); ctx.scale(m.facing, 1);
    ctx.fillStyle = m.hurtT > 0 ? "#e66" : "#6b4a3a"; ctx.beginPath(); ctx.ellipse(0, -1.9, 2.8, 1.9, 0, 0, 7); ctx.fill();
    ctx.fillStyle = "#e9a0a0"; ctx.beginPath(); ctx.arc(2.7, -1.9, .55, 0, 7); ctx.fill();
    ctx.fillStyle = "#f0c8a0"; ctx.fillRect(-1.6, -.4, 1, .5); ctx.fillRect(1, -.4, 1, .5);
    ctx.fillStyle = "#111"; ctx.fillRect(1.5, -2.7, .4, .4);
    ctx.fillStyle = "#e8c048"; ctx.beginPath(); ctx.arc(0.2, -3.6, 1.3, Math.PI, 0); ctx.fill(); // hard hat
    if (m.fleeing) { ctx.fillStyle = "#fff"; ctx.font = "2px sans-serif"; ctx.fillText("!", 0, -5); }
    ctx.restore();
    ctx.fillStyle = "#300"; ctx.fillRect(x - 2.5, y - 6.2, 5, .5); ctx.fillStyle = m.hp > 50 ? "#5c5" : "#e84"; ctx.fillRect(x - 2.5, y - 6.2, 5 * m.hp / 100, .5);
  }
  // fx
  w.fx = w.fx.filter(f => f.t < 30);
  for (const f of w.fx) { f.t++; const x = f.x / 4, y = f.y / 4;
    if (f.type === "boom") { ctx.fillStyle = `rgba(255,${180 - f.t * 5},60,${1 - f.t / 30})`; ctx.beginPath(); ctx.arc(x, y, 4 + f.t * 0.6, 0, 7); ctx.fill(); }
    else { ctx.fillStyle = f.type === "steam" ? `rgba(220,220,230,${.5 - f.t / 60})` : `rgba(150,120,90,${.5 - f.t / 60})`; ctx.beginPath(); ctx.arc(x, y - f.t * .1, 1 + f.t * .08, 0, 7); ctx.fill(); }
  }
  if (w.shake > 0) w.shake *= 0.85;
  drawDisasterMarker(ctx, disasters, 1, performance.now() / 1000, 2 * B);
  // hover
  const hb = blockAt(mouseX, mouseY);
  if (hb >= 0) { ctx.strokeStyle = "rgba(255,255,255,.6)"; ctx.lineWidth = .25; ctx.strokeRect((hb % BW) * B, ((hb / BW) | 0) * B, B, B); }
  hud(hb); updateDisasterBanner(disasters);
}
const hudEl = document.getElementById("hud")!, logEl = document.getElementById("log")!;
let lastLog = "";
function hud(hb: number) {
  const w = world, alive = w.moles.filter(m => !m.dead).length;
  let info = "";
  if (hb >= 0) {
    const x = (hb % BW) * B + 2, y = ((hb / BW) | 0) * B + 2, m = w.mat[y * CW + x], L = w.logic, d = L.dev[hb];
    info = ` · [${hb % BW},${(hb / BW) | 0}] ${MAT_NAMES[m]}${L.wire[hb] ? " · wire " + (L.netOn[L.netId[hb]] ? "ON" : "off") : ""}${d ? " · " + DEV_NAMES[d] + (isLora(d) ? " ch" + L.devP[hb] : d === D.TIMER ? " period " + L.devP[hb] : "") + (L.devS[hb] || L.powered[hb] ? " (active)" : "") : ""}`;
  }
  hudEl.textContent = `${paused ? "⏸ PAUSED" : "▶ " + speed + "x"} · tick ${w.tick} · moles ${alive}/${w.moles.length} · wood ${w.res.wood} · TNT ${w.res.tnt} · rubble ${w.res.rubble} · ore ${w.res.ore} · chunks ${w.activeChunks()}${info}`;
  const lg = w.log.slice(-7).map(e => e.text).join("\n"); if (lg !== lastLog) { logEl.innerText = lg; lastLog = lg; }
}
requestAnimationFrame(frame);
