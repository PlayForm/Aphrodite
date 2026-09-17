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
