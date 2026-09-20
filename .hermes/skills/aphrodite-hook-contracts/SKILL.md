---
name: aphrodite-hook-contracts
description: "Use when implementing or debugging Aphrodite Hermes hooks. Canonical per-hook contracts: verified invocation points, keyword names, return semantics, safety boundaries, and test cases."
version: 1.0.0
platforms: [macos]
tags: [aphrodite, hermes, plugin, hooks, contracts]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - Per-hook contracts (invocation point, keyword names, return semantics, safety boundary, verification) for the six registered hooks
    - Hook parameter-name compatibility (wrong-param -> correct-param mapping and its negative tests)
    - The 10-case hook test matrix and the terminal sentinel-output test
depends_on:
    - aphrodite-boundaries (context boundaries, failure-behavior policy, terminology)
    - aphrodite-orientation (preflight gate before any live probe)
supersedes: []
verification:
    source_of_truth:
        - ~/.hermes/hermes-agent (each invoke_hook call site; VALID_HOOKS in hermes_cli/plugins.py)
        - plugins/aphrodite/plugin.yaml (provides_hooks)
        - plugins/aphrodite/__init__.py (registration shim)
        - crates/aphrodite-hermes/src/lib.rs (dylib hook-dispatch arms, get_hooks list)
mutation_level: read-only
---

# Aphrodite Hook Contracts

Canonical per-hook contracts for the six hooks the Aphrodite plugin registers.
**Source-derived facts, not prose:** keyword names, call sites, and return
semantics drift between Hermes releases; every contract names the exact
`invoke_hook` site to re-derive from. Line numbers are observational evidence
(snapshots), never durable coordinates - identify functions and source-relative
paths instead.

**Confidence labels:** `invariant` = architecture-level, stable unless the
framework changes shape; `source-derived` = verify in the currently checked-out
Hermes source before relying on it; `historical` = explanatory only, never copy
into live implementation.

## Registration names (verified)

`plugin.yaml` `provides_hooks` and the dylib `aphrodite_hermes_get_hooks` list
(`crates/aphrodite-hermes/src/lib.rs`, get_hooks arm) agree on exactly six
names:

```text
on_session_start  pre_tool_call  transform_tool_result
transform_terminal_output  pre_llm_call  post_llm_call
```

The plugin registers hooks dynamically from the dylib list via
`ctx.register_hook(hook_name, _dispatch)` (`plugins/aphrodite/__init__.py`,
register()); every handler therefore receives Hermes' kwargs verbatim, JSON-serialized,
and is dispatched to the Rust `aphrodite_hermes_call_hook` arm. Registration
uses the `on_`-prefixed session names - `VALID_HOOKS` in
`hermes_cli/plugins.py` is the registry of accepted names.

## Global contract (all six hooks)

- **Minimum compatible signature always ends in `**kwargs`.** Hermes adds
  fields across releases (e.g. `parent_session_id` on pre_llm_call,
  `tool_call_id` on transform_terminal_output); a handler without `**kwargs`
  breaks the moment Hermes adds one.
- **Hook input structures are read-only** unless the framework documents
  mutability; `conversation_history` is a copy, so in-place edits are discarded
  (owner: `aphrodite-boundaries`, context boundaries).
- **A hook that replaces output must return the original output for
  pass-through; an empty string is a destructive replacement, never "no
  change"** (owner: `aphrodite-boundaries`).
- **Fail-open is the default** for transform/lifecycle hooks: log structured
  error, return the original content. The single fail-closed hook is
  `pre_tool_call` (see below).
- **Never compress a retrieval or Aphrodite diagnostic response** and never
  alter a valid CCR marker without revalidating marker syntax (owner:
  `aphrodite-compression-safety` / `aphrodite-ccr-protocol`; asserted per-hook
  under Safety boundary).

---

## Hook: on_session_start

