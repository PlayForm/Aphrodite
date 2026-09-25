---
name: hermes-shell-hooks
description: "Use when adding, testing, or debugging Aphrodite shell hooks - lifecycle interception scripts fired on tool/LLM events that keep monorepo edits formatted and inject prior-work context."
version: 2.2.0
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
scope:
    repositories: [PlayForm/Aphrodite]
owns:
    - The `hooks:` wiring contract in ~/.hermes/config.yaml (documentation only; config edits go through hermes-config-maintenance)
    - The hook-script patterns and worked probes (references/hook-script-patterns.md)
depends_on:
    - hermes-config-maintenance
    - hermes-background-workers
    - aphrodite-hook-contracts
    - aphrodite-hook-reference
supersedes: []
verification:
    source_of_truth: "~/.hermes/config.yaml `hooks:` block; `hermes hooks list` / `hermes hooks doctor` output"
mutation_level: local
---

# Hermes Shell Hooks (Aphrodite)

Drop-in shell scripts declared in `~/.hermes/config.yaml` under the `hooks:`
block, fired on tool/lifecycle events in CLI and gateway sessions. In
Aphrodite development they keep monorepo edits (plugin source
`plugins/aphrodite`, crates `crates/aphrodite` + `crates/aphrodite-hermes`)
clean: Unicode dashes normalized, literal `\t` corruption repaired, sources
formatted with `cargo fmt`/`shfmt`/`prettier` on every `write_file`/`patch`.
Use when wiring lifecycle automation (auto-format, QA checks, context
injection, guard rails) or debugging why a hook does/doesn't fire. Detached
workers: `hermes-background-workers`; config hook removal:
`hermes-config-maintenance`.

## Shell hooks vs Aphrodite CCR plugin hooks

Two hook systems coexist in an Aphrodite session; a shell hook is not a CCR
plugin hook.

| System | Where declared | Events | Owner |
| --- | --- | --- | --- |
| Hermes shell hooks (this skill) | `~/.hermes/config.yaml` → `hooks:` | `pre_tool_call`, `post_tool_call`, `pre_llm_call`, `post_llm_call`, `on_session_start`, `on_session_end`, `subagent_stop` | Scripts in `~/.hermes/agent-hooks/` |
| Aphrodite CCR plugin hooks | Loader at `~/.hermes/plugins/aphrodite/__init__.py` (plugin.yaml + loader only; engine in `crates/aphrodite`, runtime in `~/.hermes/aphrodite/`) | `on_session_start`, `transform_tool_result`, `pre_llm_call`, `transform_terminal_output`, `post_llm_call` | `aphrodite-hook-contracts`, `aphrodite-hook-reference` |

The CCR hooks are the compression pipeline (markers, retrieval,
`aphrodite_retrieve`); the shell hooks here are plain scripts for
formatting/QA/context injection. If a shell hook's output - or any file a hook
reads - comes back as a `<<<CCR:hash|type|size>>>` marker, resolve it with
`aphrodite_retrieve(hash)` before acting; never re-read the source file behind
a marker you already hold, the marker is that content.

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

Fields: `event` (required), `matcher` (optional, regex on `tool_name`),
`command` (required, `~` expansion), `timeout` (optional, default 60, max
300). Full field table: `references/hook-script-patterns.md`.

Available events: `pre_tool_call`, `post_tool_call`, `pre_llm_call`,
`post_llm_call`, `on_session_start`, `on_session_end`, `on_session_finalize`,
`on_session_reset`, `subagent_stop`.

## Consent Model

- First-run prompts for approval per `(event, command)` pair, persisted to
  `~/.hermes/shell-hooks-allowlist.json`
- Three bypass methods (any one suffices):
    1. `hermes --accept-hooks chat` (top-level flag, NOT `hermes chat --accept-hooks`)
    2. `HERMES_ACCEPT_HOOKS=1` env var
    3. `hooks_auto_accept: true` in config.yaml

`--accept-hooks` is a top-level flag, NOT a subcommand argument: CORRECT
`hermes --accept-hooks hooks test post_tool_call`; WRONG
`hermes hooks test post_tool_call --accept-hooks`.

Allowlist format: `{"approvals": [{ "event": "post_tool_call", "command":
"~/.hermes/agent-hooks/hook.sh" }]}`.

