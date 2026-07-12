# CCR Lifecycle

CCR (Compress-Cache-Retrieve) provides lossless end-to-end compression for LLM
proxy traffic. Content is hashed, compressed, stored, and replaced with a
marker in the response; the LLM retrieves the original by hash if it needs the
full content. This doc walks through the six phases of that lifecycle, from
initial compression to eventual expiry.

## Phase 1: Compress

```
1. Parse Chat Completions response JSON
2. For each choice.message.content AND each tool_call.function.arguments:
   a. detect_content_type() → ct (e.g., "code_rust", "error", "diff")
   b. threshold_for(ct) × budget_mult (from x-headroom-budget) → threshold
   c. If content.len() > threshold:
      i.   compute_key(content) → hash (BLAKE3, 24 hex chars)
      ii.  Check CCR cache: ccr.get(hash) → hit or miss
      iii. If miss: ccr.put(hash, content), increment ccr_created
      iv.  Update tokens_saved counter
      v.   Generate smart_marker(hash, content, ct) → marker string
      vi.  Replace content/arguments with marker in JSON
      vii. update_compression_ratio(original_len, marker_len) → EMA
   d. Else if content.len() > INLINE_CCR_THRESHOLD (256B):
      i.   compute_key(content) → hash
      ii.  Store in inline_ccr LruCache (max 1024 entries)
```

## Phase 2: Cache

### Cache Check

```
ccr.get(hash) → hit? ccr_hits++ : ccr_misses++ (then store)
```

### Inline Cache Check

```
inline_ccr.contains(hash)? inline_ccr_hits++ : inline_ccr_misses++ (then store)
```

### LLM Response Cache

```
cache_key = FNV-1a(api_key + ":" + model + ":" + serialized_messages)
response_cache.get(cache_key) → hit? return cached : proceed to upstream
```

| Property   | Detail                                          |
| ---------- | ------------------------------------------------ |
| Hash       | FNV-1a 64-bit (deterministic across restarts)   |
| Capacity   | LRU, 128 entries                                |
| Key scope  | Includes `api_key` to prevent cross-user collision |
| Response header | `X-Aphrodite-Cache: HIT` or `MISS`          |

## Phase 3: Store

Three storage tiers by content size and mode:

| Tier       | Threshold | Backend                         | Capacity         | TTL                          |
| ---------- | --------- | ------------------------------- | ---------------- | ---------------------------- |
| Inline     | < 256B    | `lru::LruCache<String, String>` | 1,024 entries    | LRU eviction only            |
| Cache mode | > 8KB     | `InMemoryCcrStore` (DashMap)    | 10,000 entries   | Configurable (default 3600s) |
| Token mode | > 1KB     | `SqliteCcrStore` (SQLite)       | Unlimited (disk) | Configurable (default 3600s) |

### Python Plugin Inline Store

Separate from the Rust inline store: `_CappedStore`, an `OrderedDict`-backed
store capped at 500 entries. Used when the proxy is down.

## Phase 4: Return Marker

### Cache Mode Response

```
<<<CCR:HASH|TYPE|SIZE>>>
FIRST_512_BYTES_OF_CONTENT
```

Entire response JSON is rewritten. Response headers:

- `X-Aphrodite-Compressed: true`
- `X-Aphrodite-Cache: MISS` (or `HIT`)
- `X-Aphrodite-Fill-Pct: XX.X`

### Token Mode Response

```
<<<CCR:HASH|TYPE|SIZE|METADATA>>>
```

Includes structured metadata (language, functions, line count, etc.).

## Phase 5: Retrieve

Retrieval flow:

```
1. Validate hash parameter (required)
2. Check inline_ccr (LruCache):
   a. Hit → return content immediately
   b. Miss → fall through to CCR backend
3. Check CCR backend (SQLite or in-memory):
   a. ccr.get(hash) → hit or miss
4. If hit: check for zstd magic bytes (0x28, 0xB5, 0x2F, 0xFD)
   a. If zstd compressed: zstd::decode_all() → decompress
   b. Otherwise: return as-is
5. If miss: 404 NOT_FOUND
6. Apply optional query filter (case-insensitive line grep, max 512 chars)
7. Apply optional pagination (offset + limit)
8. Return {found: true/false, content: "…", source: "ccr"/"none"}
```

