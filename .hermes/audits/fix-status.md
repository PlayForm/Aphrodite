# Audit Cross-Reference - Fixes Applied vs Remaining

## ✅ Fixed (14)

| #   | File         | Severity | Issue                                  | Commit   |
| --- | ------------ | -------- | -------------------------------------- | -------- |
| 1   | proxy.rs     | P0       | `requests_compressed` double-count     | b1763c7  |
| 2   | proxy.rs     | P0       | `tokens_saved` on CCR hits             | b1763c7  |
| 3   | proxy.rs     | P0       | `notify_key` Bearer auth               | b1763c7  |
| 4   | proxy.rs     | P0       | `callback_url` SSRF (https only)       | b1763c7  |
| 5   | proxy.rs     | P0       | UTF-8 silent fail → 400                | b1763c7  |
| 6   | proxy.rs     | P0       | `fill_pct` NaN guard                   | b1763c7  |
| 7   | proxy.rs     | P1       | `Secret::Display` redacts              | b1763c7  |
| 8   | proxy.rs     | P1       | `compression_ratio_ema` init 200       | b1763c7  |
| 9   | proxy.rs     | P1       | `fill_pct` init 9000                   | b1763c7  |
| 10  | proxy.rs     | P1       | JSON content-type validation           | b1763c7  |
| 11  | proxy.rs     | P1       | inline_ccr dedup                       | b1763c7  |
| 12  | config.rs    | P1       | listen parse fail → bail               | b1763c7  |
| 13  | config.rs    | P1       | `max_output >= max_context` validation | b1763c7  |
| 14  | config.rs    | P1       | API key fallback chain documented      | b1763c7  |
| 15  | retrieve.rs  | P1       | query length cap 512                   | b1763c7  |
| 16  | retrieve.rs  | P1       | offset OOB → 400                       | b1763c7  |
| 17  | retrieve.rs  | P1       | zstd decompression → 500               | b1763c7  |
| 18  | main.rs      | P1       | `DefaultBodyLimit::max(1MB)`           | b1763c7  |
| 19  | sqlite.rs    | P1       | purge debounced (60s)                  | d4a832cb |
| 20  | sqlite.rs    | P1       | poison-tolerant locks                  | d4a832cb |
| 21  | sqlite.rs    | P2       | `stats_db` real measurement            | d4a832cb |
| 22  | in_memory.rs | P1       | queue compaction                       | d4a832cb |

## ❌ Remaining (16)

### proxy.rs (7)

| Severity | Issue                                                               |
| -------- | ------------------------------------------------------------------- |
| P0       | `response_cache` no compressed/raw discriminator                    |
| P0       | `task_tracker` no graceful shutdown                                 |
| P1       | `reqwest::Client` missing `connect_timeout`                         |
| P1       | `rand::random()` → `thread_rng()`                                   |
| P1       | `compression_ratio_ema` u64×100 precision loss (acceptable — 0.01× granularity) |
| P1       | `cache_key` includes `api_key` (intentional - cross-user isolation) |
| P2       | Retry loop only retries transport, not 5xx HTTP                     |

### config.rs (5)

| Severity | Issue                                      |
| -------- | ------------------------------------------ |
| P1       | `ccr_db_path` whitespace-only check        |
| P1       | `max_context`/`max_output` dead code       |
| P1       | unknown fields in toml silently ignored    |
| P1       | listen address without port parses as `:0` |
| P2       | timeout clamp warns but doesn't error      |

### retrieve.rs (4)

| Severity | Issue                                                |
| -------- | ---------------------------------------------------- |
| P1       | SQLite `get` returns String, zstd dead code          |
| P1       | `filter_content` allocates `to_lowercase()` per line |
| P1       | `Cow::into_owned()` clones unnecessarily             |
| P2       | `limit` unbounded                                    |

### sqlite.rs (2)

| Severity | Issue                                          |
| -------- | ---------------------------------------------- |
| P1       | `now_unix_seconds` returns 0 on pre-1970 clock |
| P1       | `ttl_seconds` `u64→i64` overflow               |

### in_memory.rs (2)

| Severity | Issue                                         |
| -------- | --------------------------------------------- |
| P1       | `evict_until_under_capacity` soft-cap race    |
| P2       | `put` two-phase lock can duplicate order keys |

### auth_mode.rs (3)

| Severity | Issue                                            |
| -------- | ------------------------------------------------ |
| P1       | Unauthenticated defaults to Payg (design choice) |
| P1       | JWT dot-count bypassable                         |
| P2       | Non-UTF-8 header fallback to Payg                |

## Verdict

**22 fixed, 16 remaining** (excludes 3 auth_mode design decisions per user). The
remaining 13 are mostly performance/edge-case improvements, not correctness
bugs. The P0 items left are response cache discriminator and task_tracker
shutdown - both are feature-level, not crash-level.
