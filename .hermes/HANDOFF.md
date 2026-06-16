# Aphrodite → Handoff
## v0.5.54 / v1.61.0 — 2026-06-16

### Active State
- Proxy binary: v0.5.54, listening on :9798 (token, token-only mode)
- Plugin: v1.61.0, 9 tools, 6 hooks, ContextEngine
- Context engine: opt-in via `APHRODITE_CONTEXT_ENGINE=1`
- Modular: 10 files (was 1656-line monolith)
- Lint: zero ruff errors, pyright config ready
- Tooling: uv, ruff, pyright installed

### 29 Bugs Fixed (4 waves)
| Wave | Version | Count | Highlights |
|---|---|---|---|
| 1 | v0.5.51 | 13 | cache_alive crash, _recent_markers shadow, EMA, health, file read security, port default |
| 2 | v0.5.52 | 8 | mode warning, listen optional, first-turn skip, wildcard, filter_content, compress size |
| 3 | v0.5.53 | 8 | circular import break, ruff/pyright, modular validation |
| 4 | v0.5.54 | — | duplicate cleanup, directory reorganization |

### Plugin Structure
```
plugins/aphrodite/
├── __init__.py         — public API, re-exports, register()
├── _core.py            — constants, thresholds, shared state, utilities
├── _inline.py          — zlib fallback compression
├── _marker.py          — CCR formatting, proxy compression, parsing
├── _binary.py          — platform detection, binary download
├── _proxy.py           — env loading, health checks, proxy launch
├── _resolve.py         — CCR resolution + recursive unpack
├── _tools.py           — retrieve + compress handlers + schemas
├── _hooks.py           — 6 hooks + 7 tool handlers + conversation memory
├── _engine.py          — ContextEngine
├── plugin.yaml
├── pyproject.toml      — ruff config (zero errors)
└── pyrightconfig.json
```

### Documentation (.hermes/)
```
.hermes/
├── MASTER-TASKS.md     — comprehensive bug + plan audit (ALL bugs #1–#91)
├── AGENTS.md           — development context
├── CLAUDE.md           — project rules
├── HANDOFF.md          — this file
├── tasks/
│   ├── 1-wave-audit-v050-v0550.md
│   ├── 2-python-plugin-bugs-48-58.md
│   ├── 3-proxy-rs-bugs-59-70.md
│   ├── 4-main-retrieve-config-bugs-71-91.md
│   └── 5-wave4-execution.md
└── plans/
    ├── 0-headroom-100-tasks.md
    ├── 1-honest-gaps.md
    └── 2-architectural-subtasks.md
```

### Profiles (7 total)
| Profile | Provider | Engine | Threshold |
|---|---|---|---|
| default | deepseek direct | compressor | 0.5 |
| aphrodite-barebone | deepseek direct | compressor | off |
| aphrodite-proxy-cache | :9797 | aphrodite | 50% |
| aphrodite-proxy-token | :9798 | aphrodite | 50% |
| aphrodite-compress-off | :9797 | aphrodite | 0% |
| aphrodite-compress-light | :9797 | aphrodite | 90% |
| aphrodite-compress-medium | :9797 | aphrodite | 50% |
| aphrodite-compress-aggressive | :9797 | aphrodite | 10% |

### Key Commands
```bash
# Start proxy
~/.hermes/aphrodite/aphrodite --listen 127.0.0.1:9798 --api-key "$APHRODITE_API_KEY" --mode token --tool-relay

# Start Hermes with profile
hermes --profile aphrodite-compress-aggressive

# Run ruff lint
cd plugins/aphrodite && ruff check .

# Run cargo tests
cargo test -p aphrodite

# Kill all profile processes
pkill -f "hermes --profile aphrodite"

# Kill proxy
lsof -ti:9798 | xargs kill
```

### Remaining Work
- **48 medium/low bugs** — mostly polish (see MASTER-TASKS.md)
- **0 critical / 3 high bugs remaining** (#51, #57, #67)
- **Structure**: `.hermes/` cleaned and organized
