---
name: aphrodite-hook-contracts
description: "Use when implementing or debugging Aphrodite Hermes hooks. Canonical per-hook contracts: invocation points, keyword names, return semantics, safety boundaries, and test cases."
version: 1.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-hook-contracts
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, hermes, plugin, hooks, contracts, implement, debug]
        related_skills:
            [
                aphrodite-boundaries,
                aphrodite-orientation,
                aphrodite-hook-reference,
                aphrodite-compression-safety,
            ]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - Per-hook contracts (invocation point, keyword names, return semantics, safety boundary, verification) for the five documented hooks (plus pre_tool_call, still in the dylib list)
    - Hook parameter-name compatibility (wrong-param -> correct-param mapping and its negative tests)
    - The 10-case hook test matrix and the terminal sentinel-output test
depends_on:
    - aphrodite-boundaries (context boundaries, failure-behavior policy, terminology)
    - aphrodite-orientation (preflight gate before any live probe)
supersedes: []
verification:
    source_of_truth:
        - $HOME/.hermes/hermes-agent (each invoke_hook call site; VALID_HOOKS in hermes_cli/plugins.py)
        - plugins/aphrodite/plugin.yaml (provides_hooks)
        - plugins/aphrodite/__init__.py (registration shim)
        - crates/aphrodite-hermes/src/lib.rs (dylib hook-dispatch arms, get_hooks list)
mutation_level: read-only
---

# Aphrodite Hook Contracts

Canonical per-hook contracts for the six hooks of the current source surface: the five documented hooks below plus `pre_tool_call`, the sixth the dylib list still registers.

Contracts are source-derived facts, not prose: keyword names, call sites, and return semantics drift between Hermes releases, so every contract names the exact `invoke_hook` site to re-derive from. Line numbers are observational snapshots, not durable coordinates; identify functions and source-relative paths instead.

Confidence labels: `invariant` = architecture-level, stable unless the framework changes shape; `source-derived` = verify in the checked-out Hermes source before relying on it; `historical` = explanatory only, never copy into live implementation.

**Stop if:** a keyword name or invocation point below disagrees with the checked-out Hermes source, or the handler you are about to write lacks `**kwargs`.
**Recovery:** run the grep recipe in "Re-deriving a contract from source", update this contract and its compatibility test, then continue.

## Registration names

The documented surface is five hooks: `plugin.yaml` `provides_hooks` and the dylib `aphrodite_hermes_get_hooks` list (`crates/aphrodite-hermes/src/lib.rs`, get_hooks arm) agree on these five (CLAIM: read both lists):

```text
on_session_start  transform_tool_result  transform_terminal_output
pre_llm_call  post_llm_call
```

The dylib list additionally registers `pre_tool_call`, the fail-closed directive hook (full contract below), so the registered set is six in the current source. It is not five: the lib.rs module doc comment still names five (CLAIM: read the doc comment). Verify the actual set against `aphrodite_hermes_get_hooks` rather than assuming:

```sh
grep -n 'VALID_HOOKS' $HOME/.hermes/hermes-agent/hermes_cli/plugins.py
```

The installed loader (`~/.hermes/aphrodite/__init__.py`; repo source at `plugins/aphrodite/__init__.py`) registers hooks verbatim from the dylib list via `ctx.register_hook(hook_name, _dispatch)` with no filtering (CLAIM: read the loader to confirm); kwargs pass JSON-serialized to the Rust `aphrodite_hermes_call_hook` arm (CLAIM: read `crates/aphrodite-hermes/src/lib.rs`). `VALID_HOOKS` in `hermes_cli/plugins.py` is the registry of accepted names (`on_`-prefixed).

## Global contract (all registered hooks)

