# CCR Empty Hash (CCR:{}) Bug Pattern

## Symptom
Catalog shows: `CCR:{} | 0B | preview text...`

The hash renders as literal `{}` — an empty dict, not a string.

## Root Causes (multiple)
1. **Empty hash in marker**: `_parse_ccr_markers` splits `<<<CCR:|type|size>>>` by `|`, giving `parts[0] = ''`. 
2. **Dict stored as hash**: Code path stored `{}` (empty dict) as hash value instead of string.
3. **Inline fallback producing empty hash**: `_compress_handler` exception path.

## Fix (Two-Layer Defense)

### Layer 1: Parse-time filter in `_parse_ccr_markers`
```python
# Cast to str, validate
markers.append({
    'hash': str(parts[0]) if parts[0] else '',
    ...
})
# Filter at return
return [m for m in markers if m['hash'] and len(m['hash']) >= 4]
```

### Layer 2: Display-time skip in catalog builder
```python
h = str(m.get('hash', '?'))
if not h or h in ('{}', '?', 'None'):
    continue
parts.append(f"      CCR:{h} | ...")
```

## Why Two Layers
Plugin changes require Hermes restart. If old plugin code is still in memory (no restart), Layer 1 won't execute. Layer 2 catches display even with old code. Both layers together prevent `CCR:{}` from appearing.

## Related
- Bi-directional store: compress/retrieve must store in `_inline_store`
- `_recent_markers` tracks all activity for search
- Content-addressable: same content = same hash = cache hit
