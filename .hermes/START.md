# Aphrodite / HermesCompress — Development Session Prompt

You are working on **Aphrodite**, a context compression system for Hermes Agent inside the **HermesCompress** repository at `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress`.

---

## Architecture

```
Hermes Agent → headroom-cache (:9799) → DeepSeek API
             └─ fallback: deepseek (direct)

HermesCompress/
├── crates/aphrodite/          Rust proxy (CCR compression, tool relay)
│   ├── src/proxy.rs            Main proxy handler + compression
│   ├── src/config.rs           CLI + TOML config
│   ├── src/retrieve.rs         CCR retrieval endpoint
│   └── src/main.rs             Router + server
├── plugins/aphrodite/          Python Hermes plugin (10 modules)
│   ├── __init__.py              Public API, register()
│   ├── _core.py                 Constants, thresholds, shared state
│   ├── _engine.py               ContextEngine
│   ├── _hooks.py                6 hooks + 7 tool handlers
│   └── _tools.py, _resolve.py, _marker.py, _inline.py, _binary.py, _proxy.py
├── vendor/headroom/            Upstream headroom (submodule)
├── profiles/                   7 Hermes profiles (symlinked to ~/.hermes)
├── scripts/                    run-headroom-proxy.py, generate-soul.py, setup-headroom-providers.py
└── .hermes/                    MASTER-TASKS.md, HANDOFF.md, plans/, tasks/
```

---

## Current State

### Releases
- **v0.5.57** — zero clippy, zero ruff, full codebase clean
- **v1.62.0** — plugin version
- **29 bugs fixed** across 5 releases (v0.5.51 → v0.5.57)

### Bug Status
| Severity | Total | Done | Remaining |
|---|---|---|---|
| 🔴 Critical | 7 | 7 | 0 |
| 🟠 High | 6 | 3 | 3 |
| 🟡 Medium/Low | 64 | 17 | 47 |
| 🟢 Improvement | 6 | 0 | 6 |

Remaining high: #51 (hash mix), #57 (tool-chain off-by-one), #67 (CcrStore trait)

### Running Services
- **headroom cache proxy**: `:9799` — response caching, saves API costs
- **aphrodite token proxy**: `:9798` — (when started) SQLite CCR + tool relay

### Hermes Config
- Default provider: `headroom-cache` → `http://127.0.0.1:9799`
- Fallback: `deepseek` (direct API)
- Main model: `deepseek-v4-pro` (65K max_tokens, reasoning high)
- All 13 auxiliary tasks: `deepseek-v4-flash`
- Delegation: `deepseek-v4-flash`

---

## Profiles (7 available)

```bash
hermes --profile aphrodite-barebone              # direct, no plugin, no compression
hermes --profile aphrodite-proxy-cache           # :9797, engine 50%
hermes --profile aphrodite-proxy-token           # :9798, engine 50%
hermes --profile aphrodite-compress-off          # :9797, engine 0%
hermes --profile aphrodite-compress-light        # :9797, engine 90%
hermes --profile aphrodite-compress-medium       # :9797, engine 50%
hermes --profile aphrodite-compress-aggressive   # :9797, engine 10%
```

Each has `SOUL.md` teaching it about HermesCompress, testing protocols, and release steps.

---

## Key Commands

```bash
# Build + test
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p aphrodite && cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite
cargo test -p aphrodite
cargo clippy -p aphrodite

# Lint
cd plugins/aphrodite && ruff check . && ruff format --check .

# Proxy management
source ~/.privateenvsh
headroom proxy --port 9799 --host 127.0.0.1 --openai-api-url https://api.deepseek.com/v1 --mode token --workers 1 --no-subscription-tracking --no-optimize --no-ccr-marker --no-telemetry &
kill $(lsof -ti:9799)

# Version bump
sed -i '' 's/^version = "X"/version = "Y"/' crates/aphrodite/Cargo.toml
sed -i '' 's/BIN_VERSION = "vX"/BIN_VERSION = "vY"/' plugins/aphrodite/_core.py
sed -i '' 's/PLUGIN_VERSION = "X"/PLUGIN_VERSION = "Y"/' plugins/aphrodite/_core.py
sed -i '' 's/^version: X/version: Y/' plugins/aphrodite/plugin.yaml

# Release
git add crates/ plugins/ scripts/ profiles/ .hermes/
git commit -m "..."
git push aphrodite Current
git tag -f -m "..." vX.Y.Z && git push aphrodite vX.Y.Z --force
gh release create vX.Y.Z --repo PlayForm/Aphrodite --title "..." --notes "..." ~/.hermes/aphrodite/aphrodite
```

---

## Load These Skills

```
/skill execution-blocks
/skill aphrodite-dev-workflow
/skill aphrodite-iterate-release
/skill hermes-plugin-development
```

---

## Your Task

We're continuing aphrodite development. Start by:
1. Checking proxy health: `curl -s http://127.0.0.1:9799/health`
2. Verifying the build: `cargo build --release -p aphrodite`
3. Reading `.hermes/MASTER-TASKS.md` for remaining work
4. Proposing the next bug to fix from the high/medium priority list

Be concise. Report numbers, not opinions. Use execution blocks for repeatable operations.