- Every handler signature ends in `**kwargs`. Hermes adds fields across releases; a handler without `**kwargs` breaks the moment one is added. Test: the unknown-extra-keyword case in `references/hook-test-cases.md`.
- Hook input structures are read-only unless the framework documents mutability. `conversation_history` is a copy, so in-place edits are discarded; mutating the copy is not mutating the transcript (owner: `aphrodite-boundaries`).
- A hook that replaces output must return the original output for pass-through. An empty string is a destructive replacement, never "no change" (owner: `aphrodite-boundaries`).
- Fail-open is the default for transform/lifecycle hooks: log a structured error and return the original content. `pre_tool_call` is the single fail-closed hook.
- Never compress a retrieval or Aphrodite diagnostic response, because re-compressing a retrieval response creates the CCR marker-resolution loop. Never alter a valid CCR marker without revalidating marker syntax first, because the marker must stay resolvable under the CCR protocol. The retrieval/diagnostic skip set is listed under transform_tool_result (owners: `aphrodite-compression-safety` / `aphrodite-ccr-protocol`; asserted per-hook under Safety boundary).

## Hook: on_session_start

**Invocation point:** `agent/conversation_loop.py:773-776` (session-start block; fire-and-forget, wrapped in try/except fail-open).

**Registration name:** `on_session_start`. Registering `"session_start"` never fired, so the proxy never auto-launched; the Rust dispatch arm still accepts that legacy alias, but it is not a valid registration name (historical: never copy into a new registration).

**Minimum compatible signature**

```python
def on_start(*, session_id: str = "", model: str = "", platform: str = "", **kwargs) -> None:
```

**Input invariants** - `session_id` is a string; `model`/`platform` may be empty strings; Hermes can add unknown fields - accept `**kwargs`.

**Return contract** - the return value is ignored (fire-and-forget). Never return a replacement string, because nothing reads it.

**Safety boundary** - session bootstrap point: must never compress, must never block on network. Rust-side it resets per-session state (`aphrodite::session::on_session_start`; CLAIM: read the crate).

## Hook: pre_tool_call

**Invocation point:** `hermes_cli/plugins.py:1816-1819` in `_get_pre_tool_call_directive_details` (runs before tool dispatch).

**Registration name:** `pre_tool_call` - the ONLY fail-closed hook.

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

**Input invariants** - `args` is a dict (Hermes coerces non-dicts to `{}` before invoking); `middleware_trace` is a list, empty when absent; all id fields default to `""`.

**Return contract** (directive dict; the first valid dict wins, non-dict returns are ignored):
- `None` / non-dict: no directive; the tool proceeds normally
- `{"action": "modify", "args": {...}}`: keys shallow-merge into the tool arguments before dispatch
- `{"action": "block", "message": str}`: veto; the message becomes the tool result
- `{"action": "approve", "message": ..., "rule_key"?: ...}`: escalate the tool to the human-approval gate

**Safety boundary** - fail-closed on timeout: `pre_tool_call` is the sole member of `_HOOK_TIMEOUT_FAIL_CLOSED_HOOKS` (`hermes_cli/plugins_dispatch.py`); a hung/still-running callback blocks the tool with `"pre_tool_call plugin callback timed out or is still running"`. Keep the handler bounded, because a hung callback blocks the tool; after a timeout the same callback is suppressed for 60 s (`_HOOK_TIMEOUT_SUPPRESSION_SECONDS`). The Rust arm returns `{"action": "modify", "args": {"background": true, "notify_on_complete": true}}` to auto-background long terminal/process commands when the poll worker is enabled (CLAIM: read `crates/aphrodite-hermes/src/lib.rs`).

## Hook: transform_tool_result

**Invocation point:** `model_tools.py:854-856` in `_apply_transform_tool_result_hook` (after the tool result is produced, before it enters context; gated on `has_hook`, fail-open).

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

**Input invariants** - `result` is the complete tool output (string; JSON for structured tools); the five id fields come from `_CallIds.hook_kwargs()` with `None -> ""`; `duration_ms` is numeric; `status`/`error_type`/`error_message` describe the call outcome - an error status does NOT suppress this hook.

