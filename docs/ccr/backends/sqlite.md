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

There are no secondary indexes: lookups are single-row-per-PK, and the only
non-PK query (the purge sweep) runs against a small table. Databases created
by older binaries are migrated in place (the `last_accessed` column is added
and backfilled from `created_at`).

## Expiry: Sliding Idle Window

The TTL is an idle window, not a wall clock: every successful `get` restarts
the row's clock via `last_accessed`. An absolute max lifetime - 8x the idle
TTL, measured from `created_at` - caps the sliding window so constant access
can never pin a row forever. A row expires when:

```sql
DELETE FROM ccr_entries
WHERE last_accessed + ttl_seconds < now
   OR created_at + max_lifetime < now;
```

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
for diagnostics. Purge failures and poisoned mutexes are logged and recovered

- the proxy keeps serving traffic.

## Operations

| Operation  | Behavior                                                                                                                         |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `open`     | Opens or creates the DB, prepares the schema, applies migrations; errors surface to the caller - no silent fallback to in-memory |
| `put`      | Upserts by primary key (`ON CONFLICT(hash) DO UPDATE`), idempotent re-store; returns `false` on SQL error (logged, no panic)     |
| `get`      | Runs the purge sweep, then selects the row; on a hit, refreshes `last_accessed`; converts bytes to UTF-8                         |
| `del`      | Deletes by hash; returns whether a row was removed                                                                               |
| `len`      | `COUNT(*)` over the table                                                                                                        |
| `stats_db` | Structured telemetry (see below)                                                                                                 |

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

## Default Path

When no path is configured, the store opens `~/.hermes/aphrodite/ccr.db`
(home directory + `.hermes/aphrodite/ccr.db`).
