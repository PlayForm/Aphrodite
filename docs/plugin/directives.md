# Conversational Directives

Directives are short behavioral instruction files - one `.md` file each -
injected inline into the LLM's context on every `pre_llm_call`. Unlike file
content (which gets compressed into CCR markers), directives are always
injected verbatim: they are compact enough to never need compression, and the
active set can be listed, swapped, and stacked mid-conversation via the
`aphrodite_directive` tool without touching any config file.

## Design principle: usage, not mechanism

Directives teach the LLM **retrieval vocabulary and decision-making only**:

- how to read a `<<<CCR:hash|type|size>>>` marker (hash = the key, type =
  kind of content, size = how large),
- when to retrieve vs. skip (retrieve when the next action needs the full
  content; act on the marker alone when the type/size already answers),
- how to find content (`aphrodite_search`, `aphrodite_catalog`,
  `aphrodite_prefetch`) and prefer granular over wholesale retrieval,
- retrieval fallbacks (a failed `aphrodite_retrieve` falls back to the
  original tool for that specific item).

They never explain the compression mechanism - no marker-injection details,
no chain/split/segment vocabulary, no storage internals. The story the LLM
sees stays the same; the machinery changes underneath, and behavior is
learned from consequences, not from instructions about the mechanism.

## Design principle: usage, not mechanism

Directives teach the LLM **retrieval vocabulary and decision-making only**:

- how to read a `<<<CCR:hash|type|size>>>` marker (hash = the key, type =
  kind of content, size = how large),
- when to retrieve vs. skip (retrieve when the next action needs the full
  content; act on the marker alone when the type/size already answers),
- how to find content (`aphrodite_search`, `aphrodite_catalog`,
  `aphrodite_prefetch`) and prefer granular over wholesale retrieval,
- retrieval fallbacks (a failed `aphrodite_retrieve` falls back to the
  original tool for that specific item).

They must **never** explain the compression mechanism - no marker-injection
details, no chain/split/segment vocabulary, no storage internals (SQLite,
LRU, hot-reload, engine state). The story the LLM sees stays the same; the
machinery changes underneath, and behavior is learned from consequences, not
from instructions about the mechanism.

## Built-in directives

The binary ships seven directives, baked in at compile time and materialized
into `~/.hermes/aphrodite/directives/` on plugin registration (user-modified
files are never overwritten):

| Directive      | Role                                                                                                             |
| -------------- | ---------------------------------------------------------------------------------------------------------------- |
| `focus`        | Targeted execution: one primary action per turn, marker-aware retrieval                                          |
| `explore`      | Broad context: read related files, prefetch aggressively, resolve the markers that matter                        |
| `foresight`    | Anticipation: prefetch what the next turn will need, never wait on I/O                                           |
| `cleanup`      | Hygiene: catalog sweep, summarize, verify nothing was left unresolved                                            |
| `lazy`         | Deferral: only what the current turn strictly requires; pull heavier context in when a later turn needs it       |
| `lazy-eval`    | Deferred resolution: markers accumulate across turns, and you decide when to resolve them from type and size     |
| `ccr-handling` | Marker vocabulary: how to read `<<<CCR:hash\|type\|size>>>` and when to retrieve vs. skip - the shared reference |

Any `.md` file you drop into a discovered directives directory becomes a
directive named after its file stem - the built-ins are not special-cased.

## Discovery and loading

| Rule              | Behavior                                                                                                                                                                                                  |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Search order      | `APHRODITE_DIRECTIVES_DIR` (if set) -> `./directives/` (working directory) -> `~/.hermes/aphrodite/directives/` -> binary-relative - the **first directory that exists** wins; they are never merged      |
| File filter       | Only `*.md` files; anything else is silently skipped                                                                                                                                                      |
| Naming            | Directive name = file stem (`focus.md` -> `focus`)                                                                                                                                                        |
| Per-file cap      | 2,000 characters per directive body (char-safe truncation, `...` appended)                                                                                                                                |
| Combined cap      | 4,000 characters across all active directives' injected text combined - several active directives cannot blow past the context budget together                                                            |
| Load condition    | Directories load **unconditionally** when present - loading is not gated on `[directives] active` being non-empty, so runtime `add`/`swap` works from a cold start with the shipped `active = []` default |
| Empty directory   | An existing directives directory that yields zero readable `.md` files is an **intentionally empty** directive set - the built-ins are NOT substituted                                                    |
| Built-in fallback | When no directives directory exists on disk, the seven baked-in directives load automatically - the fallback activation is logged                                                                         |
| Active default    | When `[directives] active` is empty and directives ARE loaded, `focus` + `foresight` + `lazy` are seeded active automatically (`lazy` keeps the session from over-eagerly stacking directives)            |

