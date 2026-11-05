#!/usr/bin/env bash
# Build the wasm32 web bundle into ../dist.
set -euo pipefail
cd "$(dirname "$0")"
cargo build -p molehill --profile wasm-release --target wasm32-unknown-unknown
rm -rf ../dist && mkdir -p ../dist
wasm-bindgen --no-typescript --target web --out-dir ../dist --out-name molehill target/wasm32-unknown-unknown/wasm-release/molehill.wasm
if command -v wasm-opt >/dev/null; then wasm-opt -Os --enable-bulk-memory --enable-nontrapping-float-to-int -o ../dist/molehill_bg.wasm ../dist/molehill_bg.wasm || true; fi
cp web/index.html ../dist/
ls -la ../dist
