# SQLite CCR Backend

The SQLite backend is the production CCR store: persistent across worker
restarts and shareable across workers via a single database file. It runs in
WAL mode with lazy, debounced TTL purging - there are no background threads.

## Schema

```sql
CREATE TABLE IF NOT EXISTS ccr_entries (
    hash          TEXT PRIMARY KEY,
    original      BLOB NOT NULL,
    created_at    INTEGER NOT NULL,   -- unix-seconds
    ttl_seconds   INTEGER NOT NULL,   -- idle window, restarted on get
    last_accessed INTEGER NOT NULL    -- unix-seconds
);
```

Three secondary indexes back the non-PK queries (the expiry sweep and the LRU
batch delete), and the schema is at `user_version` 3:

| Index                 | Definition                                        | Serves                                            |
| --------------------- | ------------------------------------------------- | ------------------------------------------------- |
| `ccr_entries_expiry`  | expression index on `last_accessed + ttl_seconds` | the idle-window disjunct of the purge sweep       |
| `ccr_entries_created` | `created_at`                                      | the max-lifetime disjunct (`created_at < cutoff`) |
| `ccr_entries_lru`     | `last_accessed`                                   | `ORDER BY last_accessed` during cap enforcement   |

Point lookups are still single-row-per-PK. The sweep predicate is
`last_accessed + ttl_seconds < now OR created_at < cutoff`, and SQLite's
OR-optimisation drives each disjunct from its own index, so the sweep is a
range scan rather than a full table walk - which is what keeps retrieval cost
independent of store size. `CREATE INDEX IF NOT EXISTS` is idempotent, so an
existing DB pays one catalog lookup per index at open.

Databases created by older binaries are migrated in place: `user_version` 2
added `last_accessed` (backfilled from `created_at`), and 3 added the indexes.

## Expiry: Sliding Idle Window

The TTL is an idle window, not a wall clock: every successful `get` restarts
the row's clock via `last_accessed`. An absolute max lifetime - 8x the idle
TTL, measured from `created_at` - caps the sliding window so constant access
can never pin a row forever. A row expires when:

```sql
DELETE FROM ccr_entries
WHERE last_accessed + ttl_seconds < now
   OR created_at < now - max_lifetime;
```

The lifetime bound is written as `created_at < cutoff` with the cutoff
precomputed, rather than `created_at + max_lifetime < now`, so SQLite can drive
it from `ccr_entries_created`. Both forms are equivalent at whole-second
resolution.

## Retention: Space Caps

The TTL bounds the store in _time_ only: a burst of compression inside one TTL
window is unbounded in rows and bytes, and a hot entry's idle clock keeps
restarting. Two caps bound it in _space_:

| Cap           | Unit                                   | Config key          | Env override                  | Shipped value |
| ------------- | -------------------------------------- | ------------------- | ----------------------------- | ------------- |
| `max_entries` | live rows                              | `store_max_entries` | `APHRODITE_STORE_MAX_ENTRIES` | `100000`      |
| `max_bytes`   | payload bytes, `SUM(LENGTH(original))` | `store_max_bytes`   | `APHRODITE_STORE_MAX_BYTES`   | `536870912`   |

`0` (or an absent key) means unlimited. After each `put`, rows are evicted in
`last_accessed ASC, created_at ASC` order until both caps hold - the same
recency signal the sliding TTL uses, so an actively retrieved entry survives a
burst from another session and only genuinely cold rows are reclaimed.
Eviction runs in bounded batches of 256 rows, so one over-cap write cannot hold
the connection mutex for an unbounded stretch.

A store opened over an already-oversized DB is trimmed at open rather than on
the next `put` - the state an operator hits after adding a cap to an existing
deployment. The caps are read when the store is opened, so a change takes
effect on the next proxy restart; they are not hot-reloaded, unlike the
compression thresholds.

Eviction is a cache policy, not data loss: an evicted hash degrades to a
retrieval miss, exactly like an expired one.

