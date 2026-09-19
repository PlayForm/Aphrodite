# Tool Relay Tools

Aphrodite exposes 13 tools to the Hermes agent for compression, retrieval,
stats, and session management. Every tool is a thin relay into the Rust dylib:
the plugin registers each tool from the dylib's schema list (toolset
`aphrodite`), and every call dispatches through `aphrodite_hermes_dispatch_tool`
and returns the JSON result verbatim. There is no separate Python-side tool
logic to know about.

> **Schema detail lives in the schema, not here.** Each registered
> `description` documents its own return shape, and every parameter carries a
> type and a description plus any `enum`/`default` constraints. `tool_describe`
> is a verbatim passthrough of `{name, description, parameters}`, so what an
> agent sees is exactly what the dylib defines. Run
> `python3 Maintain/scripts/verify_tool_schemas.py` to print the live records
> straight off the built dylib.

## Tool registry

| #   | Tool                        | What it's for                                        |
| --- | --------------------------- | ---------------------------------------------------- |
| 1   | `aphrodite_compress`        | Store content in CCR and get a marker back           |
| 2   | `aphrodite_retrieve`        | Expand a CCR marker, or read a file, to full content |
| 3   | `aphrodite_stats`           | Session and proxy health counters for the CCR engine |
| 4   | `aphrodite_files`           | List file paths touched this session                 |
| 5   | `aphrodite_diff`            | Per-turn history of what was compressed              |
| 6   | `aphrodite_search`          | Find already-compressed entries by keyword or type   |
| 7   | `aphrodite_directive`       | Inspect or change the active behavioral directives   |
| 8   | `aphrodite_test`            | Smoke-test the compress and retrieve round trip      |
| 9   | `aphrodite_catalog`         | List every CCR entry recorded this session           |
| 10  | `aphrodite_reclassify`      | Re-detect type and preview for stored entries        |
| 11  | `aphrodite_prefetch`        | Read files now and store them as CCR markers         |
| 12  | `aphrodite_prefetch_status` | Live prefetch schedule - what is loaded and ready    |
| 13  | `aphrodite_rebuild`         | Report binary and proxy state (never rebuilds)       |

All handlers share one session store, so content compressed by a hook or by
`aphrodite_compress` stays resolvable by `aphrodite_retrieve` for the life of
the session. The one caveat: markers do not survive a dylib hot-reload, which
starts a fresh session store. There is also an internal 14th registry entry,
`context_engine_pre_llm` - the context engine's own pre-LLM hook, not a tool an
agent calls directly.

## 1. aphrodite_compress

```json
{
	"name": "aphrodite_compress",
	"description": "Store content in CCR and get a marker back. Hashes the content, classifies it (or trusts the `type` hint), keeps it in the session store, and hands back a resolvable `<<<CCR:hash|type|size>>>` marker - park bulky text here instead of carrying it in context, then pull it back with aphrodite_retrieve when you actually need it. Storage is in-process and sub-millisecond; markers stay resolvable for the life of the session but do not survive a dylib hot-reload. Returns {hash, type, size, preview, marker}.",
	"parameters": {
		"type": "object",
		"properties": {
			"content": {
				"type": "string",
				"minLength": 1,
				"description": "Exact text to store. Round-trips byte-for-byte through aphrodite_retrieve."
			},
			"type": {
				"type": "string",
				"enum": ["code", "log", "diff", "error", "json", "build_output", "text"],
				"description": "Content-type hint that steers the preview and compression profile. Omit to auto-detect; \"text\" is treated as no hint."
			},
			"_ccr_center": {
				"type": "string",
				"description": "Optional label carried alongside the marker and echoed back by aphrodite_catalog - useful for tagging what a stored blob was for."
			}
		},
		"required": ["content"],
		"additionalProperties": false
	}
}
```