## JSON Protocol

**stdin** fields received by the script: `hook_event_name`, `tool_name`,
`tool_input` (`path`, `content` for `write_file`/`patch`), `session_id`,
`cwd`, `extra` (`task_id`, `tool_call_id`).

**stdout** response shapes: block
(`{"decision": "block", "reason": "..."}` or `{"action": "block",
"message": "..."}`), modify (`{"action": "modify", "args": {...}}` or
Claude-Code-style `{"decision": "modify", "tool_input": {...}}`), context
(`{"context": "..."}` - pre_llm_call only), silent no-op `{}`.

Full stdin/stdout JSON examples: `references/hook-script-patterns.md`.

## Porting from Claude Code

The field names in `tool_input` are not interchangeable: Hermes uses
`tool_input.path` for `write_file` and `patch`, Claude uses
`tool_input.file_path` (so
`FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // empty')`
fails on Hermes; use
`FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.path // empty')`).

## Testing Hooks

Shell hooks fire only inside the agent's own tool loop
(`handle_function_call()` in `model_tools.py`); calling `write_file` via your
own tool calls does NOT trigger hooks, because you are not running through the
tool dispatch pipeline.

Never test hooks inside the Aphrodite repo working tree: the `post_tool_call`
normalize-dashes hook modifies files in-place, creating mtime-change warnings
and touching files you didn't intend to edit. Test under `~/.hermes/` only
(oneshot command + WRONG in-repo variant:
`references/hook-script-patterns.md`).

**Stop if** - a hook test wrote into the Aphrodite repo working tree or
`/tmp`; scratch belongs under `~/.hermes/tmp/`, never `/tmp`.

**Recovery** - move the artifact to `~/.hermes/tmp/` and re-test only under
`~/.hermes/`. Never `git reset`/checkout to erase probe artifacts; the reason
is UNKNOWN (the source does not state one).

The `-z` flag is a top-level flag on `hermes`, NOT a subcommand argument:
CORRECT `hermes -z "prompt..." --accept-hooks`; WRONG `hermes chat -z "prompt..."`.

Proof the hook fired (pre_tool_call modify): the file content shows the
modification on first read - no mtime warning. Proof (post_tool_call): the
`write_file` tool emits "file was modified since you last read it on disk
(external edit or unrecorded writer). Re-read the file before writing." - the
post-hook signature.

Diagnostics (`hermes hooks list`, `hermes hooks doctor`, `hermes hooks test
<event> --for-tool <name>`, `hermes hooks revoke <command>`):
`references/hook-script-patterns.md`.

## pre_tool_call: Pre-Execution Interception

Fires BEFORE a tool executes; returns block (stop the operation) or modify
(transform `tool_input` before dispatch). `modify` args are shallow-merged
over the original args - return only the fields you changed. Returning
`modify` DOES NOT skip tool execution; for transform AND guard, return modify
first, then let the tool run. Block takes precedence over modify - if one hook
returns modify and another returns block, the tool is blocked.

No feedback channel for block: the AGENT sees only a generic "tool call was
blocked" message, not your `reason`; use modify + the mtime signal instead.
Hook scripts run with full user credentials - keep them in
`~/.hermes/agent-hooks/` for auditing. Never disable a hook script by
overwriting it via the agent's `write_file`/`patch` - the hook fires on its
OWN write and prepends its marker to the disable-content; disable in
config.yaml instead.

Patterns (pre-write normalization `normalize-tabs-pre.sh` with
`perl -pe 's/\\\\t/\\t/g'`; blocking writes on the runtime home
`~/.hermes/aphrodite/` - `aphrodite.toml`, `binaries/`, `hotreload/`),
design principle, full pitfalls, Stop-if/Recovery:
`references/hook-script-patterns.md`.

**Stop if** - a hook blocks a write the current workflow legitimately needs
(e.g. an approved `aphrodite.toml` change during Release phase).

**Recovery** - narrow the guarded path list to files that must never be
touched by the tool loop; route intended edits through the owning skill's
procedure.

## Internal Architecture: Single Hook Invocation

