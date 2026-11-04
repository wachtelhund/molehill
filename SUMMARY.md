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

## Status
WIP: the TypeScript port is stopped because Molehill is moving to Bevy (Rust). The notes below describe the engine-agnostic design for the Rust port.

## Design notes for the Bevy port
### Shape: core + thin adapter
- **Engine-neutral core.** Knows nothing about tiles, materials or the world struct. It is the sound synth plus event vocabulary below, and the lighting engine on `feat/ambience` (`src/ambience/light.ts`, `lightRender.ts`).
- **One adapter per engine.** The only code that reads the world. It never mutates the sim; a test checks that the sim hash is unchanged. In Bevy this becomes a read-only system that runs after the sim systems.

### Sound vocabulary (`events.ts`)
- `SoundEvent { kind, x, y, intensity? }`, with positions in grid cells. The kinds are: explosion, cavein, thud, dig, dug, beam, hurt, death, steam, ignite. In Bevy, send these as `Event`s.
- `AmbientLevels { water, lava, burning, depth }`, each 0..1, measured over the visible region. In Bevy this is a `Resource` updated each frame.
- `Listener { x, y, range }` is the camera centre. `spatialize()` maps an event to a pan in -1..1 and a gain of `max(0.08, 1/(1+d²))`, where `d` is distance divided by range.
- `RateLimiter` enforces a minimum gap per kind (`MIN_GAP`, for example cavein 1.2 s and thud 60 ms) so bursts don't clip.

### Deriving events by diffing snapshots (`sandAdapter.ts`)
The adapter emits nothing from the sim directly. It compares the previous frame with the current one, which keeps the sim untouched:
- **New fx:** boom becomes explosion, dust becomes thud, steam becomes steam. Each fx is tracked as "seen".
- **New log line "Cave-in near X,Y":** becomes cavein. In Rust, prefer a structured `CaveIn` event from the engine.
- **Per mole:** an hp drop becomes hurt, and alive→dead becomes death. When a mole's job ends:
  - a dig whose designation was cleared becomes dug;
  - a finished beam becomes beam;
  - a finished light job becomes ignite.
- **While a mole digs with no path left:** a dig scrape every 6 ticks.
- **When the world is replaced (new colony):** adopt the new world silently, emitting nothing for what was already there.
- **Rust simplification:** the engine can emit these events itself, which removes most of the diffing.

### Synthesis recipes (`audio.ts`, no audio files)
Master gain 0.55 feeds a compressor. The buffers are 2 s of white noise and 4 s of brown noise (a leaky integrator of white noise).

| Sound | Recipe |
|---|---|
| explosion | white noise through a lowpass sweeping 2400→60 Hz over 1.6 s; a sine 90→28 Hz over 0.9 s; brown noise through a lowpass sweeping 300→80 Hz over 2.5 s |
| cavein | brown noise through a lowpass sweeping 500→90 Hz over 2.8 s, plus 6 random bandpass clicks in the first 1.5 s |
| thud | lowpass noise sweeping 380→120 Hz over 0.18 s, plus a sine 120→55 Hz |
| dig | bandpass noise burst, 0.07 s, at 900–1800 Hz; dug is a longer lowpass burst |
| beam | two triangle "knocks", 220→180 Hz and 260→200 Hz |
| hurt | square wave 900→600 Hz, 0.09 s |
| death | triangle wave 880→180 Hz, 0.7 s |
| steam | highpass noise sweeping 2.5→5 kHz over 0.9 s |
| ignite | highpass noise plus a short sine chirp |

The ambient layers are steered with `setTargetAtTime`:
- **Rumble:** a brown-noise loop through a 110 Hz lowpass, with gain 0.12 + 0.2×depth, breathing via a 0.07 Hz LFO.
- **Sizzle:** a white-noise loop through a 4.5 kHz highpass, gain 0.05×burning.
- **Lava churn:** a brown-noise loop through a 260 Hz lowpass, gain 0.18×lava.
- **Drips:** a sine 1.4–3 kHz falling to 0.45× over 0.12 s, sometimes with an echo. The interval is (0.6–3.6 s) / (0.25 + 1.5×water + 0.5×depth).
- **Lava bubbles:** a sine 60–140 Hz rising 2.2×, rate scaled by the lava level.

All sound envelopes are exponential, from 0.0001 to the peak and back. In Bevy, `bevy_kira_audio`, `fundsp` or a custom `Decodable` source can reproduce these; lazy unlock is not needed natively.

### Lighting design (on `feat/ambience`, not ported to v2)
- **Interface.** The world is read only through `LightGrid { width, height, opacity(x, y) -> 0..1 }`. A light is `LightSource { points: [x, y]..., radius, intensity, color }`; one point makes a point light, many make an emissive field such as lava or sky.
- **Spread.** Per source, a Dijkstra search over 8 neighbours (diagonal step √2). Entering a cell costs `step × (1 + opacity × wallCost)`, with wallCost 4. Light does not continue out of opaque cells, so walls get only a one-cell lit rim, and it does not squeeze diagonally between two opaque cells.
- **Falloff and mixing.** Falloff is `1 − smoothstep(d / radius)`. Sources add together per RGB channel. Keep the distance buffer f64 or compare with a tolerance; an f32 buffer caused a real bug.
- **Skylight seeds.** Open cells that have a clear vertical line to the top of the grid.
- **Rendering.** One texel per cell, upscaled with linear filtering and composited with multiply, with an ambient floor of about 0.12. An additive pass adds haze (0.13 × lum) and bloom (0.3 × (lum − 1)) for strong sources. In Bevy this maps to a per-cell light texture plus a fullscreen material or shader, or a compute shader doing jump-flood or iterative propagation.
- **Region-limited compute.** Lighting is computed only for the visible region plus the largest light radius.
- **Resolution independence.** The adapter states radii in world units times `cellsPerUnit`.