The counters that decide "over cap" (`entries_total`, `live_bytes`) are
per-process - seeded from the DB at open and updated by this store's own
writes. When several processes share one DB file (the supported multi-worker
layout), each one's view drifts from the others until it reopens, so cap
enforcement is approximate in that arrangement. `stats_db()` always reports
exact `COUNT(*)` / `SUM(LENGTH(original))` values.

## Connection Configuration

| Setting      | Value  | Rationale                                                     |
| ------------ | ------ | ------------------------------------------------------------- |
| journal_mode | WAL    | Readers don't block writers                                   |
| synchronous  | NORMAL | Tolerable loss on power failure for a CCR cache               |
| busy_timeout | 5 s    | Blocks briefly instead of failing on cross-process contention |

## Purge

Purge is lazy and debounced: a sweep runs from `get` and `put`, but at most
once every 60 seconds (`PURGE_DEBOUNCE_SECS`), so a compress-heavy,
retrieve-light workload cannot accumulate expired rows and concurrent writes
do not re-execute the same DELETE. `force_purge_now()` runs a sweep on demand
for diagnostics. Purge failures and poisoned mutexes are logged and recovered -
the proxy keeps serving traffic.

`get` shares that same debounce, so retrieval no longer pays for store size.
The sweep used to run un-debounced on every `get`, measured 0.15 ms on an empty
store, 27 ms at 100k rows and 37.40 ms at 300k - all serialized behind the
connection mutex. With the shared debounce plus the index-driven sweep, a
missing-hash `POST /retrieve` on a 300k-row / 182 MB store went from 37.40 ms
to 0.22 ms. Skipping a sweep never serves stale content: the lookup predicate
itself refuses a row past its idle window or lifetime, so a deferred sweep only
delays reclaiming space.

## Operations

| Operation  | Behavior                                                                                                                                                                                                   |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `open`     | Opens or creates the DB, prepares the schema, applies migrations and indexes, applies the retention caps (an oversized DB is trimmed here); errors surface to the caller - no silent fallback to in-memory |
| `put`      | Runs the debounced sweep, upserts by primary key (`ON CONFLICT(hash) DO UPDATE`), then enforces the retention caps; returns `false` on SQL error (logged, no panic)                                        |
| `get`      | Runs the debounced sweep, then selects the row (the predicate itself refuses an expired row); on a hit, refreshes `last_accessed`; converts bytes to UTF-8                                                 |
| `del`      | Deletes by hash; returns whether a row was removed                                                                                                                                                         |
| `len`      | Maintained live-row counter, O(1) (seeded from `COUNT(*)` at open)                                                                                                                                         |
| `stats_db` | Structured telemetry (see below)                                                                                                                                                                           |

All hot statements are prepared once on connection setup and reused per call.
The connection is a single `rusqlite::Connection` behind a `Mutex`
(poison-tolerant); CCR reads and writes are short and rare relative to the
proxy hot path. Operators who measure contention can shard by running N
stores backed by N database files (one per worker) - multi-worker safety is
provided by SQLite's own file locking.

## Telemetry

`stats_db()` returns:

| Field                      | Meaning                                                                             |
| -------------------------- | ----------------------------------------------------------------------------------- |
| `total_entries`            | Row count                                                                           |
| `total_bytes_original`     | `SUM(LENGTH(original))`                                                             |
| `total_bytes_compressed`   | Entries x 24 (heuristic - the 40-char hash key is stored, not a compressed payload) |
| `oldest_entry_age_seconds` | Age of the oldest row                                                               |
| `database_size_bytes`      | Size of the database file on disk                                                   |
| `freelist_bytes`           | Reclaimable slack: pages freed by purge and eviction that the file still holds      |
| `max_entries`              | Configured row cap, or `null` when unlimited                                        |
| `max_bytes`                | Configured payload-byte cap, or `null` when unlimited                               |
| `evicted_total`            | Rows evicted by cap enforcement since this store was opened                         |

`freelist_bytes` plateaus once eviction keeps pace with writes: the file stops
growing because freed pages are reused by later inserts.

## Default Path

When no path is configured, the store opens `~/.hermes/aphrodite/ccr.db`
(home directory + `.hermes/aphrodite/ccr.db`).
