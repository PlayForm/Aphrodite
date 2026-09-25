---
name: hermes-shell-hooks
description: "Use when adding, testing, or debugging Aphrodite shell hooks - lifecycle interception scripts fired on tool/LLM events that keep monorepo edits formatted and inject prior-work context."
version: 2.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: hermes
category_taxonomy: hermes/hermes-shell-hooks
date: 2026-09-25
metadata:
    hermes:
        tags: [hermes, wiring, testing, debugging, normalizing, injecting]
        related_skills:
            - hermes-agent
            - hermes-config-maintenance
            - hermes-diagnostics
status: active
---

# Hermes Shell Hooks (Aphrodite)

Drop-in shell scripts that fire on tool/lifecycle events in both CLI and
gateway sessions. Declared in `~/.hermes/config.yaml` under the `hooks:`
block. In Aphrodite development these hooks keep every edit to the monorepo -
plugin source at `plugins/aphrodite`, crates at `crates/aphrodite` and
`crates/aphrodite-hermes` - clean: Unicode dashes normalized, literal `\t`
corruption repaired, and sources formatted with `cargo fmt`/`shfmt`/`prettier`
on every `write_file`/`patch`.

**When to use:** wiring lifecycle automation (auto-format, QA checks, context
injection, guard rails) into Hermes, or debugging why a hook does/doesn't
fire. For launching detached worker agents, see `hermes-background-workers`.
For removing hook entries from config, see `hermes-config-maintenance`.

## Shell hooks vs Aphrodite CCR plugin hooks

Two hook systems coexist in an Aphrodite session - do not confuse them.

| System                          | Where declared                                                                                                                                   | Events                                                                                                                    | Owner                                                  |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Hermes shell hooks (this skill) | `~/.hermes/config.yaml` → `hooks:`                                                                                                               | `pre_tool_call`, `post_tool_call`, `pre_llm_call`, `post_llm_call`, `on_session_start`, `on_session_end`, `subagent_stop` | Scripts in `~/.hermes/agent-hooks/`                    |
| Aphrodite CCR plugin hooks      | Loader at `~/.hermes/plugins/aphrodite/__init__.py` (plugin.yaml + loader only; engine in `crates/aphrodite`, runtime in `~/.hermes/aphrodite/`) | `on_session_start`, `transform_tool_result`, `pre_llm_call`, `transform_terminal_output`, `post_llm_call`                 | `aphrodite-hook-contracts`, `aphrodite-hook-reference` |

The CCR hooks are the compression pipeline (markers, retrieval,
`aphrodite_retrieve`); the shell hooks in this skill are plain scripts for
formatting/QA/context injection. If a shell hook's output - or any file a hook
reads - comes back as a `<<<CCR:hash|type|size>>>` marker, resolve it with
`aphrodite_retrieve(hash)` before acting; never re-read the source file behind
a marker you already hold.

## Configuration

```yaml
# ~/.hermes/config.yaml
hooks:
    post_tool_call:
        - matcher: "write_file|patch"
          command: "~/.hermes/agent-hooks/my-hook.sh"
          timeout: 30
hooks_auto_accept: true
```

| Field     | Required | Description                                                |
| --------- | -------- | ---------------------------------------------------------- |
| `event`   | Yes      | Must be a valid hook event (see Plugin Hooks list below)   |
| `matcher` | No       | Regex matching `tool_name` - only fires for matching tools |
| `command` | Yes      | Shell command to execute (supports `~` expansion)          |
| `timeout` | No       | Seconds (default 60, max 300)                              |

Available events: `pre_tool_call`, `post_tool_call`, `pre_llm_call`, `post_llm_call`, `on_session_start`, `on_session_end`, `on_session_finalize`, `on_session_reset`, `subagent_stop`.

## Consent Model

- First-run prompts for approval per `(event, command)` pair
- Persisted to `~/.hermes/shell-hooks-allowlist.json`
- Three bypass methods (any one suffices):
    1. `hermes --accept-hooks chat` (top-level flag, NOT `hermes chat --accept-hooks`)
    2. `HERMES_ACCEPT_HOOKS=1` env var
    3. `hooks_auto_accept: true` in config.yaml

