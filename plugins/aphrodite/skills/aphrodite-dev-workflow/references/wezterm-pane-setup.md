# WezTerm Multi-Pane Launch Pattern

Launch multiple Hermes profiles simultaneously in separate WezTerm panes
for parallel debugging and comparison testing.

## Pane Creation

```bash
# Create splits from current pane
wezterm cli split-pane --bottom --percent 50   # creates pane N+1
wezterm cli split-pane --right --percent 50    # creates pane N+2
# Repeat to build grid layout
```

## Profile Launch (one per pane)

Use `wezterm cli send-text --no-paste` for interactive Hermes sessions.
MCP's `send_text` won't work for Enter in TUI apps — the `\n` is a literal
newline, not the Enter key binding.

```bash
# Launch profile in pane N
wezterm cli send-text --pane-id N --no-paste $'hermes --profile aphrodite-compress-light\n'
```

## Proxy Start (shared infrastructure)

Start the aphrodite proxy ONCE before launching profiles:

```bash
# Via terminal(background=true)
terminal(command="~/.hermes/aphrodite/aphrodite --listen 127.0.0.1:9798 --api-key $APHRODITE_API_KEY --mode token --tool-relay", background=true)

# Verify
curl -s http://127.0.0.1:9798/health
```

## Full Launch Sequence (7 profiles)

```bash
# 1. Start proxy
# 2. Create 7 panes (split→right→bottom→…)
# 3. Launch one profile per pane with 1s gaps
wezterm cli send-text --pane-id 4 --no-paste $'hermes --profile aphrodite-barebone\n'
sleep 1
wezterm cli send-text --pane-id 5 --no-paste $'hermes --profile aphrodite-proxy-cache\n'
# … etc for all profiles
```

## Verification

```bash
# Confirm all running
ps aux | grep "hermes --profile aphrodite" | grep -v grep

# Read a specific pane's output
wezterm cli get-text --pane-id 5
```

## Cleanup

```bash
# Kill all profile processes
pkill -f "hermes --profile aphrodite"

# Kill proxy
kill <proxy-pid>

# Close extra panes
wezterm cli kill-pane --pane-id 4
# … etc
```

## Pitfalls

- **MCP send_text vs CLI**: `mcp_wezterm_send_text` does NOT trigger Enter
  in TUI apps. Use `wezterm cli send-text --no-paste` via `terminal()`.
- **Pane IDs change on restart**: Re-check with `wezterm cli list` after
  terminal restarts.
- **Keep commands under 200 chars**: Longer text gets garbled in WezTerm.
- **Proxy first**: Start the proxy binary before any profiles that use
  aphrodite-cache or aphrodite-token providers, or they'll fall back to
  direct deepseek.
