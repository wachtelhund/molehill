// Disaster presentation: warning/countdown banner (DOM) and a site marker drawn on the world canvas.
import { DisasterDirector, DISASTER_INFO } from "./disasters";


let banner: HTMLDivElement | null = null;
export function updateDisasterBanner(d: DisasterDirector) {
  if (!banner) {
    banner = document.createElement("div");
    banner.style.cssText = "position:fixed;top:56px;left:50%;transform:translateX(-50%);padding:8px 16px;border-radius:8px;" +
      "font:bold 16px system-ui,sans-serif;color:#fff;background:rgba(0,0,0,.6);border:2px solid;pointer-events:none;display:none;text-align:center";
    document.body.appendChild(banner);
  }
  const s = d.state;
  if (s.phase === "calm" || !s.kind) { banner.style.display = "none"; return; }
  const info = DISASTER_INFO[s.kind], secs = Math.ceil(d.secondsLeft);
  banner.style.display = "block";
  banner.style.borderColor = info.color;
  banner.textContent = s.phase === "warning" ? `⚠ ${info.icon} ${info.name} in ${secs}s` : `${info.icon} ${info.name.toUpperCase()}! (${secs}s)`;
  banner.style.opacity = s.phase === "warning" && secs <= 3 ? String(0.6 + 0.4 * Math.abs(Math.sin(performance.now() / 150))) : "1";
}

/** Pulsing ring around the disaster site; call with the canvas already in world space.
 *  `size` is the marker's unit in canvas units (defaults to one cell; pass more on fine-grained grids). */
export function drawDisasterMarker(ctx: CanvasRenderingContext2D, d: DisasterDirector, cellSize: number, time: number, size = cellSize) {
  const s = d.state;
  if (s.phase === "calm" || !s.kind) return;
  const cx = (s.x + 0.5) * cellSize, cy = (s.y + 0.5) * cellSize, pulse = 0.5 + 0.5 * Math.sin(time * 6);
  ctx.save();
  ctx.strokeStyle = DISASTER_INFO[s.kind].color; ctx.globalAlpha = 0.5 + 0.5 * pulse; ctx.lineWidth = size / 8;
  ctx.setLineDash(s.phase === "warning" ? [size / 4, size / 6] : []);
  ctx.beginPath(); ctx.arc(cx, cy, size * (1.5 + pulse), 0, Math.PI * 2); ctx.stroke();
  ctx.font = `${size * 0.8}px sans-serif`; ctx.textAlign = "center"; ctx.globalAlpha = 1;
  ctx.fillText(DISASTER_INFO[s.kind].icon, cx, cy - size * 2.2);
  ctx.restore();
}
