# Session Discoveries — 2026-06-15

## Critical Bugs Found

### 1. Transform Terminal Output — stdout vs output
**File**: tools/terminal_tool.py line 2393
**Bug**: Hermes passes `output=` but plugin expected `stdout=`
**Effect**: ALL terminal output returned empty (default "" replaced real content)
**Fix**: Changed parameter to `output=`, `returncode=` (removed stderr, exit_code)

### 2. pre_llm_call — conversation_history is a COPY
**File**: agent/turn_context.py line 326
**Bug**: `conversation_history=list(messages)` — mutations discarded
**Effect**: Message compression in pre_llm_call is impossible
**Fix**: Return context string instead. Use ContextEngine for actual message removal.

### 3. ContextEngine — isinstance check
**File**: hermes_cli/plugins.py line 518-525
**Bug**: Engine must inherit from ContextEngine (ABC) or silently rejected
**Log**: "does not inherit from ContextEngine. Ignoring."
**Fix**: `class AphroditeContextEngine(ContextEngine):` with `@property name`

### 4. CCR Compression Loop
**Bug**: headroom_retrieve/headroom_stats output not in skip list
**Effect**: Retrieve results re-compressed → infinite CCR nesting
**Fix**: Added to skip list in _transform_tool_result

### 5. Session vs Hook Name
**Bug**: `register_hook("session_start", ...)` vs Hermes `"on_session_start"`
**Effect**: Proxy never auto-launched by plugin
**Fix**: Changed to `"on_session_start"`

### 6. Hermes turn_id is UUID String
**Bug**: `turn_id` from post_llm_call is `session_id:task_id:uuid`
**Effect**: Memory showed UUIDs instead of sequential numbers
**Fix**: Use `_turn_counter += 1` for human-readable T1, T2, T3...

### 7. Stale _DEV Guards
**Bug**: `if _DEV: return result` where `result` undefined in scope
**Effect**: Would crash with NameError if _DEV ever True
**Fix**: Removed or replaced with plain `return`

### 8. Tool-Chain Boundary Split
**Bug**: ContextEngine.compress() could split tool_call→tool_result
**Effect**: LLM loses context of tool call chain
**Fix**: Extend tail boundary to include orphan tool_results

## Hook Source Files (for verification)

- agent/turn_context.py:320-331 — pre_llm_call invocation
- agent/turn_finalizer.py:294-304 — post_llm_call invocation
- agent/conversation_loop.py:331-336 — on_session_start invocation
- tools/terminal_tool.py:2390-2396 — transform_terminal_output invocation
- model_tools.py:1175-1189 — transform_tool_result invocation
- agent/agent_init.py:1440-1500 — context engine selection
- hermes_cli/plugins.py:502-526 — register_context_engine (isinstance check)
- agent/context_engine.py:32-226 — ContextEngine ABC
