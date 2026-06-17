# ContextEngine Integration for Hermes

Discovered during aphrodite v1.4.0-1.5.0 development (2026-06-15).

## Registration Flow

1. Plugin calls `ctx.register_context_engine(engine)` in register()
2. Hermes checks `isinstance(engine, ContextEngine)` — CRITICAL gate
3. If False: silently rejected with log warning, built-in compressor takes over
4. If True: stored on plugin manager's `_context_engine`
5. At session start, `agent_init.py` reads `context.engine` config
6. If engine name != "compressor", calls `get_plugin_context_engine()`
7. If found and name matches, replaces built-in ContextCompressor

## Key Files

- `agent/context_engine.py` — ContextEngine ABC (8 abstract methods)
- `agent/agent_init.py:1440-1500` — Engine selection logic
- `hermes_cli/plugins.py:502-526` — register_context_engine + isinstance check
- `agent/turn_context.py:280-314` — Preflight compression loop
- `agent/context_compressor.py` — Built-in ContextCompressor (fallback)

## Required Interface

```python
from agent.context_engine import ContextEngine

class MyEngine(ContextEngine):
    # Identity
    @property
    def name(self) -> str:  # MUST be @property, NOT class attribute
        return "my-engine"
    
    # Token tracking (called after each API response)
    def update_from_response(self, usage: dict) -> None:
        self.last_prompt_tokens = usage.get("prompt_tokens", 0)
    
    # Should we compress this turn?
    def should_compress(self, prompt_tokens=None) -> bool:
        return True  # always compress
    
    # Compress messages — return new (shorter) message list
    def compress(self, messages, current_tokens=None, focus_topic=None):
        head = messages[:2]
        tail = messages[-5:]
        middle = messages[2:-5]
        # Offload middle to CCR, return head + marker + tail
        return head + [marker] + tail
    
    # Optional lifecycle
    def update_model(self, model, context_length, base_url="", api_key="", provider="", api_mode=""): ...
    def get_status(self): ...
    def on_session_reset(self): ...
```

## Engine-to-Plugin Hooks

Engines fire Hermes hooks for other plugins to observe:

```python
from hermes_cli.plugins import invoke_hook
invoke_hook("aphrodite_engine_compressed", engine=self, stats={...})
```

Any plugin can listen:
```python
ctx.register_hook("aphrodite_engine_compressed", my_callback)
```

## Common Pitfalls

1. **Not inheriting from ContextEngine**: Silent failure — no error, just ignored
2. **name as class attr instead of @property**: isinstance still passes but property lookup fails
3. **update_model signature mismatch**: Wrong param count causes TypeError at startup
4. **Config set mid-session**: Engine only activates on NEW sessions
5. **pre_api_request hook**: Exists in VALID_HOOKS but never invoked — don't rely on it
6. **conversation_history in pre_llm_call**: It's a COPY — mutations are discarded
7. **Control chars in WezTerm MCP**: Send \x03 BEFORE new commands to avoid corruption
