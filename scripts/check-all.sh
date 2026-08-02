#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets

generated_dir="$(mktemp -d "${TMPDIR:-/tmp}/osarena-wasm-check.XXXXXX")"
trap 'rm -rf "$generated_dir"' EXIT
WASM_OUT_DIR="$generated_dir" ./scripts/build-wasm.sh
diff -ru web/app/wasm "$generated_dir"

cd "$repo_root/web"
CI=true pnpm lint
CI=true pnpm run build
CI=true node --test tests/*.test.mjs
