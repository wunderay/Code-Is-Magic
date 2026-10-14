# Code Is Magic

Turn a GitHub repository into a connected grimoire of magic circles. A shared Rust engine powers both a terminal program and a static HTML Canvas website compiled to WebAssembly. No backend, analytics, source execution, or server-side repository upload.

## What the diagram means

- Every discovered function has a seal. Functions with at least **12 non-comment body source lines** get full circles; shorter functions get compact seals. Adjust the threshold without losing call connections.
- Syntax is sampled in source order around the circle. Branches become Δ, loops ∞, returns Ω, bindings ᚠ, assignments ᚷ, comparisons ⚖, member access ☿, async ☽, literals ◈. Identifiers deterministically map to Elder Futhark runes, Greek letters (including archaic Greek forms), and esoteric marks. Greek letters, alchemical symbols, and portable Unicode dingbats supply additional ornament.
- Layouts form an irregular, source-seeded constellation: variable seal sizes, fractured eccentric rings, off-axis satellites, uneven rune bands, and tangled curved channels. Linked functions cluster around already placed neighbors; collision checks reserve breathing room for labels. Identical source yields identical chaos in the native and browser engines.
- Arrows connect caller → callee. Dashed channels connect a type to its members. Recursive calls form loops. Ambiguous/external calls stay unconnected and are counted.
- Structs use hexagons, enums layered diamonds, classes framed octagons/squares, interfaces opposing triangles, and named JavaScript object literals cubes. Type geometry is layered with the language's motif, and source syntax gives each entity its own inscription.
- Optional **Wingdings ornament** uses an installed Wingdings font for every third syntax glyph. The proprietary font is not bundled. SVG viewers need that font; PNG captures the browser's rendered glyphs. Portable Unicode mode is the default.

| Language | Source extensions | Signature |
| --- | --- | --- |
| Rust | `.rs` | Angular forge: octagram and inner square |
| Python | `.py` | Lunar spiral: overlapping moons and spiral |
| JavaScript | `.js .jsx .mjs .cjs` | Solar wheel: twelve-point star |
| TypeScript | `.ts .tsx` | Prismatic seal: opposing triangles and hexagon |
| Go | `.go` | Flowing triad: three overlapping circles |
| C | `.c .h` | Twelve-sided clockwork |
| C++ | `.cpp .hpp .cc` | Sixteen-sided clockwork |
| Java | `.java` | Cathedral frame: crossed squares and cardinal medallions |
| C# | `.cs` | Diamond lattice: nested diamonds and octagram |

## Terminal

