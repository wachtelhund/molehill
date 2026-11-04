import { World, W, H, N, T, SIM_HZ, demoScenario, TILE_NAMES, isSolid } from "./sim";

const TS = 32;
const PAL: Record<number, string> = { [T.DIRT]: "#7A5A3A", [T.GRAVEL]: "#9A8F7E", [T.CLAY]: "#A86B4C", [T.ROCK]: "#5E6470", [T.GRANITE]: "#3F4652", [T.BEDROCK]: "#22262D", [T.DEBRIS]: "#6B5847", [T.STONE]: "#4A4552", [T.COPPER]: "#7A5A3A" };
const VOID = "#1B1714", SKY = "#2B3A4A";

const params = new URLSearchParams(location.search);
const seed = Number(params.get("seed") || 7);
let world = new World(seed);
if (params.get("scenario") === "demo") demoScenario(world);
(window as any).__world = () => world;

const cv = document.getElementById("c") as HTMLCanvasElement;
const ctx = cv.getContext("2d")!;
const cam = { x: W * TS / 2, y: 14 * TS, z: 1 };
let tool = "dig", paused = false, speed = 1, overlay = false;
const TOOLS = [["dig", "1 Dig"], ["beam", "2 Beam"], ["fuse", "3 Fuse"], ["tnt", "4 TNT"], ["ignite", "5 Ignite"], ["cancel", "6 Cancel"], ["water", "7 Water"], ["lava", "8 Lava"]];
const bar = document.getElementById("tools")!;
for (const [id, label] of TOOLS) { const b = document.createElement("button"); b.textContent = label; b.dataset.t = id; b.onclick = () => setTool(id); bar.appendChild(b); }
function setTool(t: string) { tool = t; bar.querySelectorAll("button").forEach(b => b.classList.toggle("on", (b as HTMLElement).dataset.t === t)); }
setTool("dig");

function resize() { cv.width = innerWidth * devicePixelRatio; cv.height = innerHeight * devicePixelRatio; }
addEventListener("resize", resize); resize();

// ---------- input ----------
const keys = new Set<string>();
let painting = false, panning = false, lastX = 0, lastY = 0, mouseX = 0, mouseY = 0;
function cellAt(sx: number, sy: number) {
  const wx = (sx * devicePixelRatio - cv.width / 2) / cam.z + cam.x, wy = (sy * devicePixelRatio - cv.height / 2) / cam.z + cam.y;
  const x = Math.floor(wx / TS), y = Math.floor(wy / TS); return x >= 0 && y >= 0 && x < W && y < H ? y * W + x : -1;
}
function paint(e: MouseEvent) { const c = cellAt(e.clientX, e.clientY); if (c >= 0) world.designate(tool, c); }
cv.addEventListener("contextmenu", e => e.preventDefault());
cv.addEventListener("mousedown", e => { if (e.button === 0) { painting = true; paint(e); } else { panning = true; } lastX = e.clientX; lastY = e.clientY; });
addEventListener("mouseup", () => { painting = panning = false; });
cv.addEventListener("mousemove", e => {
  mouseX = e.clientX; mouseY = e.clientY;
  if (painting) paint(e);
  if (panning) { cam.x -= (e.clientX - lastX) * devicePixelRatio / cam.z; cam.y -= (e.clientY - lastY) * devicePixelRatio / cam.z; }
  lastX = e.clientX; lastY = e.clientY;
});
cv.addEventListener("wheel", e => { e.preventDefault(); cam.z = Math.min(3, Math.max(0.35, cam.z * (e.deltaY < 0 ? 1.15 : 1 / 1.15))); }, { passive: false });
addEventListener("keydown", e => {
  keys.add(e.key.toLowerCase());
  const n = Number(e.key); if (n >= 1 && n <= 8) setTool(TOOLS[n - 1][0]);
  if (e.key === " ") { paused = !paused; e.preventDefault(); }
  if (e.key === "]") speed = Math.min(4, speed + 1);
  if (e.key === "[") speed = Math.max(1, speed - 1);
  if (e.key.toLowerCase() === "o") overlay = !overlay;
  if (e.key.toLowerCase() === "r") { world = new World(seed); }
  if (e.key.toLowerCase() === "x") { world = new World(seed); demoScenario(world); }
});
addEventListener("keyup", e => keys.delete(e.key.toLowerCase()));

