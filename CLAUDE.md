# Code is Magic

Before pushing, run `bash scripts/check.sh`. It runs the same fmt, clippy, cargo test,
glyph, inscription, recovery and npm checks as the Verify job in `.github/workflows/check.yml`, excluding the profiles job.

- Fastest first signal: `cargo fmt --all -- --check` (seconds).
- Add `--full` to also run `scripts/build-web.sh` and the wasm test (needs the `wasm32-unknown-unknown` target).
- A cold `cargo test --locked` takes several minutes; later runs are fast.
- Keep `check.sh` and `check.yml` in sync when either changes.
- Do not edit license or font text.
