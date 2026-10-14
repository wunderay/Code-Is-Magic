# Code Is Magic

A Rust CLI and GitHub Pages website that turn code into chaotic, connected magic seals **and preserve the complete selected repository snapshot inside the exported image**.

The artwork has no readable names, filenames, captions, or SVG text elements. Syntax symbols are vector outlines. Each qualified name has an injective, reversible radial contour built from its exact UTF-8 bytes, length, and checksum. The same qualified name always has the same normalized contour; identical names intentionally repeat that contour. Language motifs and type structures remain distinct.

## Exact image → source recovery

An original exported **PNG** carries the compressed snapshot in the low three bits of its RGB pixel channels. Recovery uses pixel values, not PNG metadata, filename labels, a hash lookup, or external storage. Recompressing without changing decoded RGB values preserves it. **JPEG conversion, resizing, cropping, screenshots, color conversion, and other pixel edits can destroy data.** A damaged capsule produces an error; the program never claims an incomplete reconstruction is exact.

A source-carrying **SVG** keeps the same capsule in a non-rendered metadata element. Its visible name contours are also independently reversible from the polygon geometry. A JSON diagram preserves the complete snapshot beside the visualization.

Captured data includes every tracked file's exact bytes and path, Git file mode (`100644` / `100755`), symlink target bytes (`120000`), blob object ID, the selected commit ID, and submodule commit pointers (`160000`). Unsupported programming languages, binary files, configuration, fonts, lockfiles, and line-ending/BOM differences are preserved even when the structural visualizer cannot parse them.

Recovery downloads a deterministic ZIP with:

- `repository/`: the original file tree, exact bytes, executable bits, and ZIP symlink attributes;
- `restoration.json`: the complete snapshot, including commit/blob IDs and submodule pointers.

This is the selected **tracked working tree**, not the repository's entire Git history. External submodule contents and expanded Git LFS objects are not stored in the parent Git tree; their tracked pointers are preserved exactly. Uncommitted files, filesystem timestamps, ACLs, and other untracked filesystem metadata are outside that snapshot. Exported images contain the code itself, including private repository content when supplied.

## Terminal