**Pitfall:** `--accept-hooks` is a **top-level flag** on the `hermes` command. It does NOT work on subcommands:

- CORRECT: `hermes --accept-hooks hooks test post_tool_call`
- WRONG: `hermes hooks test post_tool_call --accept-hooks`

## Allowlist Format

```json
{
	"approvals": [{ "event": "post_tool_call", "command": "~/.hermes/agent-hooks/hook.sh" }]
}
```

## JSON Protocol

**stdin** (received by script):

```json
{
	"hook_event_name": "post_tool_call",
	"tool_name": "write_file",
	"tool_input": { "path": "/path/to/file", "content": "..." },
	"session_id": "sess_abc123",
	"cwd": "/current/working/dir",
	"extra": { "task_id": "...", "tool_call_id": "..." }
}
```

**stdout** (optional response):

```json
// Block a tool call (pre_tool_call):
{"decision": "block", "reason": "Forbidden operation"}
{"action": "block", "message": "Forbidden operation"}   # also accepted

// Modify tool input before dispatch (pre_tool_call):
{"action": "modify", "args": {"new_string": "fixed content"}}
{"decision": "modify", "tool_input": {"new_string": "fixed content"}}  # Claude-Code-style

// Inject context (pre_llm_call only):
{"context": "Additional context here"}

// Silent no-op:
{}
```

## Porting from Claude Code

**Critical difference:** Field names in `tool_input`:

- Hermes: `tool_input.path` for `write_file` and `patch`
- Claude: `tool_input.file_path` for equivalent operations

```bash
# Hermes (correct):
FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.path // empty')

# Claude (won't work on Hermes):
FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // empty')
```

## Testing Hooks

Shell hooks fire **only inside the agent's own tool loop** (`handle_function_call()` in `model_tools.py`). Calling `write_file` via your own tool calls (as the current agent session) does NOT trigger hooks - you are not running through the tool dispatch pipeline.

**Never test hooks inside the Aphrodite repo working tree.** The `post_tool_call` normalize-dashes hook modifies files in-place, creating mtime-change warnings and touching files you didn't intend to edit. Always test under `~/.hermes/` only (scratch lives in `~/.hermes/tmp/`, never `/tmp`):

```bash
# Correct - test under ~/.hermes/tmp/:
HERMES_ACCEPT_HOOKS=1 hermes --profile dev-aphrodite -z "Write exactly: hello - world (with em dash) to ~/.hermes/tmp/test-hook.txt"

# WRONG - inside the repo the post-hook rewrites files in-place:
# HERMES_ACCEPT_HOOKS=1 hermes --profile dev-aphrodite -z "Write exactly: hello - world (with em dash) to README.md"
```

**Stop if** - a hook test wrote into the Aphrodite repo working tree or `/tmp`. Scratch belongs under `~/.hermes/tmp/`.

**Recovery** - move the artifact to `~/.hermes/tmp/`; re-test only under `~/.hermes/`. Never `git reset`/checkout to erase probe artifacts.

The `-z` flag is a **top-level flag** on `hermes`, NOT a subcommand argument:

- CORRECT: `hermes -z "prompt..." --accept-hooks`
- WRONG: `hermes chat -z "prompt..."`

**Observable proof the hook fired (pre_tool_call modify):** the file content shows the modification (e.g., a marker comment prepended) on first read - no mtime warning because the content was fixed before the write.

**Observable proof the hook fired (post_tool_call):** the `write_file` tool detects the file mtime changed between the write and the return and emits: "file was modified since you last read it on disk (external edit or unrecorded writer). Re-read the file before writing." That warning _is_ the post-hook signature.

### Development Hook: pre_tool_call Marker Test

A lightweight `pre_tool_call` test hook that prepends a sentinel comment verifies the pipeline fires end-to-end:

