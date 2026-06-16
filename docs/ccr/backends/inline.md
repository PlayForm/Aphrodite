# Inline CCR (LruCache)

Origin: Tiny entries (< 256 bytes) bypass the CCR backend round-trip. An `lru::LruCache` in `AppState` provides O(1) retrieval without I/O. Avoids spawning a blocking task for SQLite read or acquiring a DashMap shard lock for content that's trivially small.

Source of truth: `crates/aphrodite/src/proxy.rs` (line 140: `inline_ccr` field, lines 1419-1431: store logic, lines 1423-1430: dedup check)

## Struct

```rust
pub inline_ccr: std::sync::Mutex<lru::LruCache<String, String>>
```

Constructed at startup:
```rust
inline_ccr: Mutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap()))
```
From `proxy.rs` line 482.

## Capacity

| Parameter | Value |
|-----------|-------|
| Entries | 1,024 (hard cap via LruCache) |
| Size threshold | < 256 bytes (`INLINE_CCR_THRESHOLD`) |

## TTL

None  -  pure LRU eviction. Entries evicted when LruCache exceeds capacity on `put()`.

## Storage Decision

Content is stored in inline_ccr when:
1. Content size > `INLINE_CCR_THRESHOLD` (256B)
2. Content size ≤ compression threshold for its type
3. (i.e., too big to ignore entirely, too small to compress to CCR backend)

From `proxy.rs:compress_chat_completion()` (line 1419):
```rust
} else if content.len() > INLINE_CCR_THRESHOLD {
    // Below compression threshold but above inline threshold
    let hash = compute_key(content.as_bytes());
    if let Ok(mut map) = state.inline_ccr.lock() {
        if map.contains(&hash) {
            state.inline_ccr_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            state.inline_ccr_misses.fetch_add(1, Ordering::Relaxed);
            map.put(hash, content.to_string());
        }
    }
}
```

Same logic applies to tool call arguments (`compress_chat_completion` line 1470).

## Dedup

Before storing, `contains()` check prevents duplicate entries. Hits/misses tracked via `inline_ccr_hits` / `inline_ccr_misses` AtomicU64 counters.

## Retrieval Priority

In `retrieve.rs:handle_retrieve()` (line 54):
1. Check inline_ccr first (lock dropped before any `.await`)
2. If hit: return immediately, increment `inline_ccr_hits` + `ccr_hits`
3. If miss: increment `inline_ccr_misses`, fall through to CCR backend

In `proxy.rs:execute_tool_relay()` (line 1568):
1. Check inline_ccr first for `aphrodite_retrieve`
2. If miss: fallback to CCR store

## Lock Safety

`Mutex<LruCache>`  -  lock is held only for cache operations (lookup/insert). Lock is dropped before any `.await` (e.g., before spawning blocking CCR store tasks) to avoid `!Send` MutexGuard crossing await points.

## Python Plugin Inline Store (Separate)

The Python plugin maintains its own inline store (`_core.py:_CappedStore`, max 500 entries) for when the proxy is down. This is NOT the same store  -  it lives in the Hermes Python process, not in the proxy Rust process.

| Property | Rust Inline (LruCache) | Python Inline (_CappedStore) |
|----------|----------------------|------------------------------|
| Type | `lru::LruCache<String, String>` | `OrderedDict` (subclass) |
| Capacity | 1,024 | 500 |
| TTL | None (LRU) | None (LRU) |
| Eviction | `put()` triggers LRU pop | `__setitem__` triggers LRU pop |
| Index | None | Trigram index (lazy) |
| Hash format | BLAKE3, 24 hex | SHA-256, 24 hex; or `i:` prefix |
| Process | Rust proxy | Python Hermes plugin |
