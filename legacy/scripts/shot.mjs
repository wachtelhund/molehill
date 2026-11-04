import { chromium } from "playwright-core";
import http from "http"; import fs from "fs"; import path from "path";
const srv = http.createServer((q, r) => { const f = path.join("dist", q.url.split("?")[0] === "/" ? "index.html" : q.url.split("?")[0]); fs.existsSync(f) ? r.end(fs.readFileSync(f)) : (r.statusCode = 404, r.end()); }).listen(5199);
const b = await chromium.launch({ executablePath: "/usr/bin/google-chrome", args: ["--no-sandbox"] });
const p = await b.newPage({ viewport: { width: 1400, height: 860 } });
const errs = []; p.on("pageerror", e => errs.push(String(e))); p.on("console", m => m.type() === "error" && errs.push(m.text()));
await p.goto((process.env.URL || "http://localhost:5199/") + "?scenario=demo"); await p.waitForTimeout(500);
await p.keyboard.press("Space"); // pause realtime, drive manually
await p.screenshot({ path: "shots/01-start.png" });
await p.evaluate(() => window.__advance(90)); // pan to bridge
await p.mouse.move(10, 10);
for (const [name, ticks, cx, cy, z] of [["02-colony-digging", 0, 48*32, 11*32, 1], ["03-cavein-fuse", 10, 66*32, 26*32, 1.1], ["04-cavein-after", 120, 66*32, 26*32, 1.1], ["05-lava-water-stone", 200, 21*32, 32*32, 1.1], ["06-overview", 400, 48*32, 32*32, 0.42]]) {
  await p.evaluate(([t, x, y, zz]) => { window.__cam(x, y, zz); window.__advance(t); }, [ticks, cx, cy, z]);
  await p.screenshot({ path: `shots/${name}.png` });
}
const st = await p.evaluate(() => { const w = window.__world(); return { tick: w.tick, moles: w.moles.map(m => m.hp), log: w.log.map(e => e.text), stone: [...w.tile].filter(t => t === 8).length, debris: [...w.tile].filter(t => t === 7).length, res: w.res }; });
console.log(JSON.stringify(st), "errors:", errs);
await b.close(); srv.close();
