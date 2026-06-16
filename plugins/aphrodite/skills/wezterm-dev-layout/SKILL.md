---
name: wezterm-dev-layout
description: Recreate the 3-window WezTerm development layout - proxy (headroom), cargo watch (aphrodite), and dual Hermes instances (barebone + full plugin). Single command to launch the entire environment.
version: 1.1.0
author: Nikola Hristov + Hermes Agent
license: MIT
platforms: [macos]
metadata:
  hermes:
    tags: [wezterm, layout, development, proxy, hermes, cargo-watch]
    related_skills: [aphrodite-dev-workflow, plan-then-delegate]
---

# WezTerm Dev Layout

## Layout Map

```
┌─────────────────────────────────────────────────────┐
│ WINDOW 0                                           │
│ ┌─────────────────────────────────────────────────┐ │
│ │ pane 0: headroom proxy :9799                    │ │
│ │ ./scripts/run-headroom-proxy.py cache           │ │
│ │ or: source ~/.privateenvsh && headroom proxy... │ │
│ └─────────────────────────────────────────────────┘ │
│ CWD: /Volumes/CORSAIR/Developer/macOS/Application/ │
│      PlayForm/HermesCompress                        │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ WINDOW 1                                           │
│ ┌─────────────────────────────────────────────────┐ │
│ │ pane 1: cargo watch - aphrodite Rust proxy      │ │
│ │ cargo watch -x "run -p aphrodite"               │ │
│ │ Outputs: :9797 cache + :9798 token              │ │
│ │ RUST_LOG=aphrodite=info (quiet)                 │ │
│ └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ WINDOW 2                                           │
│ ┌───────────────────┬─────────────────────────────┐ │
│ │ pane 2: barebone  │ pane 3: full Hermes         │ │
│ │ APHRODITE_PASSTHROUGH=1   │ APHRODITE_PASSTHROUGH=0             │ │
│ │ hermes            │ hermes -p aphrodite-proxy-   │ │
│ │ -m deepseek-v4-pro│ cache -m deepseek-v4-pro    │ │
│ │                   │                             │ │
│ │ No plugins        │ Plugin + CCR + hooks        │ │
│ │ Stock tools       │ Proxy :9797/:9798           │ │
│ └───────────────────┴─────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

## Launch Commands

### Quick launch (all 3 windows):

```bash
# Window 0 - headroom proxy
wezterm start --cwd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress -- bash -c "source ~/.privateenvsh && headroom proxy --port 9799 --host 127.0.0.1 --openai-api-url https://api.deepseek.com/v1 --mode token --workers 1 --no-subscription-tracking --no-optimize --no-ccr-marker --no-telemetry" &

# Window 1 - cargo watch (aphrodite)
wezterm start --cwd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress -- bash -c "export PATH=\$HOME/.cargo/bin:\$PATH && source ~/.privateenvsh && RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'" &

# Window 2 - dual Hermes
wezterm start --cwd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress -- bash -c "wezterm cli split-pane --right --percent 50 && sleep 1 && wezterm cli send-text --pane-id 0 --no-paste 'APHRODITE_PASSTHROUGH=1 APHRODITE_DEBUG=0 hermes -m deepseek-v4-pro\n' && wezterm cli send-text --pane-id 1 --no-paste 'APHRODITE_PASSTHROUGH=0 APHRODITE_DEBUG=0 hermes -p aphrodite-proxy-cache -m deepseek-v4-pro\n'" &
```

### From existing WezTerm session (sequential):

```bash
# 1. Kill all
pkill -f "headroom proxy"; pkill -f "cargo watch"; pkill -f aphrodite; lsof -ti:9797:9798:9799 | xargs kill -9; sleep 1

# 2. Window 0: open new window for headroom
wezterm cli spawn --new-window --cwd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress
wezterm cli send-text --pane-id 0 --no-paste 'source ~/.privateenvsh && headroom proxy --port 9799 --host 127.0.0.1 --openai-api-url https://api.deepseek.com/v1 --mode token --workers 1 --no-subscription-tracking --no-optimize --no-ccr-marker --no-telemetry &'$'\n'

