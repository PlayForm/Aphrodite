---
name: aphrodite-hook-reference
description:
    "Complete Hermes hook API reference for aphrodite plugin: exact invocation
    parameters vs plugin expectations, return semantics, known mismatches, hook
    registration rules. Includes ContextEngine registration pitfalls."
version: 1.2.0
platforms: [macos]
---

# Aphrodite Hook Reference (v1.9.0+ CCR format)

Exact Hermes hook invocations and the plugin parameters that must match. Every
mismatch discovered so far and how to detect new ones.

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

In **init**.py register():

```python
ctx.register_hook("on_session_start", on_start)        # NOT "session_start"
ctx.register_hook("pre_llm_call", _pre_llm_hook)
ctx.register_hook("post_llm_call", _store_conversation_turn)
ctx.register_hook("transform_terminal_output", _transform_terminal_hook)
ctx.register_hook("transform_tool_result", _transform_tool_result)
```

VALID*HOOKS reference (from hermes_cli/plugins.py): session hooks use 'on*'
prefix.

## Hook: on_session_start

**Invoked**: agent/conversation_loop.py:331-336, once per session start.

**Hermes passes**:

```python
invoke_hook("on_session_start",
    session_id=agent.session_id,
    model=agent.model,
    platform=getattr(agent, "platform", None) or "",
)
```

**Plugin receives**: `on_start(**kw)` — uses catch-all kwargs.

**Return value**: Ignored (fire-and-forget hook).

## Hook: pre_llm_call

**Invoked**: agent/turn_context.py:320-331, before each LLM turn.

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

**Return semantics**: Return a STRING to inject it as context into user message.
Hermes calls `"\n\n".join(_ctx_parts)` and appends as `plugin_user_context`.

**CRITICAL**: conversation_history is `list(messages)` — a COPY. In-place
mutations (pop, insert) are DISCARDED. To modify messages, use
ContextEngine.compress() instead.

## Hook: post_llm_call

**Invoked**: agent/turn_finalizer.py:294-304, after LLM response.

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

**Note**: Hermes' turn_id is a UUID string like `session_id:task_id:uuid`. Use a
sequential counter for human-readable memory entries.

## Hook: transform_terminal_output

**Invoked**: tools/terminal_tool.py:2390-2396, after every terminal command.

**Hermes passes**:

```python
invoke_hook("transform_terminal_output",
    command=command,
    output=output,              # NOT 'stdout' — this was the terminal-output bug
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
non-None string wins. Returning empty string REPLACES output with empty — MUST
return `output` parameter for pass-through.

## Hook: transform_tool_result

**Invoked**: model_tools.py:1175-1189, after every tool call returns.

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

**Return semantics**: Return a STRING to REPLACE tool result. First non-None
string wins.

## CCR Marker Format

## CCR Marker Format

Standard ASCII format — compatible across all terminals, shells, and LLM
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

- `<<<...>>>` is pure ASCII — works in every terminal, LLM tokenizer, and log
  viewer
- `[CCR:...]` collides with JSON arrays and markdown link syntax, confusing LLM
  parsers
- Unicode brackets `⫷⫸` break on terminals that don't support full Unicode, and
  confuse BPE tokenizers
- The triple-angle-bracket convention is widely recognized (here-docs, C++
  templates, etc.)
- Rust source files use literal `<<<CCR:` and `>>>` characters directly in
  string literals — no escape sequences needed **Rust format string**:
  `format!("<<<CCR:{}|{}|{}>>> {}", hash, ct, size, oneliner)`

## Known Mismatches (Fixed)

| Hook                      | Wrong Param                         | Correct Param                                     | Effect                           |
| ------------------------- | ----------------------------------- | ------------------------------------------------- | -------------------------------- |
| transform_terminal_output | stdout, stderr, exit_code           | output, returncode                                | ALL terminal output empty        |
| pre_llm_call              | api_messages, response              | conversation_history, user_message                | Hook returned early (None guard) |
| post_llm_call             | api_messages, response, turn_number | conversation_history, assistant_response, turn_id | Hook returned early              |
| on_session_start          | session_start (hook name)           | on_session_start                                  | Proxy never auto-launched        |

## Hermes turn_id is NOT a sequential number

Hermes `turn_id` is `{session_id}:{task_id}:{uuid}` — a UUID string, not a
counter. Use your own sequential `_turn_counter` for memory indexing. Do NOT use
Hermes' turn_id as a dict key for conversation memory.

## pre_api_request hook does NOT exist

`pre_api_request` is in VALID_HOOKS but has zero invocation sites in Hermes
source. It cannot be used for message compression. Use
`ContextEngine.compress()` instead, which receives the actual mutable message
list and returns a shortened one.

## ContextEngine registration pitfall: must inherit ContextEngine

`ctx.register_context_engine(engine)` does `isinstance(engine, ContextEngine)`.
If the engine is a plain class (not a subclass), it is SILENTLY rejected with a
log warning: "does not inherit from ContextEngine. Ignoring."

Fix: `class MyEngine(ContextEngine)` and use `@property` for `name`.

## headroom tools MUST be excluded from compression

`_transform_tool_result` compresses all tool outputs >1KB. If
`aphrodite_retrieve` output gets compressed, the LLM sees another CCR marker
instead of actual content — causing infinite recursion. Add headroom tools to
the skip list:

```python
skip = {"read_file", "read_terminal", "aphrodite_retrieve", "headroom_stats"}
```

## Tool-chain safety in context engine compress()

When `compress()` splits messages into head/middle/tail, it must NOT split a
`tool_call` from its `tool_result`. If messages[boundary] has role="tool",
extend the tail backwards to include the orphan tool results.

## CCR Marker Format Consistency Across Codebase

Every code path that produces or consumes CCR markers MUST use the same format.
The master format is defined in `_ccr_marker()` (Python) and `smart_marker()`
(Rust). Both functions MUST produce identical output. When changing the format:

1. Update `_CCR_RE` regex in Python `__init__.py`
2. Update `_ccr_marker()` function
3. Update `smart_marker()` in Rust `proxy.rs`
4. Update all inline format strings in `_transform_tool_result`,
   `_transform_terminal_hook`
5. Update `_resolve_recursive` replacement logic
6. Update `_retrieve_handler` startswith check
7. Update `_parse_ccr_markers` docstring
8. Update tool injection description in `compress_chat_completion`
9. Update docstrings referencing the marker format
10. There are NO other places — if you find one, add it to this checklist

## Proxy Health Check — Decoupled Pattern

`/health` (GET) — local-only check, no upstream API call:

```rust
// Returns immediately — just checks CCR backend is alive
pub async fn health_check(State(state): ...) -> impl IntoResponse {
    let ccr_ok = state.ccr.is_some();
    Json(json!({"status": if ccr_ok { "healthy" } else { "degraded" }, ...}))
}
```

`/health/upstream` (GET) — separate endpoint for DeepSeek API probe:

```rust
// Used for diagnostics, not hot-path health checks
.route("/health/upstream", get(|| async { ... probes api.deepseek.com/models ... }))
```

Python `_alive()` caches results with 5-second TTL to avoid per-turn socket
overhead.

## ContextEngine Registration (v1.4.0+, opt-in from v1.26.0+)

Hermes has a pluggable context engine system via
`ctx.register_context_engine(engine)`.

**As of v1.26.0**: Context engine is opt-in. The plugin only registers the
engine when `APHRODITE_CONTEXT_ENGINE=1` is set. Default mode is hooks + proxy
only — no engine.

```python
engine_configured = os.environ.get("APHRODITE_CONTEXT_ENGINE", "") == "1"
if engine_configured:
    ctx.register_context_engine(AphroditeContextEngine())