// ---------- loop ----------
let acc = 0, last = performance.now(), time = 0;
function frame(now: number) {
  const dt = Math.min(250, now - last); last = now; time += dt / 1000;
  const pan = 600 * dt / 1000 / cam.z;
  if (keys.has("a") || keys.has("arrowleft")) cam.x -= pan; if (keys.has("d") || keys.has("arrowright")) cam.x += pan;
  if (keys.has("w") || keys.has("arrowup")) cam.y -= pan; if (keys.has("s") || keys.has("arrowdown")) cam.y += pan;
  if (!paused) { acc += dt * speed; let steps = 0; while (acc >= 1000 / SIM_HZ && steps++ < 8) { world.step(); acc -= 1000 / SIM_HZ; } }
  draw(); requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
(window as any).__cam = (x: number, y: number, z: number) => { cam.x = x; cam.y = y; cam.z = z; };
(window as any).__advance = (n: number) => { for (let i = 0; i < n; i++) world.step(); draw(); };

// ---------- drawing ----------
const hash = (i: number) => { let h = i * 2654435761; h ^= h >>> 13; return (h >>> 0) % 1000 / 1000; };
function shade(hex: string, k: number) { const v = parseInt(hex.slice(1), 16); const f = (c: number) => Math.max(0, Math.min(255, Math.round(c * k))); return `rgb(${f(v >> 16)},${f((v >> 8) & 255)},${f(v & 255)})`; }
const shadeCache = new Map<string, string>();
const sh = (hex: string, k: number) => { const key = hex + k; let s = shadeCache.get(key); if (!s) { s = shade(hex, k); shadeCache.set(key, s); } return s; };

function draw() {
  const w = world;
  ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.fillStyle = VOID; ctx.fillRect(0, 0, cv.width, cv.height);
  const sx = w.shake ? (Math.random() - 0.5) * w.shake : 0, sy = w.shake ? (Math.random() - 0.5) * w.shake : 0;
  ctx.setTransform(cam.z, 0, 0, cam.z, cv.width / 2 - cam.x * cam.z + sx, cv.height / 2 - cam.y * cam.z + sy);
  const x0 = Math.max(0, Math.floor((cam.x - cv.width / 2 / cam.z) / TS)), x1 = Math.min(W - 1, Math.ceil((cam.x + cv.width / 2 / cam.z) / TS));
  const y0 = Math.max(0, Math.floor((cam.y - cv.height / 2 / cam.z) / TS)), y1 = Math.min(H - 1, Math.ceil((cam.y + cv.height / 2 / cam.z) / TS));
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
    const i = y * W + x, t = w.tile[i], px = x * TS, py = y * TS;
    if (t === T.AIR || t === T.BEAM) { ctx.fillStyle = y < 6 ? SKY : VOID; ctx.fillRect(px, py, TS, TS); }
    else {
      const base = PAL[t]; ctx.fillStyle = sh(base, 0.92 + Math.round(hash(i) * 4) * 0.04); ctx.fillRect(px, py, TS + 0.5, TS + 0.5);
      if (t === T.GRAVEL || t === T.DEBRIS) { ctx.fillStyle = sh(base, 0.75); for (let k = 0; k < 4; k++) { ctx.beginPath(); ctx.arc(px + 6 + hash(i + k * 7) * 20, py + 6 + hash(i + k * 13) * 20, 2.5, 0, 7); ctx.fill(); } }
      if (t === T.COPPER) { ctx.fillStyle = "#E07B39"; for (let k = 0; k < 3; k++) { ctx.beginPath(); ctx.arc(px + 8 + hash(i + k) * 16, py + 8 + hash(i + k * 3) * 16, 3.5, 0, 7); ctx.fill(); } }
      if (t === T.ROCK || t === T.GRANITE) { ctx.strokeStyle = sh(base, 1.25); ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(px + 6, py + 10 + hash(i) * 8); ctx.lineTo(px + 18, py + 6 + hash(i + 1) * 8); ctx.stroke(); }
      if (t === T.STONE) { ctx.fillStyle = "#4B4556"; ctx.fillRect(px + 4, py + 4, 8, 4); }
      if (w.hp[i] > 0) { ctx.fillStyle = "rgba(0,0,0,0.35)"; ctx.fillRect(px, py, TS, TS * Math.min(1, w.hp[i] / 60)); }
    }
    if (t === T.BEAM) {
      ctx.fillStyle = w.fire[i] ? "#7a3d1a" : "#C89B5B"; ctx.fillRect(px + 12, py, 8, TS);
      ctx.fillStyle = "#9c7440"; ctx.fillRect(px + 4, py, 24, 4);
      if (w.fire[i]) { ctx.fillStyle = `rgba(255,${120 + Math.random() * 80 | 0},40,0.9)`; ctx.beginPath(); ctx.moveTo(px + 8, py + TS); ctx.lineTo(px + 16, py + 4 + Math.random() * 8); ctx.lineTo(px + 24, py + TS); ctx.fill(); }
    }
    // fluids: fill from bottom by mass; full if fluid above
    const wm = w.water[i], lm = w.lava[i];
    if (wm > 0.01) { const full = y > 0 && w.water[i - W] > 0.05; const h = full ? TS : Math.min(1, wm) * TS; ctx.fillStyle = `rgba(58,143,217,${0.45 + Math.min(1, wm) * 0.4})`; ctx.fillRect(px, py + TS - h, TS, h); }
    if (lm > 0.01) { const full = y > 0 && w.lava[i - W] > 0.05; const h = full ? TS : Math.min(1, lm) * TS; const f = 0.5 + 0.5 * Math.sin(time * 3 + x * 0.7 + y * 0.4);
      ctx.fillStyle = `rgb(255,${(106 + f * 90) | 0},${(31 + f * 46) | 0})`; ctx.fillRect(px, py + TS - h, TS, h); }
    if (w.pending[i] > 0) { ctx.strokeStyle = "#f4e6c8"; ctx.lineWidth = 2; ctx.beginPath(); ctx.moveTo(px + 4, py + 6); ctx.lineTo(px + 14, py + 16); ctx.lineTo(px + 10, py + 26); ctx.moveTo(px + 14, py + 16); ctx.lineTo(px + 26, py + 12); ctx.stroke(); }
    if (w.fuse[i]) { ctx.strokeStyle = "#d9c9a0"; ctx.setLineDash([4, 3]); ctx.lineWidth = 3; ctx.beginPath(); ctx.moveTo(px, py + TS - 6); ctx.lineTo(px + TS, py + TS - 6); ctx.stroke(); ctx.setLineDash([]);
      if (w.burn[i]) { ctx.fillStyle = "#FFD27A"; ctx.beginPath(); ctx.arc(px + TS / 2, py + TS - 6, 5 + Math.random() * 3, 0, 7); ctx.fill(); } }
    if (w.tnt[i]) { ctx.fillStyle = "#C8302F"; ctx.beginPath(); ctx.roundRect(px + 6, py + 8, 20, 22, 4); ctx.fill(); ctx.fillStyle = "#f2e6d0"; ctx.fillRect(px + 6, py + 16, 20, 5); ctx.strokeStyle = "#222"; ctx.lineWidth = 2; ctx.beginPath(); ctx.moveTo(px + 16, py + 8); ctx.quadraticCurveTo(px + 20, py + 2, px + 24, py + 4); ctx.stroke(); }
    if (w.items[i]) { ctx.fillStyle = "#b39a7a"; for (let k = 0; k < Math.min(3, w.items[i]); k++) { ctx.beginPath(); ctx.arc(px + 9 + k * 7, py + TS - 5, 4, 0, 7); ctx.fill(); } }
    if (w.dig[i]) { ctx.strokeStyle = "#ffd27a"; ctx.lineWidth = 2; ctx.setLineDash([5, 4]); ctx.strokeRect(px + 3, py + 3, TS - 6, TS - 6); ctx.setLineDash([]); }
    if (w.beamDes[i]) { ctx.strokeStyle = "rgba(200,155,91,0.8)"; ctx.setLineDash([4, 4]); ctx.lineWidth = 2; ctx.strokeRect(px + 12, py + 1, 8, TS - 2); ctx.setLineDash([]); }
    if (w.lightReq[i]) { ctx.fillStyle = "#ff8a3d"; ctx.font = "bold 14px sans-serif"; ctx.fillText("!", px + 13, py + 14); }
    if (overlay && isSolid(t)) { const s = w.support[i]; ctx.fillStyle = s === 255 ? "rgba(229,72,77,0.6)" : s === 0 ? "rgba(70,200,120,0.25)" : `rgba(255,${200 - s * 50},60,0.45)`; ctx.fillRect(px, py, TS, TS); }
  }
  ctx.fillStyle = "rgba(255,210,122,0.35)"; const st = w.stock; ctx.fillRect((st % W) * TS + 2, Math.floor(st / W) * TS + TS - 4, TS - 4, 3);
  // falling debris
  for (const d of w.debris) { ctx.fillStyle = PAL[d.mat] || PAL[T.DEBRIS]; ctx.save(); ctx.translate(d.x * TS + TS / 2, d.y * TS + TS / 2); ctx.rotate(Math.sin(d.fallT) * 0.1); ctx.fillRect(-TS / 2 + 2, -TS / 2 + 2, TS - 4, TS - 4); ctx.restore(); }
  // moles
  for (const m of w.moles) drawMole(m);
  // fx
  for (const f of w.fx) {
    const cx = f.x * TS + TS / 2, cy = f.y * TS + TS / 2, k = f.t / 40;
    if (f.type === "boom") { ctx.strokeStyle = `rgba(255,200,90,${1 - k})`; ctx.lineWidth = 6 * (1 - k); ctx.beginPath(); ctx.arc(cx, cy, 10 + k * TS * 4, 0, 7); ctx.stroke(); if (f.t < 6) { ctx.fillStyle = "rgba(255,240,200,0.8)"; ctx.beginPath(); ctx.arc(cx, cy, TS * 2.5, 0, 7); ctx.fill(); } }
    if (f.type === "dust") { ctx.fillStyle = `rgba(200,180,150,${0.6 * (1 - k)})`; for (let j = 0; j < 5; j++) { ctx.beginPath(); ctx.arc(cx + Math.cos(j * 1.3) * k * 24, cy + Math.sin(j * 1.3) * k * 12, 4 * (1 - k) + 1, 0, 7); ctx.fill(); } }
    if (f.type === "steam") { ctx.fillStyle = `rgba(230,235,240,${0.5 * (1 - k)})`; ctx.beginPath(); ctx.arc(cx, cy - k * 40, 6 + k * 10, 0, 7); ctx.fill(); }
    if (f.type === "spark") { ctx.fillStyle = `rgba(255,220,120,${1 - k * 2})`; ctx.fillRect(cx + (hash(f.t + f.x) - 0.5) * 20, cy + 8 - k * 30, 3, 3); }
    if (f.type === "crush") { ctx.strokeStyle = `rgba(229,72,77,${1 - k})`; ctx.lineWidth = 3; ctx.beginPath(); ctx.arc(cx, cy, 8 + k * 20, 0, 7); ctx.stroke(); }
  }
  // hover
  const hc = cellAt(mouseX, mouseY);
  if (hc >= 0) { ctx.strokeStyle = "rgba(255,255,255,0.5)"; ctx.lineWidth = 1.5; ctx.strokeRect((hc % W) * TS, Math.floor(hc / W) * TS, TS, TS); }
  hud(hc);
}

