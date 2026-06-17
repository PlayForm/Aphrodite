# Hermes Hook Invocation Reference (verified against Hermes v0.16.0 source)

## on_session_start

**File**: agent/conversation_loop.py:331-336 **Params**: session_id, model,
platform **Return**: ignored (fire-and-forget)

## pre_llm_call

**File**: agent/turn_context.py:320-331 **Params**: session_id, task_id,
turn_id, user_message, conversation_history (COPY), is_first_turn, model,
platform, sender_id **Return**: string → injected into user message as context.
Dict with "context" key also accepted. **CRITICAL**: conversation_history is
`list(messages)` — in-place mutations are DISCARDED.

## post_llm_call

**File**: agent/turn_finalizer.py:294-304 **Params**: session_id, task_id,
turn_id, user_message, assistant_response, conversation_history (COPY), model,
platform **Return**: ignored **NOTE**: turn_id is
`{session_id}:{task_id}:{uuid}` — NOT sequential.

## transform_terminal_output

**File**: tools/terminal_tool.py:2390-2396 **Params**: command, output,
returncode, task_id, env_type **Return**: string REPLACES the terminal output.
First non-None string wins.

## transform_tool_result

**File**: model_tools.py:1175-1189 **Params**: tool_name, args, result,
tool_call_id, task_id, session_id, turn_id, api_request_id, duration_ms, status,
error_type, error_message **Return**: string REPLACES the tool result.

## pre_api_request

**INVALID**: In VALID_HOOKS but has ZERO invocation sites. Cannot be used. Use
ContextEngine.compress() instead for message compression.
