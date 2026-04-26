#!/usr/bin/env bash
# Rebuild the wasm-pkg/ artifacts. Pre-built versions are committed for
# convenience, so this is only needed if you change crates/fumola_wasm/
# or want a wasm-opt'd build.
#
# Requires:
#   - rustup target wasm32-unknown-unknown
#   - wasm-bindgen-cli (matching the wasm-bindgen runtime version, here 0.2.118)
#
# Install wasm-bindgen-cli prebuilt:
#   curl -sL "https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.118/wasm-bindgen-0.2.118-x86_64-unknown-linux-musl.tar.gz" \
#     | tar xz -C /tmp
#   export WBG=/tmp/wasm-bindgen-0.2.118-x86_64-unknown-linux-musl/wasm-bindgen
#
# Or install via cargo (slow but cross-platform):
#   cargo install wasm-bindgen-cli --version 0.2.118
#   export WBG=$(which wasm-bindgen)
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
WBG="${WBG:-wasm-bindgen}"

echo "[1/2] cargo build --release --target wasm32-unknown-unknown -p fumola_wasm"
cargo build --release --target wasm32-unknown-unknown \
  --manifest-path "$ROOT/Cargo.toml" -p fumola_wasm

WASM_IN="$ROOT/target/wasm32-unknown-unknown/release/fumola_wasm.wasm"
OUT="$HERE/wasm-pkg"
echo "[2/2] $WBG --target web --out-dir $OUT $WASM_IN"
mkdir -p "$OUT"
"$WBG" --target web --out-dir "$OUT" "$WASM_IN"

echo "done."
ls -la "$OUT"
