---
name: aphrodite-hook-reference
description: "Use when implementing or debugging aphrodite Hermes hooks. Invocation parameters vs plugin expectations, return semantics, registration rules."
version: 1.3.0
platforms: [macos]
tags: [aphrodite, hermes, plugin, hooks, context-engine]
---

# Aphrodite Hook Reference

Exact Hermes hook invocations and the plugin parameters that must match. Every
known mismatch is listed below; when Hermes changes an invocation, update this
skill. Hook signatures MUST use catch-all `**kwargs` so renamed parameters
never break the plugin.

For the smoke-test workflow (`aphrodite_test`), see `aphrodite-benchmarking`.

## Hook Registration

In plugin.yaml:

```yaml
provides_hooks:
    - on_session_start # NOT session_start
    - pre_llm_call
    - post_llm_call
    - transform_terminal_output
    - transform_tool_result
```

In __init__.py register():

```python
ctx.register_hook("on_session_start", on_start)        # NOT "session_start"
ctx.register_hook("pre_llm_call", _pre_llm_hook)
ctx.register_hook("post_llm_call", _store_conversation_turn)
ctx.register_hook("transform_terminal_output", _transform_terminal_hook)
ctx.register_hook("transform_tool_result", _transform_tool_result)
```

Session hooks use the `on*` prefix (VALID_HOOKS in hermes_cli/plugins.py).

## Hook: on_session_start

**Invoked**: agent/conversation_loop.py, once per session start.

**Hermes passes**:

```python
invoke_hook("on_session_start",
    session_id=agent.session_id,
    model=agent.model,
    platform=getattr(agent, "platform", None) or "",
)
```

**Plugin receives**: `on_start(**kw)` - uses catch-all kwargs.

**Return value**: Ignored (fire-and-forget hook).

## Hook: pre_llm_call

**Invoked**: agent/turn_context.py, before each LLM turn.

**Hermes passes**:

```python
invoke_hook("pre_llm_call",
    session_id=agent.session_id,
    task_id=effective_task_id,
    turn_id=turn_id,
    user_message=original_user_message,              # NOT 'response'
    conversation_history=list(messages),              # NOT 'api_messages' (and it's a COPY)
    is_first_turn=(not bool(conversation_history)),
    model=agent.model,
    platform=getattr(agent, "platform", None) or "",
    sender_id=getattr(agent, "_user_id", None) or "",
)
```

**Plugin MUST declare**:

```python
def _pre_llm_hook(conversation_history=None, user_message=None, **kwargs):
```

**Return semantics**: Return a STRING to inject it as context into the user
message. Hermes calls `"\n\n".join(_ctx_parts)` and appends as
`plugin_user_context`.

**CRITICAL**: conversation_history is `list(messages)` - a COPY. In-place
mutations (pop, insert) are DISCARDED. To modify messages, use
ContextEngine.compress() instead.

## Hook: post_llm_call

**Invoked**: agent/turn_finalizer.py, after LLM response.

**Hermes passes**:

```python
invoke_hook("post_llm_call",
    session_id=agent.session_id,
    task_id=effective_task_id,
    turn_id=turn_id,                                  # NOT 'turn_number'
    user_message=original_user_message,
    assistant_response=final_response,                 # NOT 'response'
    conversation_history=list(messages),               # NOT 'api_messages' (COPY)
    model=agent.model,
    platform=getattr(agent, "platform", None) or "",
)
```

**Plugin MUST declare**:

```python
def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
```

**Note**: Hermes' turn_id is a UUID string (`session_id:task_id:uuid`). Use a
sequential counter for human-readable memory entries.

## Hook: transform_terminal_output

**Invoked**: tools/terminal_tool.py, after every terminal command.

**Hermes passes**:

```python
invoke_hook("transform_terminal_output",
    command=command,
    output=output,              # NOT 'stdout' - this was the terminal-output bug
    returncode=returncode,      # NOT 'exit_code'
    task_id=effective_task_id or "",
    env_type=env_type,
)
```

**Plugin MUST declare**:

```python
def _transform_terminal_hook(command="", output="", returncode=0, **kwargs):
```

**Return semantics**: Return a STRING to REPLACE the terminal output. First
non-None string wins. Returning empty string REPLACES output with empty - MUST
return the `output` parameter for pass-through.

## Hook: transform_tool_result