**Invocation point:** `agent/conversation_loop.py:773-776` (session-start block
in the conversation run path; fire-and-forget, wrapped in try/except fail-open).

**Registration name:** `on_session_start` (historical bug: registering
`session_start` never fired - proxy never auto-launched. The Rust dispatch arm
still accepts the legacy alias `"session_start"`).

**Minimum compatible signature**

```python
def on_start(*, session_id: str = "", model: str = "", platform: str = "", **kwargs) -> None:
```

**Input invariants**

- `session_id` is a string; `model` and `platform` may be empty strings
- Unknown fields can be added by Hermes; accept `**kwargs`

**Return contract**

- Return value is **ignored** (fire-and-forget lifecycle hook). Never return a
  replacement string.

**Safety boundary**

- This is the session bootstrap point: it must never compress, and must never
  block on network. Rust-side it resets per-session state
  (`aphrodite::session::on_session_start`).

**Verification**

- Pass-through (return value ignored), handler exception (Hermes logs a
  warning and continues), unknown-kwarg, registered-but-never-invoked.

---

## Hook: pre_tool_call

**Invocation point:** `hermes_cli/plugins.py:1816-1819` in
`_get_pre_tool_call_directive_details` (runs before tool dispatch).

**Registration name:** `pre_tool_call` (the ONLY fail-closed hook).

**Minimum compatible signature**

```python
def pre_tool_hook(
    *,
    tool_name: str = "",
    args: dict | None = None,
    task_id: str = "",
    session_id: str = "",
    tool_call_id: str = "",
    turn_id: str = "",
    api_request_id: str = "",
    middleware_trace: list | None = None,
    **kwargs,
) -> dict | None:
```

**Input invariants**

- `args` is a dict (Hermes coerces non-dicts to `{}` before invoking)
- `middleware_trace` is a list (empty list when absent)
- All id fields default to `""`

**Return contract** (directive dict; first valid dict wins, non-dict returns
ignored)

- `None` / non-dict: no directive; tool proceeds normally
- `{"action": "modify", "args": {...}}`: keys shallow-merge into the tool
  arguments before dispatch
- `{"action": "block", "message": str}`: veto; the message becomes the tool
  result
- `{"action": "approve", "message": ..., "rule_key"?: ...}`: escalate the tool
  to the human-approval gate

**Safety boundary**

- **Fail-closed on timeout:** `pre_tool_call` is the sole member of
  `_HOOK_TIMEOUT_FAIL_CLOSED_HOOKS` (`hermes_cli/plugins_dispatch.py`) - a
  hung/still-running callback blocks the tool with
  `"pre_tool_call plugin callback timed out or is still running"`. Keep the
  handler bounded; after a timeout the same callback is suppressed for 60 s
  (`_HOOK_TIMEOUT_SUPPRESSION_SECONDS`).
- Aphrodite's Rust arm returns
  `{"action": "modify", "args": {"background": true, "notify_on_complete": true}}`
  to auto-background long terminal/process commands when the poll worker is
  enabled.

**Verification**

- modify-merge (args changed before dispatch), block (tool result is the
  message), approve (approval gate), non-dict ignored, handler exception /
  timeout (tool blocked, message visible), unknown-kwarg.

---

## Hook: transform_tool_result

**Invocation point:** `model_tools.py:854-856` in
`_apply_transform_tool_result_hook` (after the tool result is produced, before
it enters context; gated on `has_hook`, fail-open).

**Registration name:** `transform_tool_result`.

**Minimum compatible signature**

```python
def _transform_tool_result(
    *,
    tool_name: str = "",
    args: dict | None = None,
    result: str = "",
    tool_call_id: str = "",
    task_id: str = "",
    session_id: str = "",
    turn_id: str = "",
    api_request_id: str = "",
    duration_ms: int = 0,
    status: str = "",
    error_type: str = "",
    error_message: str = "",
    **kwargs,
) -> str | None:
```

**Input invariants**

