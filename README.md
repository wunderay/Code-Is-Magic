# Code Is Magic

Code Is Magic turns a source repository into a deterministic field of connected magic circles and can reconstruct the captured repository from the resulting artifact.

New transmutations have **one canonical output format: a scripted, perceptually compressed SVGZ**. Most of the drawing is stored as the parameters that generate it, and a small embedded script unfolds it when the image opens in a browser. The artwork changes only by sub-pixel amounts chosen to be invisible, and the embedded repository snapshot stays byte-exact.

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

Add `--output name.svg` to write the same drawing uncompressed. Chromium does not decompress a `.svgz` opened from disk (Firefox does, and every browser does when a web server sends it), so the `.svg` form is the one to double-click.

### How the image stays small

A Code Is Magic drawing is almost entirely *generated* geometry. Each braided call channel is 192 points sampled from one cubic curve. Every glyph placement in an inscription band follows from the cell proportions. Collars repeat a language template, and each vertex of a name contour encodes one nibble of the name. The canonical encoder:

1. analyzes and renders the repository deterministically. The renderer tags each generated subtree with its parameters;
2. rounds ordinary literal geometry below pixel scale (paths 0.01, placement 0.001, transforms 0.000001 SVG units), as before;
3. replaces each braid, inscription band, collar, regular polygon and name contour with a one-line macro holding its parameters. It drops values the expander can recompute, such as cell weights, and stores glyph outlines as relative paths on a 2-font-unit grid;
4. embeds a ~5 KB script that unfolds the macros in the browser, plus the glyph metrics and collar templates it needs;
5. embeds the repository snapshot in a binary, brotli-compressed (quality 11) capsule, authenticated and optionally encrypted as before;
6. compresses the complete result as deterministic gzip/SVGZ with a zero timestamp.

Rust unfolds the same macros with the very functions the renderer drew them with. With unrounded parameters, the expansion reproduces the renderer's markup **byte for byte**, which the tests check. Recovery tools, `--decode-names` and the website all work on that expansion. A wasm test checks every number the embedded script produces against it.

Measured on this repository's own tree (7394×6850 drawing, 72 entries):

| | before | after |
| --- | ---: | ---: |
| whole SVGZ | 1,686,757 B | **385,845 B** (−77%) |
| artwork (gzip share) | 1,342,901 B | 167,365 B (−88%) |
| source capsule (gzip share) | 329,703 B | 213,785 B (−35%) |

Against the earlier canonical render, the scripted image as unfolded by Chromium measures **52.5 dB PSNR** for the whole drawing (rendered at ¼ scale). A dense 1:1 crop measures 37.1 dB, where the only differences are glyph-edge anti-aliasing. The capsule is now about the size of `git archive | xz -9e`, and exact source recovery has to carry it. That is why the whole file cannot shrink by 99% while it still holds the repository.

`--expand image.svgz --output static.svg` writes a static, script-free copy of the artwork, without the source, for viewers that do not run scripts (`<img>` tags, GitHub previews, Inkscape).

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

- transmute a public or authorized GitHub repository into the same canonical scripted SVGZ used by the native CLI, or the same drawing as plain SVG;
- transmute a selected local folder in the browser;
- apply an optional magic key without storing it;
- open Code is Magic itself by default, with language colors and a bundled, immutable source snapshot that needs no GitHub API requests;
- accept pasted or dropped repository links, direct magic-image links (where the host permits browser access), source files/folders, and recoverable magic images through the same input;
- display a celestial grimoire with light/dark themes, drifting motes, a turning rune ring and gilded spell-charging effects (all still under reduced motion), a prominent **Download magic circles** button for the bundled default and all generated diagrams, and a canvas fitted to the actual seals and call paths;
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