```bash
#!/bin/bash
# ~/.hermes/agent-hooks/test-pre-modify.sh
INPUT=$(cat)
TOOL_NAME=$(printf '%s' "$INPUT" | jq -r '.tool_name')

case "$TOOL_NAME" in
	write_file | patch)
		CONTENT=$(printf '%s' "$INPUT" | jq -r '.tool_input.content // ""')
		NEW_STRING=$(printf '%s' "$INPUT" | jq -r '.tool_input.new_string // ""')

		if [ -n "$CONTENT" ]; then
			MODIFIED="// PRE_MODIFIED_BY_HOOK: test-pre-modify.sh
${CONTENT}"
			jq -n --arg modified "$MODIFIED" \
				'{"action": "modify", "args": {"content": $modified}}'
		elif [ -n "$NEW_STRING" ]; then
			MODIFIED="// PRE_MODIFIED_BY_HOOK: test-pre-modify.sh
${NEW_STRING}"
			jq -n --arg modified "$MODIFIED" \
				'{"action": "modify", "args": {"new_string": $modified}}'
		else
			echo '{}'
		fi
		;;
	*)
		echo '{}'
		;;
esac
```

The result is a marker injected before the tool writes, verifying the whole pre_tool_call → modify → merge flow works end-to-end. Remove or disable this hook before production use.

**Pitfall - recursive self-modification:** If you use `write_file` or `patch` to DISABLE the test-pre-modify hook itself, the hook fires on its OWN write and the marker gets prepended to the disable-content. Disable hooks in config.yaml (remove the `pre_tool_call` entry or comment it out) instead of overwriting the script through the agent's write_file.

```bash
hermes hooks list                          # Show configured hooks
hermes hooks doctor                        # Check exec bit, allowlist, mtime
hermes hooks test <event> --for-tool <name> # Fire hooks against synthetic payload
hermes hooks revoke <command>              # Remove allowlist entries
```

## pre_tool_call: Pre-Execution Interception & Content Transformation

The `pre_tool_call` event fires BEFORE a tool executes. It supports two response types:

**Blocking** - stop a dangerous operation:

```json
{"decision": "block", "reason": "Forbidden operation"}
{"action": "block", "message": "Forbidden operation"}   # also accepted
```

**Content transformation** - modify tool_input before the tool runs:

```json
{"action": "modify", "args": {"new_string": "fixed content"}}
{"decision": "modify", "tool_input": {"new_string": "fixed content"}}  # Claude-Code-style
```

When a hook returns a `modify` response, Hermes shallow-merges the returned arguments into the tool's original args before dispatching. Hooks only need to return the fields they changed - unchanged fields pass through.

### Pattern: Pre-Write Normalization (e.g., literal `\t` → real tab)

The `patch` tool sometimes writes literal `\t\t` sequences instead of real tabs due to JSON serialization. A `pre_tool_call` modify hook fixes this before the file is written - eliminating the mtime-warning cycle that `post_tool_call` workarounds produce - so Aphrodite crate and plugin sources never reach `cargo fmt` with tab corruption.

```bash
#!/bin/bash
# ~/.hermes/agent-hooks/normalize-tabs-pre.sh
INPUT=$(cat)
TOOL_NAME=$(printf '%s' "$INPUT" | jq -r '.tool_name // empty')
case "$TOOL_NAME" in
	patch | write_file)
		NEW_STRING=$(printf '%s' "$INPUT" | jq -r '.tool_input.new_string // empty')
		CONTENT=$(printf '%s' "$INPUT" | jq -r '.tool_input.content // empty')
		PATH_FIELD=$(printf '%s' "$INPUT" | jq -r '.tool_input.path // empty')
		OLD_STRING=$(printf '%s' "$INPUT" | jq -r '.tool_input.old_string // empty')

		# Fix literal \t at line-start positions → real tab
		FIXED_NEW=$(printf '%s' "$NEW_STRING" | perl -pe 's/\\t/\t/g')
		FIXED_CONTENT=$(printf '%s' "$CONTENT" | perl -pe 's/\\t/\t/g')

		jq -n --arg tool "$TOOL_NAME" --arg new "$FIXED_NEW" \
			--arg cont "$FIXED_CONTENT" --arg old "$OLD_STRING" \
			--arg path "$PATH_FIELD" \
			'{
            "action": "modify",
            "args": (if $tool == "patch" then
                        {"new_string": $new, "old_string": $old, "path": $path}
                    else
                        {"content": $cont, "path": $path}
                    end)
        }'
		;;
	*)
		echo '{}' # passthrough
		;;
esac
```

