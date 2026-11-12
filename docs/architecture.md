# How code-is-magic works

One Rust library turns a source tree into a drawing of magic circles and
stores the tree inside the drawing, so it can be recovered byte for byte.
The command-line program and the browser app (through WebAssembly) call the
same library. What the output looks like and how to use it:
[README](../README.md).

## Layout

| Path | Purpose |
| --- | --- |
| `src/main.rs` | The CLI: reads a GitHub repository (API) or a local folder, or an image to decode, expand or optimize |
| `src/lib.rs` | The library's public API and the `wasm_bindgen` functions the browser calls |
| `src/analyze.rs` | Structural lexer and parser: files to declarations (`Entity`) and calls (`Edge`) |
| `src/profiles.rs` | Per-language rules, read from `languages/profiles.json` at compile time |
| `src/render.rs`, `src/inscription.rs` | Lay out seals and channels and draw the SVG; glyph bands from `src/glyphs.json` |
| `src/sigil.rs` | Declaration names as reversible contours (`--decode-names`) |
| `src/archive.rs` | The source capsule (`Snapshot` of every tracked entry), PNG/SVG/SVGZ embedding, recovery ZIP |
| `src/crypto.rs` | The optional key: AES-256-SIV for the capsule |
| `src/perceptual.rs`, `src/magic.rs`, `src/magic.js` | The canonical output: sub-pixel rounding, generated geometry folded into macros, and the script that unfolds them in a browser |
| `src/compression.rs` | Lossless optimization of legacy PNG and SVG files |
| `languages/` | Language profiles written in their own languages, compiled by `scripts/build-profiles.py` |
| `web/` | The static site: `app.js` (UI), `worker.js` (calls the wasm), `repository.js` (GitHub reader), canvas effects |
| `scripts/` | Generators: glyph atlas, profiles, the site's bundled snapshot (`build-default.mjs`), `build-web.sh` |
| `tests/` | Rust integration tests (`*.rs`), Python CLI checks, Node tests for `web/` and the wasm build (`*.test.js`) |
| `examples/check_compression.rs` | Compares a PNG with its optimized exports |

## How data flows

1. **Input**: a `Snapshot` of paths, Git modes and bytes, from the GitHub
   API or a local folder (in the browser also a chosen folder or the
   bundled snapshot of this project). It is sorted by path and checked
   against the size limits.
2. **Analysis**: `analyze` reads the files of supported languages and finds
   declarations and the calls between them. It never runs the code.
3. **Rendering**: `render_with_key` places one seal per declaration and one
   braided channel per call; a key varies the drawing, not its structure.
4. **Encoding**: a `Capsule` (drawing plus snapshot) becomes the canonical
   SVGZ (`encode_magic_with_key`) or SVG: the artwork is rounded and
   folded into macros, the snapshot is brotli-compressed, authenticated and
   optionally encrypted, and SVGZ is gzipped with a fixed header.
5. **Recovery** runs back: `decode_image_with_key` reads the capsule from
   any supported image and `snapshot_zip` writes the files; `expand_svg`
   unfolds the macros into a static SVG.

The same input, options and key give the same bytes, natively and in wasm.

## Run and test

From the Avalon root: `./avalon run code-is-magic setup`, then
`./avalon verify code-is-magic`. `.avalon/ci` runs only the parts a change
affects (`.avalon/parts.sh`): `cargo fmt --all -- --check`,
`cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`;
`scripts/build-glyphs.py --check`; `python3 tests/inscriptions.py`;
`python3 tests/recovery_cli.py`; `npm test`; and `bash scripts/build-web.sh`
followed by `node --test tests/wasm.test.js`. The language profiles check
needs Go, Java and Mono and runs in its own workflow, not in the gate.