## `[directives]` in aphrodite.toml

```toml
[directives]
active = []                        # e.g. ["focus", "foresight"]
```

`active` only seeds which loaded directives start active. Names not found in
the loaded set are filtered out rather than erroring. Everything else -
activation, deactivation, stacking - happens at runtime through the tool
below. See [aphrodite.toml Configuration](https://github.com/PlayForm/Aphrodite/tree/Current/docs/config/aphrodite-toml.md#directives)
for where this section sits in the full schema.

## Injection mechanics

`pre_llm_call` builds the directive block via the shared context assembler
and places it at the **top** of the per-turn injected context - the first
section after the first-turn orientation, ahead of nudges and the recall
catalog. Directives and nudges are never dropped when the context budget is
tight; the recall catalog is the first section to go.

```text
[directives: focus]
focus:
  focus - targeted execution, marker-aware retrieval
Markers are content. A <<<CCR:hash|type|size>>> marker in tool output
  stands in for the content you asked for. The hash is the key:
  aphrodite_retrieve(hash) returns the full text.
```

| Detail      | Behavior                                                                                                                                                       |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Header line | `[directives: name1, name2]` - active names, comma-joined                                                                                                      |
| Body        | Each active directive's **full** (per-file-capped) body, not just its title line - leading `#` markers stripped, blank lines dropped, remaining lines indented |
| Placement   | First section of the assembled context string returned by `pre_llm_call`; empty when no directives are active                                                  |
| Frequency   | Every `pre_llm_call` - the block reflects the active set at that moment, so a `swap` takes effect on the very next turn                                        |

## The `aphrodite_directive` tool

Registered in the Hermes bridge's tool registry with this schema (see
[Tool Relay: Tools](https://github.com/PlayForm/Aphrodite/tree/Current/docs/tool-relay/tools.md#7-aphrodite_directive)
for its place in the full 13-tool reference):

```json
{
	"name": "aphrodite_directive",
	"parameters": {
		"action": "list (default) | swap | add | load | remove | reset",
		"name": "Directive name - required for swap/add/load/remove"
	}
}
```

| Action   | Effect                                                 | Response                                                                             |
| -------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| `list`   | Enumerate loaded + active directives (default)         | `{available: [...], active: [...], ephemeral: [{name, inline, expires_after_turn}]}` |
| `swap`   | Replace the active set with one directive              | `{swapped: name, active: [name]}` or `{error: "unknown directive: ..."}`             |
| `add`    | Append a directive to the active set (idempotent)      | `{active: [...]}`                                                                    |
| `load`   | Activate a directive on demand (lazy)                  | `{loaded: name, active: [...]}` or `{error: "unknown directive: ..."}`               |
| `remove` | Drop a directive from the active set                   | `{active: [...]}`                                                                    |
| `reset`  | Clear the active set, ephemerals, and the manual latch | `{active: []}`                                                                       |

`load` is the lazy-activation action: unlike `add` (which is silent when the
name is unknown or already active), `load` returns a distinct
`{loaded, active}` shape and **errors** on an unknown name, so a lazy-load
typo surfaces instead of being swallowed. It is idempotent when the directive
is already active.

An unknown action returns
`{error: "unknown action: ... (use list|swap|add|load|remove|reset)"}`.

### Dispatch paths

All three entry points delegate to the same shared action handler, so they
expose the identical action set and error shape:

| Path                                         | Caller                                          |
| -------------------------------------------- | ----------------------------------------------- |
| Hermes tool dispatch (`aphrodite_directive`) | The agent, in a live Hermes session             |
| Core C ABI export `aphrodite_directive`      | Handle-based FFI consumers of `libaphrodite`    |
| `aphrodite_dispatch`'s `"directive"` arm     | The universal string-dispatch C ABI entry point |

The active set persists across a session reset - it is per-process state, not
per-turn.

## See also

- [Plugin Hooks](https://github.com/PlayForm/Aphrodite/tree/Current/docs/plugin/hooks.md) - the `pre_llm_call` lifecycle this feature rides
- [Tool Relay: Tools](https://github.com/PlayForm/Aphrodite/tree/Current/docs/tool-relay/tools.md) - full tool reference
- [aphrodite.toml Configuration](https://github.com/PlayForm/Aphrodite/tree/Current/docs/config/aphrodite-toml.md) - the `[directives]` section in context
- [Environment Variables](https://github.com/PlayForm/Aphrodite/tree/Current/docs/config/env-vars.md) - the separate config path that feeds the dylib session
