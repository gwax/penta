#!/usr/bin/env bash
# Builds the C ABI and Python bindings and runs their smoke tests: full games
# played through each surface against the built-in opponents, plus the error
# paths. The Python half is skipped when no python3 is available.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="$(mktemp -d "${TMPDIR:-/tmp}/penta-bindings.XXXXXX")"
trap 'rm -rf "$out_dir"' EXIT

echo "== C ABI =="
cargo build --release -p penta-ffi
cc bindings/penta-ffi/smoke.c target/release/libpenta_ffi.a \
    -I bindings/penta-ffi -o "$out_dir/smoke"
"$out_dir/smoke"

echo "== Python =="
if ! command -v python3 >/dev/null; then
    echo "python3 not found; skipping the Python bindings"
    exit 0
fi
(cd bindings/penta-py && cargo build --release)
case "$(uname)" in
    Darwin) built="bindings/penta-py/target/release/libpenta.dylib" ;;
    *) built="bindings/penta-py/target/release/libpenta.so" ;;
esac
cp "$built" "$out_dir/penta.so"
cp bindings/penta-py/smoke.py "$out_dir/"
(cd "$out_dir" && python3 smoke.py)
