# Three-Layer Ghost Entry Defense

## Problem

Catalog shows `CCR:abc123 | 0B` or `CCR:{} | 0B | <preview>` entries that:
- Don't correspond to real compressed content
- Would fail on retrieve (hash doesn't exist in store)
- Waste agent turns trying to fetch dead references
- Come from documentation example markers or parse bugs

## Defense Layers (apply in order)

### Layer 1: Hex Validation (v0.5.43)
Real CCR hashes are hex strings (0-9, a-f) from BLAKE3/SHA256. Filter at parse:

```python
return [m for m in markers 
        if m['hash'] and len(m['hash']) >= 8 
        and all(c in '0123456789abcdef' for c in m['hash'].lower())]
```

This kills: `abc123` (6 chars, even though hex), `ABC`, `test`, `example`

### Layer 2: Type Cast + Skip (v0.5.36)
At display time, cast hash to str() and skip known bad values:

```python
h = str(m.get('hash', '')).strip()
if len(h) < 4 or h in ('{}', '?', 'None', 'null', 'undefined'):
    continue
```

This catches: empty string `""`, literal `{}` dict, `None`, `null`, `undefined`

### Layer 3: Liveness Check (v0.5.44)
Before displaying, verify the hash is actually retrievable:

```python
live = [m for m in markers if m['hash'] in _inline_store or _inline_retrieve(m['hash'])]
if not live and markers:
    live = markers  # fallback — never hide all from a buggy filter
```

This kills: any hash that would fail on retrieve, regardless of format validity. The fallback ensures a buggy filter never hides real data.

## Root Causes

| Symptom | Root Cause | Fix |
|---------|-----------|-----|
| `CCR:abc123` | Documentation example markers parsed as real | Layer 1: hex + length validation |
| `CCR:{}` | Empty string/dict stored as hash from bad parse | Layer 2: type cast + skip known bad values |
| `CCR:deadhash` | Marker from old session with expired CCR entry | Layer 3: liveness check before display |
| `\|tool\|token>>>` in previews | `_extract_preview` split on `]` (old bracket format) | Split on `>>>` instead: `after.split('>>>', 1)[-1]` |
| Preview starting inside marker | `findall()` no position info → wrong text offset | Use `finditer()` with `match.end()` |