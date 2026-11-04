# v2/ambience: summary

Procedural WebAudio sound effects and ambience on the v2 falling-sand engine (`SandWorld`), with a mute toggle. This is a port of the sound half of `feat/ambience`. Lighting was **not** ported, because it is being rebuilt inside the engine.

## What was added
- **Effects.** Explosion, cave-in, falling-debris thud, claw scraping while digging, block dug out, beam knock, mole hurt and death, steam hiss, fuse ignite.
- **Ambient layers.** A low drone that deepens with depth, a sizzle for lit fuses and fire, lava churn and bubbles, and water drips whose rate scales with water in view.
- **How it sounds.** Every sound is synthesised at runtime from noise buffers and oscillators; there are no audio files. Sounds are panned and attenuated relative to the camera, and repeats of the same kind are rate-limited so bursts don't clip.
- **Controls.** A 🔊/🔇 button in the top bar, plus the `N` key. `M` and `L` are already taken on v2. The setting persists in localStorage. The AudioContext is created only on the first user gesture.

## Files
- Copied unchanged from `feat/ambience` (engine-agnostic): `src/ambience/audio.ts`, `src/ambience/events.ts`.
- New:
  - `src/ambience/sandAdapter.ts`: the only file that knows `SandWorld`. It provides:
    - `SandEventTracker`, which diffs `fx`, `log`, moles and their jobs into `SoundEvent`s. It is read-only and never touches the world.
    - `ambientLevels()`, which reads materials only through `KIND[]` and the named `M.*` members, never numeric ids.
  - `src/ambience/index.ts`: the `Ambience` controller (mute button, `N` key, per-frame wiring).
  - `test/ambience.test.ts`: 24 checks covering the rate limiter, panning, the synth on a mock WebAudio, and SandWorld events (TNT, cave-in, dig, beam, ignite, hurt/death). It also checks that ambient levels respond to the world and that tracking doesn't change the sim hash.
- Not touched: `src/sand/world.ts`, `src/sand/logic.ts`, `src/sand/materials.ts`.

## Hook lines
`src/main.ts` (+3 lines):
```ts
import { Ambience } from "./ambience";
const ambience = new Ambience();
// in draw(), just before the hover outline:
{ const hw = cv.width / 2 / s, hh = cv.height / 2 / s; ambience.frame(w, { x0: cam.x / PX - hw, y0: cam.y / PX - hh, x1: cam.x / PX + hw, y1: cam.y / PX + hh }); } // sound
```
`index.html`: help line gains ` · N sound on/off`.

## Tests
```
npx esbuild test/ambience.test.ts --bundle --platform=node --outfile=/tmp/a.js && node /tmp/a.js
npx esbuild test/sand.test.ts --bundle --platform=node --outfile=/tmp/s.js && node /tmp/s.js
```
Both pass, and `bash scripts/build.sh` succeeds. In headless Chrome, the demo scenario ran with sound unlocked and no console errors, and `N` suspends audio and persists the setting.

## Notes for merging
- Cave-in positions are parsed from the engine's `"Cave-in near X,Y"` log line (block coordinates). If that text changes, the cave-in sound stops playing (nothing else breaks). A structured cave-in event on the engine would be more robust.
