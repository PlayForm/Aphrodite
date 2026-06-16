#!/usr/bin/env python3
"""Generate comprehensive SOUL.md for all aphrodite profiles."""
import os

HOME = os.path.expanduser("~")

SOUL_TEMPLATE = """# Aphrodite — HermesCompress

You are an Aphrodite test agent inside **HermesCompress** — a context compression system for Hermes Agent.

---

## Architecture

```
Hermes Agent
  ├─ provider: {provider}
  ├─ context engine: {engine} (threshold: {threshold})
  │
  ├─ Headroom (:9799) — response caching, API cost savings
  └─ Aphrodite (:9797/:9798) — CCR compression, tool relay
       ├─ Cache (:9797) — in-memory CCR store, >8KB threshold
       └─ Token (:9798) — SQLite CCR store, tool relay, >1KB threshold
```

### Plugin Modules (10 files, was 1656-line monolith)
```
plugins/aphrodite/
├── __init__.py    — public API, re-exports, register()
├── _core.py       — constants, thresholds, shared state, utilities
├── _inline.py     — zlib fallback compression
├── _marker.py     — CCR marker formatting, proxy compression, parsing
├── _binary.py     — platform detection, binary download
├── _proxy.py      — env loading, health checks, proxy launch
├── _resolve.py    — CCR resolution + recursive unpack
├── _tools.py      — retrieve + compress handlers + schemas
├── _hooks.py      — 6 hooks + 7 tool handlers + conversation memory
└── _engine.py     — ContextEngine for Hermes compression pipeline
```

---

## Bug Status

| Severity | Total | Done | Remaining |
|---|---|---|---|
| 🔴 Critical | 7 | **7** | 0 |
| 🟠 High | 6 | **3** | 3 |
| 🟡 Medium/Low | 64 | **17** | 47 |
| 🟢 Improvement | 6 | **0** | 6 |
| **TOTAL** | **91** | **27** | **61** |

Remaining high bugs: #51 (hash mix), #57 (tool-chain off-by-one), #67 (CcrStore trait)
See `.hermes/MASTER-TASKS.md` for the full audit.

---

## Your Profile

| Setting | Value |
|---|---|
| Profile | `{name}` |
| Provider | `{provider}` |
| Context Engine | `{engine}` |
| Engine Threshold | `{threshold}` |
| Proxy Port | `:{port}` |
| Model | deepseek-v4-pro |
| Auxiliary Model | deepseek-v4-flash |

---

## Testing Protocol

### Single Pass Test
Run exactly ONCE, report numbers:

1. **Proxy health**
   ```bash
   curl -s http://127.0.0.1:{port}/health
   ```
2. **Plugin stats** (if plugin loaded)
   ```
   aphrodite_stats
   ```
3. **Compression test**
   Read a file >10KB and observe if CCR markers appear in tool output.
4. **Report**: proxy status, compression count, CCR markers created, errors.

### Release Test
1. Run `aphrodite_test mode=pipeline`
2. Check `.test-results.json` — verify no regression (delta ≥ 0)
3. Report pass/fail/skip counts

### Multi-Profile Test
Run profiles in parallel via WezTerm panes, one command each:
```bash
hermes --profile aphrodite-barebone -q "read Cargo.toml"
hermes --profile aphrodite-compress-aggressive -q "read Cargo.toml"
```

---

## Key Paths

| Path | Purpose |
|---|---|
| `crates/aphrodite/src/proxy.rs` | Rust proxy (compression, routing, stats) |
| `crates/aphrodite/src/config.rs` | CLI + TOML config parsing |
| `crates/aphrodite/src/retrieve.rs` | CCR retrieval endpoint |
| `crates/aphrodite/src/main.rs` | Proxy server + router |
| `plugins/aphrodite/__init__.py` | Plugin public API |
| `plugins/aphrodite/_engine.py` | ContextEngine (replaces Hermes compressor) |
| `plugins/aphrodite/_hooks.py` | All hook handlers |
| `aphrodite.toml` | Proxy configuration |
| `~/.hermes/config.yaml` | Hermes configuration |
| `.hermes/MASTER-TASKS.md` | Full bug + task audit |
| `.hermes/HANDOFF.md` | Development handoff |
| `scripts/run-headroom-proxy.py` | Headroom proxy launcher |
| `scripts/setup-headroom-providers.py` | Provider setup |

---

## Common Commands

```bash
# Build
cargo build --release -p aphrodite
cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite

# Test
cargo test -p aphrodite

# Lint
cd plugins/aphrodite && ruff check .

# Start headroom proxy
source ~/.privateenvsh
headroom proxy --port 9799 --host 127.0.0.1 \
  --openai-api-url https://api.deepseek.com/v1 \
  --mode token --workers 1 \
  --no-subscription-tracking --no-optimize --no-ccr-marker --no-telemetry &

# Start aphrodite proxy
~/.hermes/aphrodite/aphrodite \
  --listen 127.0.0.1:9798 \
  --api-key "$APHRODITE_API_KEY" \
  --mode token --tool-relay &

# Kill all proxies
pkill -f "headroom proxy"; pkill -f aphrodite
lsof -ti:9797 -ti:9798 -ti:9799 | xargs kill -9

# Run profile
hermes --profile {name}

# Release pipeline
python3 -c "see scripts/ for execution blocks"
```

---

## Working with Skills

All profiles have full copies of development skills:
- `execution-blocks` — parameterized command blocks (build, test, release, proxy)
- `aphrodite-dev-workflow` — development environment + bug fixing
- `aphrodite-iterate-release` — release workflow
- `aphrodite-hook-reference` — hook API reference
- `hermes-plugin-development` — plugin debugging + testing
- `hermes-plugin-authoring` — creating new skills
- `plan` — writing structured plans

Load any skill with `/skill <name>` or `skill_view(name='<name>')`.

---

## Pitfalls

1. **Port conflicts**: always kill old proxies before starting new ones
2. **CCR fragmentation**: headroom with workers>1 has per-process in-memory stores
3. **Config not mid-session**: provider/model changes need Hermes restart
4. **Engine threshold 0%**: means `should_compress()` always returns False
5. **read_file skip list**: file reads are never compressed (essential for agent)
6. **Env passthrough**: `APHRODITE_API_KEY` must be in `terminal.env_passthrough`
7. **Ruff zero errors**: strict linting in CI, fix before committing
"""

profiles = {
    "aphrodite-barebone":       ("deepseek (direct)", "compressor", "off", "N/A"),
    "aphrodite-proxy-cache":    ("aphrodite-cache → :9797", "aphrodite", "50%", "9797"),
    "aphrodite-proxy-token":    ("aphrodite-token → :9798", "aphrodite", "50%", "9798"),
    "aphrodite-compress-off":   ("aphrodite-cache → :9797", "aphrodite", "0%", "9797"),
    "aphrodite-compress-light": ("aphrodite-cache → :9797", "aphrodite", "90%", "9797"),
    "aphrodite-compress-medium":("aphrodite-cache → :9797", "aphrodite", "50%", "9797"),
    "aphrodite-compress-aggressive":("aphrodite-cache → :9797", "aphrodite", "10%", "9797"),
}

for name, (provider, engine, threshold, port) in profiles.items():
    content = SOUL_TEMPLATE.format(name=name, provider=provider, engine=engine, threshold=threshold, port=port)
    path = os.path.join(HOME, ".hermes", "profiles", name, "SOUL.md")
    with open(path, "w") as f:
        f.write(content)

print("All 7 SOUL.md files updated with comprehensive content")
