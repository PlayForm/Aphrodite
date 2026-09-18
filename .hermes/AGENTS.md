# Aphrodite - Development Context

Welcome! You're working on Aphrodite: a blazing-fast CCR compression engine
(Rust binary proxy + Hermes plugin) that turns large tool outputs into tiny
`<<<CCR:hash|type|size>>>` markers so agents keep full context at a fraction
of the token cost. Read this file first - it is the map.

## Current state (verified 2026-09-18)

- **Binary 1.4.6** - `crates/aphrodite` (plus `crates/aphrodite-hermes`,
  `package.json`, `plugins/aphrodite/BINARY_VERSION`). For live values ask
  `aphrodite --version` / `aphrodite_stats`, never trust a hardcoded doc
  number.
- **Plugin 2.1.4** - `plugins/aphrodite` is a git submodule
  (PlayForm/Aphrodite-Hermes, remote `Source`). It is a **pure loader**: all
  compression/engine logic lives in the Rust dylib
  (`libaphrodite_hermes.dylib`); the FFI bindings are **generated**
  (`_bindings.py` via cbindgen → ctypesgen → finalize). Never hand-edit
  them - regenerate through the codegen pipeline and keep
  `test_finalize_bindings.py` green.
- **Runtime home `~/.hermes/aphrodite/`** - `aphrodite.toml`, `binaries/`
  (binary + dylib, auto-downloaded/auto-updated), `directives/`,
  `hotreload/` (dylib hot-reloads on mtime), `ccr.db`. The layout
  self-heals on start (re-copies binaries/directives/hotreload).
- **Skills live dev-side** in `.hermes/skills/` (13 skills) and auto-load
  because the repo is listed in `skills.trusted_project_dirs`
  (`~/.hermes/config.yaml`) - never ship them with the plugin, never re-trust.
- **Issue #11 preview machinery (landed in 1.4.6)** - honest previews: the
  `ok`-collapse is gone, the caller's type hint wins, build/error/linter/log
  arms surface real lines, `total_count` is real. `preview_max_chars` is
  wired end-to-end: env `APHRODITE_PREVIEW_MAX_CHARS` > TOML > default 120;
  absent key = unlimited. A broken/misleading preview is a bug, not cosmetic.
- **The ceremony** - Development builds, Current distributes (snapshot
  transplant, submodule first); the **B4 branch-identity audit gate (I11)**
  runs before ANY sync/tag (keyword-scan both refs for identity leaks);
  hotfixes work directly on Current and cherry-pick back with `-x`.
  `.githooks/` are REMOVED (2026-09-17) - never re-create them.
- **Two version tracks, never conflated**: binary 1.4.x vs plugin 2.1.x.
  `BINARY_VERSION` is a live distribution pointer - bump it LAST at tag time.

## Key paths

| What            | Where                                                    |
| --------------- | -------------------------------------------------------- |
| Core engine     | `crates/aphrodite/` (Rust)                               |
| Agent bridge    | `crates/aphrodite-hermes/` (dylib exports)               |
| Plugin (loader) | `plugins/aphrodite/` (`plugin.yaml`, `BINARY_VERSION`)   |
| Forked deps     | `vendor/headroom/` (submodule; `aphrodite-headroom-core`) |
| Config          | `aphrodite.toml.example` (tracked); `aphrodite.toml` (local, gitignored) |
| Runtime         | `~/.hermes/aphrodite/`                                   |
| Release         | `.hermes/release/RELEASE-TEMPLATE.md`, `.hermes/release-notes/` (v1.4.0…v1.4.3-draft) |
| Dev archive     | `.hermes/` - skills/, tmp/ (scratch), scripts/, classification/, notes/, uml/ |
| Maintenance     | `Maintain/` (scripts/, tests/, CHANGELOG.md)             |

## Dev flow - the joyful loop

- **Pane 0**: `cargo watch -x 'build -p aphrodite -p aphrodite-hermes' -x
  'run -p aphrodite'` - instant feedback on every save. Watch BOTH
  packages: `-p aphrodite` alone never rebuilds
  `libaphrodite_hermes.dylib` (a sibling package, so Cargo has no reason
  to touch it) and the plugin keeps running old code while the proxy looks
  alive.
