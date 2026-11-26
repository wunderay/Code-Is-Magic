# Code is Magic

## Avalon monorepo

This project lives at `projects/code-is-magic/` in the Avalon monorepo; the root `AGENTS.md` also applies. Run from the monorepo root:

- `./avalon run code-is-magic setup` installs toolchains and dependencies.
- `./avalon verify code-is-magic` runs setup and ci for only the parts your change affects (compared with the base branch). Run this before pushing.
- `./avalon run code-is-magic ci` runs the whole gate, whatever changed.

The gate has parts (rust, glyphs, inscript, recovery, js, wasm); `.avalon/parts.sh` says which paths each depends on and `.avalon/ci` lists them. Unlisted paths run everything. Changing `.avalon/setup`, `ci` or `parts.sh` runs everything.

Verification is `./avalon verify code-is-magic --offline` where the work is done. GitHub Actions run only when strictly necessary (the owner, 2026-11-11), so both root workflows start by hand only: `.github/workflows/code-is-magic--verify.yml` (`ci: custom`; it calls `./avalon verify`) and `.github/workflows/code-is-magic--profiles.yml`. The native-language profiles check (`scripts/build-profiles.py --check`, `tests/profile_build.py`; needs Go, Java and Mono) is not part of the gate: after changing `languages/**` or its scripts, run it on a machine that has those toolchains, or start the profiles workflow when none does. `.github/workflows/pages.yml` and `profiles.yml` inside this project are inert in Avalon (GitHub only runs the root workflows); they become active again if the project is published as its own repository. In the published repository (public, so hosted runners are free), `.github/workflows/ci.yml` runs the same gate on every pull request, and `pages.yml` runs it again on `main` before publishing.

`bash scripts/check.sh` is a quicker, unconditional variant without the wasm build (`--full` adds it).

- Fastest first signal: `cargo fmt --all -- --check` (seconds).
- Add `--full` to also run `scripts/build-web.sh` and the wasm test (needs the `wasm32-unknown-unknown` target).
- A cold `cargo test --locked` takes several minutes; later runs are fast.
- Keep `scripts/check.sh`, `.avalon/ci` and `.avalon/parts.sh` in sync when any changes.
- `scripts/build-default.mjs` snapshots the git commit and tree at the current directory; nested here it embeds the Avalon monorepo commit and only this project's subtree.
- Do not edit license or font text.
