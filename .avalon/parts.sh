# Sourced by setup and ci: which parts of the gate a change affects.
# With AVALON_CHANGED unset every part is affected. A path that no part
# claims below counts as affecting everything (when in doubt, run).
#
#   rust       cargo fmt, clippy, test: src/**, tests/*.rs, examples/**,
#              Cargo.toml/Cargo.lock, and languages/profiles.json, which
#              src/profiles.rs embeds with include_str!. Toolchain pins too.
#   glyphs     scripts/build-glyphs.py check: the script, glyph-sources.json,
#              the fonts it lists (web/fonts/**) and the output src/glyphs.json.
#   inscript   tests/inscriptions.py: the script plus the whole crate (it
#              builds and runs the CLI) and src/glyphs.json.
#   recovery   tests/recovery_cli.py: the script plus the whole crate.
#   js         npm test: web/**, package.json, tests/*.test.js.
#   wasm       build-web.sh and tests/wasm.test.js: the crate (the wasm is
#              built from it), web/**, package.json, the two build scripts
#              and the wasm test. Needs wasm32 target and wasm-bindgen.
#   (profiles) languages/**, scripts/build-profiles.py, tests/profile_build.py
#              are verified by the separate code-is-magic--profiles.yml
#              workflow (needs Go, Java, Mono), not by this gate.
# No check reads README.md, AGENTS.md, .avalon/commit-map, .avalon/meta, the
# project's own .github/ or scripts/check.sh, so those affect nothing.

RUST_RE='^(src/|tests/[^/]*\.rs$|examples/|Cargo\.(toml|lock)$|languages/profiles\.json$|rust-toolchain(\.toml)?$|\.cargo/|rustfmt\.toml$|\.rustfmt\.toml$|clippy\.toml$|\.clippy\.toml$)'
GLYPH_RE='^(scripts/build-glyphs\.py$|scripts/glyph-sources\.json$|src/glyphs\.json$|web/fonts/)'
JS_RE='^(web/|package\.json$|tests/[^/]*\.test\.js$)'
WASM_RE='^(web/|package\.json$|scripts/build-web\.sh$|scripts/build-default\.mjs$|tests/wasm\.test\.js$)'
PROFILES_RE='^(languages/|scripts/build-profiles\.py$|tests/profile_build\.py$)'
INERT_RE='^(README\.md$|AGENTS\.md$|LICENSE|scripts/check\.sh$|\.github/|\.avalon/(commit-map|meta)$)'
SELF_RE='^\.avalon/(setup|ci|parts\.sh)$'

affected() { [ -z "${AVALON_CHANGED:-}" ] || grep -Eq "$1" "$AVALON_CHANGED"; }

# Everything runs when unset, when the gate itself changed, or when a path is unclaimed.
ALL=0
if [ -z "${AVALON_CHANGED:-}" ] || affected "$SELF_RE" ||
   grep -Evq "$RUST_RE|$GLYPH_RE|$JS_RE|$WASM_RE|$PROFILES_RE|$INERT_RE|^tests/(inscriptions|recovery_cli)\.py$" "$AVALON_CHANGED"; then
  ALL=1
fi
part() { [ "$ALL" = 1 ] || affected "$1"; }

P_RUST=0 P_GLYPHS=0 P_INSCRIPT=0 P_RECOVERY=0 P_JS=0 P_WASM=0
part "$RUST_RE" && P_RUST=1
part "$GLYPH_RE" && P_GLYPHS=1
{ [ "$P_RUST" = 1 ] || part '^tests/inscriptions\.py$'; } && P_INSCRIPT=1
{ [ "$P_RUST" = 1 ] || part '^tests/recovery_cli\.py$'; } && P_RECOVERY=1
part "$JS_RE" && P_JS=1
{ [ "$P_RUST" = 1 ] || part "$WASM_RE"; } && P_WASM=1
true
