---
name: aphrodite-tool-guide
description:
    "Full reference for aphrodite CCR tools: retrieve, compress, stats, rebuild,
    files, diff, search, test, catalog. Usage examples, master-worker pattern,
    common pitfalls."
version: 1.0.0
platforms: [macos]
related_skills: [aphrodite-dev-workflow, aphrodite-hook-reference]
---

# Aphrodite Tool Guide

User-facing reference for the 9 aphrodite CCR tools. Load this skill when you
need to understand what each tool does, how to chain them, the master-worker
pattern, and common failure modes.

**Loaded automatically by Layer 2 (per-turn catalog hint):** The [APHRODITE]
catalog in every turn mentions this skill. If you see
`load aphrodite-tool-guide skill` in the catalog, load it here.

**Cross-ref:** For development workflow, releases, and profile switching, load
`aphrodite-dev-workflow` (Layer 4).

---

## Tool Reference

All tools are registered under `toolset="aphrodite"` and available when the
plugin is active. Profiles must include `aphrodite` in their `toolsets` list.

### 1. `aphrodite_retrieve(hash, query, path)`

Resolve a CCR marker hash to its full content via the proxy (token :9798 or
cache :9797), falling back to inline zlib store.

- **hash** - 16-char hex hash from a `<<<CCR:hash|type|size>>>` marker
- **query** - optional substring filter; only lines containing `query` are
  returned
- **path** - optional file path (bypasses CCR entirely, reads file directly)

```python
# Resolve a marker from the catalog
aphrodite_retrieve(hash="a1b2c3d4e5f6g7h8")

# Filter by keyword
aphrodite_retrieve(hash="a1b2c3d4e5f6g7h8", query="error")

# Read a file directly (no CCR)
aphrodite_retrieve(path="/path/to/file.py")
```

**Pitfall:** Hash must be exactly the 16-char hex from the marker. Truncated or
padded hashes return empty results.

### 2. `aphrodite_compress(content, type)`

Compress content and store as CCR. Type hint improves adaptive threshold
selection.

- **type** - optional content type hint: `code`, `log`, `diff`, `error`, `json`,
  `build_output`, `text`

```python
# Compress a code block
aphrodite_compress(content="fn main() { println!(\"hello\"); }", type="code")

# Compress build output
aphrodite_compress(content="Compiling foo v1.0\n   Compiling bar v2.0", type="build_output")
```

Returns a `<<<CCR:hash|type|size>>>` marker.

### 3. `aphrodite_stats()`

Proxy health, CCR stats, engine status, inline store size. Dumps all proxy
endpoints in a single call.

```python
# Full health check
aphrodite_stats()
# Returns JSON with proxy.{token,cache}.{alive,ccr_created,ccr_hits,...}
#          engine.{active,compressions,threshold_tokens,...}
#          inline_store.{entries,total_bytes}
```

**Key fields:**

- `proxy.token.ccr_created` - total CCR entries created this session on token
  proxy
