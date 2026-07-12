# SQLite CCR Backend

The SQLite backend is a persistent, shareable CCR store for production (token
mode) proxy deployments. It survives worker restarts and runs in WAL mode
with lazy TTL purging - there are no background threads.

## Schema

```sql
CREATE TABLE IF NOT EXISTS ccr_entries (
    hash         TEXT PRIMARY KEY,
    original     BLOB NOT NULL,
    created_at   INTEGER NOT NULL,   -- unix-seconds
    ttl_seconds  INTEGER NOT NULL
);
```

No secondary indexes - single-row-per-PK lookups only. Purge sweeps use
`WHERE created_at + ttl_seconds <= ?1` on the (small) table.

## Connection Configuration

| Setting      | Value  | Rationale                                          |
| ------------ | ------ | -------------------------------------------------- |
| journal_mode | WAL    | Readers don't block writers                        |
| synchronous  | NORMAL | Tolerable data loss on power failure for CCR cache |

## Struct

```rust
pub struct SqliteCcrStore {
    conn: Mutex<Connection>,         // !Sync, wrapped in Mutex
    default_ttl_seconds: u64,
    path: PathBuf,
    last_purge: Mutex<Option<Instant>>,  // debounce last purge time
}
```

## Constructor

```rust
pub fn open(path: impl AsRef<Path>, default_ttl_seconds: u64) -> rusqlite::Result<Self>
```

Errors surface to caller - no silent fallback to in-memory.

## Methods

### get(hash: &str) -> Option<String>

```
1. lock conn mutex (poison-tolerant)
2. maybe_purge(): if last_purge > 60s ago, DELETE expired rows
3. SELECT original FROM ccr_entries WHERE hash = ?1 AND created_at + ttl_seconds > ?2
4. Convert bytes → String via from_utf8()
```

### put(hash: &str, payload: &str) -> bool

```
INSERT INTO ccr_entries (hash, original, created_at, ttl_seconds)
VALUES (?1, ?2, ?3, ?4)
ON CONFLICT(hash) DO UPDATE SET
    original    = excluded.original,
    created_at  = excluded.created_at,
    ttl_seconds = excluded.ttl_seconds
```

Upsert by primary key - idempotent re-store. Returns `false` on SQL error (logs
warning, does not panic).

### del(hash: &str) -> bool

```sql
DELETE FROM ccr_entries WHERE hash = ?1
```

Returns `true` if rows affected > 0.

### len() -> usize

```sql
SELECT COUNT(*) FROM ccr_entries
```

Returns 0 on error.

### stats_db() -> Option<serde_json::Value>

```json
{
    "total_entries": i64,
    "total_bytes_original": i64,       // SUM(LENGTH(original))
    "total_bytes_compressed": i64,     // entries × 24 (heuristic)
    "oldest_entry_age_seconds": i64?,
    "database_size_bytes": u64         // fs::metadata(path).len()
}
```

## TTL & Purge

### Lazy Purge

```sql
DELETE FROM ccr_entries WHERE created_at + ttl_seconds <= ?1
```

Called from `get()` via `maybe_purge()`. Returns number of purged rows.

### Debounce

```rust
const PURGE_DEBOUNCE_SECS: u64 = 60;
```

`last_purge` mutex tracks last purge time. Purge only fires if ≥ 60s since last
sweep.

### Now

```rust
SystemTime::now().duration_since(UNIX_EPOCH).as_secs()
```

Falls back to 0 on clock-before-epoch (impossible on sane hosts).

## Poison Resilience

All `Mutex::lock()` calls recover from poison:

```rust
fn lock_conn(conn: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    match conn.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(target = "ccr.sqlite", "ccr_sqlite_mutex_poisoned_recovered");
            poisoned.into_inner()
        }
    }
}
```

## Default Path

When not specified: `~/.hermes/aphrodite/ccr.db`, constructed as:

```rust
dirs::home_dir()
    .unwrap_or_else(|| PathBuf::from("/tmp"))
    .join(".hermes")
    .join("aphrodite")
    .join("ccr.db")
```

## Concurrency Model

- Single `rusqlite::Connection` behind `Mutex` (Connection is `!Sync`)
- CCR reads/writes are short and rare relative to proxy hot path
- Sharding: operators can spin up N stores backed by N DB files (one per worker)
- Multi-worker safety: SQLite file locking
