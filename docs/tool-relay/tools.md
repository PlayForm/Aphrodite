# Tool Relay Tools

Aphrodite exposes 13 tools to the Hermes agent for compression, retrieval,
stats, and session management. All 13 dispatch entirely inside the Rust
dylib - the Python plugin shim forwards tool calls in and returns the JSON
result verbatim; there's no separate Python-side tool logic to know about.

## Tool registry

| #   | Tool                        | What it's for                                                   |
| --- | --------------------------- | --------------------------------------------------------------- |
| 1   | `aphrodite_compress`        | Compress content into CCR for later retrieval                   |
| 2   | `aphrodite_retrieve`        | Resolve a CCR marker (or read a workspace file) back to content |
| 3   | `aphrodite_stats`           | Proxy health, engine status, session counters                   |
| 4   | `aphrodite_files`           | File paths referenced this session                              |
| 5   | `aphrodite_diff`            | Conversation turn history                                       |
| 6   | `aphrodite_search`          | Search compressed entries by keyword or type                    |
| 7   | `aphrodite_directive`       | List/swap/add/remove/reset active behavioral directives         |
| 8   | `aphrodite_test`            | In-process smoke test: compress → retrieve round trips          |
| 9   | `aphrodite_catalog`         | Full or table-of-contents view of everything compressed         |
| 10  | `aphrodite_reclassify`      | Re-detect type/preview for already-stored entries               |
| 11  | `aphrodite_prefetch`        | Read + compress files ahead of time                             |
| 12  | `aphrodite_prefetch_status` | What prefetch has loaded so far                                 |
| 13  | `aphrodite_rebuild`         | Report binary/proxy version and rebuild instructions            |

All handlers share one session state, so content compressed by a hook or
`aphrodite_compress` stays resolvable by `aphrodite_retrieve` for the life of
the session. There's also an internal 14th entry, `context_engine_pre_llm` -
the context engine's own pre-LLM hook, not a tool an agent calls directly.

## 1. aphrodite_compress

```json
{
	"name": "aphrodite_compress",
	"description": "Compress content into CCR via aphrodite proxy for later retrieval. Specify type for adaptive compression: code, log, diff, error, json, build_output.",
	"parameters": {
		"type": "object",
		"properties": {
			"content": { "type": "string", "description": "Content to compress and store in CCR" },
			"type": {
				"type": "string",
				"description": "Optional: content type hint - code, log, diff, error, json, build_output, text"
			},
			"_ccr_center": {
				"type": "string",
				"description": "Optional: center string that travels with the marker"
			}
		},
		"required": ["content"]
	}
}
```

Detects a content type automatically unless a `type` hint is given (a hint
of `"text"` is treated as no hint). Hashes and stores the content, then
returns `{hash, type, size, preview, marker}`.

## 2. aphrodite_retrieve

```json
{
	"name": "aphrodite_retrieve",
	"description": "Resolve CCR markers to original content via aphrodite proxy. Optionally filter by query. Supports file path reads.",
	"parameters": {
		"type": "object",
		"properties": {
			"hash": { "type": "string", "description": "CCR marker hash to retrieve" },
			"query": {
				"type": "string",
				"description": "Optional: filter content to lines containing this query"
			},
			"path": {
				"type": "string",
				"description": "Optional: workspace file path to read directly (bypasses CCR)"
			}
		}
	}
}
```

| Input        | Behavior                                                                                                                                             |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `path` given | Reads the file directly - confined to the current workspace and capped at 10 MiB. Refuses reads outside the workspace (e.g. `/etc/passwd`, `~/.ssh`) |
| `hash` given | Resolves the marker from the session's inline store / CCR backend                                                                                    |
| `query` set  | Filters the result to matching lines (case-insensitive)                                                                                              |

Returns `{found, source: "path"|"ccr", content}` or `{found: false, error}`.

## 3. aphrodite_stats

```json
{
	"name": "aphrodite_stats",
	"description": "Check aphrodite proxy health, CCR stats, engine compression status.",
	"parameters": { "type": "object", "properties": {} }
}
```

Returns session + proxy state in one call:

```json
{
	"version": "1.2.1",
	"engine": "aphrodite-hermes",
	"inline_entries": 0,
	"markers": 0,
	"referenced_files": 0,
	"archived_turns": 0,
	"turn": 0,
	"engine_enabled": true,
	"threshold_pct": 45,
	"tool_threshold": 512,
	"terminal_threshold": 1024,
	"proxies": { "cache": { "alive": false }, "token": { "alive": true } }
}
```

