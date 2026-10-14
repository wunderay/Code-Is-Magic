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
code-is-magic magic.png --output recovered.zip
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
code-is-magic https://github.com/owner/repo --language-colors --output magic.png
code-is-magic --decode magic.svg --output recovered.zip --json snapshot.json
code-is-magic --decode-names magic.svg
```

The same positional `INPUT` accepts a GitHub link, a local source directory, or an original exported PNG/SVG. Image input automatically recovers the complete source snapshot into `recovered.zip` unless `--output` specifies another path. The explicit `--decode` option remains available.

`--palette` accepts `gold`, `astral`, or `parchment`. `--reference` accepts a branch, tag, or commit; all GitHub reads use the resolved commit's immutable blob IDs. Set `GITHUB_TOKEN` in your shell environment for private repositories or more API quota. Tokens are never put into the snapshot.

`--language-colors` optionally gives each seal its language's hue. The website has the same **Language colors** checkbox, off by default, with matching legend swatches. Base colors come from [GitHub Linguist](https://github.com/github-linguist/linguist/blob/main/lib/linguist/languages.yml) and are authored in the native-language profiles. Tones adapt to the selected background for readability. Connections retain the chosen palette's ink. Colors affect neither geometry nor source recovery, and exported PNG/SVG/JSON diagrams preserve the setting.

Local CLI mode preserves file bytes, executable flags, and symlink targets without following symlinks. It excludes directories named `.git`, `target`, `node_modules`, and `pkg`; GitHub mode preserves every entry in the selected tracked tree. Windows files do not expose Unix executable flags through ordinary local metadata; use GitHub input to preserve the Git modes precisely.

## Optional cryptographic magic key

A random **32-byte key** (64 hexadecimal digits) changes the central emblems, contour geometry, rune alphabet, collar rotations, satellites, constellation layout, and braided connections. Circular rings, language motifs, graph relationships, optional language colors, and function area scaling remain. Same canonical snapshot, key, renderer version, and settings give the same output bytes in native Rust and WebAssembly; a different key produces a different inscription. Without a key, existing output and older image recovery remain compatible.

Generate a key **outside your source folder**, then use it in either direction:

```sh
code-is-magic --generate-key ../my-magic.key
code-is-magic ./my-project --key-file ../my-magic.key --output keyed.png
code-is-magic keyed.png --key-file ../my-magic.key --output recovered.zip
code-is-magic --decode-names keyed.png --key-file ../my-magic.key
```

The CLI also accepts `CODE_IS_MAGIC_KEY` from the environment. In Bash:

```sh
export CODE_IS_MAGIC_KEY="$(cat ../my-magic.key)"
code-is-magic https://github.com/owner/repo --output keyed.png
code-is-magic keyed.png --output recovered.zip
unset CODE_IS_MAGIC_KEY
```

In Windows PowerShell:

```powershell
$env:CODE_IS_MAGIC_KEY = (Get-Content ..\my-magic.key -Raw).Trim()
code-is-magic https://github.com/owner/repo --output keyed.png
code-is-magic keyed.png --output recovered.zip
Remove-Item Env:CODE_IS_MAGIC_KEY
```

`--key-file` overrides the environment; `--key-file -` reads the key from stdin. Secrets are not accepted as command-line values, printed in errors, or serialized into images. Key generation uses the OS cryptographic random generator, creates a new file without overwriting an existing file, and applies mode `0600` on Unix. Windows file permissions follow the destination folder's ACL. A key file inside the captured local directory is rejected. Keep keys and files containing secrets out of captured source trees. These are random keys, **not passphrases**: arbitrary passwords are rejected instead of being silently weakened into encryption keys.

On the website, **Optional magic key** accepts the same hex key for transmutation and image recovery. **Generate** uses Web Crypto's secure random generator; **Save key** downloads the key separately. The key stays in this tab's memory and is sent only to the local Rust WebAssembly worker, never to GitHub or a backend, and is not kept in browser storage. Enter the original key before submitting a keyed image. Downloads use the key associated with the displayed artwork. PNG, SVG, and keyed offline JSON all encrypt the embedded source capsule; keyed JSON contains public artwork and ciphertext, not plaintext analysis/source. Import keyed JSON with its original key. Explicit source recovery downloads an ordinary source ZIP.

The scheme uses RustCrypto **AES-256-SIV** ([RFC 5297](https://www.rfc-editor.org/rfc/rfc5297.html)) for deterministic authenticated encryption, with domain-separated HMAC-SHA-512 derivation from the 32-byte key. Different domains separate source capsules, name contours, visual seeds, and the glyph permutation. Encrypted name contour v2 remains independently reversible from SVG coordinates with the key. An encrypted capsule starts with `CIMKEY01`; the existing PNG frame transports this ciphertext in pixel bits. Recovery authenticates it before decompressing or exposing files. Missing/wrong keys and altered ciphertext fail; keyed SVG recovery also validates its visible artwork against the authenticated copy. Tests include an independent Python AESSIV vector, key separation, legacy recovery, and native/WASM byte parity.

**Keep the key:** it cannot be recovered from the image. Deterministic encryption reveals repeated identical inputs under the same key. The public diagram also reveals structure, seal sizes, language motifs/colors, and approximate name lengths; it is an encrypted source carrier with a visible visualization, not a way to hide every fact about a repository. Retain the original exported PNG for recovery; resizing and other pixel edits can still destroy its payload.

## Website

The website's main **GitHub link or magic image** input accepts both directions. Paste a GitHub link and select **Transmute** to generate circles on the canvas. Choose, drop, or paste an original exported PNG/SVG in that same input and select **Recover source** to download the complete reconstructed source ZIP entirely in your browser. Editing the field switches back to repository input. Source folder input, pan/zoom, an inspector outside the artwork, three palettes, PNG/SVG/JSON downloads, and a geometry name decoder are also available.

The website is an inscription chamber with a manuscript-style index, engraved controls, and **Light mode / Dark mode**. The theme button also selects vellum/bronze or midnight/gold ink for freshly generated diagrams. Imported offline diagrams retain their original ink until a source-based re-render. The theme preference is stored locally; secret keys are never stored.

**Glow** adds luminous ink and **Motion** animates quiet collar filaments and sparks following the actual braided-channel curves. Both can be toggled independently. Motion starts paused when the browser requests reduced motion and pauses when the tab or canvas is not visible. Raster layers are cached at viewport resolution, so the animation does not re-analyze or re-rasterize the entire SVG every frame. These effects belong only to live playback; exported PNG/SVG/JSON retain static deterministic artwork and the complete recoverable source capsule.

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

Function seal **area is directly proportional to non-comment body source lines**: radius = 24 × √lines. Four times as much code produces twice the radius. Equal-length functions have equal-sized seals regardless of their names, with no size cap or name-based jitter. The adjustable default of 12 lines controls ornament detail, not circle size. Braided invocation channels connect unambiguous caller → callee targets, with engraved knots and round endpoint seals. No arrowheads are drawn; direction remains in the inspector and graph metadata. Dotted tethers with diamond joints express type membership. Recursive calls use woven return loops. Structs use hexagons, enums diamonds, classes frames, interfaces opposing triangles, and named JavaScript object literals cubes. The reversible name contour distinguishes different names independently of those motifs.

Outer collars stay circular. A name-derived center chooses and varies twelve original emblem families: ritual triangles, crescents, floral knots, heraldic crests, eyes, interlocking facets, branching runes, hourglasses, square labyrinths, orbital seals, flames, and curved triquetras. The exact reversible name contour sits inside the emblem rather than deforming the rim. Its normalized geometry and v1 decoder remain compatible with earlier images. Source recovery preserves the original artwork and its separate full seal radius.

Visual references include TYPE-MOON / Fate, *Frieren*, *Witch Hat Atelier*, *Fairy Tail*, and *High School DxD*: concentric collars, script bands, distinct central emblems, and smaller subordinate seals. All rendered geometry is procedural; reference artwork is not bundled.

| Language | Extensions | Motif |
| --- | --- | --- |
| Rust | `.rs` | Eight angular forge notches |
| Python | `.py` | Six lunar crescents |
| JavaScript | `.js .jsx .mjs .cjs` | Solar wheel |
| TypeScript | `.ts .tsx` | Prismatic seal |
| Go | `.go` | Flowing triad |
| C / C++ | `.c .h / .cpp .hpp .cc` | Twelve gear teeth / sixteen linked square pairs |
| Java | `.java` | Four cathedral arches |
| C# | `.cs` | Diamond lattice |

The symbol atlas contains **478 measured glyphs in ten families**: Runic (89), Ogham (28), Greek including archaic forms (78), alchemical (116), celestial (44), geometric (35), dingbat (44), mathematical (6), ancient marks (14), and original bind sigils (24). Rune variants, historical marks, and broad symbols keep their natural proportions. Representative syntax includes Δ for branches, ∞ for loops, Ω for returns, and ᚠ for bindings; each seal mixes a seeded palette with syntax-derived choices. Ring inscriptions can use a continuous script, phrase-like spacing, two concentric rows on larger seals, or paired glyphs.

Visible outline bounds determine sizing and angular cells. Glyph height scales with the annular band instead of a fixed pixel cap. Each rotated outline fits its radial and angular cell; stacked glyphs have separate space. Circle-size scaling, contours, call relationships, and source recovery are unchanged. Variation is deterministic, including key-based alphabet permutations. Source recovery reads the complete capsule, not decorative syntax samples. Older images keep their original artwork when recovered and re-exported.

All inscriptions are vector geometry. `--wingdings` emphasizes portable vector dingbats; no proprietary font or readable source text is embedded in the artwork. Font outlines and original bind-sigil geometry are compiled with `python3 scripts/build-glyphs.py`; the Unicode selection and font provenance are recorded in `scripts/glyph-sources.json`. With `fonttools==4.61.1` installed, `python3 scripts/build-glyphs.py --check` verifies regeneration from bundled font subsets without downloads.

The **visualizer** is a structural lexer, not a compiler: dynamic dispatch, macros, complex generics, expression-bodied functions, import resolution, and uncommon syntax may be unresolved. This does **not** affect byte-exact source recovery: the full original tracked file bytes are archived independently of parsing.

## Format and limits

Language-specific policies and designs are authored in their own languages under [`languages/`](languages/README.md). A build-time compiler generates portable profiles for the shared Rust parser/renderer and browser extension filter. Native and WASM builds embed identical policies and need no additional user runtimes. See the authoring guide for regeneration and all-nine-language CI checks.

- Name contour v1: framed UTF-8 bytes encoded as 16 radial levels; geometry validates length and CRC32. No case folding or Unicode normalization is applied.
- Source capsule v1: canonical sorted snapshot plus the original vector artwork, serialized in fixed field order and compressed deterministically.
- PNG framing: `CIMPNG01`, compressed length, CRC32, then payload, in RGB low bits. The decoder bounds dimensions, payload size, decompression, paths, duplicates, and modes before exposing files.
- ZIP: stored entries, UTF-8 path flag, fixed 1980 date, fixed modes; no nondeterministic timestamps.
- Budget: 5,000 tracked entries, 2 MB per file, 50 MB file bytes, 128 MB uncompressed capsule, 64 million PNG pixels. Exceeding a budget fails instead of omitting files.
- Recovery uses original exported files. Arbitrary artwork or screenshots do not contain a reconstructible repository. Unkeyed checksums detect corruption; optional keyed capsules authenticate and encrypt the source. Neither mode is a digital signature.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 -m pip install fonttools==4.61.1
python3 scripts/build-glyphs.py --check
python3 tests/inscriptions.py
python3 tests/recovery_cli.py
node --test tests/repository.test.js
bash scripts/build-web.sh
node --test tests/wasm.test.js
```

Tests cover name round trips and distinctions, text-free geometry recovery, binary/CRLF/BOM source recovery, modes/symlinks/submodule pointers, pixel-preserving PNG recompression, corruption rejection, safe archive paths, native/WASM artifact parity, source acquisition, declarations, call graphs, and keyed deterministic encryption/recovery through environment, key files, stdin, and browser APIs.

`src/glyphs.json` contains vector outlines derived from [Noto fonts](https://github.com/notofonts/noto-fonts), with the third-party font notice in `web/fonts/OFL.txt`. No project-wide license is specified. Captured source repositories retain their own notices.
