# WezTerm Reset Workflow

## NEVER paste text onto a running process

This is the #1 user frustration. Always: kill → verify clean → send.

## Global Commands vs MCP

- **Use `terminal` tool**: pkill, lsof, curl health, cargo build, cargo test, gh release
- **Use MCP send_text**: Only for starting/stopping dev processes in panes (cargo watch, hermes)
- **Use MCP get_buffer**: Verify state before AND after every send_text

## Step-by-Step Reset

### 1. Kill everything (terminal tool)
```bash
pkill -9 -f "cargo-watch" 2>/dev/null
pkill -9 -f "target/.*aphrodite" 2>/dev/null
lsof -ti:9797 -ti:9798 2>/dev/null | xargs kill -9 2>/dev/null
sleep 1
```

### 2. Verify ports free (terminal tool)
```bash
curl -s http://127.0.0.1:9798/health 2>/dev/null || echo "ports free"
```
Must show "ports free" or no output. If it returns JSON, repeat step 1.

### 3. Verify panes clean (MCP)
```
mcp_wezterm_get_buffer(pane_id, lines=2)
```
Must show shell prompt (`$ ` or `# ` ending with `$`) and correct cwd.

### 4. Start proxy (MCP send to pane 0)
```
APHRODITE_LOG_COMPACT=1 RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

### 5. Wait + verify proxy
```
sleep 8
curl -s http://127.0.0.1:9798/health  # must return {"status":"healthy","version":"X.Y.Z"}
```

### 6. Start Hermes (MCP send to pane 1)
```
APHRODITE_DEBUG=1 hermes --provider custom:aphrodite-token
```

### 7. Verify Hermes
```
mcp_wezterm_get_buffer(pane_id=1, lines=10)
# Must show Hermes banner or debug lines
```

## Common Failures

| Symptom | Cause | Fix |
|---------|-------|-----|
| Address already in use (os error 48) | Old proxy still running | `lsof -ti:9797 -ti:9798 \| xargs kill -9` |
| \x03 in buffer | Ctrl+C sent via MCP | Never use \x03. Use pkill -9 only |
| /quit: no such file | Hermes not running | Hermes already exited, just restart |
| Text pasted into running process | Sent text to pane with active cargo watch | Kill first, verify buffer, then send |

## CRITICAL: Verify after every action

After EVERY mcp_wezterm_send_text, call mcp_wezterm_get_buffer to confirm. Type, look, verify — like a human.
