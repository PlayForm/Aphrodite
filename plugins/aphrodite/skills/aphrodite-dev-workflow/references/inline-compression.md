# Inline Compression Fallback

When the aphrodite proxy is unreachable, hooks fall back to inline zlib compression.

## How It Works

1. Tool/terminal output exceeds threshold (4KB for inline)
2. Content is zlib-compressed and stored in session-scoped `_inline_store` dict
3. SHA256 hash (first 16 chars) serves as the CCR marker hash
4. `headroom_retrieve` checks inline store before proxy

## Implementation

```python
def _inline_compress(content):
    compressed = base64.urlsafe_b64encode(
        zlib.compress(content.encode('utf-8'), 9)
    ).decode('ascii')
    h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
    _inline_store[h] = content
    return h, len(compressed)
```

## Bounds

- Max 500 entries in _inline_store
- Oldest entry evicted when limit exceeded
- Session-scoped (lost on Hermes restart)
- 4KB minimum threshold (higher than proxy's 1KB to avoid memory bloat)

## Marker Format

Inline markers include mode="inline":
```
[CCR:abc123def456|tool|8192|inline] {"output": "hello...
```

## Advantages

- No provider switch required (works with direct DeepSeek)
- Survives proxy restarts/crashes
- Zero network dependency for compression
- Faster than proxy round-trip for compression
