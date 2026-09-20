> **STALE-ALERT (2026-09-20, Development):** This reference is historical
> evidence. The plugin is now a pure loader - the Python-side `_transform_*`
> handlers and `_CCR_RE` guard no longer exist; the skip logic lives in the
> Rust dylib transform path. The guard rule itself is still canonical
> (owner: `aphrodite-compression-safety`): retrieval/diagnostic tool results
> are never re-compressed.

# Re-Compression Guard (v1.8.1+)

The `_transform_tool_result` and `_transform_terminal_hook` both compress
content via CCR. But when `aphrodite_retrieve` returns already-compressed
content, it should NOT be re-compressed.

## Guard Pattern

Add after the size threshold check in both hooks:

```python
# Don't re-compress content that already has CCR markers (retrieved/compressed)
if _CCR_RE.search(result):
    return result
```

## Skip Set

In `_transform_tool_result`, skip compression for aphrodite's own tools:

```python
skip = {"read_file", "read_terminal",
        "aphrodite_retrieve", "aphrodite_compress", "aphrodite_stats"}
```

## Read-Intent Detection (pre_llm_hook)

When the user's last message contains read keywords, surface
`aphrodite_retrieve(hash)` hints for the 3 most recent CCR markers. This reduces
unnecessary round-trips when the LLM was about to retrieve anyway.