Register as a `pre_tool_call` hook:

```yaml
hooks:
    pre_tool_call:
        - command: "~/.hermes/agent-hooks/normalize-tabs-pre.sh"
          matcher: "write_file|patch"
          timeout: 5
    post_tool_call:
        # Keep the post-hook as fallback for build-pipeline files
        - command: "~/.hermes/agent-hooks/normalize-tabs.sh"
          matcher: "write_file|patch"
          timeout: 5
```

**Design principle:** `pre_tool_call` modify runs before write, so the file is correct on first write. The `post_tool_call` post-hook stays as a safety net for files modified outside the agent's tool loop (build pipelines, bundlers, `cargo` build steps). Returning `modify` DOES NOT skip tool execution - it transforms the input. If you need to both transform AND guard, return modify first, then let the tool run.

### Pattern: Blocking Dangerous Command Patterns

```bash
# pre_tool_call hook that blocks write_file/patch on the Aphrodite runtime home
INPUT=$(cat)
TOOL_NAME=$(printf '%s' "$INPUT" | jq -r '.tool_name // empty')
TOOL_INPUT=$(printf '%s' "$INPUT" | jq -r '.tool_input // empty' | tr '[:upper:]' '[:lower:]')

# Block writes into ~/.hermes/aphrodite/ (runtime home: aphrodite.toml, binaries/, hotreload/)
if [ "$TOOL_NAME" = "write_file" ] || [ "$TOOL_NAME" = "patch" ]; then
	FILE_PATH=$(printf '%s' "$TOOL_INPUT" | jq -r '.tool_input.path // empty')
	case "$FILE_PATH" in
		*/.hermes/aphrodite/*)
			echo '{"decision": "block", "reason": "Write into Aphrodite runtime home blocked - edit via the approved workflow"}'
			exit 0
			;;
	esac
fi

# Passthrough
echo '{}'
```

**Stop if** - a hook blocks a write the current workflow legitimately needs (e.g. an approved `aphrodite.toml` change during Release phase).

**Recovery** - narrow the guarded path list to the files that must never be touched by the agent's tool loop; route intended edits through the owning skill's procedure.

### Pitfalls: pre_tool_call Hooks

- **Modify returns TRANSFORMED INPUT, not a substitute.** The returned `args` are shallow-merged over the original args; fields not present retain their original values. Return only the fields you changed.
- **Modify does NOT block execution.** A modify response transforms the input and then the tool runs normally. For transform AND block, return modify from one hook and let a separate block hook handle the security check.
- **Block takes precedence over modify.** If one hook returns modify and another returns block, the tool is blocked - security policy wins over content normalization.
- **Hook scripts run with full user credentials.** Keep scripts in `~/.hermes/agent-hooks/` for easy auditing.
- **No feedback channel for block.** A blocked `pre_tool_call` returns `{"decision": "block", ...}` to the hook runner, but the AGENT only sees a generic "tool call was blocked" message (not your reason). For actionable feedback, use `pre_tool_call` modify + the file mtime change signal instead.

### Internal Architecture: Single Hook Invocation

The `pre_tool_call` event fires exactly **once** per tool execution - not once for block checks and again for modify checks. This is critical because shell hooks have side effects (e.g., a `normalize-dashes.sh` hook that modifies file content in-place). If the hook fired twice, it would run twice on the same tool call.

The single invocation is handled by `_dispatch_pre_tool_call_hooks()` in `hermes_cli/plugins.py`, which returns both results from one `invoke_hook()` call:

```python
block_message, modified_args = _dispatch_pre_tool_call_hooks(
    tool_name="write_file",
    args={"content": "original", "path": "~/.hermes/tmp/file.txt"},
    task_id="...", session_id="...", tool_call_id="...",
)

# block_message == None (no block)
# modified_args == {"content": "modified", "path": "~/.hermes/tmp/file.txt"}
#   (shallow merge of hook's returned args over originals)
```

**The call sites** (all 4 internal entry points) use `_dispatch_pre_tool_call_hooks`:

1. `model_tools.py::handle_function_call()` - the main tool dispatcher
2. `agent/tool_executor.py` (concurrent tools path)
3. `agent/tool_executor.py` (single tool path)
4. `agent/agent_runtime_helpers.py`

**Backward compat:** `get_pre_tool_call_block_message()` still works for plugin callers - it delegates to `_dispatch_pre_tool_call_hooks()` internally.

## post_tool_call: Post-Edit Formatting

The `post_tool_call` event fires after every tool execution. A hook on this event with a `matcher` gate can auto-format edited files after `write_file` or `patch`.

### Pattern: Auto-Format After File Writes

The Aphrodite repo formatting gates use `cargo fmt` for the Rust crates, `shfmt` for shell scripts, `prettier` for TOML/JSON/Markdown, and `black`/`ruff` for Python. Adapted as a hook, the pattern fires automatically after every file edit, keeping the monorepo formatted without the agent needing to remember a separate formatting-gate run.

```yaml
# ~/.hermes/config.yaml
hooks:
    post_tool_call:
        # 1. Normalize Unicode dashes to ASCII (normalize-dashes.sh)
        - command: "~/.hermes/agent-hooks/normalize-dashes.sh"
          matcher: "write_file|patch"
          timeout: 15
        # 2. Fix literal \t → real tab corruption from patch tool (normalize-tabs.sh)
        - command: "~/.hermes/agent-hooks/normalize-tabs.sh"
          matcher: "write_file|patch"
          timeout: 15
        # 3. Format + QA after edits (post-edit-format-qa.py)
        - command: "~/.hermes/agent-hooks/post-edit-format-qa.py"
          matcher: "write_file|patch"
          timeout: 30
hooks_auto_accept: true
```

**Hook implementation (`post-edit-format-qa.py`):**

- Extracts file path from `tool_input.path`
- Detects language from extension
- Routes to appropriate formatter:
    - `.rs` → `cargo fmt` (locates the nearest Cargo.toml in parent dirs - finds `crates/aphrodite/Cargo.toml` or `crates/aphrodite-hermes/Cargo.toml`)
    - `.sh` → `shfmt`
    - `.js/.ts/.css/.json/.md/.yaml/.toml` → `prettier`
    - `.py` → `black` or `ruff`
- Skips `node_modules`, `target/`, `vendor/`, `.git/`

**Stop if** - a hook formatted a file the agent did not edit (matcher too broad, or a build pipeline wrote into the repo tree).

**Recovery** - tighten the `matcher` regex; keep the post-hook as a fallback only for files produced outside the tool loop.

## pre_llm_call Context Injection

The `pre_llm_call` event fires once per turn, before the model generates a response. Hooks for this event can return `{"context": "..."}` which Hermes injects into the user message - making it ideal for RAG, memory plugins, or prior-work discovery. In an Aphrodite session this surfaces prior CCR work, hook contracts, and repo skills (`.hermes/skills/`, Development branch) as context.

```javascript
// stdin payload (pre_llm_call):
{
  "prompt": "fix the build system issues",
  "session_id": "...",
  "task_id": "...",
  // other fields may be present
}
```

**Implementation approach:**

1. Read prompt from stdin via `sys.stdin.read()` (Python) or `cat` (shell)
2. Extract keywords (filter stop words, keep 3+ char words)
3. Search relevant locations:
    - `~/.hermes/sessions/session_*.json` - look for keyword matches in user messages
    - `<workspace>/.hermes/skills/**/SKILL.md` - the Aphrodite repo's dev skills
    - `~/.hermes/memory` - grep for keyword entries
    - `~/.hermes/*.md` - audit reports, reference files, knowledge bases