Install [Rust](https://rustup.rs), then:

```sh
git clone https://github.com/wunderay/Code-Is-Magic.git
cd Code-Is-Magic
cargo install --path . --locked
code-is-magic https://github.com/owner/repo --output magic.svg
```

The terminal reports counts and writes an SVG image. Open the SVG in your browser or an image editor.

```sh
code-is-magic ./my-project --output magic.svg --json diagram.json
code-is-magic https://github.com/owner/repo.git --reference main --min-lines 20 --palette parchment --output magic.svg
code-is-magic ./my-project --wingdings --output magic.svg
```

`--palette` accepts `gold`, `astral`, or `parchment`. `--reference` accepts a branch, tag, or commit; downloaded files are pinned to the resolved commit's immutable blob SHAs. Set `GITHUB_TOKEN` in your shell environment for private repositories or a higher GitHub rate limit. Never pass a token as a command argument.

On Windows Command Prompt, chain setup commands:

```bat
git clone https://github.com/wunderay/Code-Is-Magic.git && cd Code-Is-Magic && cargo install --path . --locked
```

Use a local source directory for fully offline operation after installation. Local traversal excludes `.git`, `target`, `node_modules`, and symlinks. GitHub traversal reads every supported source file, including vendored dependencies. Submodules are independent repositories and are not traversed.

## Website

The website accepts a GitHub repository URL, optional revision/read token, or a local source folder. It renders the Rust SVG into an HTML canvas, with pan, zoom, a searchable declaration inspector, three palettes, adjustable large-function threshold, and PNG/SVG/JSON downloads. Downloaded JSON diagrams can be reopened offline. Tokens are sent only to `api.github.com`, cleared from the input immediately, and never saved to storage.

Build locally (Bash, Git Bash, or WSL):

```sh
bash scripts/build-web.sh
python3 -m http.server 8000 --directory web
```

Open `http://localhost:8000`. Opening `index.html` directly through `file://` cannot load WebAssembly modules. The build script installs the exact wasm-bindgen CLI version in `Cargo.lock`, builds the Rust library without native CLI dependencies, and creates `web/pkg/`. No Node package installation is needed.

For Windows without Bash, read the wasm-bindgen version in `Cargo.lock` and run these commands with that exact version in place of `<version>`:

```bat
rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli --version <version> --locked && cargo build --locked --release --target wasm32-unknown-unknown --lib --no-default-features && wasm-bindgen --target web --out-dir web/pkg --no-typescript target/wasm32-unknown-unknown/release/code_is_magic.wasm
```

### GitHub Pages

1. In this repository, open **Settings → Pages → Build and deployment → Source → GitHub Actions**.
2. Push to `main`, or run **Actions → Build and publish grimoire → Run workflow**.
3. The included workflow verifies the source, compiles WebAssembly, uploads `web/`, and publishes it to GitHub Pages.

The expected URL for this repository is `https://wunderay.github.io/Code-Is-Magic/` after a successful deployment. All asset URLs are relative, including the worker and WASM, so project-site subpaths work. There is no custom domain or secret required for hosting.

GitHub's unauthenticated API quota can be consumed quickly because source blobs require requests per file. Use an optional read token or local-folder mode for larger repositories. The browser is not a suitable place for a token you do not trust the page with; use a narrowly scoped read token.

## Scope and accuracy

This is a **structural visualizer**, not a compiler or a complete runtime call graph. Its dependency-light Rust lexer handles common declarations, brace/indentation scopes, comments, quoted strings (including Rust raw strings), arrow-function block bodies, method ownership, Rust `impl` ownership, and unambiguous name-based call resolution. It does not execute repository code.

It does not perform type inference or module/import resolution. Dynamic dispatch, overloaded functions, complex generics, macros, expression-bodied functions, destructured names, uncommon syntax, inline template expressions, and languages outside the table may be incomplete. A call from `self`, `this`, or `Self` connects to an unambiguous method of the owning type; an explicitly named type can resolve its own method. Ordinary receiver-variable methods stay unresolved. A unique function name across files may be inferred; that is a structural guess, not proof of runtime linkage. Header `.h` files are classified as C.

GitHub input excludes extensions outside the language table and reports their count. The default budget is 5,000 supported source files, 2 MB per file, and 50 MB total. Any download/UTF-8/budget error stops generation instead of silently creating a partial diagram. `--max-files` adjusts the native file-count cap. Very large diagrams can become expensive to inspect. PNG export fits within 8,192 pixels per side and 32 million pixels total; SVG preserves full resolution.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
node --test tests/repository.test.js
bash scripts/build-web.sh
node --test tests/wasm.test.js
```

- `src/analyze.rs`: shared lexer, declarations, ownership, call graph.
- `src/render.rs`: deterministic language motifs, syntax glyphs, layout, SVG.
- `src/main.rs`: CLI, GitHub fetching, local scanning, file export.
- `src/lib.rs`: shared library and WebAssembly entrypoint.
- `web/`: static responsive website and worker; `web/pkg/` is generated.
- `tests/`: declaration, call-resolution, symbol, input, and GitHub-fetch regression checks.

Portable glyphs are embedded into exported SVGs using Noto Sans Runic and small Noto Sans / Noto Sans Symbols 2 subsets. The fonts come from the [Noto fonts project](https://github.com/notofonts/noto-fonts) under the SIL Open Font License; see `web/fonts/OFL.txt`. The Greek and symbol fonts are subsetted to the characters used by the renderer and interface.

MIT license for this project. Source repositories retain their respective licenses; optional system fonts retain theirs.
