# Context Engine

The context engine is an optional integration point with Hermes' own
context-management slot. It is a thin, **non-destructive** engine: it records
token usage and defers compression decisions to Hermes, because the actual
compression already happens in the transform hooks and the compression proxy.
It exists so Hermes' context-engine slot is filled without double-compressing
the conversation.

## Activation

The plugin manifest declares the capability:

```yaml
# plugin.yaml
provides_context_engine: true
```

The engine instance is registered only when explicitly requested:

```bash
APHRODITE_CONTEXT_ENGINE=1
```

Without the flag the plugin registers hooks and tools only and runs the
hook + proxy path. If registration fails (for example the host does not
expose the `ContextEngine` base class), the plugin logs a warning and
continues on the hook + proxy path - registration is best-effort, never
fatal.

Hermes selects the registered engine by name; the plugin registers under the
name `aphrodite`.

## What the engine does

The registered engine is a small subclass of Hermes' `ContextEngine`:

| Method                 | Behavior                                                                                                       |
| ---------------------- | -------------------------------------------------------------------------------------------------------------- |
| `name`                 | `"aphrodite"`                                                                                                  |
| `update_from_response` | Records `prompt_tokens`, `completion_tokens`, and `total_tokens` from each response's usage payload            |
| `should_compress`      | Always returns `False` - defers to Hermes' own threshold accounting; the proxy and hooks do the heavy lifting  |
| `compress`             | Returns the transcript **unchanged** - non-destructive, because the transform hooks already shrink tool output |

The engine never forces a compaction, never rewrites messages, and never
touches the inline store. Compression of large tool and terminal output
happens in `transform_tool_result` and `transform_terminal_output` - see
[Plugin Hooks](https://github.com/PlayForm/Aphrodite/tree/Current/docs/plugin/hooks.md).

## Engine configuration

The dylib session carries engine-related settings, exposed through the
dylib's config surface (`aphrodite_stats`, `config_get`, `config_set`) for
observability. The standalone HTTP proxy never reads them and has no gated
behavior tied to them.

| Setting                     | Env Var                          | TOML key                       | Default |
| --------------------------- | -------------------------------- | ------------------------------ | ------- |
| Engine enabled (dylib flag) | `APHRODITE_CONTEXT_ENGINE`       | `[compression] context_engine` | true    |
| Threshold percent           | `APHRODITE_ENGINE_THRESHOLD_PCT` | `engine_threshold_pct`         | 45      |
| Protect first N             | `APHRODITE_ENGINE_PROTECT_FIRST` | `engine_protect_first`         | 2       |
| Protect last N              | `APHRODITE_ENGINE_PROTECT_LAST`  | `engine_protect_last`          | 5       |
| Min messages                | `APHRODITE_ENGINE_MIN_MSGS`      | `engine_min_msgs`              | 8       |

Threshold semantics (per the shipped manifest comment): `-1` always
compresses, `0` disables, `>0` is the fill percentage. Environment overrides
TOML, which overrides the default.

## See also

- [Plugin Hooks](https://github.com/PlayForm/Aphrodite/tree/Current/docs/plugin/hooks.md) - where the actual per-turn compression happens
- [aphrodite.toml Configuration](https://github.com/PlayForm/Aphrodite/tree/Current/docs/config/aphrodite-toml.md) - the `[compression]` section in context
- [Environment Variables](https://github.com/PlayForm/Aphrodite/tree/Current/docs/config/env-vars.md) - the env-var equivalents
