# Code Is Magic

Code Is Magic turns a source repository into a deterministic field of connected magic circles and can reconstruct the captured repository from the resulting artifact.

New transmutations have **one canonical output format: perceptually compressed SVGZ**. The artwork is allowed only sub-pixel geometry changes selected to be visually imperceptible, while the embedded repository snapshot remains byte-exact. There is no PNG/SVG/JSON choice when transmuting code to magic.

The project itself is not offered under a project-wide license. Bundled font subsets retain their own upstream license notices in `web/fonts/`.

## Canonical transmutation

```sh
cargo run --release -- https://github.com/wunderay/Code-Is-Magic --output code-is-magic.svgz
```

For a local source tree:

```sh
cargo run --release -- ./my-project --output my-project.svgz
```

`magic.svgz` is the default output name, so `--output` is optional.

The canonical pipeline is deliberately vector-first. Code Is Magic drawings are synthetic geometry with sharp glyphs, thin rings, braided invocation channels, and large flat regions; a JPEG-like raster was substantially larger than the already-compressed vector representation at visually conservative settings. The canonical encoder therefore:

1. analyzes and renders the repository deterministically;
2. shares repeated glyph outlines without changing their shapes;
3. rounds ordinary path coordinates to 0.01 SVG units and ordinary scalar placement geometry to 0.001 SVG units;
4. keeps transform parameters at 0.000001 precision so tiny glyph scale factors do not suffer proportionally large changes;
5. leaves the independent reversible name-sigil geometry byte-for-byte unchanged;
6. embeds the complete repository snapshot in the existing authenticated source capsule;
7. compresses the complete result as deterministic gzip/SVGZ with a zero timestamp.

For the project reference drawing at 4258×3758, the selected geometry strategy measured about **51 dB PSNR** with roughly **0.08 mean absolute channel error** against the unquantized vector render. The final transform precision is stricter than that benchmarked candidate. More aggressive precision reduction was rejected. The quality rule is intentionally conservative because the smallest runes and braided strokes are more sensitive than ordinary photographic detail.

The output remains deterministic: equal canonical source, options, and key produce equal bytes.

## Exact source recovery

Recover the repository from a canonical magic artifact:

```sh
cargo run --release -- code-is-magic.svgz --output recovered.zip
```

or equivalently:

```sh
cargo run --release -- --decode code-is-magic.svgz --output recovered.zip
```

The recovered snapshot preserves every captured tracked entry's exact bytes and path, Git file mode (`100644` / `100755`), symlink target bytes (`120000`), blob object ID when available, the selected commit ID, and submodule commit pointers (`160000`). Unsupported languages, binary files, configuration, lockfiles, line endings, and BOMs are preserved even when they are not structurally visualized.

The visual layer is perceptually compressed; **the source capsule is not lossy**.

## Optional magic key

Generate a 256-bit key outside the source tree:

```sh
cargo run --release -- --generate-key ../my-magic.key
cargo run --release -- ./my-project --key-file ../my-magic.key --output my-project.svgz
```

Or set `CODE_IS_MAGIC_KEY` to the same 64-hex-digit value.

A key deterministically changes circle geometry, glyph choices, inscriptions, and connection ornament while preserving structural meaning. The source capsule is protected with RustCrypto AES-256-SIV, so keyed recovery requires the original key and rejects the wrong key or altered authenticated data. Keep the key separately from the repository being captured.

```sh
cargo run --release -- my-project.svgz --key-file ../my-magic.key --output recovered.zip
```

## Legacy artifacts

Older Code Is Magic PNG, SVG, and SVGZ exports remain readable so existing artifacts are not stranded. They are recovery/migration formats only; the CLI and website no longer offer them as alternate outputs for a new code-to-magic transmutation.

Existing artifacts can still be losslessly optimized without rerendering:

```sh
cargo run --release -- --optimize old.png --output smaller.png
cargo run --release -- --optimize old.svg --output smaller.svgz
```

A keyed legacy SVG/SVGZ needs its original key when its authenticated source capsule is rebuilt.

## Website

The static website is built entirely for GitHub Pages. It can:

- transmute a public or authorized GitHub repository into the same canonical perceptual SVGZ used by the native CLI;
- transmute a selected local folder in the browser;
- apply an optional magic key without storing it;
- open Code is Magic itself by default, with language colors and a bundled, immutable source snapshot that needs no GitHub API requests;
- accept pasted or dropped repository links, direct magic-image links (where the host permits browser access), source files/folders, and recoverable magic images through the same input;
- display a celestial grimoire with light/dark themes, a prominent **Download magic circles** button for the bundled default and all generated diagrams, and a canvas fitted to the actual seals and call paths;
- illuminate whole caller seals and their braided invocation paths as each function calls the next, then awaken the destination and trace returns back; a live caption identifies the active call;
- recover exact source ZIPs from canonical SVGZ and supported legacy artifacts;
- decode reversible geometric declaration names.

The live call-flow rehearsal follows the resolved static graph, including argument calls before their enclosing call. It does not run source code or predict branch choices, loop counts, asynchronous scheduling, or runtime values. Recursive channels rehearse once per tour, and independent entry points may play together. Glow and motion can be disabled; reduced-motion preferences are respected. The animation is display-only and never changes canonical exports.

`scripts/build-web.sh` also generates `web/default-project.json` from the build commit’s Git tree, preserving all tracked entries and file modes. `npm run serve` refreshes that snapshot for local development after building the WebAssembly package.

Native Rust and WebAssembly use the same encoder, and the test suite requires their canonical output bytes to match.

The browser File API cannot expose Unix executable modes or symlinks for ordinary folder selection. Use GitHub input or the native CLI when those filesystem details matter.

## What becomes a magic circle

The structural visualizer currently has authored profiles for:

| Language | Typical source | Visual family |
| --- | --- | --- |
| Rust | `.rs` | Forge / runic |
| Python | `.py` | Lunar |
| JavaScript | `.js`, `.mjs`, `.cjs` | Solar |
| TypeScript | `.ts`, `.tsx` | Prism |
| Go | `.go` | Triad |
| C | `.c`, `.h` | Clockwork |
| C++ | `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh`, `.hxx` | Linked facets |
| Java | `.java` | Cathedral |
| C# | `.cs` | Lattice |

Functions, methods, and related declarations become seals. Calls become braided invocation channels without arrowheads. Circle area follows source-code length independently of declaration-name length. `--language-colors` optionally colors seals using their language palette while adapting tones for the selected background.

The analyzer is structural rather than a compiler. Dynamic dispatch, macros, complex generics, expression-bodied functions, import resolution, and uncommon syntax may be unresolved; this affects visualization only, never byte-exact source recovery.

## Inscriptions

The built-in symbol atlas contains 478 measured glyphs in ten families: Runic, Ogham, Greek including archaic forms, alchemical, celestial, geometric, dingbat, mathematical, ancient marks, and original bind sigils.

Visible glyph bounds determine sizing and angular cells. Larger seals may use multiple concentric inscription rows or paired glyphs. Variation is deterministic, including key-based alphabet permutations. The artwork contains vector geometry rather than readable source names or SVG text.

Declaration names also have an independent reversible contour representation. The perceptual encoder deliberately excludes every `data-name-sigil='1'` element from geometry quantization so this recovery path remains exact.

## Limits

A captured snapshot is bounded to protect native and browser decoders:

- at most 5,000 entries;
- at most 2 MB per individual file;
- at most 50 MB of captured file content;
- bounded image/vector decode budgets.

Git history, expanded Git LFS objects, and the contents of external submodules are not embedded. A submodule entry records its commit pointer.

## Development and verification

Run the same core checks used by CI:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 -m pip install fonttools==4.61.1
python3 scripts/build-glyphs.py --check
python3 tests/inscriptions.py
python3 tests/recovery_cli.py
npm test
bash scripts/build-web.sh
node --test tests/wasm.test.js
```

The verification suite checks, among other things, deterministic canonical SVGZ output, native/WebAssembly byte parity, a rasterized perceptual-quality regression floor, exact source and mode recovery, exact reversible names, keyed authentication, wrong-key rejection, legacy migration, glyph spacing, and rejection of PNG/SVG as new transmutation outputs.