- `result` is the complete tool output (string; JSON for structured tools)
- The five id fields come from `_CallIds.hook_kwargs()` with `None -> ""`
- `duration_ms` is numeric; `status`/`error_type`/`error_message` describe the
  call outcome (error status does NOT suppress this hook)

**Return contract**

- `None` or any non-string: do not claim ownership; original `result` is used
- Original `result` returned: explicit pass-through
- Non-empty replacement string: replace the tool result (first non-None string
  across handlers wins)
- Empty string: **destructive replacement**, permitted only by an explicit
  redaction rule - never use it for "no change"

**Safety boundary**

- Must bypass compression for retrieval and diagnostic tools: the skip set
  covers `aphrodite_retrieve`, `aphrodite_stats`, `aphrodite_search`,
  `aphrodite_catalog`, `aphrodite_files`, `aphrodite_diff`,
  `aphrodite_directive`, `aphrodite_prefetch_status` (and any other tool whose
  output is a retrieval/diagnostic response) - re-compressing a retrieval
  response creates the CCR marker-resolution loop
- Must not alter a valid CCR marker without revalidating marker syntax

**Verification**

- Pass-through, replacement, empty-result, handler exception (fail-open),
  unknown-kwarg, error-status (non-zero / error result still reaches the
  hook), large payload (threshold + marker behavior).

---

## Hook: transform_terminal_output

**Invocation point:** `tools/terminal_tool_result.py:144-146` in
`_apply_output_transform_hook` - **the file was renamed** from
`tools/terminal_tool.py` (older reference notes and the Rust comment at
`crates/aphrodite-hermes/src/lib.rs` still cite the old path/line; the
function is the durable anchor).

**Registration name:** `transform_terminal_output`.

**Minimum compatible signature**

```python
def _transform_terminal_hook(
    *,
    command: str | None = None,
    output: str = "",
    returncode: int = 0,
    task_id: str = "",
    env_type: str = "",
    tool_call_id: str = "",
    **kwargs,
) -> str | None:
```

**Input invariants**

- `output` is the complete candidate terminal output - **NOT `stdout`**; using
  `stdout`/`exit_code` made ALL terminal output empty (the historical
  terminal-output bug)
- `returncode` is numeric - **NOT `exit_code`**
- `tool_call_id` was added to the invocation (from the approval context) after
  the older reference notes; unknown fields will keep being added - accept
  `**kwargs`

**Return contract**

- `None`: do not claim ownership; allow later hook/default behavior
- Original `output` value returned: explicit pass-through (must return the
  `output` parameter itself)
- Non-empty replacement string: replace the terminal output (first non-None
  string across handlers wins; replacements are still subject to the output
  limit applied afterwards)
- Empty string: **destructive replacement** - permitted only by an explicit
  redaction rule, never "no change"

**Safety boundary**

- Must bypass compression for retrieval and diagnostic tools
- Must not alter a valid CCR marker without revalidating marker syntax

**Verification**

- Pass-through, replacement, empty-result, exception, unknown-kwarg,
  non-zero-exit (returncode != 0 must not erase output), and the **sentinel
  test** below.

### Sentinel-output test (mandatory for this hook)

The known `stdout`-vs-`output` mismatch yields an empty replacement despite a
successful-looking invocation. Guard it permanently:

1. Invoke the handler with a sentinel output (`output="SENTINEL_XYZ"`) and a
   non-zero `returncode`.
2. Assert the handler observed the sentinel (a handler reading `stdout` gets
   its `""` default and returns `""` - the failure shape).
3. Assert the pass-through path returns the exact sentinel bytes.

---

## Hook: pre_llm_call

**Invocation point:** `agent/turn_context.py:696-708` in
`_collect_pre_llm_call_context` (before each LLM turn).

**Registration name:** `pre_llm_call`.

**Minimum compatible signature**