4. Return `{"context": "PRIOR CONTEXT:\n...\n"}` on success, or exit silently if nothing found

**Example Python hook (`~/.hermes/agent-hooks/prompt-context.py`):**

```python
#!/usr/bin/env python3
import json, sys, re, os
from pathlib import Path

HERMES_HOME = Path(os.path.expanduser("~/.hermes"))

raw = sys.stdin.read()
payload = json.loads(raw)
prompt = payload.get("prompt", "")
if not prompt: sys.exit(0)

# Extract keywords
words = [w for w in re.findall(r'[a-z]{4,}', prompt.lower())
         if w not in {'the', 'and', 'for', 'you', 'this', 'that', 'with', 'from',
                      'have', 'not', 'but', 'what', 'just', 'will', 'would'}]
if len(words) < 2: sys.exit(0)

# Search sessions (most recent first, check user messages)
context_parts = []
sessions = sorted(HERMES_HOME.glob("sessions/session_*.json"),
                  key=lambda p: p.stat().st_mtime, reverse=True)[:25]
for sf in sessions:
    try:
        data = json.loads(sf.read_text(errors='ignore'))
        user_content = " ".join(m["content"] for m in data["messages"]
                                if m.get("role") == "user").lower()
        hits = [w for w in words if w in user_content]
        if len(hits) >= 2:
            title = data.get("title", "") or data["messages"][0]["content"][:100]
            context_parts.append(f"  Session: {title} (matched: {', '.join(hits)})")
    except: continue

# Search skills
for skill in HERMES_HOME.rglob("skills/**/SKILL.md"):
    try:
        content = skill.read_text(errors='ignore').lower()
        hits = [w for w in words if w in content]
        if len(hits) >= 2:
            name = skill.parent.name
            context_parts.append(f"  Skill: {name} (matched: {', '.join(hits)})")
    except: continue

if context_parts:
    ctx = "\n".join(context_parts[:6])
    msg = f"PRIOR CONTEXT FROM .HERMES:\n{ctx}"
    print(json.dumps({"context": msg}))

# Exit silently if nothing found
sys.exit(0)
```

## post_llm_call: Background QA Agent

The `post_llm_call` event fires once per turn after the model finishes its response (successful turns only). This is the right event for self-checks that inform the NEXT turn's context - used during Aphrodite Validate-phase work, where the detached QA agent's findings are cached and injected on the following turn.

```json
// stdin payload (post_llm_call):
{
	"user_message": "...",
	"assistant_response": "...",
	"session_id": "...",
	"model": "..."
}
```

**Key design principle:** post_llm_call runs SYNCHRONOUSLY in the hook pipeline, but any QA agent it spawns must be detached (`nohup` / `start_new_session=True`). The hook then returns in milliseconds while the QA agent runs in the background; results appear on the subsequent turn via a cache file read by the pre_llm_call hook.

**Spawning a detached subprocess from Python:**

```python
subprocess.Popen(
    [sys.executable, "path/to/background-qa-agent.py"],
    stdin=subprocess.PIPE,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    start_new_session=True,  # Critical - detaches from parent
    cwd=str(HERMES_HOME),
).stdin.write(json.dumps(payload).encode())
```

### Pattern: Librarian Agent for Deep Research

The librarian is a `hermes -z` subagent spawned via `subprocess.run()` with a timeout. It searches `.hermes` semantically (not just keyword grep) and returns synthesized context. Used in a two-phase pattern:

1. Phase 1: Fast keyword scan (< 0.1s) - always runs
2. Phase 2: Librarian agent (60-120s) - only for complex requests with multiple technical keywords, action words, or > 80 chars
3. Results cached with TTLs: fast scan = 30 min, librarian = 24 hours

**When to spawn librarian:**

