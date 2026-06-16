# ContextEngine Integration Discoveries

Session: 2026-06-15 — Comprehensive aphrodite plugin debugging and enhancement.

## Critical Discovery: ContextEngine Inheritance Check

Hermes `register_context_engine()` at `hermes_cli/plugins.py:518-525` checks:
```python
from agent.context_engine import ContextEngine
if not isinstance(engine, ContextEngine):
    logger.warning("...does not inherit from ContextEngine. Ignoring.")
    return
```

**Without this inheritance, the engine is silently ignored.** The `name` must be a `@property` (abstract in base class), not a class attribute. `update_model()` signature must match the abstract (7 params).

## Hook Parameter Verification Method

The reliable way to verify hook parameters:
1. `grep -rn '"hook_name"' ~/.hermes/hermes-agent/ --include='*.py' | grep -v tests/`
2. Read the `invoke_hook()` call with surrounding 30 lines of context
3. Match EVERY parameter name exactly — Hermes passes kwargs, mismatched names default to empty

Key files for each hook:
- `agent/turn_context.py:320` — pre_llm_call
- `agent/turn_finalizer.py:294` — post_llm_call  
- `agent/conversation_loop.py:331` — on_session_start
- `tools/terminal_tool.py:2391` — transform_terminal_output
- `model_tools.py:1175` — transform_tool_result
- `hermes_cli/plugins.py:502` — register_context_engine

## Output Suppression Root Cause

The "terminal output always empty" bug was NOT a sandbox/proxy issue. Root cause:
- Hermes passes `output=` to transform_terminal_output hook (terminal_tool.py:2393)
- Plugin expected `stdout=` parameter — defaulted to `""`
- Hook returned `""` for ALL outputs
- Hermes replaced real output with empty string

The fix: change hook signature from `stdout=, stderr=, exit_code=` to `output=, returncode=`.

## Context Engine vs pre_llm_call

- `pre_llm_call`: receives COPY of conversation_history — mutations discarded, can only return context string
- `ContextEngine.compress()`: receives and returns the ACTUAL message list — mutations take effect, this is where messages get removed
- `should_compress()`: controls when compression fires. Returns True to always compress (emulate token proxy internally)

## Custom Hook Firing

`invoke_hook()` does NOT validate against VALID_HOOKS. Any hook name works. Plugins can fire custom hooks for other plugins to listen to:
```python
from hermes_cli.plugins import invoke_hook
invoke_hook("aphrodite_engine_compressed", engine=self, stats=...)
```

## Turn ID vs Sequential Counter

Hermes `turn_id` is `f"{session_id}:{task_id}:{uuid.uuid4().hex[:8]}"` — a complex string, not a number. For conversation memory display, use an internal `_turn_counter` that increments sequentially to show `T1, T2, T3` instead of UUIDs.