function drawMole(m: import("./sim").Mole) {
  const cx = m.px * TS + TS / 2, cy = m.py * TS + TS / 2, s = TS;
  const moving = Math.abs(m.x - m.px) + Math.abs(m.y - m.py) > 0.05;
  const working = !!m.job && m.path.length === 0;
  let bounce = 0, rot = 0, sx = 1, sy = 1;
  if (m.dead) { rot = Math.PI / 2; }
  else if (moving) bounce = -Math.abs(Math.sin(time * Math.PI * 2 * 3)) * 3;                     // y = |sin(t·2πf)|·A
  else if (working) rot = 0.25 * Math.sin(time * Math.PI * 2 * 6);                               // dig wobble
  else { sy = 1 + 0.04 * Math.sin(time * Math.PI * 2 * 0.5 + m.id); sx = 2 - sy; }                // idle breathe
  ctx.save(); ctx.translate(cx, cy + 4 + bounce); ctx.rotate(rot); ctx.scale(sx * (m.facing < 0 ? -1 : 1), sy);
  ctx.globalAlpha = m.dead ? 0.5 : 1;
  ctx.fillStyle = m.hurtT > 0 ? "#e5484d" : "#5A4636"; ctx.strokeStyle = "#2a1f17"; ctx.lineWidth = 1.5;
  ctx.beginPath(); ctx.roundRect(-s * 0.36, -s * 0.26, s * 0.72, s * 0.5, s * 0.25); ctx.fill(); ctx.stroke();
  ctx.fillStyle = "#F29BA5"; ctx.beginPath(); ctx.arc(s * 0.36, -s * 0.02, s * 0.1, 0, 7); ctx.fill();     // nose
  ctx.fillStyle = "#e8d8c4"; ctx.beginPath(); ctx.arc(s * 0.2, s * 0.2, s * 0.07, 0, 7); ctx.arc(-s * 0.1, s * 0.22, s * 0.07, 0, 7); ctx.fill(); // claws
  ctx.fillStyle = "#111"; if (m.dead) { ctx.font = "bold 9px sans-serif"; ctx.fillText("x", s * 0.12, -s * 0.06); } else { ctx.beginPath(); ctx.arc(s * 0.18, -s * 0.1, 1.8, 0, 7); ctx.fill(); }
  ctx.restore();
  if (!m.dead) {
    if (m.job) { ctx.fillStyle = "rgba(255,255,255,0.85)"; ctx.font = "11px sans-serif"; ctx.fillText({ dig: "⛏", beam: "🪵", haul: "📦", light: "🔥" }[m.job.type], cx - 6, cy - 14 + Math.sin(time * 4) * 1.5); }
    if (m.hp < 100) { ctx.fillStyle = "#333"; ctx.fillRect(cx - 12, cy - 18, 24, 3); ctx.fillStyle = m.hp > 40 ? "#46c878" : "#e5484d"; ctx.fillRect(cx - 12, cy - 18, 24 * m.hp / 100, 3); }
    if (m.o2 < 100) { ctx.fillStyle = "#3A8FD9"; ctx.fillRect(cx - 12, cy - 22, 24 * m.o2 / 100, 2); }
  }
}

const hudEl = document.getElementById("hud")!, logEl = document.getElementById("log")!;
let lastLog = -1;
function hud(hc: number) {
  const w = world, alive = w.moles.filter(m => !m.dead).length;
  let info = "";
  if (hc >= 0) info = ` · ${hc % W},${Math.floor(hc / W)} ${TILE_NAMES[w.tile[hc]]}${w.water[hc] > 0.01 ? ` water ${w.water[hc].toFixed(2)}` : ""}${w.lava[hc] > 0.01 ? ` lava ${w.lava[hc].toFixed(2)}` : ""} support ${w.support[hc] === 255 ? "NONE" : w.support[hc]}`;
  hudEl.textContent = `${paused ? "⏸ PAUSED" : "▶ " + speed + "x"} · tick ${w.tick} · moles ${alive}/${w.moles.length} · wood ${w.res.wood} · TNT ${w.res.tnt} · rubble ${w.res.rubble} · ore ${w.res.ore}${info}`;
  if (w.log.length && w.log[w.log.length - 1].tick !== lastLog) { lastLog = w.log[w.log.length - 1].tick; logEl.innerHTML = w.log.slice(-6).map(e => `<div>${e.text}</div>`).join(""); }
}
