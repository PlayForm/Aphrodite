---
name: aphrodite-tool-testing
description: Use when teaching or verifying aphrodite CCR tool usage. The 13
    tools, CCR marker handling, retrieve-first rule, and the test roundtrip.
category: software-development
version: 1.1.0
platforms: [macos]
tags: [aphrodite, ccr, compression, retrieve, testing]
---

# Aphrodite Tool Testing & CCR Retrieval

## When to Use

- You see `<<<CCR:hash|type|size>>>` markers in tool output and must handle
  them correctly instead of losing the content behind them.
- Verifying the CCR engine is healthy, or teaching agents the 13-tool API.

## The 13 Aphrodite Tools (Quick Reference)

| Tool                        | Purpose                                         | Key Parameters                                                              |
| --------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------- |
| `aphrodite_stats`           | Check health, version, thresholds, proxy status | none                                                                        |
| `aphrodite_test`            | Smoke test: compress→retrieve→search roundtrip  | `mode` (quick=1 sample, full=3 samples)                                     |
| `aphrodite_compress`        | Compress content into CCR                       | `content` (required), `type` (code/log/diff/error/json/build_output/text)   |
| `aphrodite_retrieve`        | **Retrieve original content from CCR**          | `hash` (required), `query` (optional filter), `path` (optional file bypass) |
| `aphrodite_search`          | Search CCR entries                              | `query` (required), `type` (optional filter)                                |
| `aphrodite_catalog`         | List all CCR entries                            | `mode` ("toc" for compact, default full)                                    |
| `aphrodite_diff`            | Show conversation turn history                  | none                                                                        |
| `aphrodite_rebuild`         | Report binary version + proxy health            | none                                                                        |
| `aphrodite_reclassify`      | Retroactively classify/metadata-enrich entries  | `hash` (optional, omit for all)                                             |
| `aphrodite_prefetch`        | Read files in background → compress to CCR      | `paths` (array of file paths)                                               |
| `aphrodite_prefetch_status` | Live prefetch schedule                          | none                                                                        |
| `aphrodite_files`           | List all file paths referenced in session       | none                                                                        |
| `aphrodite_directive`       | Manage behavioral directives                    | `action`, `name`                                                            |

## CCR Marker Handling (the retrieve-first rule)

Marker shape:

```
<<<CCR:f398d3a3ffe4ca5ff42ee1275a28baa48874c983|text|172>>>
```

- `f398d3a3...` - the hash (SQLite primary key)
- `text` - the content type (text, code, log, diff, error, json,
  build_output, terminal)
- `172` - byte size of the original content

> **Every `<<<CCR:hash|type|size>>>` in tool output IS compressed content.
> Retrieve it immediately with `aphrodite_retrieve(hash)`, before any other
> action.** Markers are not opaque - the content is behind them, cheaply
> retrievable (sub-ms from SQLite).

Why agents fail:

1. They ignore the marker and proceed without the content.
2. They re-read the source file with `read_file` instead of retrieving - the
   marker IS the content; re-reading just yields another marker.
3. They treat the marker as opaque, not knowing it contains the content.
4. They retrieve one marker at a time - retrieve ALL markers in the same turn.

## Using `aphrodite_retrieve`

1. **Full content by hash**: `aphrodite_retrieve(hash="abc123...")` →
   `{found: true, source: "ccr", hash: "...", content: "..."}`
2. **Query-filtered**: `aphrodite_retrieve(hash="abc123...", query="timeout")`
   returns only lines matching the query.
3. **Direct file read (fallback)**: `aphrodite_retrieve(path="README.md")` →
   `{found: true, source: "path", ...}`. Path-based reads enforce workspace
   containment - paths outside the workspace return `found: false`.
4. **When retrieval fails**: if `aphrodite_retrieve(hash=...)` returns
   `found: false`, fall back to `read_file` / `terminal` on the original path.

## Testing Process (step-by-step)

1. **Check engine health**: `aphrodite_stats()` - verify
   `engine_enabled=true`, `proxies.token.alive=true`, `proxies.cache.alive=true`.
2. **Run the smoke test**: `aphrodite_test(mode="quick")` - 1-sample roundtrip;
   `mode="full"` (any non-quick value) runs the 3-check round-trip set
   (source_code/build/json_array). There is no `matrix`/`pipeline` mode.
   Expect `status="ok"`.
3. **Compress test content**:
    ```python
    result = aphrodite_compress(content="Your test content here\nLine 2\nLine 3", type="text")
    hash = result["hash"]      # e.g. "f398d3a3..."
    marker = result["marker"]  # e.g. "<<<CCR:hash|text|172>>>\n[preview]"
    ```
4. **Retrieve it**: `aphrodite_retrieve(hash=hash)` (full) and
   `aphrodite_retrieve(hash=hash, query="fox")` (filtered).
5. **Search and catalog**: `aphrodite_search(query="fox")`,
   `aphrodite_catalog(mode="toc")`.

The returned marker IS the proof of compression - retrieve only when you need
the actual content, never to verify storage (that wastes tokens and context).

## Auto-Expand vs. Manual Retrieval

Auto-expand is vestigial in the current codebase (the `auto_expand*` config
keys have no consumer - see `aphrodite-auto-expand-testing`). Raw markers are
the normal state for terminal output, background worker logs, and compressed
context. Whenever you SEE a marker, retrieve it immediately - the
retrieve-first rule is unconditional.

## Active Directives

- **focus** - targeted execution, CCR-first retrieval; every marker must be
  retrieved immediately.
- **foresight** - anticipate I/O; after `search_files`, prefetch the top 5-10
  results; use `aphrodite_prefetch` for batches of 3+ files.

Manage with:

```python
aphrodite_directive(action="list")                # list active/available
aphrodite_directive(action="swap", name="explore")  # swap directives
```

## Proxy Architecture (context)

- **Token proxy** (token listener - port is a config property, default
  `:9798`; read live from `aphrodite.toml` `ports`, never assume) -
  token-level compression; management endpoints require an API key.
- **Cache proxy** (cache listener - port is a config property, default
  `:9797`; read live) - cache-mode compression; management endpoints
  accept any loopback caller.
- Both share the CCR database at `~/.hermes/aphrodite/ccr.db`.
- Binary: `~/.hermes/aphrodite/binaries/aphrodite` (auto-updated).
- Dylib hot-reloads on file modification.

## Checklist for Agents

- [ ] See `<<<CCR:hash|type|size>>>` → `aphrodite_retrieve(hash=...)` NOW
- [ ] Multiple markers → retrieve ALL in the same turn
- [ ] Retrieval fails → fall back to `read_file` or `terminal`
- [ ] Never re-read a file you hold a live marker for - the marker IS the content
- [ ] `aphrodite_test(mode="quick")` to verify engine health
- [ ] `aphrodite_stats()` to check proxy health before relying on CCR tools
- [ ] `aphrodite_prefetch(paths=[...])` to batch-read 3+ files in background