```

To enable:
`APHRODITE_CONTEXT_ENGINE=1 hermes config set context.engine aphrodite` To
disable: `hermes config set context.engine default` (engine not registered
without env var)

**CRITICAL**: Hermes' built-in compression (`compression.enabled: true` in
config.yaml) runs independently of the aphrodite engine. If enabled, it causes
constant `🗜️ Compacting context` messages. Always ensure
`hermes config set compression.enabled false` when using aphrodite.

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

Applied in `_compress_handler` and `_transform_tool_result` CCR path. Also used
in `_resolve_one` — retrieved content is cached in `_inline_store` for future
searches.

## Bi-Directional Store

All operations feed the search index:

- Compress → `_inline_store[hash] = content` + `_recent_markers.append({...})`
- Retrieve → `_inline_store[hash] = content` + `_recent_markers.append({...})`
- Search → scans `_inline_store` + `_conv_index` + `_recent_markers`, returns
  actionable results

Result: compress → search → retrieve → cache → search again finds everything. No
operation goes one-way. Content that enters the system must be findable.

## Smoke Test Tool (aphrodite_test)

8th tool in the plugin. 4 modes: quick (9 tests), full (13), matrix (settings
sweep), pipeline (feature toggles + regression). Saves `.test-results.json` for
regression comparison.

See `aphrodite-dev-workflow` skill → `references/smoke-test-pipeline.md` for
full reference.

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

Activate: `hermes config set context.engine my-engine` Revert:
`hermes config set context.engine compressor`

## Engine-to-Plugin Hooks

Engines can fire Hermes hooks that other plugins listen to:

```python
from hermes_cli.plugins import invoke_hook
invoke_hook("my_engine_compressed", engine=self, stats={...})
```

Other plugins: `ctx.register_hook("my_engine_compressed", callback)`

## CCR Compression Loop (CRITICAL)

`_transform_tool_result` compresses ALL tool outputs >1KB. If
`aphrodite_retrieve` or `headroom_stats` are NOT in the skip list, their output
gets re-compressed — creating an infinite loop where retrieve results are always
CCR markers instead of content.

**Skip list MUST include retrieval tools**:

```python
skip = {"read_file", "read_terminal", "aphrodite_retrieve", "headroom_stats"}
```

Without this, the agent sees `<<<CCR:hash|tool|size>>>` markers instead of
actual content, making debugging impossible. Every retrieve call produces
another compressed result.

## Tool-Chain Safety in ContextEngine.compress()

When the context engine compresses messages, it keeps `protect_last_n` tail
messages raw. But the tail boundary can split a `tool_call→tool_result` pair,
breaking context. Fix: scan for orphan tool_results at the boundary and extend
the tail to include them.

```python
boundary = len(messages) - tail_n
while boundary < len(messages) and messages[boundary].get("role") == "tool":
    boundary += 1
    tail_n += 1
```

## pre_api_request — NOT Invoked

Defined in VALID_HOOKS but has NO invocation sites in non-test code. Registering
is a no-op. Use ContextEngine.compress() for message modification instead.

## Source Files

Key files for verifying hook signatures:

- agent/turn_context.py — pre_llm_call
- agent/turn_finalizer.py — post_llm_call
- agent/conversation_loop.py — on_session_start
- tools/terminal_tool.py — transform_terminal_output (line 2390-2396)
- model_tools.py — transform_tool_result
- agent/agent_init.py — context engine selection (context.engine config, line
  1440-1500)
- agent/context_engine.py — ContextEngine ABC
- hermes_cli/plugins.py — register_context_engine (isinstance check at line
  518-525)

## Reference Files

- `references/ccr-marker-format.md` — CCR marker format, regex, pipeline flow,
  thresholds
- `references/health-check-pattern.md` — \_alive() 5s TTL cache, retry loop,
  Rust health decoupling
