#!/usr/bin/env bash
# Headless checks for the showcase site. Does not run the game or load wasm.
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$root/.." && pwd)"

"$root/build.sh" >/tmp/molehill-showcase-build.log

python3 - "$root" "$repo" <<'PY'
import hashlib, pathlib, re, sys

root = pathlib.Path(sys.argv[1])
repo = pathlib.Path(sys.argv[2])
dist = root / "dist"
errors = []

def fail(msg):
    errors.append(msg)

html = (dist / "index.html").read_text(encoding="utf-8")
css = (dist / "styles.css").read_text(encoding="utf-8")
page404 = (dist / "404.html").read_text(encoding="utf-8")
wrangler = (root / "wrangler.toml").read_text(encoding="utf-8")

# Isolation: the game lives only under /play, as a byte-identical copy of site/.
for pattern in ("*.wasm", "molehill.js"):
    hits = [h for h in dist.rglob(pattern) if (dist / "play") not in h.parents]
    if hits:
        fail(f"game binary outside play/: {hits}")
for name in ("index.html", "molehill.js", "molehill_bg.wasm"):
    out, src = dist / "play" / name, repo / "site" / name
    if not out.is_file():
        fail(f"web version missing play/{name}")
    elif not src.is_file() or out.read_bytes() != src.read_bytes():
        fail(f"play/{name} is not a byte-identical copy of site/{name}")
headers = (dist / "_headers").read_text(encoding="utf-8")
if "/play/*" not in headers or "wasm-unsafe-eval" not in headers:
    fail("_headers needs a /play/* rule that lets the wasm run")

# No Worker script / paid backend bindings in config.
for needle in ("\nmain =", "binding =", "[[r2_buckets]]", "[[kv_namespaces]]",
               "[[d1_databases]]", "analytics", "observability"):
    if needle.lower() in wrangler.lower() and "no analytics" not in wrangler.lower():
        # comments may mention these; only fail on assignment-like lines
        pass
if re.search(r"(?m)^main\s*=", wrangler):
    fail("wrangler.toml has a Worker main script")
if re.search(r"(?m)^\s*binding\s*=", wrangler):
    fail("wrangler.toml has an assets binding (needs a Worker script)")
if re.search(r"(?m)^\[\[(r2_buckets|kv_namespaces|d1_databases|services)\]\]", wrangler):
    fail("wrangler.toml declares paid/backend bindings")
if not re.search(r'(?m)^directory\s*=\s*"./dist"', wrangler):
    fail("wrangler.toml must serve showcase/dist only")
if re.search(r'(?m)^directory\s*=\s*".*(site|engine)', wrangler):
    fail("wrangler.toml must not point at site/ or engine/")
if not re.search(r'(?m)^name\s*=\s*"molehill"', wrangler):
    fail("wrangler.toml Worker name must be molehill")
if re.search(r'(?m)^account_id\s*=', wrangler):
    fail("wrangler.toml must not commit an account id (Wrangler takes it from the login)")
if "molehill.hampusnilsson.dev" not in wrangler or "custom_domain" not in wrangler:
    fail("wrangler.toml must attach molehill.hampusnilsson.dev as a custom domain")

# Required pages and config files.
for rel in ("index.html", "404.html", "styles.css", "favicon.svg",
            "robots.txt", "sitemap.xml", "site.webmanifest", "_headers"):
    if not (dist / rel).is_file():
        fail(f"missing {rel}")

# Manifest screenshots present, checksums match the repo originals.
manifest = (root / "shots.manifest").read_text(encoding="utf-8")
for line in manifest.splitlines():
    if not line.strip() or line.startswith("#"):
        continue
    dest, rel, sha = line.split("\t")
    out = dist / "img" / dest
    src = repo / rel
    if not out.is_file():
        fail(f"dist missing screenshot {dest}")
        continue
    if not src.is_file():
        fail(f"repo missing screenshot source {rel}")
        continue
    got = hashlib.sha256(out.read_bytes()).hexdigest()
    src_sha = hashlib.sha256(src.read_bytes()).hexdigest()
    if got != sha or src_sha != sha:
        fail(f"checksum drift for {dest}")
    if out.read_bytes() != src.read_bytes():
        fail(f"{dest} is not a byte-identical copy of {rel}")

# Every local asset referenced by HTML/CSS exists.
refs = set(re.findall(r'(?:src|href)=["\']([^"\']+)["\']', html + page404))
refs |= set(re.findall(r'url\(["\']?(/[^"\')\s]+)["\']?\)', css))
for href in sorted(refs):
    if href.startswith(("http://", "https://", "mailto:", "#", "data:")):
        continue
    if href in ("/", "/index.html"):
        if not (dist / "index.html").is_file():
            fail("broken local ref: /")
        continue
    path = href.split("?")[0]
    if path.startswith("/"):
        target = dist / path.lstrip("/")
    else:
        target = dist / path
    if path.endswith("/"):
        target = target / "index.html"
    if not target.is_file():
        fail(f"broken local ref: {href}")

