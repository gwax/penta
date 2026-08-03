#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${WASM_OUT_DIR:-$repo_root/web/app/wasm}"

cd "$repo_root"

cargo build \
  --package penta-wasm \
  --target wasm32-unknown-unknown \
  --release \
  --locked

wasm-bindgen \
  "$repo_root/target/wasm32-unknown-unknown/release/penta_wasm.wasm" \
  --out-dir "$output_dir" \
  --target web