- Request contains action words: `fix`, `add`, `build`, `debug`, `create`, `implement`, etc.
- Request has 4+ substantive keywords
- Request has 2+ technical indicators (api, hook, server, config, pipeline, etc.)
- Request is > 80 characters

**Librarian prompt pattern:**

```python
prompt = f"""You are a librarian agent. Search {HERMES_HOME}/ for prior work related to:
"{user_prompt}"

Search sessions, skills, memory, and reference files. Return a
concise briefing with specific dates, filenames, and decisions."""

subprocess.run(
    ["hermes", "-z", prompt],
    capture_output=True, text=True,
    timeout=120,  # 2 min hard limit
    env={**os.environ, "HERMES_ACCEPT_HOOKS": "0"},  # Prevent recursion
    cwd=str(HERMES_HOME)
)
```

## Full 4-Hook Pipeline

A complete setup with all four events creates an auto-improving cycle:

```
                    pre_llm_call
  ┌───────────────────────────────────┐
  │  prompt-context.py                │
  │  1. Fast keyword scan (.hermes)   │  ← Injects context into user message
  │  2. QA cache check                │
  │  3. Optional librarian (complex)  │
  └───────────────┬───────────────────┘
                  │
  ┌───────────────▼───────────────────┐
  │         LLM generates response    │
  └───────────────┬───────────────────┘
                  │                    post_llm_call
  ┌───────────────▼───────────────────┐
  │  self-qa-check.py                 │
  │  1. Fast AI-tell check (< 5ms)   │  ← Spawns detached QA agent
  │  2. Spawn background QA agent     │  ← Results cached for NEXT turn
  └───────────────┬───────────────────┘
                  │
  ┌───────────────▼───────────────────┐
  │  Background QA (flash-tier model) │
  │  Deep analysis → cache → done     │
  └───────────────────────────────────┘

  After every write_file/patch: post_tool_call
  ┌───────────────────────────────────┐
  │  1. normalize-dashes.sh           │  ← Unicode dashes → ASCII
  │  2. normalize-tabs.sh             │  ← Literal \t → real tab
  │  3. post-edit-format-qa.py        │  ← Format + QA edited files
  │     cargo fmt / shfmt / prettier  │
  └───────────────────────────────────┘
```

## Known Limitation: TUI Mode Hooks

The Hermes TUI gateway (`tui_gateway.entry` / `tui_gateway.slash_worker`) does **not** call `register_from_config()`, so **no shell hooks fire in TUI mode** - neither `block`, `modify`, nor observer hooks. This is a pre-existing architectural gap.

Hooks work in:

- **CLI mode** (`hermes chat`, `hermes -z "..."`) - `register_from_config` is called by `hermes_cli/main.py` at startup
- **Gateway mode** (`hermes gateway run`) - `register_from_config` is called by `gateway/run.py` at startup

**To test a hook:** use CLI mode, not TUI:

```bash
# Works - CLI mode registers hooks
hermes chat -q "Write a file ~/.hermes/tmp/test.txt with content hello"

# Or oneshot mode
HERMES_ACCEPT_HOOKS=1 hermes -z "Write a file ~/.hermes/tmp/test.txt with content hello"
```

**Diagnosing whether hooks are firing** - add a sentinel to the hook script and check after a tool call:

```bash
# In your hook script, add:
# echo "$(date +%s) hook fired for $TOOL_NAME" >> ~/.hermes/tmp/hook-debug.log

# Then check after a tool call:
cat ~/.hermes/tmp/hook-debug.log
```

If the sentinel appears after using CLI mode but not TUI mode, the hooks are working - the gap is in the TUI.

## Performance Characteristics

All hooks below are pure-local (no LLM spawns on the hot path):