# Outside links go only to the public repo and its releases; no store, no embed.
for href in re.findall(r'href="(https?://[^"]+)"', html + page404):
    if not (href.startswith("https://github.com/wachtelhund/molehill") or href.startswith("https://molehill.hampusnilsson.dev/")):
        fail(f"unexpected outside link: {href}")
if "https://github.com/wachtelhund/molehill/releases/latest" not in html:
    fail("download links must point at the latest release")
if 'href="/play/"' not in html:
    fail("missing the link to the web version")
if "<iframe" in html.lower():
    fail("the game must not be embedded in the landing page")
forbidden = (
    "site/molehill",
    ".wasm",
    "Add to cart",
    "Buy now",
    "Download now",
    "Play in browser",
    "Play now",
    "Steam",
    "Itch.io",
    "★★★★",
    "5/5",
)
blob = (html + page404).lower()
for word in forbidden:
    if word.lower() in blob:
        fail(f"forbidden copy or link: {word}")

# Honest development status.
for phrase in ("development build", "not signed", "more info", "run anyway", "xattr -d com.apple.quarantine"):
    if phrase not in html.lower():
        fail(f"missing honest status line: {phrase}")

# Concept art must be labeled as illustration, never as a screenshot.
if "concept illustration" not in html.lower():
    fail("hero must name the concept illustration")
if "not a screenshot of the game" not in html.lower():
    fail("concept art alt text must say it is not a screenshot")
if "not gameplay" not in html.lower():
    fail("concept art must be distinguished from gameplay")
concept_files = (
    "concept-holt.jpg",
    "concept-holt.webp",
    "concept-holt-960.jpg",
    "concept-holt-960.webp",
)
for name in concept_files:
    if not (dist / "img" / name).is_file():
        fail(f"missing concept illustration {name}")
# Gameplay <img> tags (not the concept picture) should say screenshot.
shot_alts = re.findall(r'<img[^>]+src="/img/(?!concept-)[^"]+"[^>]+alt="([^"]*)"', html)
if len(shot_alts) < 4:
    fail("expected several gameplay screenshot alts")
for alt in shot_alts:
    if "screenshot" not in alt.lower():
        fail(f"gameplay image missing screenshot label: {alt}")
    if "concept" in alt.lower():
        fail(f"gameplay image must not be called concept: {alt}")

# Lore / verified gameplay (must appear); unreleased promises must not.
for phrase in ("delver", "deepholt", "the lid", "survival", "creative", "holt"):
    if phrase not in html.lower():
        fail(f"missing verified term: {phrase}")
oversell = (
    "now available",
    "out now",
    "wishlist",
    "early access live",
    "world-enrichment",
)
for phrase in oversell:
    if phrase in html.lower():
        fail(f"overselling: {phrase}")

# SEO / a11y basics.
for needle in ('<html lang="en">', 'rel="canonical"', 'og:image',
               'name="description"', 'class="skip"', 'id="main"'):
    if needle not in html:
        fail(f"missing {needle}")
if "<form" in html.lower():
    fail("no forms allowed")
if re.search(r"<script[^>]+src=", html):
    fail("no external or file scripts")
if html.count("<img") < 4:
    fail("expected authentic screenshots in the page")
if not re.search(r"<img[^>]+alt=", html):
    fail("images need alt text")
for m in re.finditer(r"<img\b([^>]*)>", html):
    attrs = m.group(1)
    if "alt=" not in attrs:
        fail("img without alt")
    alt = re.search(r'alt="([^"]*)"', attrs)
    if alt and not alt.group(1).strip():
        fail("empty alt")

# Contrast of the published palette (WCAG AA for body text).
def lum(hex_color):
    h = hex_color.lstrip("#")
    rgb = [int(h[i:i+2], 16) / 255 for i in (0, 2, 4)]
    def f(c):
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = (f(c) for c in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b

def contrast(a, b):
    la, lb = lum(a), lum(b)
    lighter, darker = max(la, lb), min(la, lb)
    return (lighter + 0.05) / (darker + 0.05)

pairs = {
    "ink on bg": ("#f3eadc", "#120e0b", 4.5),
    "dim on bg": ("#d2c4ae", "#120e0b", 4.5),
    "mute on bg": ("#b7a58d", "#120e0b", 4.5),
    "lamp on bg": ("#e4b36a", "#120e0b", 3.0),  # large / kicker
    "foot-quiet on bg": ("#8f7f6a", "#120e0b", 4.5),
}
for name, (fg, bg, need) in pairs.items():
    ratio = contrast(fg, bg)
    if ratio < need:
        fail(f"contrast {name} {ratio:.2f} < {need}")

# No third-party network in CSS (fonts and images are local).
# The grain overlay uses an SVG data URI whose xmlns is http://www.w3.org/2000/svg.
if re.search(r"url\(\s*['\"]?https?://", css):
    fail("styles.css must not load remote URLs")

if errors:
    print("FAIL")
    for e in errors:
        print(" -", e)
    sys.exit(1)
print("ok")
print(f"dist={dist}")
PY