```python
def _pre_llm_hook(
    *,
    session_id: str = "",
    task_id: str = "",
    turn_id: str = "",
    user_message: str | None = None,
    conversation_history: list | None = None,
    is_first_turn: bool = False,
    model: str = "",
    platform: str = "",
    parent_session_id: str = "",
    sender_id: str = "",
    **kwargs,
) -> str | dict | None:
```

**Input invariants**

- `conversation_history` is `list(messages)` - a **COPY**. In-place mutations
  (pop, insert) are DISCARDED. To modify messages, use the context engine, not
  this hook.
- `user_message` is the original user message - **NOT `response`**
- `parent_session_id` exists in the current Hermes source (added after the
  older reference notes); unknown fields will keep being added

**Return contract**

- `None`, empty string, or non-dict: no injection
- Dict with a non-empty `"context"` value, or a non-empty string: injected as
  plugin context into the **user message** (all plugin pieces joined with
  `"\n\n"`); oversized pieces spill to disk
  (`tools/hook_output_spill.py`) so a runaway plugin cannot inflate the prompt

**Safety boundary**

- Fail-open: any handler exception logs a warning and yields no injection
- Never mutate the copy; never compress the copied history here

**Verification**

- Context injection (string and `{"context": ...}` forms), empty/no-op,
  handler exception, unknown-kwarg, copy-discard test (mutate
  `conversation_history` in place; assert the live transcript is unchanged).

---

## Hook: post_llm_call

**Invocation point:** `agent/turn_finalizer.py:433-443` in
`_apply_output_hooks` (after the LLM response; invoked through
`_invoke_hook_safely`, which swallows handler exceptions).

**Registration name:** `post_llm_call`.

**Minimum compatible signature**

```python
def _store_conversation_turn(
    *,
    session_id: str = "",
    task_id: str = "",
    turn_id: str = "",
    user_message: str | None = None,
    assistant_response: str = "",
    conversation_history: list | None = None,
    model: str = "",
    platform: str = "",
    **kwargs,
) -> None:
```

**Input invariants**

- `assistant_response` is the final response text - **NOT `response`**
- `turn_id` is a UUID string (`session_id:task_id:uuid`) - **NOT a sequential
  `turn_number`**; keep your own counter for human-readable memory indexing
- `conversation_history` is a copy; read-only

**Return contract**

- Return value is ignored. Return `None`.

**Safety boundary**

- Read-only observer; never mutate history, never compress here.

**Verification**

- Pass-through, handler exception (swallowed by `_invoke_hook_safely`),
  unknown-kwarg.

---

## Hooks the plugin does NOT register (boundary notes)

- `transform_llm_output` - sibling of `post_llm_call`
  (`agent/turn_finalizer.py:420-427`): first non-empty string replaces the
  final response. The plugin does not provide it; a future provider must
  re-verify the current call site.
- `pre_api_request` / `post_api_request` - **the old claim "in VALID_HOOKS but
  zero invocation sites" is STALE**: current Hermes fires `pre_api_request`
  from `agent/turn_api_request.py:52-63` and `post_api_request` from
  `agent/turn_response_intake.py:67-69` (both gated on `has_hook`). Aphrodite
  still does not register them, and message mutation still belongs to the
  context engine - but re-derive the contract before relying on either.

## Known parameter mismatches (historical evidence -> negative tests)

| Hook                      | Wrong Param                         | Correct Param                                     | Effect                           |
| ------------------------- | ----------------------------------- | ------------------------------------------------- | -------------------------------- |
| transform_terminal_output | stdout, stderr, exit_code           | output, returncode                                | ALL terminal output empty        |
| pre_llm_call              | api_messages, response              | conversation_history, user_message                | Hook returned early (None guard) |
| post_llm_call             | api_messages, response, turn_number | conversation_history, assistant_response, turn_id | Hook returned early              |
| on_session_start          | session_start (hook name)           | on_session_start                                  | Proxy never auto-launched        |

