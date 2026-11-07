# Molehill showcase site

Public landing page for **https://molehill.hampusnilsson.dev**.

Isolated from the playable game. It does not load the wasm build, does not offer downloads, and has no analytics, accounts, or forms.

## Build

No npm install. From this directory:

```bash
./build.sh
./test.sh
```

`build.sh` copies `src/` to `dist/` and copies **byte-identical gameplay screenshots** listed in `shots.manifest`. It refuses to emit `.wasm` or `molehill.js`.

**Site output:** `showcase/dist/`

## Local preview

```bash
python3 -m http.server -d dist 4173
```

Open http://localhost:4173/ — landing page only. No canvas, no wasm, no play control. Python’s server will not apply `404.html` or `_headers`; Cloudflare Workers Static Assets will.

## Deploy (owner / coordinator only)

This repository does **not** deploy the site. Static-asset requests on Workers are free and unlimited. There is **no Worker script**, so there is no Worker-invocation billing.

Create a **new** assets-only Worker named `molehill`. Do not update, rename, or overwrite any other Worker.

Account (already authorized on the owner’s Mac.home Wrangler OAuth): `9887f0a567827483359f7f79e2bd2734`.

1. `cd showcase && ./build.sh`
2. Confirm `dist/` has no `.wasm`.
3. From this directory, with that existing Wrangler login (no new login or token):

   ```bash
   cd showcase
   npx wrangler deploy
   ```

   `wrangler.toml` sets:

   * `name = "molehill"` — new Worker; do not change this
   * `account_id = "9887f0a567827483359f7f79e2bd2734"`
   * `assets.directory = "./dist"` — never `site/` or the game
   * `[[routes]]` `pattern = "molehill.hampusnilsson.dev"` `custom_domain = true`
   * no `main`, no bindings, no R2/KV/D1, no observability

4. The zone `hampusnilsson.dev` must already sit on that account. The first deploy of Worker `molehill` attaches the custom domain. If that hostname is already bound to a *different* Worker, unbind it there first — do not deploy this config onto the other Worker.
5. Open https://molehill.hampusnilsson.dev and confirm the landing page. There should be no game.

A `*.workers.dev` preview also appears (`workers_dev = true`) and is sent `X-Robots-Tag: noindex`.

### What not to deploy

- Do not upload `site/` (the playable wasm build).
- Do not add R2, KV, D1, Durable Objects, or a `main` Worker script.
- Do not enable analytics or forms.
- Do not run this config against any Worker except `molehill`.

## Pictures

**Concept illustration** (user-supplied artwork, not the game), optimized in `src/img/`:

* `concept-holt.jpg` / `.webp` (1600×900)
* `concept-holt-960.jpg` / `.webp` (960×540)

Captions and alt text call this a concept illustration, not a screenshot.

**Gameplay screenshots** (repo captures only), checksummed in `shots.manifest`:

| file in `dist/img` | source |
|---|---|
| `hud-digging.png` | `docs/wiki/images/hud-digging.png` |
| `blast-front.png` | `docs/wiki/images/blast-front.png` |
| `water-flood.png` | `engine/shots/gameplay-water-flood.png` |
| `torch-tunnel.png` | `engine/shots/gameplay-torch-tunnel.png` |
| `regions.jpg` | `engine/shots/showcase-regions.jpg`: `--dump-wide` of seed 4242, whole window, scaled to 1200 wide |
| `rope-gun.png` | `engine/shots/showcase-rope-gun.png`: `--scene=climb-rope --dump-wide`, cropped, 3× |
| `lift.png` | `engine/shots/showcase-lift.png`: `--scene=liftride --dump-wide`, cropped, 3× |
| `borer-wagons.png` | `engine/shots/showcase-borer-wagons.png`: `--scene=wagons --dump-wide`, cropped, 3× |

The scene stills come from `molehill --fresh --seed=4242 --scene=<name> --shot=x.bmp --shot-at=150 --dump-art=DIR --dump-wide` (`view.bmp`, every cell evenly lit), cropped round the scene and scaled up 3× nearest-neighbour.

Open Graph uses `hud-digging.png` (a real screenshot), not the concept art.

## Fonts

Self-hosted OFL faces in `src/fonts/` (Fraunces, Source Sans 3). Licenses: `OFL-Fraunces.txt`, `OFL-SourceSans3.txt`.

## Layout of this folder

```
showcase/
  wrangler.toml     assets-only Worker `molehill`
  build.sh
  test.sh
  shots.manifest    gameplay screenshots only
  src/              html, css, fonts, concept art, headers
  dist/             build output (gitignored)
```
