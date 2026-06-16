# Hook Invocation Verification

How to verify what parameters Hermes actually passes to hooks.

## Method: Search Hermes Source

```bash
# Find actual hook invocation
grep -rn '"hook_name"' ~/.hermes/hermes-agent/ --include='*.py' | grep -v tests/

# Read the invoke_hook() call site with surrounding context
# Example for transform_terminal_output:
grep -n "transform_terminal_output" ~/.hermes/hermes-agent/tools/terminal_tool.py
```

## Key Files

| Hook | File | Line (Hermes v0.16.0) |
|------|------|------------------------|
| on_session_start | agent/conversation_loop.py | ~331 |
| pre_llm_call | agent/turn_context.py | ~320 |
| post_llm_call | agent/turn_finalizer.py | ~294 |
| transform_terminal_output | tools/terminal_tool.py | ~2390 |
| transform_tool_result | model_tools.py | ~1175 |
| pre_api_request | NOT INVOKED | — |

## Common Pitfalls Discovered

1. **parameter name mismatch**: Plugin expects `stdout` but Hermes passes `output`. Fix: read the actual invoke_hook call in Hermes source, don't guess parameter names.

2. **hook name mismatch**: `session_start` vs `on_session_start`. The `on_` prefix is required. Check VALID_HOOKS in hermes_cli/plugins.py.

3. **copy semantics**: `conversation_history=list(messages)` creates a copy. Mutations to the parameter are discarded. Return a string for context injection instead.

4. **undefined variable in dead code**: `if _DEV: return result` where `result` is not a parameter. Would cause NameError if `_DEV` ever became True. Always verify both branches of dev/prod code paths.

5. **hook exists but never invoked**: `pre_api_request` is in VALID_HOOKS but has zero non-test invocation sites. Registering it is harmless but the callback never fires.

## Verification Recipe

1. Add debug logging to hook handler: `_log.warning("HOOK FIRED: %s params=%s", hook_name, list(kwargs.keys()))`
2. Restart Hermes
3. Trigger the hook (send a message for pre_llm, run a command for terminal, etc.)
4. Check logs for the debug message
5. If no message appears, the hook is not firing — check hook name and Hermes source
