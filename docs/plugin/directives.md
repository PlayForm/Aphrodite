# Conversational Directives

Directives are short behavioral instructions - one `.md` file each - injected
inline into the LLM's context on every `pre_llm_call`. Unlike file content
(which gets compressed into CCR markers), directives are always injected
verbatim: they're compact enough to never need compression, and they can be
listed, swapped, and stacked mid-conversation via the `aphrodite_directive`
tool without touching any config file.

Shipped dark in v1.2.3 (loader + FFI existed, but nothing on the live Hermes
path called them); wired end-to-end in v1.3.2 - the bridge's `pre_llm_call`
now injects active directives, and `aphrodite_directive` is a registered,
dispatchable Hermes tool.

## Built-in directives

The repo ships four directives under `directives/`:

| Directive   | Behavior                                                                                     |
| ----------- | -------------------------------------------------------------------------------------------- |
| `focus`     | Stay targeted: at most 1-2 tools per turn, prefer `aphrodite_retrieve` over re-reading files |
| `explore`   | Read broadly: 2-3 related files per turn, `aphrodite_prefetch` batches of related paths      |
| `foresight` | Anticipate next steps: prefetch imports/references after reads, top search results ahead     |
| `cleanup`   | Summarize and prune: progress summary every 5 turns, `aphrodite_catalog(mode="toc")` sweeps  |

Any `.md` file you drop into a discovered directives directory becomes a
directive named after its file stem - the built-ins aren't special-cased.

## Discovery and loading

| Rule            | Behavior                                                                                                                                      |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Search order    | `./directives/` (working directory), then `~/.hermes/directives/` - the **first directory that exists** wins; they are not merged             |
| File filter     | Only `*.md` files; anything else is silently skipped                                                                                          |
| Naming          | Directive name = file stem (`focus.md` → `focus`)                                                                                             |
| Per-file cap    | 2,000 chars per directive body (char-safe truncation, `…` appended)                                                                           |
| Combined cap    | 4,000 chars across all active directives' injected text combined - several active directives can't blow past the context budget together     |
| Load condition  | Directories load **unconditionally** when present - loading is not gated on `[directives] active` being non-empty (it was before v1.3.2, which made runtime `add`/`swap` impossible from a cold start with the shipped `active = []` default) |

## `[directives]` in aphrodite.toml

```toml
[directives]
active = []                        # e.g. ["focus", "foresight"]
```

`active` only seeds which loaded directives start active. Names not found in
the loaded set are filtered out rather than erroring. Everything else -
activation, deactivation, stacking - happens at runtime through the tool
below. See [aphrodite.toml Configuration](../config/aphrodite-toml.md#directives)
for where this section sits in the full schema.

## Injection mechanics

`pre_llm_call` builds the directive block via `build_directive_context` and
appends it to the same `context` string that carries the catalog summary -
Hermes injects that string into the conversation each turn:

```text
[directives: focus]
focus:
  focus - stay targeted, minimal tool usage
  Each turn: use at most 1-2 tools. Prefer retrieval over re-reading.
  One primary action per turn
  Use aphrodite_retrieve(hash) for any <<<CCR:...>>> you see
```

| Detail            | Behavior                                                                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------------------------ |
| Header line       | `[directives: name1, name2]` - active names, comma-joined                                                         |
| Body              | Each active directive's **full** (per-file-capped) body, not just its title line - leading `#` markers stripped, blank lines dropped, remaining lines indented |
| Placement         | Appended after the catalog summary in the hook's returned `{"context": "..."}`; empty when no directives are active |
| Frequency         | Every `pre_llm_call` - the block reflects the active set at that moment, so a `swap` takes effect on the very next turn |

Before v1.3.2 only each directive's first line (a markdown title) was
injected - the behavioral bullets never reached the model. The full body now
travels, under the combined 4,000-char cap.

## The `aphrodite_directive` tool

Registered in the Hermes bridge's tool registry with this schema (see
[Tool Relay: Tools](../tool-relay/tools.md#7-aphrodite_directive) for its
place in the full 13-tool reference):

```json
{
	"name": "aphrodite_directive",
	"parameters": {
		"action": "list (default) | swap | add | remove | reset",
		"name": "Directive name - required for swap/add/remove"
	}
}
```

| Action   | Effect                                          | Response                                     |
| -------- | ----------------------------------------------- | -------------------------------------------- |
| `list`   | Enumerate loaded + active directives (default)  | `{available: [...], active: [...]}`          |
| `swap`   | Replace the active set with one directive       | `{swapped: name, active: [name]}` or `{error: "unknown directive: ..."}` |
| `add`    | Append a directive to the active set (idempotent) | `{active: [...]}`                          |
| `remove` | Drop a directive from the active set            | `{active: [...]}`                            |
| `reset`  | Clear the active set                            | `{active: []}`                               |

An unknown action returns
`{error: "unknown action: ... (use list|swap|add|remove|reset)"}`.

### Dispatch paths

All three entry points delegate to the single shared
`directives::handle_action`, so they expose the identical action set and
error shape:

| Path                                        | Caller                                            |
| ------------------------------------------- | -------------------------------------------------- |
| Hermes tool dispatch (`aphrodite_directive`) | The agent, in a live Hermes session               |
| Core C ABI export `aphrodite_directive`     | Handle-based FFI consumers of `libaphrodite`      |
| `aphrodite_dispatch`'s `"directive"` arm    | The universal string-dispatch C ABI entry point   |

The active set persists across a session reset (like the inline store) - it's
per-process state, not per-turn.

## See also

- [Plugin Hooks](hooks.md) - the `pre_llm_call` lifecycle this feature rides
- [Tool Relay: Tools](../tool-relay/tools.md) - full tool reference
- [aphrodite.toml Configuration](../config/aphrodite-toml.md) - the `[directives]` section in context
- [Environment Variables](../config/env-vars.md) - the separate config path that feeds the dylib session