**Invoked**: model_tools.py, after every tool call returns.

**Hermes passes**:

```python
invoke_hook("transform_tool_result",
    tool_name=function_name, args=function_args, result=result,
    tool_call_id=tool_call_id or "", task_id=task_id or "",
    session_id=session_id or "", turn_id=turn_id or "",
    api_request_id=api_request_id or "", duration_ms=duration_ms,
    status=status, error_type=error_type, error_message=error_message,
)
```

**Plugin MUST declare**:

```python
def _transform_tool_result(
    tool_name="", args=None, result="", tool_call_id="",
    task_id="", session_id="", turn_id="", api_request_id="",
    duration_ms=0, status="", error_type="", error_message="",
    **kwargs,
):
```

**Return semantics**: Return a STRING to REPLACE the tool result. First
non-None string wins.

## Known Mismatches (wrong param -> correct param)

| Hook                      | Wrong Param                         | Correct Param                                     | Effect                            |
| ------------------------- | ----------------------------------- | ------------------------------------------------- | --------------------------------- |
| transform_terminal_output | stdout, stderr, exit_code           | output, returncode                                | ALL terminal output empty         |
| pre_llm_call              | api_messages, response              | conversation_history, user_message                | Hook returned early (None guard)  |
| post_llm_call             | api_messages, response, turn_number | conversation_history, assistant_response, turn_id | Hook returned early              |
| on_session_start          | session_start (hook name)           | on_session_start                                  | Proxy never auto-launched         |

## CCR Marker Format

Standard ASCII format - compatible across all terminals, shells, and LLM
tokenizers:

```
<<<CCR:{hash}|{type}|{size}|{mode}>>> {preview}
```

| Hook                      | type     | mode               |
| ------------------------- | -------- | ------------------ |
| transform_tool_result     | tool     | token/cache/inline |
| transform_terminal_output | terminal | (or inline)        |

Parsed by regex: `r'<<<CCR:([^>]+)>>>'`

**Why ASCII `<<<>>>` not Unicode `⫷⫸` or `[...]`:**

- `<<<...>>>` is pure ASCII - works in every terminal, LLM tokenizer, and log
  viewer
- `[CCR:...]` collides with JSON arrays and markdown link syntax, confusing LLM
  parsers
- Unicode brackets `⫷⫸` break on terminals without full Unicode support and
  confuse BPE tokenizers
- Rust source uses literal `<<<CCR:` / `>>>` in string literals - no escapes:
  `format!("<<<CCR:{}|{}|{}>>> {}", hash, ct, size, oneliner)`

**Marker format consistency**: every code path that produces or consumes CCR
markers MUST use the same format. When changing it, update: the `_CCR_RE`
regex, `_ccr_marker()` (Python), `smart_marker()` (Rust proxy.rs), inline
format strings in `_transform_tool_result` / `_transform_terminal_hook`,
`_resolve_recursive` replacement logic, `_retrieve_handler` startswith check,
and the tool-injection description in `compress_chat_completion`.

## ContextEngine Registration

`ctx.register_context_engine(engine)` does `isinstance(engine, ContextEngine)` -
a plain class is SILENTLY rejected ("does not inherit from ContextEngine.
Ignoring."). The engine MUST subclass `ContextEngine` and expose `name` as a
`@property`.

The engine is opt-in: register it only when `APHRODITE_CONTEXT_ENGINE=1` is set
(plugin default is hooks + proxy only, no engine):

```python
engine_configured = os.environ.get("APHRODITE_CONTEXT_ENGINE", "") == "1"
if engine_configured:
    ctx.register_context_engine(AphroditeContextEngine())
```

Enable: `APHRODITE_CONTEXT_ENGINE=1 hermes config set context.engine aphrodite`
Disable: `hermes config set context.engine default` (engine not registered
without the env var)

**Never leave Hermes' built-in compression on** (`compression.enabled: true` in
config.yaml) - it runs independently of the aphrodite engine and causes
constant `🗜️ Compacting context` messages. Always
`hermes config set compression.enabled false` when using aphrodite.

Example engine:

```python
from agent.context_engine import ContextEngine

class MyEngine(ContextEngine):
    @property
    def name(self) -> str:  # MUST be @property, not class attribute
        return "my-engine"

    def should_compress(self, prompt_tokens=None) -> bool: ...
    def compress(self, messages, current_tokens=None, focus_topic=None): ...
    def update_from_response(self, usage): ...
    def update_model(self, model, context_length, base_url="", api_key="", provider="", api_mode=""): ...
```

