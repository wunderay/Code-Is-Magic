#!/usr/bin/env bash
# Mirrors the "verify" job in .github/workflows/check.yml (not the profiles job). Run before pushing.
# Add --full to also run the wasm build and test (needs the wasm32 target).
set -euo pipefail
cd "$(dirname "$0")/.."

full=0
case "${1:-}" in
  "") ;;
  --full) full=1 ;;
  *) echo "usage: $0 [--full]" >&2; exit 2 ;;
esac
if [[ $# -gt 1 ]]; then
  echo "usage: $0 [--full]" >&2
  exit 2
fi

cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
py=python3
if ! python3 -c 'import fontTools' 2>/dev/null; then
  python3 -m venv target/check-venv
  target/check-venv/bin/python -m pip install fonttools==4.61.1
  py=target/check-venv/bin/python
fi
"$py" scripts/build-glyphs.py --check
"$py" tests/inscriptions.py
"$py" tests/recovery_cli.py
npm test
if [[ $full == 1 ]]; then
  bash scripts/build-web.sh
  node --test tests/wasm.test.js
fi