Each row is a permanent negative test: the handler signature must accept the
correct names, and a fixture invoking the hook with only the wrong names must
NOT silently produce a destructive empty result.

## Hook test cases (10-case matrix, every active hook)

| Case                              | Purpose                                                                             |
| --------------------------------- | ----------------------------------------------------------------------------------- |
| Valid nominal invocation          | Verifies the ordinary contract                                                      |
| Unknown extra keyword             | Ensures forward compatibility through `**kwargs`                                    |
| Missing optional keyword          | Verifies defaults and null handling                                                 |
| Empty text input                  | Separates pass-through from destructive output                                      |
| Large payload                     | Exercises threshold and marker behavior                                             |
| Error status / non-zero exit      | Confirms errors do not disappear                                                    |
| Handler exception                 | Defines fallback and diagnostic behavior (fail-open; fail-closed for pre_tool_call) |
| Multiple registered handlers      | Verifies ordering and first-non-None semantics                                      |
| Hook registered but never invoked | Detects dead integration (e.g. a hook name drift)                                   |
| Restarted process                 | Ensures results are not from a stale plugin/dylib (fresh-process test)              |

The terminal hook additionally requires the sentinel-output test above.

## Re-deriving a contract from source

Implementation properties are never trusted from prose:

```sh
grep -rn 'invoke_hook("transform_tool_result"' ~/.hermes/hermes-agent/ --include='*.py'
grep -rn 'invoke_hook("transform_terminal_output"' ~/.hermes/hermes-agent/ --include='*.py'
grep -n 'VALID_HOOKS' ~/.hermes/hermes-agent/hermes_cli/plugins.py
```

Read the call site's full kwargs, compare each name against the handler
parameters, and confirm `**kwargs` is present. If a keyword changed, update
this contract and its compatibility test before touching the plugin.

## References

Evidence notes moved here from `aphrodite-hook-reference` (keep as evidence;
their Hermes v0.16.0 line numbers have drifted - treat as observational):

- `references/hook-invocations.md` - per-hook invocation reference (v0.16.0
  coordinates; pre_api_request row is stale, see boundary notes above)
- `references/hook-invocation-verification.md` - the source-verification recipe
- `references/hook-parameter-mismatches.md` - wrong-param incidents mapped to
  correct params

## Claim-to-test matrix

| Claim                                        | Evidence source                                                    | Test                                                     | Pass condition                                       | Failure response                                 |
| -------------------------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------ |
| Hook receives `output`, not `stdout`         | `tools/terminal_tool_result.py:144`                                | Sentinel-output invocation                               | Handler observes original sentinel                   | Update handler parameters and this contract      |
| Hook registration uses `on_session_start`    | `hermes_cli/plugins.py` VALID_HOOKS + `plugin.yaml`                | Start session with registration log                      | One log entry appears                                | Inspect registration name/source                 |
| pre_tool_call is fail-closed on timeout      | `hermes_cli/plugins_dispatch.py` `_HOOK_TIMEOUT_FAIL_CLOSED_HOOKS` | Stall a handler past the timeout and dispatch a tool     | Tool blocked with the timeout message                | Fix handler boundedness; re-derive dispatch code |
| `conversation_history` is a discardable copy | `agent/turn_context.py:702`                                        | Mutate it in place; assert transcript unchanged          | Live transcript unchanged                            | Move mutation to the context engine              |
| Empty string replaces, never passes through  | Return contracts above                                             | Invoke with `output="x"`, return `""`                    | Result is empty (documented destructive replacement) | Fix handler to return the original output        |
| pre_api_request now has a call site          | `agent/turn_api_request.py:52-63`                                  | Grep current Hermes source                               | Call site present (old "zero sites" claim is stale)  | Update boundary notes; re-derive contract        |
| Registered-but-never-invoked detection       | VALID_HOOKS vs production `invoke_hook` sites                      | Script: registered names minus non-test invocation names | Zero dead registrations                              | Fix registration name or remove dead hook        |
