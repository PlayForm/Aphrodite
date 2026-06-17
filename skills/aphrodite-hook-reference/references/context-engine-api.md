# Hermes Context Engine API

Hermes v0.16.0+ supports pluggable context engines that can replace the built-in
ContextCompressor. This is the ONLY hook point where messages can actually be
REMOVED from the API payload.

## Registration

In plugin's `register(ctx)`:

```python
from agent.context_engine import ContextEngine

class MyEngine(ContextEngine):
    @property
    def name(self) -> str:
        return "my-engine"

    def should_compress(self, prompt_tokens=None):
        threshold = int(self.context_length * 0.30)
        return prompt_tokens is not None and prompt_tokens > threshold

    def compress(self, messages, current_tokens=None, focus_topic=None):
        # Return modified (possibly shorter) message list
        return messages

    def update_from_response(self, usage):
        self.last_prompt_tokens = usage.get("prompt_tokens", 0)
        self.last_completion_tokens = usage.get("completion_tokens", 0)
        self.last_total_tokens = usage.get("total_tokens", 0)

ctx.register_context_engine(MyEngine())
```

Config: `context.engine: my-engine` in config.yaml.

## Required Abstract Methods

- `name` — short identifier string
- `update_from_response(usage)` — called after each API response with token
  counts
- `should_compress(prompt_tokens)` — return True to trigger compression
- `compress(messages, current_tokens, focus_topic)` — main compression entry,
  returns modified list

## Optional Methods

- `should_compress_preflight(messages)` — quick pre-API check
- `has_content_to_compress(messages)` — gateway /compress guard
- `on_session_start(session_id, **kwargs)` — session lifecycle
- `on_session_end(session_id, messages)` — flush state
- `on_session_reset()` — reset counters
- `get_tool_schemas()` — expose engine tools to agent
- `handle_tool_call(name, args)` — handle engine tool calls
- `get_status()` — display/logging status
- `update_model(model, context_length, ...)` — model switch handling

## Default Attributes (set on instance)

- `last_prompt_tokens`, `last_completion_tokens`, `last_total_tokens` — token
  tracking
- `threshold_tokens`, `context_length` — compression thresholds
- `compression_count` — counter
- `threshold_percent: 0.75` — override for custom trigger ratio
- `protect_first_n: 3` — head messages always preserved
- `protect_last_n: 6` — tail messages always preserved

## Discovery

Source files:

- `/agent/context_engine.py` — ContextEngine base class
- `/agent/agent_init.py:1440-1500` — engine selection logic
- `/hermes_cli/plugins.py:502` — `register_context_engine()`
- `/hermes_cli/plugins.py:1920` — `get_plugin_context_engine()`

Selection order:

1. Config `context.engine` setting
2. `plugins/context_engine/<name>/` directory (repo-shipped)
3. `get_plugin_context_engine()` (general plugin system)
4. Fall back to built-in ContextCompressor

## Key Difference from pre_llm_call

`pre_llm_call` hook receives `conversation_history=list(messages)` — a COPY.
Mutations are discarded. The hook can only RETURN a context string.

`compress()` receives the ACTUAL message list and can modify it in place. This
is where real message removal happens.
