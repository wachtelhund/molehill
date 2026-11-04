import { chromium } from "playwright-core";
import http from "http"; import fs from "fs"; import path from "path";
const srv = http.createServer((q, r) => { const f = path.join("dist", q.url.split("?")[0] === "/" ? "index.html" : q.url.split("?")[0]); fs.existsSync(f) ? r.end(fs.readFileSync(f)) : (r.statusCode = 404, r.end()); }).listen(5199);
const b = await chromium.launch({ executablePath: "/usr/bin/google-chrome", args: ["--no-sandbox"] });
const p = await b.newPage({ viewport: { width: 1400, height: 860 } });
const errs = []; p.on("pageerror", e => errs.push(String(e))); p.on("console", m => m.type() === "error" && errs.push(m.text()));
await p.goto((process.env.URL || "http://localhost:5199/") + "?scenario=demo"); await p.waitForTimeout(400);
await p.evaluate(() => window.__pause(true)); await p.mouse.move(5, 5);
fs.mkdirSync("shots", { recursive: true });
const S = [ // name, advance-to tick, cam x,y (px = cell*4), zoom
  ["v2-01-overview", 0, 768, 420, 0.95], ["v2-02-logic-gates", 70, 896, 280, 3.2], ["v2-03-flood-trap-before", 85, 300, 330, 2.6],
  ["v2-04-flood-trap-open", 130, 300, 330, 2.6], ["v2-05-cavein-crumbling", 175, 650, 290, 3.2], ["v2-06-cavein-pile", 260, 650, 290, 3.2],
  ["v2-07-lora-through-rock", 240, 1250, 270, 2.2], ["v2-08-lava-tnt-stone", 420, 300, 380, 2.2], ["v2-09-overview-late", 600, 768, 420, 0.95]];
for (const [name, t, x, y, z] of S) {
  await p.evaluate(([t, x, y, z]) => { const w = window.__world(); window.__cam(x, y, z); window.__advance(Math.max(0, t - w.tick)); }, [t, x, y, z]);
  await p.screenshot({ path: `shots/${name}.png` });
}
const st = await p.evaluate(() => { const w = window.__world(); return { tick: w.tick, moles: w.moles.map(m => m.hp), log: w.log.map(e => e.text), res: w.res }; });
console.log(JSON.stringify(st, null, 1), "errors:", errs);
await b.close(); srv.close();