- `proxy.cache.ccr_entries` - number of cached responses (may show "?" if plugin
  can't query)
- `inline_store.entries` - Python-side zlib cache entries
- `engine.compression_count` - context engine compressions (0 = engine off or
  not triggered yet)

**Pitfall:** 0 compressions on engine ≠ broken. The engine only fires when
Hermes decides context is full enough. Check `last_prompt_tokens` vs
`context_length` proportion.

### 4. `aphrodite_rebuild()`

Rebuild the aphrodite Rust binary from source and copy to
`~/.hermes/aphrodite/aphrodite`.

```python
# After Rust source changes
aphrodite_rebuild()
# Returns {"ok": true, "size": N, "path": "..."}
```

**Requires:** Cargo in PATH, source at HermesCompress root. Timeout: 300
seconds.

### 5. `aphrodite_files()`

List all file paths referenced in the current session, grouped by tool type.

```python
# See what files were read/written
aphrodite_files()
```

Returns JSON with grouped file paths. Only tracks tools in `_FILE_TOOLS` set.

### 6. `aphrodite_diff()`

Show conversation turn history - what was discussed, compressed, and stored
across turns.

```python
# Browse recent turn history
aphrodite_diff()
```

Returns user/assistant/tool summaries from the turn index (last 100 turns).

### 7. `aphrodite_search(query, type)`

Search across all CCR entries for matching keywords. Scans inline store,
conversation index, and recent markers.

- **query** - string to search for in compressed content
- **type** - optional filter by CCR type (tool, terminal, code, error, etc.)

```python
# Search for build errors
aphrodite_search(query="error:")

# Filter by type
aphrodite_search(query="compilation", type="build_output")
```

**Pitfall:** Search is local to Python's inline store. Proxy-side CCR entries
(token :9798 SQLite) are NOT searched unless they've been recently resolved into
`_recent_markers`.

### 8. `aphrodite_test(mode)`

Run the smoke test suite: compress → retrieve → search → stats → files → diff →
proxy health.

- **mode** - `quick` (default, basic smoke), `full` (comprehensive), `matrix`
  (multi-variant)

```python
# Quick smoke test
aphrodite_test()

# Full suite
aphrodite_test(mode="full")
```

### 9. `aphrodite_catalog()`

Return the full compression catalog - all CCR items with hashes, sizes, types,
and previews.

```python
# List all available context markers
aphrodite_catalog()
```

Returns the complete list of `<<<CCR:hash|type|size>>>` entries with descriptive
previews. Use this to discover what context can be retrieved.

---

## Master-Worker Pattern

When you have a complex task with independent workstreams, use `delegate_task`
(not aphrodite tools directly) for parallelism:

```
Main agent (you)
  ├─ Worker A: analyze code in src/foo.rs
  ├─ Worker B: analyze code in src/bar.rs
  └─ Worker C: write test for baz module
```

**What workers need to know:**

- Workers CAN use `aphrodite_retrieve(hash)` to fetch compressed content
- Workers CAN use `aphrodite_compress()` to store large results
- Workers CAN NOT delegate further (leaf agents)
- Workers return summaries only - final results appear in your context

**Pitfall - CCR content per-worker:** Each worker has its OWN inline store and
catalog. Compressed content created by Worker A is NOT visible to Worker B
unless stored via the proxy (token :9798) which has a shared SQLite database.
For cross-worker data sharing:

- Compress via `aphrodite_compress()` (uses proxy when alive)
- Pass the resulting CCR hash in the worker's `context` string
- Worker resolves with `aphrodite_retrieve(hash)`
- OR: write results to a shared file, pass path to worker

**Pitfall - proxy state per-worker:** Workers have independent
terminal/file/browser sessions. They share the same proxy servers but not the
same Python plugin state. `inline_store` is per-worker.

---

## Common Pitfalls

### CCR Markers in Responses

The LLM sees `<<<CCR:hash|type|size>>>` markers in tool/terminal output. These
are NOT errors - they're compressed content. Use `aphrodite_retrieve(hash)` to
expand them. The pre_llm_hook auto-expands small tool-type markers (<50KB), but
context/terminal markers stay as references.

### Catalog vs Search

- `aphrodite_catalog()` - lists ALL markers with previews. Use when you don't
  know what's available.
- `aphrodite_search(query)` - finds markers by content keyword. Use when you
  know what you're looking for.

Both are complementary. Start with catalog to discover, then search to narrow.

### Stats Zero-Compression

If `aphrodite_stats()` shows zero compression:

1. Check `proxy.token.alive` - if False, proxy is down
2. Check `inline_store.entries` - if 0, no inline compression happened either
3. Check `_DEV` - if dev mode (APHRODITE_PASSTHROUGH=1), all compression is
   bypassed
4. Check tool output sizes - tools producing <1KB output don't trigger
   compression
5. Check `TOOL_THRESHOLD_TOKEN=1024` / `TOOL_THRESHOLD_CACHE=8192` - threshold
   might be too high

### No Search Results

`aphrodite_search` only scans Python's inline store. If content was compressed
via the token proxy and never resolved locally, it won't appear in search. Fix:
resolve the marker first with `aphrodite_retrieve(hash)`, which populates the
inline store, then search.

### Auto-Expand and Missing Content

Small tool markers (<50KB by default, configurable via
`APHRODITE_AUTO_EXPAND_LIMIT`) are automatically resolved in `_pre_llm_hook`
before the LLM sees them. The content is inline. If a tool marker was NOT
expanded but should have been:

1. Check `AUTO_EXPAND_LIMIT` - default 51200 bytes
2. Check the proxy was alive at the time of `_pre_llm_hook` execution
3. The auto-expanded markers are removed from the catalog (no need to retrieve
   them)

### Tools Not Available

If aphrodite tools don't appear in the tool list:

1. `hermes plugins list` - must show `aphrodite | enabled`
2. Profile config must include `toolsets: ["hermes-cli", "aphrodite"]`
3. Not in passthrough mode (`unset APHRODITE_PASSTHROUGH`)
4. Hermes restart required after plugin toggle

---

## Chaining Tools

Common workflows:

### Debug a Build Failure

```python
# 1. Check system health
aphrodite_stats()
# 2. Run the build
terminal(command="cargo build 2>&1", timeout=120)
# 3. Search for errors in compressed output
aphrodite_search(query="error", type="terminal")
# 4. Retrieve full context if needed
aphrodite_retrieve(hash="<from catalog>")
```

### Investigate a Known Issue

```python
# 1. Read previous discussion
session_search(query="build failure")
# 2. Check current state
aphrodite_files()  # recent files
aphrodite_stats()  # proxy/engine health
# 3. Retrieve compressed discussion turns
aphrodite_diff()   # turn history
```

---

## Layer Reference

This is **Layer 3** of the aphrodite layered instruction system:

| Layer | Source                                         | Content                                                           | Cross-Ref                            |
| ----- | ---------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------ |
| 1     | `_inject_session_instruction()` in `_hooks.py` | Session start: version, proxy state, engine threshold             | → Layer 2                            |
| 2     | `_pre_llm_hook` catalog                        | Per-turn: markers available, auto-expand count                    | → Layer 3 (this skill)               |
| 3     | `aphrodite-tool-guide` skill (this file)       | Full tool reference, examples, master-worker, pitfalls            | → Layer 4 (`aphrodite-dev-workflow`) |
| 4     | `aphrodite-dev-workflow` skill                 | Release pipeline, profile switching, build monitor, flash workers | → back to Layer 3                    |

Load the next layer with `skill_view(name="aphrodite-dev-workflow")`.
