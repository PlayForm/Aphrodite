# Context Engine

The context engine is the dylib session's middle-turn archiver: when the
conversation index fills up, it packs the middle turns into one compact CCR
entry and replaces them with a single marker, so the model sees a short
history (head + marker + tail) instead of a growing list of archived turns.

The archive is **in-place but recoverable**: the middle turns are removed
from Aphrodite's own conversation index and stored (BLAKE3-keyed) in the
inline/CCR store, where `aphrodite_retrieve` can restore them. The engine
never touches Hermes' message store - Hermes holds the raw conversation; the
engine compacts Aphrodite's archived-turn index only.

## Activation

The plugin manifest declares the capability:

```yaml
# plugin.yaml
provides_context_engine: true
```

The plugin registers the engine instance only when explicitly requested:

```bash
APHRODITE_CONTEXT_ENGINE=1
```

Without the flag the plugin registers hooks and tools only and runs the
hook + proxy path, and the dylib session never consults the engine. If
registration fails (for example the host does not expose the `ContextEngine`
base class), the plugin logs a warning and continues on the hook + proxy
path - registration is best-effort, never fatal.

Hermes selects the registered engine by name; the plugin registers under the
name `aphrodite`.

## What the engine does

The engine runs inside the dylib session, triggered by the `pre_llm_call`
context assembler each turn. All gates run **before any mutation**; when any
gate fails the engine is a no-op and the session is byte-identical to an
engine-less run.

| Gate           | Condition for archiving                                                                           |
| -------------- | ------------------------------------------------------------------------------------------------- |
| Engine enabled | `context_engine_enabled` is true (`[compression] context_engine`, env `APHRODITE_CONTEXT_ENGINE`) |
| Threshold      | `engine_threshold_pct` is neither `0` (never) nor `>= 100` (off escape hatch)                     |
| Minimum turns  | `conv_index` length is at least `engine_min_msgs` (default 8)                                     |
| Fill ratio     | `total * 100 / CONV_INDEX_CAP` (`CONV_INDEX_CAP` = 50) is at or above `engine_threshold_pct`      |
| Middle size    | `total - engine_protect_first - engine_protect_last` is at least 3                                |

When all gates pass:

1. The middle slice - keeping exactly `engine_protect_first` turns at the
   start and `engine_protect_last` at the end, oldest first - is packed into
   **one compact JSON payload** with one entry per turn
   (`{"turn", "hash", "summary", "size"}`).
2. The payload is hashed with BLAKE3 (`headroom_core::ccr::compute_key`),
   stored in the inline/CCR store, and summarized as
   `engine offload: N middle turns archived (keep H/T)`.
3. The middle turns are **removed from the conversation index** and replaced
   by a single marker entry, so the model sees head + marker + tail.
4. The engine returns the marker plus the rendered `engine_offload` prompt
   (template vars `{hash}` and `{tail}`), which the assembler injects as the
   `[engine-offload: ...]` section - it is never dropped under budget
   pressure, because the archive already mutated the index.

The marker is a normal CCR entry of type `context`: its `size` is the
retrieval cost, and `aphrodite_retrieve` returns the packed JSON of the
archived turns.

## Engine configuration

Settings resolve env > TOML > compiled default in the dylib session. The
standalone HTTP proxy never reads them.

| Setting           | Env Var                          | TOML key                       | Compiled default | Shipped example |
| ----------------- | -------------------------------- | ------------------------------ | ---------------- | --------------- |
| Engine enabled    | `APHRODITE_CONTEXT_ENGINE`       | `[compression] context_engine` | true             | true            |
| Threshold percent | `APHRODITE_ENGINE_THRESHOLD_PCT` | `engine_threshold_pct`         | 45               | 30              |
| Protect first N   | `APHRODITE_ENGINE_PROTECT_FIRST` | `engine_protect_first`         | 2                | 2               |
| Protect last N    | `APHRODITE_ENGINE_PROTECT_LAST`  | `engine_protect_last`          | 5                | 5               |
| Min messages      | `APHRODITE_ENGINE_MIN_MSGS`      | `engine_min_msgs`              | 8                | 8               |

Threshold semantics: `0` = never archive, `>= 100` = engine off (escape
hatch), anything between = the fill percentage of the conversation index
(`total * 100 / 50`) at or above which archiving triggers. Environment
overrides TOML, which overrides the default.

## See also

- [Plugin Hooks](https://github.com/PlayForm/Aphrodite/tree/Development/docs/plugin/hooks.md) - `pre_llm_call` runs the engine trigger each turn
- [aphrodite.toml Configuration](https://github.com/PlayForm/Aphrodite/tree/Development/docs/config/aphrodite-toml.md) - the `[compression]` section in context
- [Environment Variables](https://github.com/PlayForm/Aphrodite/tree/Development/docs/config/env-vars.md) - the env-var equivalents
