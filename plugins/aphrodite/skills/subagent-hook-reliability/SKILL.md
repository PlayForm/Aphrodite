---
name: subagent-hook-reliability
description: "Documentation and workarounds for the gap where post_tool_call hooks do not fire in delegate_task subagent sessions."
version: 1.0.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
metadata:
  hermes:
    tags: [hermes, hooks, subagent, delegate_task, reliability]
    related_skills: [hermes-shell-hooks, hermes-agent]
---

# Subagent Hook Reliability

## Problem

`post_tool_call` hooks (e.g., `normalize-dashes.sh`) do NOT fire reliably when
`write_file`/`patch` tools are invoked inside a `delegate_task` subagent.

This means content normalization that depends on `post_tool_call` — Unicode dash
normalization, literal tab fixes, auto-formatting — silently fails on files
written by subagents.

## Evidence

Discovered 2026-06-13. The `normalize-dashes.sh` hook missed 50 em dashes (U+2014)
across 32 documentation files written by subagents during session
`20260613_002525_0127a2`. All 50 occurrences were U+2014, all in
Documentation/GitHub/README.md and Architecture.md files.

- Hook script: works when tested manually (perl regex, jq parsing)
- Hook config: correct in config.yaml (post_tool_call, matcher=write_file|patch|execute_code)
- Parent commit: clean (zero em dashes); child commit: 24 em dashes in Wind alone
- Subagents used: delegate_task with toolsets=["terminal", "file"]

## Suspected Root Causes

1. **Tool dispatch bypass**: subagents dispatch through `agent_runtime_helpers.py`
   which may skip `register_from_config()` for the subagent's tool loop. This means
   hooks registered in the parent's config.yaml never wire up in child sessions.

2. **terminal tool path**: subagents may use shell commands (`cat > file`, `cp`, `sed`)
   via the `terminal` tool, which has no hook at all, instead of `write_file`/`patch`.

## Diagnosis

Add a sentinel to the hook script to detect whether it fires:
```bash
echo "HOOK_FIRED:$(date +%s):$TOOL_NAME:$FILE_PATH" >> /tmp/hook-debug.log
```

Then inspect the log after a delegate_task round.

## Workarounds (in priority order)

### Workaround 1: pre_tool_call modify hooks (BEST)

Convert normalization from a `post_tool_call` (post-hoc perl editing) to a
`pre_tool_call` content-transform modify hook. `pre_tool_call` modify hooks fire
through `_dispatch_pre_tool_call_hooks()` at all 4 dispatch entry points:

1. `model_tools.py::handle_function_call()` — main dispatcher
2. `agent/tool_executor.py` (concurrent tools, ~line 128)
3. `agent/tool_executor.py` (single tool, ~line 508)
4. `agent/agent_runtime_helpers.py` (~line 1501)

Entry point #4 is the path subagents use, so this catches them.

**Before** (post_tool_call, misses subagents):
```yaml
hooks:
  post_tool_call:
    - command: "~/.hermes/agent-hooks/normalize-dashes.sh"
      matcher: "write_file|patch"
      timeout: 5
```

**After** (pre_tool_call modify, catches subagents):
```yaml
hooks:
  pre_tool_call:
    - command: "~/.hermes/agent-hooks/normalize-dashes-pre.sh"
      matcher: "write_file|patch"
      timeout: 5
```

The pre-hook reads `tool_input.content` (for write_file) or `tool_input.new_string`
(for patch), runs perl substitution, and returns:
```json
{"action": "modify", "args": {"content": "<normalized content>"}}
```

This transforms the content BEFORE the file is written, so the file is correct
on first write and no mtime warning is generated.

### Workaround 2: Batch fix from parent

After subagents complete, run a scan and fix from the parent session:

```bash
# Scan for em dashes
python3 dash-normalization-check.py Land/

# Batch fix
find Land/ -name '*.md' -exec perl -i -CSD -pe 's/[\x{2014}]/-/g' {} +
```

### Workaround 3: execute_code with companion hook

Use Python `write_file()` from `hermes_tools` inside subagents. The companion
hook `normalize-dashes-for-execute-code.sh` scans the CWD after code execution.
Note: uses `-maxdepth 1` so subdirectory files are NOT scanned.

## When to load this skill

- When content normalization hooks seem to be working in the parent but files
  written by subagents have un-normalized content
- When diagnosing why em dashes, smart quotes, or other Unicode characters
  appear in files written by delegate_task workers
- When setting up a normalization workflow that must survive subagent delegation