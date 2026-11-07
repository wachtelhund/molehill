#!/usr/bin/env bash
# Copy the showcase source and verified repo screenshots into ./dist.
# No npm, no bundler, no generated gameplay art. The web version is copied from site/ into play/.
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$root/.." && pwd)"
src="$root/src"
dist="$root/dist"
manifest="$root/shots.manifest"

if [[ ! -d "$src" ]]; then
  echo "missing $src" >&2
  exit 1
fi

rm -rf "$dist"
mkdir -p "$dist/img" "$dist/fonts"

# Static source (html, css, fonts, headers, robots, etc.)
cp -a "$src/." "$dist/"

# Verified screenshots from the game repo — checksum-checked, never invented.
while IFS=$'\t' read -r dest rel sha; do
  [[ -z "${dest:-}" || "$dest" == \#* ]] && continue
  from="$repo/$rel"
  if [[ ! -f "$from" ]]; then
    echo "missing screenshot source: $rel" >&2
    exit 1
  fi
  got="$(sha256sum "$from" | awk '{print $1}')"
  if [[ "$got" != "$sha" ]]; then
    echo "screenshot checksum mismatch: $rel" >&2
    echo "  expected $sha" >&2
    echo "  got      $got" >&2
    echo "Update showcase/shots.manifest after reviewing the new file." >&2
    exit 1
  fi
  cp -a "$from" "$dist/img/$dest"
done < "$manifest"

# The landing page itself never carries the game: no wasm or game js outside /play.
if find "$dist" -path "$dist/play" -prune -o -type f \( -name '*.wasm' -o -name 'molehill.js' \) -print | grep -q .; then
  echo "refusing to emit game wasm/js outside showcase/dist/play" >&2
  exit 1
fi

# The web version, byte-identical to the committed build in site/, served at /play/.
mkdir -p "$dist/play"
for f in index.html molehill.js molehill_bg.wasm; do
  if [[ ! -f "$repo/site/$f" ]]; then
    echo "missing web build file: site/$f" >&2
    exit 1
  fi
  cp -a "$repo/site/$f" "$dist/play/$f"
done

echo "built $dist"
find "$dist" -type f | sort