Activate: `hermes config set context.engine my-engine`
Revert: `hermes config set context.engine compressor`

## Engine-to-Plugin Hooks

Engines can fire Hermes hooks that other plugins listen to:

```python
from hermes_cli.plugins import invoke_hook
invoke_hook("my_engine_compressed", engine=self, stats={...})
```

Other plugins: `ctx.register_hook("my_engine_compressed", callback)`

## compress() Safety Rules

**Never split a tool_call from its tool_result.** When `compress()` splits
messages into head/middle/tail, the tail boundary can orphan a tool result.
Extend the tail to include orphan tool results:

```python
boundary = len(messages) - tail_n
while boundary < len(messages) and messages[boundary].get("role") == "tool":
    boundary += 1
    tail_n += 1
```

**Never let retrieval tools get compressed.** `_transform_tool_result`
compresses ALL tool outputs >1KB; if `aphrodite_retrieve` or `headroom_stats`
output is re-compressed, the LLM sees another CCR marker instead of content -
an infinite loop. The skip list MUST include retrieval tools:

```python
skip = {"read_file", "read_terminal", "aphrodite_retrieve", "headroom_stats"}
```

## pre_api_request does NOT exist

`pre_api_request` is in VALID_HOOKS but has zero invocation sites in Hermes
source - registering it is a no-op. Never use it for message compression. Use
`ContextEngine.compress()` instead, which receives the actual mutable message
list and returns a shortened one.

## turn_id is NOT a sequential number

Hermes `turn_id` is `{session_id}:{task_id}:{uuid}` - a UUID string, not a
counter. Never use it as a dict key for conversation memory; keep your own
sequential `_turn_counter` for memory indexing.

## Content-Addressable Store Pattern

Every compression operation first checks the local cache before hitting the
proxy ("pop the API"). Same content → same SHA256 hash → cache hit → no API
call.

```python
h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
if h in _inline_store:
    return cached_result  # cache hit
# Only call proxy on miss
```

Applied in `_compress_handler` and `_transform_tool_result` CCR path;
`_resolve_one` also caches retrieved content in `_inline_store`.

## Bi-Directional Store

All operations feed the search index:

- Compress → `_inline_store[hash] = content` + `_recent_markers.append({...})`
- Retrieve → `_inline_store[hash] = content` + `_recent_markers.append({...})`
- Search → scans `_inline_store` + `_conv_index` + `_recent_markers`, returns
  actionable results

Content that enters the system must be findable - never let an operation go
one-way.

## Proxy Health Check - Decoupled Pattern

`/health` (GET) - local-only check, no upstream API call:

```rust
pub async fn health_check(State(state): ...) -> impl IntoResponse {
    let ccr_ok = state.ccr.is_some();
    Json(json!({"status": if ccr_ok { "healthy" } else { "degraded" }, ...}))
}
```

`/health/upstream` (GET) - separate endpoint for the DeepSeek API probe; used
for diagnostics, not hot-path health checks. Python `_alive()` caches results
with a 5-second TTL to avoid per-turn socket overhead.

## Source Files

Key files for verifying hook signatures:

- agent/turn_context.py - pre_llm_call
- agent/turn_finalizer.py - post_llm_call
- agent/conversation_loop.py - on_session_start
- tools/terminal_tool.py - transform_terminal_output
- model_tools.py - transform_tool_result
- agent/agent_init.py - context engine selection (context.engine config)
- agent/context_engine.py - ContextEngine ABC
- hermes_cli/plugins.py - register_context_engine (isinstance check)

## Reference Files

- `references/ccr-marker-format.md` - CCR marker format, regex, pipeline flow,
  thresholds
- `references/health-check-pattern.md` - `_alive()` 5s TTL cache, retry loop,
  Rust health decoupling
- `references/hook-invocations.md` - full hook invocation reference
- `references/hook-parameter-mismatches.md` - wrong-param incidents mapped to
  correct params
- `references/context-engine-api.md`, `references/context-engine-integration.md`,
  `references/context-engine-pitfalls.md` - ContextEngine API and registration
- `references/recompression-guard.md`, `references/ccr-infinite-recursion.md` -
  the retrieval-tool skip list and recompression loop