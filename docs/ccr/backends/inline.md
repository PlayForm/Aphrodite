# Inline CCR

Tiny entries bypass the CCR backend round-trip entirely. Two inline stores
exist: one in the proxy process for chat-completions and HTTP tool-relay
traffic, and one in the Hermes session engine for the plugin-side
compress/retrieve path. Both provide O(1) retrieval without backend I/O for
content that is trivially small.

## Proxy Inline Store

An `lru::LruCache` held on the proxy's shared state:

```rust
pub inline_ccr: std::sync::Mutex<lru::LruCache<String, String>>
```

Constructed at startup with capacity 1,024.

| Parameter      | Value                                                                                                                                                |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entries        | 1,024 (hard cap via LruCache)                                                                                                                        |
| Size threshold | inline threshold, default 256 B (`INLINE_CCR_THRESHOLD`), configurable via `APHRODITE_INLINE_THRESHOLD` env or TOML `[compression] inline_threshold` |
| TTL            | None - pure LRU eviction                                                                                                                             |

Content lands here when it is above the inline threshold but at or below the
compression threshold for its type - too big to ignore, too small to send to
the backend. Before inserting, a `contains()` check prevents duplicate
entries; hits and misses are tracked via `inline_ccr_hits` /
`inline_ccr_misses` counters.

In the tool-relay path, tiny content is stored inline AND best-effort to the
durable backend when one is configured: the inline map is process memory, so
a busy session could evict the entry while its marker still looks durable.
The inline copy serves reads until eviction; a failed durable put does not
fail the call.

## Session Inline Store

The Hermes session engine keeps its own inline store in state
(`state.rs`): a `HashMap<String, String>` with an LRU order queue
(`VecDeque`), entry-count and byte-budget bounds, and a promoting `get`.

| Parameter   | Value                                                                                      |
| ----------- | ------------------------------------------------------------------------------------------ |
| Entries     | 500 (`INLINE_MAX`)                                                                         |
| Byte budget | 256 MiB (`DEFAULT_INLINE_BYTE_BUDGET`)                                                     |
| TTL         | None - LRU + byte-budget eviction                                                          |
| Membership  | `inline_store_contains` is non-promoting (checking resolvability never perturbs LRU order) |

Eviction drops the least-recently-used entry until both the entry-count cap
and the byte budget are satisfied; lowering the budget evicts immediately.
This store backs `aphrodite_prefetch`, `aphrodite_compress`, and recursive
marker resolution, and is the source of truth for retrieval - it round-trips
arbitrary content (NUL bytes, multibyte UTF-8, literal marker-shaped text)
byte-for-byte.

## Retrieval Priority

Both paths check the inline store first, then fall back to the CCR backend:

1. Inline hit -> return immediately (proxy: also increment `ccr_hits`).
2. Inline miss -> increment `inline_ccr_misses`, fall through to the backend.
3. Backend miss -> 404 `NOT_FOUND` (HTTP) / `{"found": false}` (tool).

## Lock Safety

The proxy's `Mutex<LruCache>` is held only for cache operations (lookup,
insert) and dropped before any `.await` (e.g. before a blocking backend
task), so a `!Send` guard never crosses an await point. The session store is
owned by the engine's single-threaded state and needs no lock.
