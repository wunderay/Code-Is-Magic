#!/usr/bin/env bash
# Mirrors the "verify" job in .github/workflows/check.yml. Run before pushing.
# Add --full to also run the wasm build and test (needs the wasm32 target).
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 -m pip install fonttools==4.61.1
python3 scripts/build-glyphs.py --check
python3 tests/inscriptions.py
python3 tests/recovery_cli.py
npm test
if [[ ${1:-} == --full ]]; then
  bash scripts/build-web.sh
  node --test tests/wasm.test.js
fi
