# Tool Relay Tools

Origin: Aphrodite exposes 9 tools to the Hermes agent for compression, retrieval, stats, and session management. Tools are registered by the Python plugin and executed via the proxy's `handle_tool_relay` endpoint.

Source of truth: `plugins/aphrodite/plugin.yaml` (lines 13-22), `plugins/aphrodite/_tools.py`, `plugins/aphrodite/_hooks.py` (lines 399, 1176, 1250, 1273, 1291, 1318, 1419), `crates/aphrodite/src/proxy.rs:execute_tool_relay()` (line 1561)

## Tool Registry

| # | Tool | Source | Proxy Support |
|---|------|--------|---------------|
| 1 | `aphrodite_retrieve` | _tools.py:22 | Yes (execute_tool_relay) |
| 2 | `aphrodite_compress` | _tools.py:68 | Yes (execute_tool_relay) |
| 3 | `aphrodite_stats` | _hooks.py:1176 | No (Python only) |
| 4 | `aphrodite_rebuild` | _hooks.py:399 | No (Python only) |
| 5 | `aphrodite_files` | _hooks.py:1250 | No (Python only) |
| 6 | `aphrodite_diff` | _hooks.py:1273 | No (Python only) |
| 7 | `aphrodite_search` | _hooks.py:1318 | No (Python only) |
| 8 | `aphrodite_test` | _hooks.py:1419 | No (Python only) |
| 9 | `aphrodite_catalog` | _hooks.py:1291 | No (Python only) |

## 1. aphrodite_retrieve

### Schema (_tools.py:134)
```json
{
    "name": "aphrodite_retrieve",
    "description": "Resolve CCR markers to original content via aphrodite proxy. Optionally filter by query. Supports file path reads. Recursively resolves nested CCR markers up to 3 levels deep.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker hash to retrieve."},
            "query": {"type": "string", "description": "Optional filter query"},
            "path": {"type": "string", "description": "Optional file path (bypasses CCR)"}
        },
        "required": []
    }
}
```

### Handler (_tools.py:22)
```
Retrieve flow:
1. Extract hash (strip <<<CCR: prefix >>> suffix if present)
2. If path: read file (workspace-bounded, 10MB cap)
3. If hash: resolve_recursive (inline store → proxy)
4. If query: filter lines (case-insensitive)
5. Return {content, hash/path, size} or {error}
```

### Proxy Support (proxy.rs:1567)
Same logic: inline_ccr → CCR store. Returns `{found: true/false, content: "..."}`.

## 2. aphrodite_compress

### Schema (_tools.py:119)
```json
{
    "name": "aphrodite_compress",
    "description": "Compress content into CCR via aphrodite proxy for later retrieval. Specify type for adaptive compression: code, log, diff, error, json, build_output.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Content to compress and store in CCR"},
            "type": {"type": "string", "description": "Content type hint: code, log, diff, error, json, build_output, text"}
        },
        "required": ["content"]
    }
}
```

### Handler (_tools.py:68)
```
Compress flow:
1. Hash content: SHA-256 → first 24 hex chars
2. Check inline_store (Python side)
3. If miss: POST to proxy :9798/ccr/create (or :9797 fallback)
4. If proxy fail: store inline as fallback
5. Return {hash, type, size, compression_ratio}
```

### Proxy Support (proxy.rs:1587)
```
1. content < 256B: inline_ccr store
2. content ≥ 256B: CCR store
3. Return {compressed: "<<<CCR:hash|compress|size>>>", hash, original_size}
```

## 3. aphrodite_stats

### Schema (_hooks.py:1228)
```json
{
    "name": "aphrodite_stats",
    "description": "Check aphrodite proxy health, CCR stats, engine compression status.",
    "parameters": {"type": "object", "properties": {}}
}
```

### Handler (_hooks.py:1176)
```
Returns:
{
    "proxy": {
        "token": {"alive": true, "ccr_hits": 1234, "ccr_created": 567, ...},
        "cache": {"alive": false}
    },
    "engine": {
        "active": true, "compressions": 12,
        "threshold_tokens": 500000, "last_prompt_tokens": 320000, ...
    },
    "inline_store": {"entries": 42, "total_bytes": 50000}
}
```

