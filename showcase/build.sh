#!/usr/bin/env bash
# Copy the showcase source and verified repo screenshots into ./dist.
# No npm, no bundler, no generated gameplay art.
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

# Refuse to ship anything from the playable web build.
if find "$dist" -type f \( -name '*.wasm' -o -name 'molehill.js' -o -name 'molehill_bg.wasm' \) | grep -q .; then
  echo "refusing to emit game wasm/js into showcase/dist" >&2
  exit 1
fi

echo "built $dist"
find "$dist" -type f | sort