Detects a content type automatically unless a `type` hint is given (a hint of
`"text"` is treated as no hint). Hashes and stores the content, then returns
`{hash, type, size, preview, marker}`. The marker is what goes into context
instead of the content; see [Content Types](https://github.com/PlayForm/Aphrodite/tree/Current/docs/classification/content-types.md)
for the taxonomy behind the type field.

## 2. aphrodite_retrieve

```json
{
	"name": "aphrodite_retrieve",
	"description": "Expand a CCR marker, or read a file, to full content. Supply `hash` to resolve a marker recorded this session (nested markers are expanded recursively), or `path` to read a file straight from disk. Give one or the other - if both are present `path` wins. `path` reads are confined to the current workspace and capped at 10 MiB, so paths outside it are refused. `query` narrows the result to matching lines, which is the cheap way to poke at a large entry without pulling all of it into context. Returns {found: true, source: \"ccr\"|\"path\", hash|path, content} on success, {found: false, error} when the hash is unknown or the read is refused.",
	"parameters": {
		"type": "object",
		"properties": {
			"hash": {
				"type": "string",
				"pattern": "^[0-9a-fA-F]{8,64}$",
				"description": "Full hex hash from a `<<<CCR:hash|type|size>>>` marker. Truncated hashes do not resolve - exact match only."
			},
			"query": {
				"type": "string",
				"description": "Case-insensitive substring; only lines containing it are returned."
			},
			"path": {
				"type": "string",
				"description": "Workspace-relative or absolute file path to read directly, bypassing CCR."
			}
		},
		"additionalProperties": false
	}
}
```

| Input        | Behavior                                                                                                                                                                    |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `path` given | Reads the file directly - confined to the current workspace and capped at 10 MiB. Refuses reads outside the workspace (e.g. `/etc/passwd`, `~/.ssh`) and files over the cap |
| `hash` given | Resolves the marker from the session store; nested markers are expanded recursively. Truncated hashes never resolve - exact match only                                      |
| `query` set  | Filters the result to matching lines (case-insensitive substring)                                                                                                           |

Returns `{found: true, source: "path"|"ccr", hash|path, content}` or
`{found: false, error}`.

## 3. aphrodite_stats

Takes no arguments. One call covers both halves of the system: session state
held by the dylib, and a live TCP poll of the two proxy ports.

```json
{
	"version": "1.4.6",
	"engine": "aphrodite-hermes",
	"inline_entries": 0,
	"markers": 0,
	"referenced_files": 0,
	"archived_turns": 0,
	"turn": 0,
	"engine_enabled": true,
	"threshold_pct": 45,
	"tool_threshold": 512,
	"terminal_threshold": 256,
	"chain_split_min_segments": 2,
	"chain_split_events": 0,
	"chain_split_produced": 0,
	"chain_split_retrieved": 0,
	"proxies": {
		"token": { "port": 9797, "alive": true },
		"cache": { "port": 9798, "alive": false }
	}
}
```

The `chain_split_*` fields are telemetry for the opt-in chain-split CCR engine
(`chain_split = true` in `aphrodite.toml`); they stay in the response even when
the engine is off. `proxies` reflects a live TCP poll of the two configured
proxy ports (400ms timeout; ports come from `APHRODITE_TOKEN_PORT` /
`APHRODITE_CACHE_PORT`, defaulting to 9797/9798). `alive: false` means nothing
answered on that port - the plugin still works, it just is not proxying.

## 4. aphrodite_files

Takes no arguments. Lists every file path referenced in the current session;
tool hooks populate it as reads, writes, and prefetches happen, so it doubles
as a record of what has already been looked at. Returns
`{total, files: [{path, tool}]}` where `tool` is the tool that first referenced
the path.

## 5. aphrodite_diff

Takes no arguments. Walks the archived conversation index in turn order, one
entry per turn that produced a compression. Returns
`{total, turns: [{turn, hash, summary, size}]}`; each `hash` is resolvable
through `aphrodite_retrieve`.

## 6. aphrodite_search

```json
{
	"name": "aphrodite_search",
	"description": "Find already-compressed entries by keyword or type. Matches case-insensitively against each entry's preview line and its CCR type - it does NOT search the full stored bodies, so treat a miss as inconclusive and fall back to aphrodite_catalog. Newest first, capped at 20 results. Returns {query, total, results: [{hash, type, size, preview}]}.",
	"parameters": {
		"type": "object",
		"properties": {
			"query": {
				"type": "string",
				"description": "Keyword or phrase to match against preview text and CCR type."
			},
			"type": {
				"type": "string",
				"description": "Restrict results to one exact CCR type, such as source_code, terminal, build_output, diff, json, search or text."
			}
		},
		"required": ["query"],
		"additionalProperties": false
	}
}
```

Matches case-insensitively against each entry's preview line and CCR type only

- stored bodies are never searched, so a miss is inconclusive: fall back to
  `aphrodite_catalog`. Newest first, capped at 20 results. Returns
  `{query, total, results: [{hash, type, size, preview}]}`.

## 7. aphrodite_directive

```json
{
	"name": "aphrodite_directive",
	"description": "Inspect or change the active behavioral directives. Directives are short instruction files injected into every turn through pre_llm_call - \"focus\" for terse, minimal-tool execution, \"explore\" for broad reading, \"lazy\" for defer-until-needed execution, and whatever else ships in directives/. `list` returns {available, active, ephemeral: [{name, inline, expires_after_turn}]}; `swap` replaces the whole active set with one directive and returns {swapped, active}; `add` appends to the set (silent if already active) and returns {active}; `load` activates a directive from the available set on demand and returns {loaded, active} (errors on an unknown name, unlike `add`); `remove` drops one and returns {active}; `reset` clears actives, ephemerals and the manual latch, handing control back to automatic selection. An unknown directive name returns {error} and changes nothing.",
	"parameters": {
		"type": "object",
		"properties": {
			"action": {
				"type": "string",
				"enum": ["list", "swap", "add", "load", "remove", "reset"],
				"default": "list",
				"description": "What to do. Defaults to `list`. `load` activates a directive just-in-time (lazy activation)."
			},
			"name": {
				"type": "string",
				"description": "Directive name. Required for swap, add, load and remove; ignored by list and reset."
			}
		},
		"additionalProperties": false
	}
}
```

| Action   | Behavior                                                                     | Returns                          |
| -------- | ---------------------------------------------------------------------------- | -------------------------------- |
| `list`   | Show available, active, and any ephemeral directives                         | `{available, active, ephemeral}` |
| `swap`   | Replace the whole active set with one directive                              | `{swapped, active}`              |
| `add`    | Append to the active set (silent if already active)                          | `{active}`                       |
| `load`   | Activate a directive on demand; errors on an unknown name                    | `{loaded, active}`               |
| `remove` | Drop one directive from the active set                                       | `{active}`                       |
| `reset`  | Clear actives, ephemerals, and the manual latch; back to automatic selection | `{active}`                       |

An unknown directive name returns `{error}` and changes nothing. Active
directive bodies are injected into every turn through the `pre_llm_call` hook -
see [Directives](https://github.com/PlayForm/Aphrodite/tree/Current/docs/plugin/directives.md)
for the full mechanism.

## 8. aphrodite_test

```json
{
	"name": "aphrodite_test",
	"description": "Smoke-test the compress and retrieve round trip. Compresses built-in samples in-process and checks each one comes back byte-identical, then reports proxy health alongside. Use it to confirm the engine is actually wired up before trusting a marker. Returns {mode, status: \"ok\"|\"fail\", passed, total, checks: [{type, hash, roundtrip}], proxies}. `status` is \"ok\" only when every check round-tripped.",
	"parameters": {
		"type": "object",
		"properties": {
			"mode": {
				"type": "string",
				"enum": ["quick", "full"],
				"default": "quick",
				"description": "`quick` runs 1 sample (source code). `full` runs 3 (source code, build output with errors and warnings, JSON). Any unrecognized value behaves like `full`."
			}
		},
		"additionalProperties": false
	}
}
```

Compresses built-in samples in-process and checks that each round-trips
byte-identical, then reports proxy health alongside. `status` is `"ok"` only
when every check passed. Returns
`{mode, status: "ok"|"fail", passed, total, checks: [{type, hash, roundtrip}], proxies}`.
This is the same tool [Troubleshooting](https://github.com/PlayForm/Aphrodite/tree/Current/docs/install/troubleshooting.md#verify-the-proxy-without-hermes)
points to for confirming things work without a full Hermes session.

## 9. aphrodite_catalog

```json
{
	"name": "aphrodite_catalog",
	"description": "List every CCR entry recorded this session. The complete inventory, newest first - unlike aphrodite_search it filters nothing, so this is the reliable way to find a marker you half-remember. Returns {mode, total, turn, items: [...]}, where each item is {hash, type, size, preview} in `toc` mode and additionally {turn, center} in `full` mode.",
	"parameters": {
		"type": "object",
		"properties": {
			"mode": {
				"type": "string",
				"enum": ["full", "toc"],
				"default": "full",
				"description": "`full` includes turn and center per entry. `toc` is the compact table-of-contents form."
			}
		},
		"additionalProperties": false
	}
}
```

The complete inventory, newest first - unlike `aphrodite_search` it filters
nothing. `mode: "toc"` returns `{hash, type, size, preview}` per entry; the
default full mode adds `{turn, center}` (the `_ccr_center` label from compress
time). Returns `{mode, total, turn, items: [...]}`.

## 10. aphrodite_reclassify

```json
{
	"name": "aphrodite_reclassify",
	"description": "Re-detect type and preview for stored entries. Re-runs classification against content already in the store and rewrites the catalog entry in place, which repairs entries that were typed before a classifier improvement or stored with a wrong hint. Content and hashes are never modified, so existing markers keep resolving. Returns {status: \"ok\", reclassified: <count>}.",
	"parameters": {
		"type": "object",
		"properties": {
			"hash": {
				"type": "string",
				"pattern": "^[0-9a-fA-F]{8,64}$",
				"description": "Reclassify only this entry. Omit to reclassify every entry in the session."
			}
		},
		"additionalProperties": false
	}
}
```

Re-runs type detection and preview generation against content already in the
store and rewrites the catalog entry in place - handy after a classifier
improvement or when an entry was stored with a wrong hint. Content and hashes
are never modified, so existing markers keep resolving. Omit `hash` to
reclassify every entry in the session. Returns `{status: "ok", reclassified:
<count>}`.

## 11. aphrodite_prefetch

```json
{
	"name": "aphrodite_prefetch",
	"description": "Read files now and store them as CCR markers. Despite the name this is synchronous: every path is read and compressed before the call returns, so each returned hash is immediately resolvable through aphrodite_retrieve. Batch the files you expect to need next instead of reading them one at a time - one call, one marker per file, no file bodies in context. Oversized files are skipped rather than truncated. Returns {total, loaded, skipped_size, missing, inline_store_bytes, inline_store_byte_budget, results: [{path, status: \"loaded\"|\"skipped\"|\"missing\", hash, type, size, preview, reason}]}. Watch inline_store_bytes against its budget: a batch larger than the budget evicts older entries, including ones from the same batch.",
	"parameters": {
		"type": "object",
		"properties": {
			"paths": {
				"type": "array",
				"items": { "type": "string" },
				"description": "File paths to read and compress."
			}
		},
		"required": ["paths"],
		"additionalProperties": false
	}
}
```

Despite the name this is synchronous: every path is read and compressed before
the call returns, so each returned hash is immediately resolvable through
`aphrodite_retrieve`. One marker per file, no file bodies in context.
Oversized files are skipped rather than truncated, and a batch larger than the
inline-store budget evicts older entries (including ones from the same batch).
Returns `{total, loaded, skipped_size, missing, inline_store_bytes,
inline_store_byte_budget, results: [{path, status: "loaded"|"skipped"|"missing",
hash, type, size, preview, reason}]}`.

## 12. aphrodite_prefetch_status

Takes no arguments. Because prefetch is synchronous, everything it loads is
already `ready` by the time the call returns - `loading` and `errors` are
always empty. Returns
`{loading: [], ready: [{path, hash, type, size}], errors: [], total_ready}`.

## 13. aphrodite_rebuild

Takes no arguments. Reports state rather than performing a rebuild - the dylib
cannot safely rebuild itself mid-session. Returns
`{status: "ok", version, proxies, hint: "rebuild via `cargo build --release -p
aphrodite`; dylib hot-reloads on mtime change"}`.

## Content-type hints

`aphrodite_compress`'s `type` hint accepts `code`, `log`, `diff`, `error`,
`json`, `build_output`, or `text`. A hint of `"text"` (or an empty hint) is
treated as no hint - the type is auto-detected instead. `aphrodite_search`'s
`type` filter matches the same taxonomy (e.g. `source_code`, `terminal`,
`build_output`, `diff`, `json`, `search`, `text`). See
[Content Types](https://github.com/PlayForm/Aphrodite/tree/Current/docs/classification/content-types.md)
for the full classification reference.