### Python Retrieve

The Python plugin's retrieve handler follows the same flow, plus:

- Recursive resolution up to 3 levels deep (nested CCR markers)
- File path reads (workspace-bounded, 10MB cap)
- Inline store fallback

## Phase 6: Expire

| Backend        | Expiry behavior                                                                                                             |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| SQLite         | Lazy purge on every `get()`: `DELETE FROM ccr_entries WHERE created_at + ttl_seconds <= now`. Debounced to once per 60 seconds. No background thread. |
| In-Memory      | Lazy TTL check on every `get()`: entries older than TTL are evicted via an atomic check-and-remove (prevents a TOCTOU race with a concurrent `put`). Queue compaction runs once the eviction queue grows past twice capacity, to clear stale keys. |
| Inline         | LRU eviction once capacity (1,024) is exceeded. No TTL - pure LRU.                                                            |
| Python Inline  | LRU eviction once the store exceeds 500 entries. No TTL.                                                                       |

## Threshold Tables

### Base Thresholds

| Constant                 | Bytes       | Mode  |
| ------------------------ | ----------- | ----- |
| CACHE_COMPRESS_THRESHOLD | 8,192 (8KB) | Cache |
| TOKEN_COMPRESS_THRESHOLD | 1,024 (1KB) | Token |
| INLINE_CCR_THRESHOLD     | 256         | All   |

### Per-Type Multipliers

| Type                        | Multiplier | Effective (Token, 1KB base) | Effective (Cache, 8KB base) |
| --------------------------- | ---------- | --------------------------- | --------------------------- |
| error                       | ×8         | 8,192                       | 65,536                      |
| code_rust/python/go/js/code | ×4         | 4,096                       | 32,768                      |
| diff, git, text             | ×2         | 2,048                       | 16,384                      |
| tool_output, json           | ×1         | 1,024                       | 8,192                       |
| linter, build_output, log   | ÷2         | 512                         | 4,096                       |

### Auto-Tune

Based on `compression_ratio_ema` (×100):

| EMA Ratio   | Tune Factor | Effect                                          |
| ----------- | ----------- | ----------------------------------------------- |
| > 20.0      | 2.0         | Raise thresholds (compress less, preserve more) |
| 3.0 .. 20.0 | 1.0         | Default                                         |
| < 3.0       | 0.5         | Lower thresholds (compress more aggressively)   |
| 0.0         | 1.0         | No history - default                            |

Note: `linter`, `build_output`, `log` types are excluded from auto-tune (always
base/2).

### Python Plugin Thresholds

| Env Var               | Default             | Scope                                                                |
| --------------------- | ------------------- | -------------------------------------------------------------------- |
| TOOL_THRESHOLD_TOKEN  | 1,024               | Tool outputs when token proxy alive                                  |
| TOOL_THRESHOLD_CACHE  | 8,192               | Tool outputs when only cache proxy alive                             |
| TERMINAL_THRESHOLD    | 2,048               | Terminal output                                                      |
| INLINE_THRESHOLD      | 4,096               | Inline fallback (bumped to 1MB if HEADROOM_SSE_BUFFER_MAX_BYTES set) |
| AUTO_EXPAND_LIMIT     | 51,200              | Max size for auto-expanding tool CCR markers                         |
| MAX_REQUEST_BODY_SIZE | 104,857,600 (100MB) | Skip compression above this                                          |

### Headroom Budget Multiplier

| Budget | Multiplier | Effect                 |
| ------ | ---------- | ---------------------- |
| < 25   | 0.25       | Aggressive compression |
| < 50   | 0.50       | Moderate compression   |
| < 75   | 0.75       | Light compression      |
| ≥ 75   | 1.00       | No adjustment          |
