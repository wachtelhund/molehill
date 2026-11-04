# Molehill prototype (M1–M6 slice)
npm i; bash scripts/build.sh  -> dist/
Tests: npx esbuild test/sim.test.ts --bundle --platform=node --outfile=/tmp/t.js && node /tmp/t.js
Screens: node scripts/shot.mjs (needs dist + Chrome)

## v2 — falling-sand engine (src/sand/)
- Tests: `npx esbuild test/sand.test.ts --bundle --platform=node --outfile=/tmp/s.js && node /tmp/s.js` (legacy: test/sim.test.ts)
- Screenshots: `bash scripts/build.sh && node scripts/shot2.mjs` → shots/v2-*.png
- Legacy grid renderer kept in src/main_v1.ts, sim in src/sim.ts (for merging the grid-branch work).