# 3. Window 1: open new window for cargo watch
wezterm cli spawn --new-window --cwd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress
wezterm cli send-text --pane-id 1 --no-paste 'export PATH="$HOME/.cargo/bin:$PATH" && source ~/.privateenvsh && RUST_LOG=aphrodite=info cargo watch -x "run -p aphrodite"'$'\n'

# 4. Window 2: open new window, split, launch both Hermes
wezterm cli spawn --new-window --cwd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress
sleep 2
wezterm cli split-pane --right --percent 50
wezterm cli send-text --pane-id 2 --no-paste 'APHRODITE_PASSTHROUGH=1 APHRODITE_DEBUG=0 hermes -m deepseek-v4-pro'$'\n'
wezterm cli send-text --pane-id 3 --no-paste 'APHRODITE_PASSTHROUGH=0 APHRODITE_DEBUG=0 hermes -p aphrodite-proxy-cache -m deepseek-v4-pro'$'\n'
```

## MCP-Based Launch (from Hermes Agent)

With MCP integration, launch panes from within the agent session:

### Full Debug Cargo Watch (single pane)

```python
# Kill old processes, then launch with max debug
terminal(command="pkill -9 -f 'cargo.watch' 2>/dev/null; pkill -9 -f 'target/.*aphrodite' 2>/dev/null; lsof -ti :9797 :9798 | xargs kill -9 2>/dev/null; sleep 1", timeout=5)
mcp_wezterm_send_text(pane_id=N, text="source /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress/.env\n")
mcp_wezterm_send_text(pane_id=N, text="export RUST_LOG=debug APHRODITE_DEBUG=1 APHRODITE_PASSTHROUGH=1 HEADROOM_DEBUG=1\n")
mcp_wezterm_send_text(pane_id=N, text="cd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress\n")
mcp_wezterm_send_text(pane_id=N, text="cargo watch --ignore .git --ignore profiles --ignore .hermes --ignore plugins --ignore scripts --ignore '*.toml.example' --ignore '*.md' -x \"run -p aphrodite\"\n")
```

**CRITICAL - key ordering**: `source .env` BEFORE setting env vars. Without `APHRODITE_API_KEY`, the proxy passes health checks but all API calls return 401 (TLS handshake completes, then CloseNotify). If Key A (APHRODITE_API_KEY) is invalid, switch to the headroom proxy on :9799 which uses Key B (HEADROOM_DEEPSEEK_KEY). Don't escalate - just use the other proxy.

**CRITICAL - `--ignore` flags**: Without them, every git commit, profile edit, or script change triggers a rebuild + restart loop, dropping active Hermes sessions. The flags limit cargo watch to Rust source changes only.

The `cargo watch` auto-rebuilds on Rust source changes. Check build errors via `mcp_wezterm_get_buffer(pane_id=N, lines=20)` - no need to run `cargo check` manually.

## Env Vars Per Pane

| Pane | Service | Key Env Vars |
|---|---|---|
| 0 | headroom proxy | `HEADROOM_DEEPSEEK_KEY` (Key B) |
| 1 | cargo watch | `APHRODITE_API_KEY` (Key A), `RUST_LOG=aphrodite=info` |
| 2 | barebone Hermes | `APHRODITE_PASSTHROUGH=1` (disables plugin routing) |
| 3 | full Hermes | `APHRODITE_PASSTHROUGH=0`, profile `aphrodite-proxy-cache` |

## Quiet Mode

All panes run in quiet mode:
- Headroom: `--no-telemetry --no-subscription-tracking`
- Cargo watch: `RUST_LOG=aphrodite=info` (no debug spam)
- Hermes: config already has telemetry=false, tool_calls=false, costs=false

## Pitfalls

1. **Port conflicts**: Always kill-all before launching
2. **Pane IDs shift**: After creating/deleting panes, verify with `list-panes`
3. **Cargo watch restart loop**: If source changes, cargo watch recompiles - expected
4. **Hermes profile not found**: Verify `~/.hermes/profiles/aphrodite-proxy-cache` symlink resolves
5. **APHRODITE_PASSTHROUGH=1 vs 0**: PASSTHROUGH=1 means plugin loaded but proxy routing skipped (for barebone testing)
