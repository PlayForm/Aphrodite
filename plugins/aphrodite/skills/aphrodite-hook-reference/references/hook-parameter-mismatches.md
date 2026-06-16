# Hook Parameter Mismatches Discovered

All mismatches between Hermes core `invoke_hook()` calls and plugin handler signatures discovered during aphrodite development.

## transform_terminal_output

**Hermes invokes** (tools/terminal_tool.py:2390-2396):
```python
invoke_hook("transform_terminal_output",
    command=command,
    output=output,          # ← NOT 'stdout'
    returncode=returncode,   # ← NOT 'exit_code'
    task_id=...,
    env_type=...,
)
```

**Plugin MUST declare**:
```python
def handler(command="", output="", returncode=0, **kwargs):
```

**WRONG** (causes ALL terminal output to return empty):
```python
def handler(command="", stdout="", stderr="", exit_code=0, **kwargs):
    # stdout defaults to "" ← Hermes never passes 'stdout'
    return stdout  # returns "" ← replaces real output with empty
```

## pre_llm_call

**Hermes invokes** (agent/turn_context.py:320-331):
```python
invoke_hook("pre_llm_call",
    conversation_history=list(messages),  # ← NOT 'api_messages' — AND it's a COPY
    user_message=original_user_message,   # ← NOT 'response'
    ...
)
```

**CRITICAL**: `conversation_history=list(messages)` creates a COPY. In-place mutations (pop, insert) are DISCARDED. To modify messages, use `ContextEngine` or `pre_api_request` (if available).

## post_llm_call

**Hermes invokes** (agent/turn_finalizer.py:294-304):
```python
invoke_hook("post_llm_call",
    conversation_history=list(messages),  # ← NOT 'api_messages' — COPY
    assistant_response=final_response,    # ← NOT 'response'
    turn_id=turn_id,                      # ← NOT 'turn_number' — UUID string, not int
    ...
)
```

## on_session_start

VALID_HOOKS uses `on_session_start`. Plugin registering as `session_start` will NOT be invoked.

**WRONG**: `ctx.register_hook("session_start", handler)`
**RIGHT**: `ctx.register_hook("on_session_start", handler)`

## How to Discover These

```bash
# Find the exact invocation
grep -rn '"hook_name"' ~/.hermes/hermes-agent/ --include='*.py' | grep -v tests/

# Read the invoke_hook() call site with full kwargs
# Compare each kwarg name to your handler's parameter names
```