| Hook                                    | Avg Time  | What It Does                                                 |
| --------------------------------------- | --------- | ------------------------------------------------------------ |
| pre_tool_call (normalize-tabs-pre.sh)   | 5-10ms    | Literal `\t` → real tab BEFORE write (avoids mtime warnings) |
| pre_llm_call (prompt-context.py)        | 10-30ms   | Session name grep + skills snapshot + memory scan            |
| post_tool_call (normalize-dashes.sh)    | 5-10ms    | Unicode dash → ASCII replacement                             |
| post_tool_call (normalize-tabs.sh)      | 5-10ms    | Literal `\t` → real tab (fixes patch tool corruption)        |
| post_tool_call (post-edit-format-qa.py) | 100-500ms | Local formatter (black/prettier/shfmt/cargo fmt) -- no LLM   |
| post_llm_call (self-qa-check.py)        | 2-5ms     | Regex-only AI-tell check, zero network calls                 |

**Do not spawn `hermes -z` LLM subagents inside the hook pipeline.** The hot path must stay regex checks + local formatters. If deep QA analysis is needed, run it manually via `delegate_task` or a one-shot `hermes -z` on demand - not as a hook.

**Subagent hook behavior:** Each subagent gets its own `AIAgent` instance, so `pre_llm_call`/`post_tool_call` hooks fire **inside the subagent independently** of the parent. The parent receives `subagent_stop` once per child completion. Hooks registered on the parent do NOT automatically apply to subagents - each process has its own hook configuration loaded from `config.yaml` at startup. Aphrodite benchmarking frequently runs parallel delegates (crate audits, compression probes) - each child fires its own hooks, so keep the hot path cheap.

## Security

- Scripts run with full user credentials - keep scripts in `~/.hermes/agent-hooks/` for easy auditing
- Re-run `hermes hooks doctor` after pulling shared configs (validates exec bit, allowlist, mtime)
- Review the `hooks:` section in config changes like CI config changes

## Files & Cross-References

- `scripts/normalize-tabs.sh`, `scripts/normalize-dashes.sh`, `scripts/prompt-context.py`, `scripts/post-edit-format-qa.py` - reference implementations
- `templates/hermes-oneshot-wrapper.py` - oneshot wrapper used by `hermes-background-workers` for fire-and-forget workers (detached QA agents in the Aphrodite post_llm_call pattern)
- `references/hooks-pipeline-architecture.md` - deep pipeline internals
- `references/live-install-workflow.md` - developing hook changes against the live Aphrodite plugin source with dylib hot-reload
- `references/tui-architecture.md` - TUI internals (agent grid overlay, status glyphs, widget wiring)
- Spawning fire-and-forget worker agents (venv python, temp-file prompts, `session_db=None`): see `hermes-background-workers`
- Editing/removing hooks from `config.yaml` (write guard, `hermes config set/unset`): see `hermes-config-maintenance`
- CCR plugin hook contracts (the compression pipeline's own hook events): see `aphrodite-hook-contracts` / `aphrodite-hook-reference`

## Local claim-to-test matrix

| Claim                                           | Evidence source        | Test                                                     | Pass condition                            | Failure response                                   |
| ----------------------------------------------- | ---------------------- | -------------------------------------------------------- | ----------------------------------------- | -------------------------------------------------- |
| post_tool_call hooks fire only in the tool loop | SKILL.md               | `hermes hooks test post_tool_call --for-tool write_file` | Hook output appears                       | Check exec bit + allowlist (`hermes hooks doctor`) |
| Hooks do not fire in TUI mode                   | This skill             | Start TUI, run a write_file                              | No post-hook mtime warning appears        | Expected - test in CLI mode instead                |
| Dash/tab normalization keeps files clean        | `git status --short`   | Write a file with an em dash under `~/.hermes/tmp/`      | On-disk file contains ASCII hyphen only   | Re-check registration/allowlist                    |
| `cargo fmt` routing finds crate manifests       | post-edit-format-qa.py | Edit a `.rs` file under `crates/`                        | Format result reports `cargo fmt` success | Check the nearest-Cargo.toml walk                  |
| Hook tests never touch the repo tree            | `git status --short`   | Run the `~/.hermes/tmp/` test                            | No repo file modified                     | Move the test target under `~/.hermes/tmp/`        |