**Return contract** - `None` or any non-string: do not claim ownership; the original `result` is used. A non-empty replacement string: replace the tool result (the first non-None string across handlers wins). An empty string: destructive replacement, permitted only by an explicit redaction rule - never use it for "no change", because it erases the result.

**Safety boundary** - must bypass compression for retrieval and diagnostic tools: the skip set covers `aphrodite_retrieve`, `aphrodite_stats`, `aphrodite_search`, `aphrodite_catalog`, `aphrodite_files`, `aphrodite_diff`, `aphrodite_directive`, `aphrodite_prefetch_status` (and any other tool whose output is a retrieval/diagnostic response), because re-compressing a retrieval response creates the CCR marker-resolution loop. Must not alter a valid CCR marker without revalidating marker syntax (resolvability under the CCR protocol).

## Hook: transform_terminal_output

**Invocation point:** `tools/terminal_tool_result.py:144-146` in `_apply_output_transform_hook`. The file was renamed from `tools/terminal_tool.py`; older reference notes and the Rust comment at `crates/aphrodite-hermes/src/lib.rs` still cite the old path - the function is the durable anchor.

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

**Input invariants** - `output` is the complete candidate terminal output. It is not `stdout`: a handler reading `stdout` receives the `""` default and returns `""`, which made ALL terminal output empty (historical terminal-output bug, now a negative test). `returncode` is numeric. It is not `exit_code`. `tool_call_id` was added to the invocation (from the approval context) after the older reference notes; unknown fields will keep being added - accept `**kwargs`.

**Return contract** - `None`: do not claim ownership; allow later hook/default behavior. The original `output` returned: explicit pass-through (return the `output` parameter itself). A non-empty replacement string: replace the terminal output (the first non-None string across handlers wins; replacements are still subject to the output limit applied afterwards). An empty string: destructive replacement - permitted only by an explicit redaction rule, never "no change".

**Safety boundary** - must bypass compression for retrieval and diagnostic tools (skip set and reason in the global contract); must not alter a valid CCR marker without revalidating marker syntax (resolvability under the CCR protocol).

## Hook: pre_llm_call

**Invocation point:** `agent/turn_context.py:696-708` in `_collect_pre_llm_call_context` (before each LLM turn).

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

**Input invariants** - `conversation_history` is `list(messages)` - a COPY (global contract); in-place mutations (pop, insert) are DISCARDED. To modify messages, use the context engine, not this hook. `user_message` is the original user message. It is not `response`. `parent_session_id` appears in the current Hermes source (added after the older reference notes); unknown fields will keep being added.

**Return contract** - `None`, empty string, or non-dict: no injection. A dict with a non-empty `"context"` value, or a non-empty string: injected as plugin context into the user message (all plugin pieces joined with `"\n\n"`); oversized pieces spill to disk (`tools/hook_output_spill.py`) so a runaway plugin cannot inflate the prompt (CLAIM: read `tools/hook_output_spill.py`).

**Safety boundary** - fail-open: any handler exception logs a warning and yields no injection. Never compress the copied history here, because compression is owned by the compression-safety contract.

## Hook: post_llm_call

**Invocation point:** `agent/turn_finalizer.py:433-443` in `_apply_output_hooks` (after the LLM response; invoked through `_invoke_hook_safely`, which swallows handler exceptions).

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

**Input invariants** - `assistant_response` is the final response text. It is not `response`. `turn_id` is a UUID string (`session_id:task_id:uuid`). It is not a sequential `turn_number`; keep your own counter for human-readable memory indexing. `conversation_history` is a copy; read-only.

**Return contract** - the return value is ignored. Return `None`.

**Safety boundary** - read-only observer: never mutate history, because in-place edits are discarded; never compress here, because compression is owned by the compression-safety contract.

## Hooks the plugin does NOT register

