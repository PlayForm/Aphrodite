# Release Scope - Aphrodite v1.4.3 / Plugin v2.1.3

Machine-checkable declaration of what ships in this release vs what stays on
Development. The release ceremony (per `aphrodite-branch-release-flow`) must
verify this manifest before tagging: every item marked **IN** is present on
Development, every item marked **OUT** is excluded from the sync (protected
path or explicitly deferred), and nothing unlisted crosses.

## IN - ships in 1.4.3 (all verified, gates green)

| Item | Evidence | Notes |
| --- | --- | --- |
| Version bumps: binary 1.4.3, plugin 2.1.3 | Cargo.toml ×2, package.json, BINARY_VERSION, plugin.yaml | committed `5d8c09a` |
| Release notes + CHANGELOG | `Maintain/release-notes-v1.4.3.md`, root `CHANGELOG.md` | committed `8afe8e0` |
| Bench suite (corpus, compression crate, proxy, agents, conversational) | `bench/` | committed `5effa73`; **DEV-INFRA: must NOT cross to Current** - add to protected set |
| Chain-split feature (`chain_split.rs` + wiring) | 8 Rust files | **default OFF** (`APHRODITE_CHAIN_SPLIT=1` to enable); in 1.4.3 as opt-in |
| Fix layer (issues #9/#6/#5/#4, PRs #7/#8) | reverified complete | already on Development |
| Hooks: branch-aware + GIT_DIR fix | `.githooks/` | already on Development |
| GATE results | clippy ✓, 393 tests ✓, 22+3 plugin ✓, advisories ✓ | verified this session |

## OUT - stays on Development (explicitly deferred)

| Item | Reason | Where it lives |
| --- | --- | --- |
| Chain-split **teaching loop** (pre_llm_call nudge, adaptive threshold) | Tier-1 improvement, not built yet | future session |
| Chain-split **marker side-stream fix** (redirected-stdout pollution) | known issue, default OFF masks it | future session |
| Segment **summary with error hints** | Tier-3 usability | future session |
| Multi-tool / pipeline splitting | Tier-2 expansion | future session |
| Benchmark measurement of chain-split (compression-aware task) | Tier-4, needs harness run | post-release or in-release if time |

## DEV-INFRA - never crosses Development → Current (protected)

| Path | Why |
| --- | --- |
| `.hermes/` | dev archive (AGENTS, notes, release/, handoffs, uml) |
| `bench/` | benchmark suite - dev tooling, not shipped product |
| `skills/` | dev skills (already removed from Current) |
| `.github/workflows/` | branch-specific triggers |
| `.gitmodules` | branch field per branch |
| `plugins/aphrodite` gitlink | points at S-Current tip |

## Verification (run before tag)

```sh
# IN present:
test -f Maintain/release-notes-v1.4.3.md && grep -q '1.4.3' crates/aphrodite/Cargo.toml
grep -q '2.1.3' plugins/aphrodite/plugin.yaml
# OUT excluded from Current after sync:
git -C Current diff HEAD -- .hermes bench skills .github/workflows .gitmodules | wc -l  # 0
# chain_split default off:
grep -q 'chain_split = false' aphrodite.toml
```