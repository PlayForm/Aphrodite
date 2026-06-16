# MCP Verification Rule — User-Corrected Pattern

## The Rule (from user frustration)

**NEVER paste text onto a running process.**

This was the #1 user frustration in the session. When text is sent to a WezTerm pane with a running cargo watch or Hermes session, it pastes into the process's stdin, not the shell. The buffer accumulates garbage and commands don't execute.

## Correct Workflow

### Before ANY mcp_wezterm_send_text:
1. `mcp_wezterm_get_buffer(pane_id, lines=2)` — check for clean shell prompt (`$ ` or `# ` ending)
2. Verify cwd matches expected project dir
3. If a process is running: kill it FIRST, verify buffer shows clean shell
4. Only then send new text

### After sending text:
1. Wait appropriate time (proxy: 6s, Hermes: 3s)
2. `mcp_wezterm_get_buffer(pane_id, lines=4)` — verify command took effect
3. If output is CCR-compressed, retrieve it before acting
4. Only then proceed to next step

### Killing Processes (correct sequence):
```
# Kill cargo watch FIRST (the watcher restarts the process)
pkill -9 -f "cargo-watch"

# Then kill the proxy process
pkill -9 -f "target/.*aphrodite"

# Then kill anything on the ports
lsof -ti:9797 -ti:9798 | xargs kill -9

# Verify ports free
curl -s http://127.0.0.1:9798/health || echo "freed"
```

### NEVER:
- Use `\x03` (Ctrl+C) via send_text — doesn't kill cargo watch
- Send text to a pane without first checking the buffer
- Send text while a process is running (cargo watch, hermes)
- Assume text landed correctly without verifying

### Global Commands:
- Use `terminal` tool for: pkill, lsof, curl health, cargo build, cargo test, gh release
- Use MCP send_text only for: starting/stopping dev processes in panes
- Use MCP get_buffer for: verifying state before and after every action
