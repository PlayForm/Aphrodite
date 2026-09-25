> **STALE-ALERT (2026-09-20, Development):** This reference is historical
> evidence. Verified current source differs: the key is BLAKE3 first-40 hex
> (`vendor/headroom/crates/headroom-core/src/ccr/mod.rs` `compute_key`), not
> SHA-256/12-16 hex; the LLM-facing marker is `<<<CCR:hash|type|size>>>` with
> preview/structure on the lines ABOVE the marker, no `mode` field
> (`crates/aphrodite/src/proxy.rs` `proxy_format_ccr_output`); parsing is in
> `crates/aphrodite/src/resolve/parse.rs` + `marker/parse.rs`. Canonical
> grammar: `aphrodite-ccr-protocol`.

# CCR Marker Format & Compression Pipeline (v1.9.0+)

## Marker Format

**Standard ASCII markers** - universal compatibility, no Unicode issues:

```
<<<CCR:{hash}|{type}|{size}|{mode}>>> {preview}
```

| Field | Values                  | Source                     |
| ----- | ----------------------- | -------------------------- |
| hash  | 12-16 hex chars         | Proxy SHA or inline SHA256 |
| type  | tool, terminal, context | Which hook produced it     |
| size  | bytes integer           | Original content size      |
| mode  | token, cache, inline, ? | Compression source         |

## Parsing

```python
_CCR_RE = re.compile(r'<<<CCR:([^>]+)>>>')

def _parse_ccr_markers(text):
    markers = []
    for m in _CCR_RE.findall(text):
        parts = m.split('|')
        if len(parts) >= 3:
            markers.append({
                'hash': parts[0],
                'type': parts[1],
                'size': int(parts[2]),
                'mode': parts[3] if len(parts) > 3 else '?'
            })
    return markers
```

## Rust smart_marker

```rust
fn smart_marker(hash: &str, content: &str, ct: &str) -> String {
    let size = content.len();
    let preview = &content[..content.len().min(120)];
    let oneliner = preview.lines().next().unwrap_or(preview).trim();
    format!("<<<CCR:{}|{}|{}>>> {}", hash, ct, size, oneliner)
}
```

## Pipeline Flow

```
Terminal output (>TERMINAL_THRESHOLD)
    → transform_terminal_output hook
    → <<<CCR:hash|terminal|size>>> preview...
    → JSON wrapped: {"output": "<<<CCR:...>>> preview", "returncode": 0}
    → transform_tool_result hook (sees JSON >1KB)
    → <<<CCR:hash|tool|size|token>>> {"output": "<<<CCR:...

Tool output (>1KB token, >8KB cache)
    → transform_tool_result hook
    → <<<CCR:hash|tool|size|mode>>> preview...
```

## Retrieval Chain

1. LLM sees `<<<CCR:hash|tool|size>>> preview`
2. Calls `aphrodite_retrieve(hash)`
3. `_resolve_one`: checks inline store first, then tries both proxies
   (9797, 9798)
4. `_resolve_recursive`: scans result for nested <<<CCR:...>>> markers, resolves
   up to 3 levels deep
5. Returns fully resolved content

## Thresholds

| Source          | Proxy Token         | Proxy Cache         | Inline Fallback   |
| --------------- | ------------------- | ------------------- | ----------------- |
| Tool output     | >1KB                | >8KB                | >4KB              |
| Terminal output | >TERMINAL_THRESHOLD | >TERMINAL_THRESHOLD | >INLINE_THRESHOLD |
