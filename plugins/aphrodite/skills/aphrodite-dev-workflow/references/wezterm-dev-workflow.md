# WezTerm MCP Dev Workflow

Complete WezTerm-based development setup for aphrodite.

## Pane Layout

Pane 0: cargo watch (proxy dev with debug logs)
Pane 3: Hermes test (split below pane 0)

## Commands (send via mcp_wezterm_send_text)

### Start proxy (pane 0)

Prefix cargo watch with the API key and RUST_LOG:
```
APHRODITE_LOG_COMPACT=1 RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

### Split pane

```
wezterm cli split-pane --bottom --pane-id 0
```

### Start Hermes (pane 3)

```
hermes --provider custom:aphrodite-token
```

## MCP Operations

| Action | Tool |
|--------|------|
| List panes | mcp_wezterm_list_panes() |
| Read buffer | mcp_wezterm_get_buffer(pane_id=N, lines=50) |
| Send command | mcp_wezterm_send_text(pane_id=N, text="cmd\\n") |
| Ctrl+C (kill) | terminal(command="pkill -9 -f cargo-watch") | NEVER use mcp_wezterm_send_text for this |

## Kill cargo watch properly
```bash
# NEVER use Ctrl+C via MCP — it doesn't kill cargo watch
pkill -9 -f "cargo-watch\|target.*aphrodite"

# If ports still held:
lsof -ti:9797 -ti:9798 | xargs kill -9
```


- Send Enter after typing into Hermes TUI (text + "\\n")
- Quit Hermes: "/quit\\n" before shell commands
- Stop cargo watch: terminal(command=\"pkill -9 -f 'cargo-watch|target/.*aphrodite'\", timeout=5) — kills everything in one shot
- Buffer output >1KB appears as CCR markers — use aphrodite_retrieve to read
- RUST_LOG=aphrodite=info shows aphrodite logs only (no hyper/rustls noise)
- APHRODITE_LOG_COMPACT=1 enables compact format (no timestamps, no target prefix)
- Debug banner: APHRODITE_DEBUG=1 hermes ... shows full config in TUI via print()

## Catalog Display Fixes
See `references/catalog-display-fixes.md` in aphrodite-iterate-release skill for:
- CCR:{} ghost entries
- |tool|token>>> preview fragments
- Essential tools skip list
- Fake hash filtering from documentation