- **Pane 1**: `hermes --profile dev-aphrodite` - test in production.
- Enable auto-expand for dev sessions (`APHRODITE_AUTO_EXPAND=1` or TOML);
  when you see a `<<<CCR:...>>>` marker, `aphrodite_retrieve(hash)` it -
  never re-read the source file behind it.
- Scratch belongs in `.hermes/tmp/` (gitignored contents, tracked
  `.gitkeep`), NEVER `/tmp`; gzip/tar.gz bulky fixtures in place.

## Quality gates - zero tolerance, record ACTUAL numbers (2026-09-18)

| Gate                                                        | Result                       |
| ----------------------------------------------------------- | ---------------------------- |
| `cargo test -p aphrodite` (lib + bins)                      | 406 passed (377 lib + 29 bins), 0 failed, 1 ignored |
| `cargo test -p aphrodite-hermes`                            | 52 passed                    |
| `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py` | 23/23 OK              |
| `python3 Maintain/tests/test_check_ffi_contract.py`         | 13/13 (50 asserts)           |
| checker: `python3 Maintain/check_ffi_contract.py`           | PASS, 0 violations           |
| drift-guard: `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` | identical |
| repro: `python3 <sigserve-scratch>/repro.py`                | SURVIVED (no crash)          |
| `ruff check plugins/aphrodite/`                             | 0 errors (2 known pre-existing perf-probe exclusions) |
| `cargo clippy -p aphrodite -- -D warnings`; `npx pyright plugins/aphrodite/` | clean               |
| `npx prettier --check .hermes/**/*.md`                      | clean (tabs, width 100, proseWrap preserve) |

Run all gates before any release claim; report what commands printed, never
"should pass".

## Dev-side skills (auto-load; edit the files directly)

- `aphrodite-release-flow` v2.0.0 - THE release/hotfix ceremony, incl. B4.
  (`aphrodite-branch-release-flow` v1.1.0 is DEPRECATED - archive only.)
- `aphrodite-testing-discipline` - probe/test rules: exercise the real
  plugin, no raw ctypes, scratch in `.hermes/tmp/`, env-var hermeticity.
- `aphrodite-tool-testing` - the 13 CCR tools + retrieve-first rule.
- `aphrodite-operations` - compressed-session workflow, rebuild, dep pins.
- `aphrodite-release-workflow` - release gates, version-sync locations,
  release-notes standards, crates.io publishing.
- `aphrodite-hook-reference` - exact Hermes hook invocations.
- `aphrodite-benchmarking` - proxy smoke/cache/threshold benchmarking.
- `aphrodite-auto-expand-testing` - auto-expand behavior matrix.
- `aphrodite-cargo-upgrade`, `aphrodite-upgrade-breakpoints` - silent
  breakage checklists for dep upgrades.
- `aphrodite-development-lessons` - session setup + imperative pitfalls.
- `aphrodite-v0.8.6-patterns` - historical snapshot only.

## Standing rules

- Never commit/tag/push unasked; the auto-committer sweeps - verify with
  `git log` / `git submodule status`, not `git status`.
- Development accumulates (never rewritten); tags live ONLY on Current;
  protected paths (`.gitmodules`, workflows, plugin gitlink) never cross a
  transplant; the B4 audit runs before every sync/tag.
- `BINARY_VERSION` bump LAST at tag time - it is a live distribution pointer.
- Env vars override config - keep test env hermetic.
- The preview the model sees must be the FINAL, honest representation of the
  stored payload.
- All `.hermes/**/*.md` stay prettier-clean.

## Reading order

1. This file. 2. `aphrodite-release-flow` (ceremony) +
`aphrodite-testing-discipline` (how to test). 3. `.hermes/notes/`
(RELEASE-METHODOLOGY.md, ISSUE-11-LANDED.md, session reports) and
`.hermes/classification/` (file ownership per phase/kind/layer).