Install [Rust](https://rustup.rs), then:

```sh
git clone https://github.com/wunderay/Code-Is-Magic.git
cd Code-Is-Magic
cargo install --path . --locked
code-is-magic https://github.com/owner/repo --output magic.png
code-is-magic --decode magic.png --output recovered.zip
code-is-magic --decode-names magic.png
```

Windows Command Prompt setup:

```bat
git clone https://github.com/wunderay/Code-Is-Magic.git && cd Code-Is-Magic && cargo install --path . --locked
```

More examples:

```sh
code-is-magic ./my-project --output magic.png --json diagram.json
code-is-magic https://github.com/owner/repo.git --reference COMMIT_SHA --palette parchment --output magic.svg
code-is-magic --decode magic.svg --output recovered.zip --json snapshot.json
code-is-magic --decode-names magic.svg
```

`--palette` accepts `gold`, `astral`, or `parchment`. `--reference` accepts a branch, tag, or commit; all GitHub reads use the resolved commit's immutable blob IDs. Set `GITHUB_TOKEN` in your shell environment for private repositories or more API quota. Tokens are never put into the snapshot.

Local CLI mode preserves file bytes, executable flags, and symlink targets without following symlinks. It excludes directories named `.git`, `target`, `node_modules`, and `pkg`; GitHub mode preserves every entry in the selected tracked tree. Windows files do not expose Unix executable flags through ordinary local metadata; use GitHub input to preserve the Git modes precisely.

## Website

The static website accepts a GitHub repository URL or source folder and displays the circles on an HTML canvas. It has pan/zoom, an inspector outside the artwork, three palettes, PNG/SVG/JSON downloads, a geometry name decoder, and **Recover source from image** upload. That upload decodes the original exported PNG/SVG and downloads the reconstructed source ZIP entirely in your browser.

PNG export uses the same Rust `resvg` rasterizer, pixel codec, and PNG encoder as the native CLI. It does not use browser canvas serialization, so native and WebAssembly output bytes are tested for equality. Equal canonical snapshots, artwork, and options produce identical PNG, SVG capsule, and ZIP bytes. There are no timestamps or random seeds in the output format; the apparent chaos is deterministic.

Browser folder input preserves every selected regular file's bytes, but the browser File API does **not** provide Unix executable modes or symlinks. Use GitHub or the native CLI when those matter. No source is uploaded to a backend; the optional token is sent only to `api.github.com`, cleared from the input, and never saved.

Build and serve locally (Bash, Git Bash, or WSL):

```sh
bash scripts/build-web.sh
python3 -m http.server 8000 --directory web
```

Open `http://localhost:8000`. `file://` cannot load the WebAssembly worker. The build script installs the wasm-bindgen CLI matching `Cargo.lock`; no Node package installation is required.

### GitHub Pages

1. Open **Settings → Pages → Source → GitHub Actions**.
2. Push to `main` or run **Actions → Build and publish grimoire**.
3. The workflow verifies the code, builds WebAssembly, and publishes `web/`.

The expected project URL after successful publication is `https://wunderay.github.io/Code-Is-Magic/`. All assets use relative URLs.

## Visual grammar

Function seal **area is directly proportional to non-comment body source lines**: radius = 24 × √lines. Four times as much code produces twice the radius. Equal-length functions have equal-sized seals regardless of their names, with no size cap or name-based jitter. The adjustable default of 12 lines controls ornament detail, not circle size. Call arrows connect unambiguous caller → callee targets; dashed links express type membership. Structs use hexagons, enums diamonds, classes frames, interfaces opposing triangles, and named JavaScript object literals cubes. The reversible name contour distinguishes different names independently of those motifs.

| Language | Extensions | Motif |
| --- | --- | --- |
| Rust | `.rs` | Angular forge |
| Python | `.py` | Lunar spiral |
| JavaScript | `.js .jsx .mjs .cjs` | Solar wheel |
| TypeScript | `.ts .tsx` | Prismatic seal |
| Go | `.go` | Flowing triad |
| C / C++ | `.c .h / .cpp .hpp .cc` | Twelve / sixteen-sided clockwork |
| Java | `.java` | Cathedral frame |
| C# | `.cs` | Diamond lattice |

Branches map to Δ, loops ∞, returns Ω, bindings ᚠ, assignment ᚷ, and comparisons ⚖. Greek, Norse, alchemical, and dingbat symbols are vector geometry. The legacy `--wingdings` flag now enables portable vector dingbat ornament; no proprietary font or ordinary text is embedded in the artwork.

The **visualizer** is a structural lexer, not a compiler: dynamic dispatch, macros, complex generics, expression-bodied functions, import resolution, and uncommon syntax may be unresolved. This does **not** affect byte-exact source recovery: the full original tracked file bytes are archived independently of parsing.

## Format and limits

- Name contour v1: framed UTF-8 bytes encoded as 16 radial levels; geometry validates length and CRC32. No case folding or Unicode normalization is applied.
- Source capsule v1: canonical sorted snapshot plus the original vector artwork, serialized in fixed field order and compressed deterministically.
- PNG framing: `CIMPNG01`, compressed length, CRC32, then payload, in RGB low bits. The decoder bounds dimensions, payload size, decompression, paths, duplicates, and modes before exposing files.
- ZIP: stored entries, UTF-8 path flag, fixed 1980 date, fixed modes; no nondeterministic timestamps.
- Budget: 5,000 tracked entries, 2 MB per file, 50 MB file bytes, 128 MB uncompressed capsule, 64 million PNG pixels. Exceeding a budget fails instead of omitting files.
- Recovery uses original exported files. Arbitrary artwork or screenshots do not contain a reconstructible repository. Checksums detect corruption; the format is not encryption or a digital signature.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 tests/recovery_cli.py
node --test tests/repository.test.js
bash scripts/build-web.sh
node --test tests/wasm.test.js
```

Tests cover name round trips and distinctions, text-free geometry recovery, binary/CRLF/BOM source recovery, modes/symlinks/submodule pointers, pixel-preserving PNG recompression, corruption rejection, safe archive paths, native/WASM artifact parity, source acquisition, declarations, and call graphs.

`src/glyphs.json` contains vector outlines derived from [Noto fonts](https://github.com/notofonts/noto-fonts), under the SIL Open Font License in `web/fonts/OFL.txt`. The project code is MIT licensed. Source repositories retain their own licenses.
