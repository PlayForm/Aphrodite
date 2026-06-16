---
name: hermes-session-inspection
description: Inspect the state of Hermes sessions and background agents — find running workers, read other sessions' transcripts, check for stuck processes, and use computer_use to diagnose the desktop app.
version: 1.0.0
author: Nikola Hristov + Hermes Agent
license: MIT
platforms: [macos]
metadata:
  hermes:
    tags: [hermes, diagnostics, inspection, sessions, agents, processes, computer-use]
    related_skills: [hermes-agent, wezterm-dev-layout]
---

# Hermes Session Inspection

How to check what Hermes sessions and agents are running, what state they're in, and whether they're stuck.

## When to Use

- User asks "what's running?", "are my agents done?", "is the desktop session still working?"
- You need to read a past session's transcript
- You suspect stuck processes (dashboard at 100% CPU, worker hung)
- You need to inspect the Hermes desktop app's UI from a different session

## Inspection Toolkit (use in this order)

### 1. Find running processes

```bash
ps aux | grep -E 'slash_worker|hermes|gateway|dashboard' | grep -v grep | grep -v 'Helper\|Renderer\|gpu-process'
```

Key processes to identify:
| Process | What it is | Red flags |
|---------|-----------|-----------|
| `slash_worker --session-key <id>` | An active Hermes session | Idle with 0 CPU = session ended but worker still alive |
| `hermes_cli.main dashboard` | Dashboard server | **100% CPU** = stuck in a loop |
| `tui_gateway.entry` | Gateway process | Normal if idle |
| `hermes --tui` | Terminal UI | The current TUI session |

### 2. Check for cron jobs

```python
cronjob(action='list')
```

### 3. Check background processes (within your session)

```python
process(action='list')
```

### 4. Read another session's transcript

```python
session_search(session_id='20260616_042057_107146')
```

Then scroll to the end with `around_message_id` to see the latest state.

### 5. Inspect the desktop app UI

```python
computer_use(action='capture', app='Hermes', mode='ax')
```

Look for:
- `AXButton "Thinking"` — agent is actively processing
- `AXButton "Agents N running"` — delegate_task subagents
- `AXButton "Cron"` — scheduled jobs
- `AXPopUpButton "Nikola's Work Update #2"` — current session name
- `AXPopUpButton "Gateway ready"` — gateway status

### 6. Check WezTerm panes

```python
mcp_wezterm_list_panes()
mcp_wezterm_get_buffer(pane_id=N, lines=80)
```

Look for headroom proxies, cargo watch, or other long-running processes.

## computer_use Pitfall: Hidden Desktop App Windows

When the Hermes desktop app is on a **different macOS Space** (virtual desktop), `computer_use(action='capture', app='Hermes')` returns all AX elements with **bounds `(0, 0, 0, 0)`** — you cannot see the rendered UI visually. SOM screenshots fail with vision errors.

**Workaround**: Use `mode='ax'` (accessibility tree only) and read element **labels** instead of relying on visual positioning. Clicks still work (cua-driver routes input to the app regardless of Space), but you won't see visual feedback.

```python
# This works for clicking, even if bounds are (0,0,0,0):
computer_use(action='click', element=239)  # click "Agents" button

# Use mode='ax' to read labels:
computer_use(action='capture', app='Hermes', mode='ax')
```

Do NOT waste time trying `focus_app(raise_window=true)` to fix this — the bounds issue is a Space-level limitation, not a focus issue.

## Interpreting Agent State

| UI Element | Meaning |
|-----------|---------|
| `"Thinking"` | Agent is actively processing an LLM call |
| `"Thinking 3:06"` | Agent has been thinking for 3 minutes — could be a long response or a hang |
| `"Edited file Nms"` | File edit completed in N milliseconds |
| `"Ran · command Nms"` | Background plan script completed |
| `"Agents 1 running"` | Delegate_task subagents active |
| `"Gateway ready"` | Gateway is connected |

## Stuck Process Recovery

If the **dashboard** is at 100% CPU (normal for active sessions, abnormal if persistent):
```bash
kill <pid>
```

If a **slash_worker** is alive but session has ended:
```bash
kill <pid>  # safe — it's orphaned, no active work
```

Do NOT kill the gateway (`tui_gateway.entry`) or your own TUI session's processes.

## Verification

After diagnosing, confirm with the user:
- "Desktop session 'Nikola's Work Update #2' is [still thinking / idle / done]"
- "X slash_workers alive, Y cron jobs, Z background processes"
- "Dashboard is [normal / stuck at 100% CPU]"