Queries both proxy ports via HTTP `/stats`.

## 4. aphrodite_rebuild

### Schema (_hooks.py:423)
```json
{
    "name": "aphrodite_rebuild",
    "description": "Rebuild aphrodite crate from source and install binary.",
    "parameters": {"type": "object", "properties": {}}
}
```

### Handler (_hooks.py:399)
```
Executes: cargo build --release -p aphrodite
Copies: target/release/aphrodite → ~/.hermes/aphrodite/aphrodite
```

## 5. aphrodite_files

### Schema (_hooks.py:1266)
```json
{
    "name": "aphrodite_files",
    "description": "List all file paths referenced in the current session. Grouped by tool type.",
    "parameters": {"type": "object", "properties": {}}
}
```

### Handler (_hooks.py:1250)
```
Returns: {count, by_tool: {tool_name: [paths]}, all: [sorted paths]}
```
Tracks files touched by `read_file`, `write_file`, `patch`, `search_files`.

## 6. aphrodite_diff

### Schema (_hooks.py:1284)
```json
{
    "name": "aphrodite_diff",
    "description": "Show conversation turn history - what was discussed, compressed, and stored across turns.",
    "parameters": {"type": "object", "properties": {}}
}
```

### Handler (_hooks.py:1273)
```
Returns: {turns: N, recent: [{turn, hash, summary, size}]}
```
Last 10 turns from `_conv_index`.

## 7. aphrodite_search

### Schema (_hooks.py:1408)
```json
{
    "name": "aphrodite_search",
    "description": "Search across compressed items by type or content pattern (trigram-indexed).",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search query (min 3 chars)"},
            "type": {"type": "string", "description": "Filter by content type"}
        }
    }
}
```

### Handler (_hooks.py:1318)
```
Searches:
1. _conv_index (turn summaries) — linear scan
2. _inline_store — trigram-indexed (lazy init)
3. _recent_markers — linear scan
Results deduplicated by hash, max 20.
```

## 8. aphrodite_test

### Schema (_hooks.py:1495+)
```json
{
    "name": "aphrodite_test",
    "description": "Full smoke test suite - exercises all tools, hooks, compression, search, retrieve.",
    "parameters": {
        "type": "object",
        "properties": {
            "mode": {"type": "string", "description": "Test mode: quick, full, matrix, pipeline"}
        }
    }
}
```

### Handler (_hooks.py:1419)
```
Modes:
- quick: compress, retrieve, stats, files, diff, proxy health
- full: quick + large payload compression, search, threshold checks
- matrix: settings sweep (pct × protect_n combinations)
- pipeline: full + feature toggles (debug, engine, catalog modes)
```

## 9. aphrodite_catalog

### Schema (_hooks.py:1311)
```json
{
    "name": "aphrodite_catalog",
    "description": "Return full compression catalog - all CCR items with hashes, sizes, types, and previews.",
    "parameters": {"type": "object", "properties": {}}
}
```

### Handler (_hooks.py:1291)
```
Returns:
{
    "total_items": 42,
    "total_saved": 500000,
    "by_type": {"code_rust": {"count": 5, "hashes": [...]}, ...},
    "items": [{"hash": "...", "type": "code_rust", "size": 1234, "preview": "..."}],
    "conv_turns": 12,
    "referenced_files": 8
}
```
Full dump of `_recent_markers` deque.

## Proxy Tool Relay

Tools handled by `execute_tool_relay()` in proxy.rs:1561:

| Tool | Proxy Handler | Response |
|------|--------------|----------|
| `aphrodite_retrieve` | inline_ccr → CCR store | `{found, content}` |
| `aphrodite_compress` | inline (<256B) or CCR store | `{hash, original_size}` |
| `aphrodite_list` | ccr.len() | `{entries, backend}` |

Other tool names → `"Unknown tool: {name}"` error.

## Content-Type Hints for compress

From schema description: `code`, `log`, `diff`, `error`, `json`, `build_output`, `text`. These map to the content type taxonomy used by `detect_content_type()` for adaptive threshold selection.