The `pre_tool_call` event fires exactly **once** per tool execution - not once
for block checks and again for modify checks; shell hooks have side effects
(e.g. a `normalize-dashes.sh` hook modifying file content in-place), so a
double fire would run the hook twice on the same tool call.

The single invocation is handled by `_dispatch_pre_tool_call_hooks()` in
`hermes_cli/plugins.py`, returning both results from one `invoke_hook()` call.
All 4 internal entry points use it: `model_tools.py::handle_function_call()`,
`agent/tool_executor.py` (concurrent tools path), `agent/tool_executor.py`
(single tool path), `agent/agent_runtime_helpers.py`.
`get_pre_tool_call_block_message()` still delegates to it for plugin callers
(CLAIM: stated by the source). Annotated call example:
`references/hook-script-patterns.md`.

## post_tool_call: Post-Edit Formatting

Fires after every tool execution; with a `matcher` gate it auto-formats files
after `write_file`/`patch`. Routing (`.rs` → `cargo fmt` nearest Cargo.toml -
`crates/aphrodite/Cargo.toml` or `crates/aphrodite-hermes/Cargo.toml`,
`.sh` → `shfmt`, `.js/.ts/.css/.json/.md/.yaml/.toml` → `prettier`, `.py` →
`black`/`ruff`; skips `node_modules`, `target/`, `vendor/`, `.git/`) +
canonical registration (normalize-dashes.sh, normalize-tabs.sh,
post-edit-format-qa.py, all `matcher: "write_file|patch"`):
`references/hooks-pipeline-architecture.md`.

**Stop if** - a hook formatted a file the agent did not edit (matcher too
broad, or a build pipeline wrote into the repo tree).

**Recovery** - tighten the `matcher` regex; keep the post-hook as a fallback
only for files produced outside the tool loop.

## pre_llm_call: Context Injection

Fires once per turn, before the model generates a response (CLAIM: no probe
in the source; a sentinel log line is the test), and can return
`{"context": "..."}`, which Hermes injects into the user message - ideal for
RAG, memory plugins, or prior-work discovery. In an Aphrodite session this
surfaces prior CCR work, hook contracts, and repo skills (`.hermes/skills/`,
Development branch) as context.

stdin payload: `{"prompt": "...", "session_id": "...", "task_id": "..."}`.
Approach (stdin read, keyword extraction, search
`~/.hermes/sessions/session_*.json`, `<workspace>/.hermes/skills/**/SKILL.md`,
`~/.hermes/memory`, `~/.hermes/*.md`, `{"context": "PRIOR CONTEXT:\n...\n"}`
return) + worked hook (`prompt-context.py`):
`references/hook-script-patterns.md`.

## post_llm_call: Background QA Agent

Fires once per turn after the model finishes its response - successful turns
only (CLAIM: no probe in the source). Runs SYNCHRONOUSLY in the hook pipeline,
but any QA agent it spawns must be detached (`nohup` / `start_new_session=True`);
results land on the NEXT turn via a cache file read by the pre_llm_call hook.

Detach code (`subprocess.Popen([sys.executable,
"path/to/background-qa-agent.py"], stdin=subprocess.PIPE,
stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
cwd=str(HERMES_HOME))`) and librarian pattern
(`subprocess.run(["hermes", "-z", prompt], capture_output=True, text=True,
timeout=120, env={**os.environ, "HERMES_ACCEPT_HOOKS": "0"},
cwd=str(HERMES_HOME))` - `HERMES_ACCEPT_HOOKS=0` prevents recursion when the
spawned `hermes -z` runs its own hooks; prompt pattern, spawn thresholds,
two-phase logic CLAIM): `references/hook-script-patterns.md`.

## Full 4-Hook Pipeline

The four events form an auto-improving cycle: `pre_llm_call` (prompt-context.py)
injects context; the LLM generates; `post_llm_call` (self-qa-check.py, < 5ms
CLAIM) spawns a detached QA agent (flash-tier model) cached for the NEXT turn;
`post_tool_call` runs normalize-dashes.sh, normalize-tabs.sh,
post-edit-format-qa.py. ASCII diagram: `references/hook-script-patterns.md`.

## Known Limitation: TUI Mode Hooks

