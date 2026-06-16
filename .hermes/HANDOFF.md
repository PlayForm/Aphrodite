# Aphrodite → Handoff
## v0.5.60 / v1.62.7 — 2026-06-16 | 30 bugs fixed

### Active State
- Proxy binary: v0.5.60, both cache (:9797) and token (:9798) running via cargo watch
- Plugin: v1.62.7, 10 modules, 9 tools, 6 hooks, ContextEngine
- Profiles: 7 configured (barebone through aggressive) with differentiated skills
- Lint: zero ruff errors, pyright strict mode

### 30 Bugs Fixed (out of 91 found)
- 🔴 7/7 critical resolved
- 🟠 4/6 high resolved (#51, #18 remain)
- 🟡 19/64 medium resolved
- 58 bugs remain (2 high, 45 medium, 6 improvement, 5 skipped)

### Recently Fixed (v1.62.1)
- #57 — tool-chain backtrack off-by-one in compress()
- #58 — asymmetric CCR truncation [:5000]/[:200] removed
- Profile skills differentiation (barebone/compress-off exclude aphrodite-dev-workflow)
- proxy-cache compression disabled (threshold=0.0)
- dev-dual.sh trap cleanup, generate-soul.py --dry-run
- gitignore profiles/cache/ logs/ sessions/ cron/
- pyproject.toml dev deps, pyrightconfig.json strict mode

### Dependency Chain (4 repos)
| # | Repo | Status | Key for bugs |
|---|---|---|---|
| 1 | `playform/aphrodite` | ✅ Audited | All 91 bugs found here |
| 2 | `chopratejas/headroom` | ❌ Not scanned | #67 (CcrStore trait), #83 (TTL), #51 (hash length) |
| 3 | Headroom fork | ❌ Not located | Local patches not consuming in build |
| 4 | Hermes agent | ❌ Not located | #91 (ContextEngine contract), hook signatures |

### Next Steps
1. Fix #51 (16-char vs 64-char hash mix) — inline "i:" prefix exists but proxy hashes don't match
2. Fix #18 (inject_tool placement in Rust proxy)
3. Scan `chopratejas/headroom` at pinned commit `126543f5` — resolves #67, #83, #51
4. Locate headroom fork + Hermes repo for contract verification
