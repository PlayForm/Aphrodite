# Release Scope - Aphrodite v1.5.0 / Plugin v2.2.0

Machine-checkable declaration of what ships in this release vs what stays on
Development. The release ceremony (per `aphrodite-release-flow`) must verify
this manifest before tagging: every item marked **IN** is present on
Development, every item marked **OUT** is excluded from the sync (protected
path or explicitly deferred), and nothing unlisted crosses.

## IN - ships in 1.5.0 (all verified, gates green)

| Item                                                                 | Evidence                                                     | Notes                                                                                                                                                                 |
| -------------------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Version bumps: binary 1.5.0, plugin 2.2.0                            | Cargo.toml ×2, BINARY_VERSION, plugin.yaml                   | crates + hermes pin + BINARY_VERSION at 1.5.0, plugin 2.2.0; **`package.json` lags at 1.4.6 - must move in the ceremony**                                             |
| Edition-2024 migration (nightly toolchain, unsafe attrs, let-chains) | `8a65a42`, `cee3113`, `778435b`, `f3388b7`, `b08d5ad`        | vendored headroom bumped for closure signatures                                                                                                                       |
| Module atomization (flat → directory-based per-item trees)           | `7cf8d2f`, `1c3f1f3`, `a81acab`, `3988513`                   | `state/`, `preview/`, `directives/`, `marker/`, `resolve/`, `setup/`, `struct_extract/`, `config/`                                                                    |
| `aphrodite_debug` tool (per-session toggle, hot-reload-persistent)   | `5e2ae8f`, `8e75ff3`, `e100b49`, `f1975ef`, `ff044dc`        | session id persisted across dylib hot-reloads                                                                                                                         |
| chrono dependency removal                                            | `7b5ac47`                                                    |                                                                                                                                                                       |
| Inspection wave 1 (19 fixes: state/catalog/session correctness)      | `a0dec1e`                                                    | LRU O(1), VecDeque markers, named caps, `always_survive`, deterministic catalog, file-delta, session reset, largest-marker archive, char-count truncation             |
| Inspection wave 2 (9 proxy.rs items + retrieve/config fixes)         | `700332b`                                                    | poisoned-lock 500, `get_bool` case, TOML warn, `/tmp` fallback warn, `body_wants_stream` fast path, `starts_with` classification, `fill_pct` derivation, AppState doc |
| Skill system restructure (canonical contract skills + governance)    | `d49bbc2`                                                    | dev-side only (`.hermes/` protected)                                                                                                                                  |
| README savings analysis + compression metrics                        | `cd858a0`                                                    |                                                                                                                                                                       |
| Release notes + CHANGELOG                                            | `.hermes/release-notes/v1.5.0-draft.md`, root `CHANGELOG.md` | draft staged; finalize at tag time                                                                                                                                    |
| GATE results (Development, 2026-09-20)                               | clippy ✅, 422 tests ✅ (393 lib, 0 failed, 1 ignored)       | re-verify at the tag commit                                                                                                                                           |

## OUT - stays on Development (explicitly deferred)

| Item                                                                       | Reason                                                         | Where it lives                                             |
| -------------------------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------- |
| `AppConfig` / `AppCounters` structural split of the 47-field `AppState`    | dedicated refactor pass; category doc comment landed in wave 2 | `proxy.rs` (documented at `AppState`)                      |
| P9/T27 token-savings measurement (`LlmCallRecord`, provider usage capture) | feature, not a fix; the named next step in the codebase        | `flow.rs` (`est_request_bytes` reserved param), `hooks.rs` |

## DEV-INFRA - never crosses Development → Current (protected)

| Path                        | Why                                                          |
| --------------------------- | ------------------------------------------------------------ |
| `.hermes/`                  | dev archive (AGENTS, notes, release/, handoffs, uml, skills) |
| `bench/`                    | benchmark suite - dev tooling, not shipped product           |
| `.github/workflows/`        | branch-specific triggers                                     |
| `.gitmodules`               | branch field per branch                                      |
| `plugins/aphrodite` gitlink | points at plugin Current tip                                 |

## Release gates to run at tag time (ledger + workflow)

1. **`package.json`** must move 1.4.6 → 1.5.0 together with the crates (the
   version-ledger drift case; the four binary-track values move in one
   ceremony).
2. **Gate R7 trigger audit** at the exact tag commit - a plain
   `Aphrodite/v*` tag push reaches `cargo publish` for `aphrodite` and
   `aphrodite-hermes`; the release owner accepts every side effect.
3. **`BINARY_VERSION` is already 1.5.0** - the plugin's
   `_check_version_published` will warn (points at a release with no assets
   yet); that warning is resolved by creating the release, NOT by changing
   `BINARY_VERSION`.
4. **Artifact contract**: all 12 staged assets (4 targets × binary + dylib +
   `SHA256SUMS-<target>.txt`) present before the tag; `Finalize` enforces it.
5. B4 branch-identity audit before any sync/tag.
