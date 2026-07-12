# In-Memory CCR Backend

The in-memory backend is a process-local, sharded concurrent CCR store used
for cache mode (`:9797`). It's lightweight by design - no persistence is
needed for the ephemeral cache proxy. Distinct keys never contend on reads;
capacity-bound eviction is the only serialized step.

## Struct

As a Rust struct:

```rust
pub struct InMemoryCcrStore {
    map: DashMap<String, Entry>,         // sharded concurrent hash map
    order: Mutex<VecDeque<String>>,      // FIFO insertion order for eviction
    ttl: Duration,                       // default 300s (from DEFAULT_TTL)
    capacity: usize,                     // default 1000 (from DEFAULT_CAPACITY)
}

struct Entry {
    payload: String,
    inserted: Instant,
}
```

## Constants

| Constant         | Value        |
| ---------------- | ------------ |
| DEFAULT_CAPACITY | 1,000        |
| DEFAULT_TTL      | 300s (5 min) |

Note: The proxy actually constructs the store with capacity 10,000 and a TTL
from `cli.ccr_ttl_seconds` (default 3600s):

```rust
InMemoryCcrStore::with_capacity_and_ttl(10_000, Duration::from_secs(cli.ccr_ttl_seconds))
```

## Constructors

```rust
pub fn new() -> Self                    // 1000 entries, 300s TTL
pub fn with_capacity_and_ttl(capacity: usize, ttl: Duration) -> Self
```

## Methods

### get(hash: &str) -> Option<String>

```
1. Check order queue size: if > capacity × 2, compact()
2. DashMap shard read-lock: look up hash
3. If found: check entry.inserted.elapsed() <= ttl
   a. Fresh: return Some(payload.clone())
   b. Expired: remove_if(hash, |e| e.inserted.elapsed() > ttl)
      - Atomic under shard write lock (no TOCTOU race)
      - If removed: return None
      - If not removed (concurrent put refreshed): re-read, return
4. If not found: return None
```

### put(hash: &str, payload: &str) -> bool

```
1. Check if hash already in map (get_mut)
   a. If yes: overwrite payload + refresh inserted timestamp, return true
   b. If no: continue
2. If map.len() >= capacity: evict_until_under_capacity()
3. Insert new Entry { payload, inserted: now }
4. If truly new key (prev.is_none()): push_back to order queue
```

### del(hash: &str) -> bool

```
DashMap::remove(hash).is_some()
```

### len() -> usize

```
DashMap::len()
```

## Eviction

### Capacity Eviction (evict_until_under_capacity)

```
While map.len() >= capacity:
  1. Pop front of order queue (oldest)
  2. DashMap::remove (no-op if already lazy-expired)
  3. Loop until map.len() < capacity
```

### Queue Compaction (compact)

```
Called when order.len() > capacity × 2:
  1. Lock order mutex
  2. retain(|key| map.contains_key(key))
```

Prevents unbounded queue growth from stale keys (entries that expired and were
removed from DashMap but still linger in the order queue).

## TTL Expiry

Lazy - checked on `get()`, not via background thread. Uses `remove_if()` for
atomic check-and-remove:

- Closes the TOCTOU race where: Thread A sees expired entry → drops read lock →
  Thread B re-inserts fresh entry with same hash → A's `remove()` wipes B's
  fresh data.
- With `remove_if`, predicate evaluation and removal happen under the same shard
  write lock.

## Concurrency Model

- **Reads**: Distinct hashes hash to distinct DashMap shards - no contention.
- **Writes**: `get_mut` on same hash serializes, but `put` on different hashes
  land in different shards.
- **Eviction**: Only serialized step is the `order` mutex (held for O(1) push or
  small sweep).
- **Poison**: Mutex locks recover from poison (log warning, continue).

## Race Condition: Soft-Cap

Between `map.len() >= capacity` check and `evict_until_under_capacity()`,
another thread could insert. The store may briefly exceed capacity. Eviction
loop handles this by continuing to pop until `map.len() < capacity`.

## Idempotency

`put` on existing hash overwrites in place via `get_mut` - same semantics as
SQLite's `ON CONFLICT(hash) DO UPDATE SET`.