`proxies` reflects a live HTTP poll of both configured proxy ports - see
[Troubleshooting: verify the proxy without Hermes](../install/troubleshooting.md#verify-the-proxy-without-hermes).

## 4. aphrodite_files

```json
{
	"name": "aphrodite_files",
	"description": "List all file paths referenced in the current session.",
	"parameters": { "type": "object", "properties": {} }
}
```

Returns `{total, files: [{path, tool}]}`, populated as tool hooks touch paths.

## 5. aphrodite_diff

```json
{
	"name": "aphrodite_diff",
	"description": "Show conversation turn history - what was discussed, compressed, and stored across turns.",
	"parameters": { "type": "object", "properties": {} }
}
```

Returns `{total, turns: [...]}`.

## 6. aphrodite_search

```json
{
	"name": "aphrodite_search",
	"description": "Search across CCR entries - find previously compressed content by keyword or type.",
	"parameters": {
		"type": "object",
		"properties": {
			"query": { "type": "string", "description": "Search keyword or phrase" },
			"type": { "type": "string", "description": "Optional: filter by CCR type" }
		},
		"required": ["query"]
	}
}
```

Case-insensitive match against preview text or type, newest first, capped at
20 results. Returns `{query, total, results: [{hash, type, size, preview}]}`.

## 7. aphrodite_directive

```json
{
	"name": "aphrodite_directive",
	"description": "List, activate, or deactivate behavioral directives - short instructions injected into context via pre_llm_call (e.g. 'focus' for minimal tool usage, 'explore' for broad reading).",
	"parameters": {
		"type": "object",
		"properties": {
			"action": {
				"type": "string",
				"description": "list (default) | swap | add | remove | reset"
			},
			"name": {
				"type": "string",
				"description": "Directive name - required for swap/add/remove"
			}
		}
	}
}
```

`list` returns `{available, active}`; `swap` replaces the active set with a
single directive; `add`/`remove` mutate the active set; `reset` clears it.
Active directives' full bodies (from `directives/*.md`, `#`-markers
stripped) are injected into `pre_llm_call`'s `context` string, appended
after the catalog summary.

## 8. aphrodite_test

```json
{
	"name": "aphrodite_test",
	"description": "Run full smoke test suite - compress, retrieve, search, stats, files, diff, proxy health.",
	"parameters": {
		"type": "object",
		"properties": {
			"mode": {
				"type": "string",
				"description": "Test mode: quick (default, 1 sample) or anything else (full, 3 samples)"
			}
		}
	}
}
```

| Mode          | Samples                                                                                                  |
| ------------- | -------------------------------------------------------------------------------------------------------- |
| `quick`       | 1 sample (source code) - compress then round-trip retrieve                                               |
| anything else | 3 samples (source code, a build with errors/warnings, a JSON array) - each compressed then round-tripped |

Returns `{mode, status: "ok"|"fail", passed, total, checks, proxies}`. This is
the same tool [Troubleshooting](../install/troubleshooting.md#verify-the-proxy-without-hermes)
points to for confirming things work without a full Hermes session.

## 9. aphrodite_catalog

```json
{
	"name": "aphrodite_catalog",
	"description": "Return full compression catalog with hashes, sizes, types, previews. Mode 'toc' for compact table-of-contents.",
	"parameters": {
		"type": "object",
		"properties": {
			"mode": {
				"type": "string",
				"description": "Optional: 'toc' for compact table-of-contents, default full catalog"
			}
		}
	}
}
```

`mode: "toc"` returns a compact `{hash, type, size, preview}` per entry; the
default full mode adds `{turn}`. Newest entries first.

## 10. aphrodite_reclassify

```json
{
	"name": "aphrodite_reclassify",
	"description": "Retroactively classify/metadata-enrich all CCR entries lacking structured metadata.",
	"parameters": {
		"type": "object",
		"properties": {
			"hash": {
				"type": "string",
				"description": "Optional: reclassify a single entry by hash"
			},
			"action": {
				"type": "string",
				"description": "Set to 'all' to reclassify all entries lacking meta."
			}
		}
	}
}
```

Re-runs type detection and preview generation against already-stored content
(all entries, or one `hash`), updating the marker in place. Returns
`{status: "ok", reclassified: <count>}`.

## 11. aphrodite_prefetch

```json
{
	"name": "aphrodite_prefetch",
	"description": "Read files in background and compress to CCR. Returns markers instantly.",
	"parameters": {
		"type": "object",
		"properties": {
			"paths": {
				"type": "array",
				"items": { "type": "string" },
				"description": "List of file paths to prefetch"
			}
		},
		"required": ["paths"]
	}
}
```

Despite "background" in the description, prefetch currently runs
synchronously - anything it reports is already resolvable by the time the
tool call returns.

## 12. aphrodite_prefetch_status

```json
{
	"name": "aphrodite_prefetch_status",
	"description": "Live prefetch schedule - what's loading, what's ready, ETAs per file.",
	"parameters": { "type": "object", "properties": {} }
}
```

Returns `{loading: [], ready: [{path, hash, type, size}], errors: [], total_ready}`.
`loading`/`errors` are always empty today since prefetch is synchronous -
anything tracked is immediately `ready`.

## 13. aphrodite_rebuild

Reports state rather than performing a rebuild - the dylib can't safely
rebuild itself mid-session. Returns
`{status: "ok", version, proxies, hint: "rebuild via cargo build --release -p aphrodite; dylib hot-reloads on mtime change"}`.

## Content-type hints for compress

`aphrodite_compress`'s `type` hint accepts `code`, `log`, `diff`, `error`,
`json`, `build_output`, or `text`. These map to the same taxonomy used
throughout - see [Content Types](../ccr/content-types.md). A hint of `"text"`
(or an empty hint) is treated as "no hint" - the type is auto-detected
instead.
