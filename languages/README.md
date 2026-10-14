# Native-language profile authoring

Language-specific lexical switches, declaration rules, call compatibility, center glyphs and collar geometry are authored in their own languages:

| Source | Language | Base policy |
| --- | --- | --- |
| `c/profile.c` | C | Shared brace-language structural heuristics |
| `cpp/profile.cpp` | C++ | C |
| `rust/profile.rs` | Rust | C, with raw strings, lifetimes and explicit function declarations |
| `python/profile.py` | Python | C, with indentation, hash comments and triple quotes |
| `javascript/profile.js` | JavaScript | C, with ECMAScript dialect pairing and solar design |
| `typescript/profile.ts` | TypeScript | JavaScript, with prismatic design |
| `go/profile.go` | Go | C, with receiver syntax and flowing triad |
| `java/Profile.java` | Java | C, with cathedral design |
| `csharp/Profile.cs` | C# | Java, with its own dialect and diamond design |

These are **build-time profile generators**, not per-file native-runtime analyzers. The Rust engine remains a generic structural lexer/parser and deterministic renderer. It executes no inspected repository code. This preserves the existing structural heuristics and limits; it does not turn the visualizer into nine compilers.

Each module emits a JSON profile or overrides. The profile compiler resolves inheritance, validates the schema and geometry, rejects duplicate extensions and cycles, and writes deterministic artifacts:

- `profiles.json`: complete policies embedded in the Rust native and WASM library.
- `sources.json`: SHA-256 provenance for all nine authoring modules.
- `../web/languages.js`: supported extensions for browser repository filtering.

Ordinary `cargo build`, `cargo install` and website builds use those checked-in artifacts. End users need **no Python, Node, Go, Java or C# runtime** to analyze repositories with the Rust CLI; the website runs in the browser without a backend.

## Regenerate after changing a language policy

Profile maintainers need Python 3, Rust, Node 24, Go 1.22 or newer, GCC/G++, Java 17 (JDK/compiler module), and Mono (`mcs` + `mono`). TypeScript uses Node's native type stripping; its authoring module uses erasable types and no extra npm dependency. Java compilation uses `javac`, or the JDK compiler module when `javac` is not on PATH. C# emits profiles through Mono, without a package restore.

From the repository root:

```sh
python3 scripts/build-profiles.py
python3 scripts/build-profiles.py --check
python3 tests/profile_build.py
cargo test --locked
bash scripts/build-web.sh
node --test tests/wasm.test.js
```

Commit both the authoring module and generated artifacts. CI compiles/runs **all nine actual native-language modules**, checks freshness, and then checks native/browser parity and source recovery. Missing authoring toolchains fail explicitly; the check never substitutes pre-generated output or skips a module.

Profile schema v1 includes comment/string flags, indented/braced declaration maps, type/object/function markers, control/prefix exclusions, member/self rules, mergeable containers, dialect compatibility groups, glyphs, six-digit base RGB language colors, and collar primitives. Browser color legends and supported extensions are generated from the same profiles. Portable geometry permits only numeric SVG paths and polygons. Inheritance reuses the prior broad brace-language structural policy; language-specific overrides retain existing behavior.