The Hermes TUI gateway (`tui_gateway.entry` / `tui_gateway.slash_worker`) does
**not** call `register_from_config()`, so **no shell hooks fire in TUI mode** -
neither `block`, `modify`, nor observer hooks (CLAIM: probe with the
sentinel-log technique below). Hooks fire in CLI mode (`hermes chat`,
`hermes -z "..."` - `register_from_config` called by `hermes_cli/main.py` at
startup) and gateway mode (`hermes gateway run` - called by `gateway/run.py`).

To test a hook, use CLI mode (exact commands: `hermes chat -q "Write a file
~/.hermes/tmp/test.txt with content hello"` / `HERMES_ACCEPT_HOOKS=1 hermes
-z "Write a file ~/.hermes/tmp/test.txt with content hello"`).

Diagnosing whether hooks fire - add a sentinel to the hook script and check
after a tool call:

```bash
# In your hook script, add:
# echo "$(date +%s) hook fired for $TOOL_NAME" >> ~/.hermes/tmp/hook-debug.log

# Then check after a tool call:
cat ~/.hermes/tmp/hook-debug.log
```

A sentinel in CLI mode but not TUI mode means the hooks are firing; the gap is
the TUI.

## Subagent Hook Behavior

Each subagent gets its own `AIAgent` instance, so `pre_llm_call`/
`post_tool_call` hooks fire **inside the subagent independently** of the
parent (CLAIM: no probe in the source). The parent receives `subagent_stop`
once per child completion. Hooks registered on the parent do NOT automatically
apply to subagents - each process loads its own hooks from `config.yaml` at
startup; with parallel delegates (crate audits, compression probes) each
child fires its own hooks, so keep the hot path cheap.

## Performance Rule

Do not spawn `hermes -z` LLM subagents inside the hook pipeline: the hot path
must stay regex checks + local formatters. Deep QA runs manually via
`delegate_task` or a one-shot `hermes -z` on demand - not as a hook. Per-hook
timings: `references/hook-script-patterns.md` (CLAIM: figures are source
measurements; re-probe with a `time` wrapper).

## Security

- Scripts run with full user credentials - keep scripts in
  `~/.hermes/agent-hooks/` for easy auditing
- Re-run `hermes hooks doctor` after pulling shared configs (validates exec
  bit, allowlist, mtime)
- Review the `hooks:` section in config changes like CI config changes

## Files & Cross-References

- `scripts/normalize-tabs.sh`, `scripts/normalize-dashes.sh`,
  `scripts/prompt-context.py`, `scripts/post-edit-format-qa.py`,
  `templates/hermes-oneshot-wrapper.py` - reference implementations
- `references/hook-script-patterns.md` - script listings, wire-protocol JSON,
  pipeline diagram, timings, librarian logic, annotated dispatch example
- `references/hooks-pipeline-architecture.md` - pipeline internals;
  `references/live-install-workflow.md` - live source + dylib hot-reload;
  `references/tui-architecture.md` - TUI internals
- Workers (venv python, temp-file prompts, `session_db=None`):
  `hermes-background-workers`; config hook edits (write guard,
  `hermes config set/unset`): `hermes-config-maintenance`; CCR hook contracts:
  `aphrodite-hook-contracts` / `aphrodite-hook-reference`

## Local claim-to-test matrix

| Claim | Evidence source | Test | Pass condition | Failure response |
| --- | --- | --- | --- | --- |
| post_tool_call hooks fire only in the tool loop | SKILL.md | `hermes hooks test post_tool_call --for-tool write_file` | Hook output appears | Check exec bit + allowlist (`hermes hooks doctor`) |
| Hooks do not fire in TUI mode | This skill | Start TUI, run a write_file | No post-hook mtime warning appears | Expected - test in CLI mode instead |
| Dash/tab normalization keeps files clean | `git status --short` | Write a file with an em dash under `~/.hermes/tmp/` | On-disk file contains ASCII hyphen only | Re-check registration/allowlist |
| `cargo fmt` routing finds crate manifests | post-edit-format-qa.py | Edit a `.rs` file under `crates/` | Format result reports `cargo fmt` success | Check the nearest-Cargo.toml walk |
| Hook tests never touch the repo tree | `git status --short` | Run the `~/.hermes/tmp/` test | No repo file modified | Move the test target under `~/.hermes/tmp/` |