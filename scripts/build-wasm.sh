#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cargo build \
  --manifest-path "$repo_root/wasm/Cargo.toml" \
  --target wasm32-unknown-unknown \
  --release

wasm-bindgen \
  "$repo_root/wasm/target/wasm32-unknown-unknown/release/osarena_wasm.wasm" \
  --out-dir "$repo_root/web/app/wasm" \
  --target web
