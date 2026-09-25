# Hook Script Patterns and Worked Probes (hermes-shell-hooks)

Full script listings, wire-protocol JSON examples, and worked probes for the
shell-hook events. SKILL.md holds the relations; this file holds the
instances. Each section names the event, the script path under
`~/.hermes/agent-hooks/`, and the probe that shows the hook fired.

## Configuration Field Table

| Field     | Required | Description                                                   |
| --------- | -------- | ------------------------------------------------------------- |
| `event`   | Yes      | Must be a valid hook event (see Available events in SKILL.md) |
| `matcher` | No       | Regex matching `tool_name` - only fires for matching tools    |
| `command` | Yes      | Shell command to execute (supports `~` expansion)             |
| `timeout` | No       | Seconds (default 60, max 300)                                 |

## JSON Protocol: Full stdin/stdout Examples

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

**post_llm_call stdin:**

```json
{
	"user_message": "...",
	"assistant_response": "...",
	"session_id": "...",
	"model": "..."
}
```

**pre_llm_call stdin:**

```json
{
	"prompt": "fix the build system issues",
	"session_id": "...",
	"task_id": "..."
	// other fields may be present
}
```

## Testing Hooks: Exact CLI-Mode Oneshot Commands

Shell hooks fire only inside the agent's own tool loop
(`handle_function_call()` in `model_tools.py`); a CLI-mode oneshot runs that
loop, so it is the test vehicle.

```bash
# Correct - test under ~/.hermes/tmp/:
HERMES_ACCEPT_HOOKS=1 hermes --profile dev-aphrodite -z "Write exactly: hello - world (with em dash) to ~/.hermes/tmp/test-hook.txt"

# WRONG - inside the repo the post-hook rewrites files in-place:
# HERMES_ACCEPT_HOOKS=1 hermes --profile dev-aphrodite -z "Write exactly: hello - world (with em dash) to README.md"
```

TUI-mode CLI equivalents:

```bash
# Works - CLI mode registers hooks
hermes chat -q "Write a file ~/.hermes/tmp/test.txt with content hello"

# Or oneshot mode
HERMES_ACCEPT_HOOKS=1 hermes -z "Write a file ~/.hermes/tmp/test.txt with content hello"
```

Diagnostics:

```bash
hermes hooks list                          # Show configured hooks
hermes hooks doctor                        # Check exec bit, allowlist, mtime
hermes hooks test <event> --for-tool <name> # Fire hooks against synthetic payload
hermes hooks revoke <command>              # Remove allowlist entries
```

## pre_tool_call: Marker Test (test-pre-modify.sh)

A lightweight `pre_tool_call` test hook that prepends a sentinel comment
verifies the pipeline fires end-to-end. Installed at
`~/.hermes/agent-hooks/test-pre-modify.sh`:

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

Probe: write a file via the tool loop and read it back. The written content
starts with `// PRE_MODIFIED_BY_HOOK: test-pre-modify.sh` on first read, with
no mtime warning, because the content was fixed before the write; the marker
is the observable of the `pre_tool_call` → `modify` → merge flow. Remove or
disable this hook before production use.

## pre_tool_call: Pre-Write Normalization (normalize-tabs-pre.sh)

Fixes literal `\t\t` sequences written by `patch` instead of real tabs (JSON
serialization artifact). Installed at
`~/.hermes/agent-hooks/normalize-tabs-pre.sh`:

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

Registration (pre + post pair):

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

Design principle: `pre_tool_call` modify runs before write, so the file is
correct on first write. The `post_tool_call` post-hook stays as a safety net
for files modified outside the agent's tool loop (build pipelines, bundlers,
`cargo` build steps).

## pre_tool_call: Blocking Dangerous Command Patterns

A `pre_tool_call` block hook that stops `write_file`/`patch` on the Aphrodite
runtime home:

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

**Stop if** - the hook blocks a write the current workflow legitimately needs
(e.g. an approved `aphrodite.toml` change during Release phase).

**Recovery** - narrow the guarded path list to the files that must never be
touched by the agent's tool loop; route intended edits through the owning
skill's procedure.

## pre_tool_call: Full Pitfalls

- Modify returns TRANSFORMED INPUT, not a substitute. The returned `args` are
  shallow-merged over the original args; fields not present retain their
  original values. Return only the fields you changed.
- Modify does NOT block execution. A modify response transforms the input and
  then the tool runs normally. For transform AND block, return modify from one
  hook and let a separate block hook handle the security check.
- Block takes precedence over modify. If one hook returns modify and another
  returns block, the tool is blocked - security policy wins over content
  normalization.
- Hook scripts run with full user credentials. Keep scripts in
  `~/.hermes/agent-hooks/` for easy auditing.
- No feedback channel for block. A blocked `pre_tool_call` returns
  `{"decision": "block", ...}` to the hook runner, but the AGENT only sees a
  generic "tool call was blocked" message (not your reason). For actionable
  feedback, use `pre_tool_call` modify + the file mtime change signal instead.
- Recursive self-modification: if you use `write_file` or `patch` to DISABLE
  the test-pre-modify hook itself, the hook fires on its OWN write and the
  marker gets prepended to the disable-content. Disable hooks in config.yaml
  (remove the `pre_tool_call` entry or comment it out) instead of overwriting
  the script through the agent's write_file.

## Internal Architecture: Annotated Dispatch Example

`_dispatch_pre_tool_call_hooks()` returns both results from one
`invoke_hook()` call:

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

## pre_llm_call: prompt-context.py (Worked Hook)

Installed at `~/.hermes/agent-hooks/prompt-context.py`:

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

## post_llm_call: Detached Subprocess and Librarian Patterns

**Spawning a detached subprocess from Python** (`start_new_session=True` is
what detaches the QA agent from the parent):

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

**Librarian prompt pattern** (a `hermes -z` subagent for deep research):

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

**Two-phase decision logic** - spawn the librarian when ALL of:

- Fast scan found >= 2 keyword matches
- Request has an action word (`fix`, `add`, `build`, `debug`, `create`,
  `implement`) OR 4+ substantive keywords OR 2+ technical indicators (api,
  hook, server, config, pipeline)
- Request is > 80 chars
- NOT already cached (fast scan TTL = 30 min, librarian TTL = 24 hours)

The fast scan (< 0.1s) always runs; the librarian (60-120s) only for complex
requests. Results are cached with TTLs: fast scan = 30 min, librarian = 24
hours.

## Full 4-Hook Pipeline (ASCII diagram)

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

CLAIM: these figures are the source's measurements; re-probe with a `time`
wrapper before relying on them.
