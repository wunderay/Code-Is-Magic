#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown
# CLI must match the version resolved by Cargo.lock.
version=$(awk '/name = "wasm-bindgen"/{getline; gsub(/"/, "", $3); print $3; exit}' Cargo.lock)
if ! command -v wasm-bindgen >/dev/null || [[ $(wasm-bindgen --version) != "wasm-bindgen $version" ]]; then
  cargo install wasm-bindgen-cli --version "$version" --locked
fi
cargo build --locked --release --target wasm32-unknown-unknown --lib --no-default-features
wasm-bindgen --target web --out-dir web/pkg --no-typescript target/wasm32-unknown-unknown/release/code_is_magic.wasm
