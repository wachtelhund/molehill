# Molehill prototype (M1–M6 slice)
npm i; bash scripts/build.sh  -> dist/
Tests: npx esbuild test/sim.test.ts --bundle --platform=node --outfile=/tmp/t.js && node /tmp/t.js
Screens: node scripts/shot.mjs (needs dist + Chrome)
