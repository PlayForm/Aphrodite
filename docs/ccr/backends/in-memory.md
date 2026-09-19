# In-Memory CCR Backend

The in-memory backend is a process-local, sharded concurrent store used for
cache-mode deployments. It is lightweight by design - no persistence is
needed for an ephemeral cache - and distinct keys never contend on reads;
capacity-bound eviction is the only serialized step.

## Structure

```rust
pub struct InMemoryCcrStore {
    map: DashMap<String, Entry>,          // sharded concurrent hash map
    order: Mutex<VecDeque<String>>,       // insertion order for eviction
    ttl: Duration,                        // idle window
    max_lifetime: Duration,               // absolute ceiling, 8x ttl
    capacity: usize,
}

struct Entry {
    payload: String,
    inserted: Instant,
    last_accessed: Instant,
}
```

## Defaults

| Constant           | Value            |
| ------------------ | ---------------- |
| `DEFAULT_CAPACITY` | 1,000            |
| `DEFAULT_TTL`      | 1,800 s (30 min) |

The proxy constructs the store with a larger capacity and a configurable TTL:

```rust
InMemoryCcrStore::with_capacity_and_ttl(10_000, Duration::from_secs(cli.ccr_ttl_seconds))
```

(`ccr_ttl_seconds` defaults to 3600; see the config reference.)

## Expiry: Sliding Idle Window

Like the SQLite backend, the TTL is an idle window: every successful `get`
restarts the entry's clock via `last_accessed`, bounded by an absolute max
lifetime of 8x the idle TTL measured from insertion - so constant access can
never pin an entry forever.

## Operations

### get(hash)

1. If the order queue has grown past `capacity x 2`, compact it (drop keys
   whose entries no longer exist).
2. Look up the entry. If fresh, refresh `last_accessed` and return the
   payload.
3. If expired, evict with an atomic `remove_if` - predicate evaluation and
   removal happen under the same shard write lock, closing the race where a
   concurrent `put` of the same hash could be wiped by a stale reader's
   `remove`. If the entry was concurrently refreshed, return the fresh
   payload.

### put(hash, payload)

1. If the hash already exists, overwrite the payload and refresh both
   timestamps (idempotent re-store, order queue untouched).
2. If at capacity, evict the oldest entries until under capacity.
3. Insert the new entry; append the hash to the order queue only for a truly
   new key.

### del / len

`del` removes by hash and reports whether it existed; `len` is the live
DashMap entry count.

## Eviction

Capacity eviction pops the front of the order queue (oldest) and removes it
from the map, looping until under capacity; a soft-cap race may briefly push
the store over capacity, and the loop handles it. Queue compaction prunes
stale keys (expired or evicted entries whose keys linger in the queue) once
the queue exceeds twice the capacity, preventing unbounded growth under
sustained churn.

## Concurrency Model

- Reads on distinct hashes land in distinct DashMap shards - no contention.
- Writes on the same hash serialize via `get_mut`; writes on different
  hashes do not contend.
- The only globally serialized step is the order-queue mutex (an O(1) push
  or a small sweep).
- Mutex locks recover from poison (log a warning, continue).

## Idempotency

Re-storing the same hash overwrites in place - the same semantics as the
SQLite backend's `ON CONFLICT(hash) DO UPDATE`.
