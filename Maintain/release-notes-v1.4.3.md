**[Compare Aphrodite/v1.4.2...Aphrodite/v1.4.3](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.4.2...Aphrodite/v1.4.3)**

## Aphrodite v1.4.3 💋 Plugin v2.1.3

### Summary

v1.4.3 moves release preparation onto the `Development` branch (tags remain a
`Current`-side ceremony), adds an `APHRODITE_DIRECTIVES_DIR` override for
directives discovery, ships the full plugin hardening layer - Windows
multi-home dylib reuse, side-effect-free PID probe, install-flow auto-download
with POSIX `od` magic check - and reformats every built-in + root directive to
common-markdown (proper headings/paragraphs, never the `#`-comment style).
Current is a **test-free release line**: no test files, no test CI jobs (all
verification lives on Development), no benchmark/dev tooling. Dependencies
bumped to latest across the workspace. Binary `1.4.2 → 1.4.3`, plugin
`2.1.2 → 2.1.3`.

### Changes

- **Feature (release pipeline)**: `auto-release.sh` derives `RELEASE_BRANCH`
  from the current HEAD; `Check.yml`/`Build.yml` triggers restricted to
  `Development`; release prep now runs on `Development`, tags on `Current`.
- **Feature (directives)**: `APHRODITE_DIRECTIVES_DIR` env override is the
  first discovery candidate; intentional-empty semantics in `directives.rs`;
  all directives (built-in `include_str!` + root `./directives/` + plugin)
  rewritten to common-markdown and shipped in the release.
- **Feature (hooks)**: branch-aware git hooks - `post-checkout` reads the
  submodule branch from `.gitmodules`, `bump-submodule-gitlink.sh` gained the
  `unset GIT_DIR GIT_WORK_TREE` fix, plus `post-commit`/`post-merge` sync
  hooks and the `pre-commit` submodule-pin guard.
- **Feature (plugin install)**: `_ensure_binaries` auto-download with
  `APHRODITE_NO_AUTO_DOWNLOAD` opt-out; `download.sh` `xxd → od`; silent proxy
  failure → stderr capture + API-key hint; `install_message` LLM-provider docs.
- **Fix (plugin, Windows)**: proxy-pair reuse + dylib state sharing across
  multi-home loads (`fc52859`); side-effect-free PID probe with `argtypes`
  (`7bba3bb`); dylib candidate depth guard (`e2330b2`); `_process_state`
  holder + `_proxy_healthy` probe with body validation (PRs #7/#8).
- **Fix (plugin, hotreload)**: dead-PID copy reaping; hotreload dir moved out
  of the plugin tree; prefix-contract tests.
- **Chore (deps)**: workspace deps bumped to latest - serde 1.0.229,
  serde_json 1.0.151, bytes 1.12.1, thiserror 2.0.20, tokio 1.53.1,
  reqwest 0.13.5, clap 4.6.7, uuid 1.26.1, blake3 1.8.7, regex 1.13.1, lru
  0.18.4; headroom `hf-hub`/`fastembed`/`ort` pinned to the known-good ML
  cluster (0.5 / 5 / rc.12 - the `ml` feature is off in shipped builds).
- **Chore (release line)**: test files + bench examples removed from Current;
  `Check.yml` Test job removed (Development runs the full suite); rustfmt
  aligns VSCode with the nightly CI gate; ruff formatter config made explicit
  with the plugin-shim template excluded from reformatting.
- **Docs/skills**: README +273 lines; new `aphrodite-branch-release-flow` +
  `aphrodite-tool-testing` skills; release-workflow/hook-reference/operations/
  development-lessons/benchmarking/cargo-upgrade/auto-expand-testing skills
  refreshed; `templates/__init__.py` synced byte-identical to the live shim.

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ✅
  (verified in release-prep, `BUILD_EXIT:0`)
- Tests: `cargo test -p aphrodite -p aphrodite-hermes` ✅
  (387 passed, 0 failed, 1 ignored; integration tests run on Development
  only - Current ships test-free, so the Test CI job is removed there)
- Lint: `cargo clippy -p aphrodite -p aphrodite-hermes --lib -- -D warnings` ✅
  (finished clean)
- Python: ruff + pyright gates per release workflow (run in CI `Check.yml`)

### What Ships

| Artifact                             | Platform                                    |
| ------------------------------------ | ------------------------------------------- |
| `aphrodite-aarch64-apple-darwin`     | macOS ARM64                                 |
| `aphrodite-x86_64-apple-darwin`      | macOS Intel                                 |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64                                |
| `aphrodite-x86_64-pc-windows-msvc`   | Windows x86_64                              |
| Plugin v2.1.3                        | Hermes (standalone repo `Aphrodite-Hermes`) |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.4.2...Aphrodite/v1.4.3
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
