# Search Index — Content-Addressable Store Pattern

## Bug: Search returned 0 matches (v0.5.24 fix)

**Root cause**: `_compress_handler` sent data to proxy's SQLite store but never mirrored to `_inline_store`. `_search_handler` only scanned `_inline_store`, `_conv_index`, and `_recent_markers` — never queried the proxy.

**Fix**: Three changes applied across v0.5.24-0.5.26:
1. `_compress_handler` stores `hash:content` in `_inline_store` after successful proxy compress
2. `_transform_tool_result` CCR path also mirrors to `_inline_store` (not just `_recent_markers`)
3. `_recent_markers` populated on every compression (both CCR and INLINE paths) with 200-entry cap
4. `_parse_ccr_markers` now extracts preview text after `>>>` for richer search results

## Content-Addressable Store Pattern (v0.5.27)

"Pop the API" — every put is a search. Before calling the proxy:

```python
h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
if h in _inline_store:
    return cached result  # hit — no API call
# Miss — call proxy, then mirror to _inline_store
```

Benefits:
- Same content always produces same hash → cache hits on re-compression
- Proxy failure falls back to inline store (no error returned)
- Local cache absorbs repeated compressions

## Search Data Flow

```
_compress_handler (tool)           _transform_tool_result (hook)
        |                                    |
        v                                    v
   sha256(content)           _compress_via_proxy(proxy)
        |                                    |
   _inline_store check          _inline_store[h] = result
   (cache hit → return)         _recent_markers.append(...)
        |                                    |
   /ccr/create (proxy)           _ccr_marker(h, type, size)
        |
   _inline_store[h] = content
   
_search_handler scans:
  1. _conv_index (turn summaries)
  2. _inline_store (hash→content mappings)
  3. _recent_markers (metadata from pre_llm_hook catalog + real-time compressions)
```

All three sources are searched with case-insensitive substring matching. Results include hash for direct retrieval.
