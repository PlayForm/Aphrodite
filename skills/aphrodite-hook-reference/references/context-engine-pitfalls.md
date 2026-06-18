# Context Engine Registration Pitfalls

Discovered 2026-06-15 during aphrodite plugin development.

## Silent Rejection: isinstance check

`ctx.register_context_engine()` in `hermes_cli/plugins.py:518-525` silently
rejects engines that don't pass `isinstance(engine, ContextEngine)`. The engine
is discarded with only a log warning.

```python
# hermes_cli/plugins.py:518-525
from agent.context_engine import ContextEngine
if not isinstance(engine, ContextEngine):
    logger.warning("Plugin '%s' tried to register a context engine that does not "
                   "inherit from ContextEngine. Ignoring.", self.manifest.name)
    return
self._manager._context_engine = engine
```

## Requirements to pass:

1. MUST inherit from `agent.context_engine.ContextEngine`
2. `name` MUST be a `@property` (it's an abstract property on the ABC, not a
   class attribute)
3. `update_model` signature MUST match the ABC (7 params: model, context_length,
   base_url, api_key, provider, api_mode, \*\*kw)
4. `should_compress(prompt_tokens)` - return bool
5. `compress(messages, current_tokens, focus_topic)` - return modified message
   list
6. `update_from_response(usage)` - track token usage

## Wrong (silently rejected):

```python
class MyEngine:
    name = "myengine"  # class attr, not @property
    # doesn't inherit from ContextEngine → isinstance fails
```

## Right:

```python
from agent.context_engine import ContextEngine

class MyEngine(ContextEngine):
    @property
    def name(self) -> str:
        return "myengine"

    def should_compress(self, prompt_tokens=None) -> bool:
        return True

    def compress(self, messages, current_tokens=None, focus_topic=None):
        # compress and return shorter message list
        return messages

    def update_from_response(self, usage):
        self.last_prompt_tokens = usage.get("prompt_tokens", 0)

    def update_model(self, model="", context_length=0, base_url="", api_key="", provider="", api_mode="", **kw):
        if context_length:
            self.context_length = context_length
```

## Discovery

Our `AphroditeContextEngine` was registered and running in the plugin, but
`compress()` was never actually called. Debugging revealed it was silently
rejected because it was a plain class (not subclassing `ContextEngine`). After
adding inheritance + @property for name, the engine is properly selected when
`context.engine: aphrodite` is set in config.yaml.
