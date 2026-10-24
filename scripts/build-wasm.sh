#!/usr/bin/env bash
# Builds every contract to wasm with cargo-near (Linux, macOS or WSL).
# Output: $CARGO_TARGET_DIR/near/<name>/<name>.wasm (default target/near/...).
set -euo pipefail
cd "$(dirname "$0")/.."

for c in contracts/checker contracts/examples/naive-vault contracts/examples/backed-vault \
         contracts/helpers/test-token contracts/helpers/evil-receiver contracts/helpers/lying-venue; do
  echo "== building $c"
  cargo near build non-reproducible-wasm --manifest-path "$c/Cargo.toml"
done