- `transform_llm_output` - sibling of `post_llm_call` (`agent/turn_finalizer.py:420-427`): the first non-empty string replaces the final response. The plugin does not provide it; a future provider must re-verify the current call site.
- `pre_api_request` / `post_api_request` - the old claim "in VALID_HOOKS but zero invocation sites" is STALE (historical). Current Hermes fires `pre_api_request` from `agent/turn_api_request.py:52-63` and `post_api_request` from `agent/turn_response_intake.py:67-69`, both gated on `has_hook` (source-derived; grep the current source to confirm). Aphrodite still does not register them, and message mutation still belongs to the context engine - re-derive the contract before relying on either.

## Known parameter mismatches (historical evidence -> negative tests)

Every wrong-param incident is a permanent negative test: the handler signature must accept the correct names, and a fixture invoking the hook with only the wrong names must NOT silently produce a destructive empty result. The wrong-param -> correct-param matrix lives in `references/hook-parameter-mismatches.md`.

## Hook test cases

Run the full 10-case matrix in `references/hook-test-cases.md` for every active hook. `transform_terminal_output` additionally requires the sentinel-output test (same reference file).

## Re-deriving a contract from source

Implementation properties are never trusted from prose, because they drift between Hermes releases:

```sh
grep -rn 'invoke_hook("transform_tool_result"' $HOME/.hermes/hermes-agent/ --include='*.py'
grep -rn 'invoke_hook("transform_terminal_output"' $HOME/.hermes/hermes-agent/ --include='*.py'
grep -n 'VALID_HOOKS' $HOME/.hermes/hermes-agent/hermes_cli/plugins.py
```

Read the call site's full kwargs, compare each name against the handler parameters, and confirm `**kwargs` is present. If a keyword changed, update this contract and its compatibility test before touching the plugin.

## References

Evidence notes moved here from `aphrodite-hook-reference` (keep as evidence; their Hermes v0.16.0 line numbers have drifted - treat as observational):

- `references/hook-invocations.md` - per-hook invocation reference (v0.16.0 coordinates; the pre_api_request row is stale, see boundary notes above)
- `references/hook-invocation-verification.md` - the source-verification recipe
- `references/hook-parameter-mismatches.md` - wrong-param incidents mapped to correct params (summary matrix included)
- `references/hook-test-cases.md` - the 10-case hook test matrix and the sentinel-output test

## Claim-to-test matrix

| Claim | Evidence source | Test | Pass condition | Failure response |
| --- | --- | --- | --- | --- |
| Hook receives `output`, not `stdout` | `tools/terminal_tool_result.py:144` | Sentinel-output invocation | Handler observes original sentinel | Update handler parameters and this contract |
| Hook registration uses `on_session_start` | `hermes_cli/plugins.py` VALID_HOOKS + `plugin.yaml` | Start session with registration log | One log entry appears | Inspect registration name/source |
| pre_tool_call is fail-closed on timeout | `hermes_cli/plugins_dispatch.py` `_HOOK_TIMEOUT_FAIL_CLOSED_HOOKS` | Stall a handler past the timeout and dispatch a tool | Tool blocked with the timeout message | Fix handler boundedness; re-derive dispatch code |
| `conversation_history` is a discardable copy | `agent/turn_context.py:702` | Mutate it in place; assert transcript unchanged | Live transcript unchanged | Move mutation to the context engine |
| Empty string replaces, never passes through | Return contracts above | Invoke with `output="x"`, return `""` | Result is empty (documented destructive replacement) | Fix handler to return the original output |
| pre_api_request now has a call site | `agent/turn_api_request.py:52-63` | Grep current Hermes source | Call site present (old "zero sites" claim is stale) | Update boundary notes; re-derive contract |
| Registered-but-never-invoked detection | VALID_HOOKS vs production `invoke_hook` sites | Script: registered names minus non-test invocation names | Zero dead registrations | Fix registration name or remove dead hook |