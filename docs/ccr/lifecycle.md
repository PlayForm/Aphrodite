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
      i.   compute_key(content) → hash (BLAKE3, 40 hex chars)
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
1. Validate hash parameter (required); normalize it (strip a trailing
   `|type|size` marker-body suffix an LLM might echo back, trim whitespace -
   marker::normalize_hash)
2. Check inline_ccr (LruCache):
   a. Hit → return content immediately
   b. Miss → fall through to CCR backend
3. Check CCR backend (SQLite or in-memory):
   a. ccr.get(hash) → hit or miss
4. If miss: 404 NOT_FOUND
5. Apply optional query filter (case-insensitive line grep, truncated to
   512 chars, char-safe)
6. Apply optional pagination (offset + limit, limit clamped to 10,000 lines)
7. Return {found: true/false, content: "…", source: "ccr"/"none"}
```

Earlier drafts of this doc (and of `retrieve.rs` itself) described a step
that checked returned content for zstd magic bytes (`0x28 0xB5 0x2F 0xFD`)
and decompressed it. That branch was dead code: `CcrStore::get` returns a
`String`, and a `String` is guaranteed valid UTF-8 - it can never legally
contain those non-UTF-8 magic bytes in the first place, since nothing in
this codebase's `CcrStore` implementations ever zstd-compresses content
before storing it. The branch has been removed; see
`.plans/05-compression-pipeline.md` T12.

### Hermes Tool Retrieve (`aphrodite_retrieve`)

The Hermes MCP tool's retrieve handler (`aphrodite-hermes/src/tools.rs`) is a
separate code path from the HTTP `/retrieve` endpoint above - it resolves
against the session's in-process inline store via `resolve::expand`, not the
CCR backend. It additionally supports:

- Recursive resolution up to `RECURSIVE_DEPTH` (currently **5**) levels deep
  for nested CCR markers (`resolve::resolve_recursive`) - an earlier version
  of this doc said "3 levels"; the code has always used 5, this doc was
  wrong. At the depth limit, the un-further-expanded raw content is returned
  rather than an `[CCR_UNRESOLVED:...]` placeholder, so hitting the limit
  never misreports content that genuinely exists in the store as missing.
- File path reads (workspace-bounded, 10MB cap, `read_path_guarded`)
- Inline store fallback via `resolve_one`
- Never writes the expanded result back over the original hash's store entry
  (a prior implementation did this and both destroyed literal
  marker-shaped text the original content merely contained, and broke the
  content-address invariant - see `.plans/05-compression-pipeline.md` F1)

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

| Type                        | Multiplier            | Effective (Token, 1KB base) | Effective (Cache, 8KB base) |
| --------------------------- | ---------------------- | --------------------------- | --------------------------- |
| error                       | ×8                     | 8,192                       | 65,536                      |
| code_rust/python/go/js/code | ×4 (config, default)   | 4,096                       | 32,768                      |
| diff, git, text             | ×2                     | 2,048                       | 16,384                      |
| tool_output, json           | ×1                     | 1,024                       | 8,192                       |
| linter, build_output, log   | ×1 (BASE, not halved)  | 1,024                       | 8,192                       |

**Correction:** `linter`, `build_output`, and `log` are pinned at the BASE
threshold, not halved. `proxy.rs::threshold_for` returns `base` for these
three types immediately - before the auto-tune multiplier or the rest of
this table is even consulted - because "coding sessions need build output
visible" (the code's own comment). An earlier draft of this table showed a
÷2 discount (effective 512B/4,096B); the real code has never halved these.

### Auto-Tune

Based on `compression_ratio_ema` (×100):

| EMA Ratio   | Tune Factor | Effect                                          |
| ----------- | ----------- | ----------------------------------------------- |
| > 20.0      | 2.0         | Raise thresholds (compress less, preserve more) |
| 3.0 .. 20.0 | 1.0         | Default                                         |
| < 3.0       | 0.5         | Lower thresholds (compress more aggressively)   |
| 0.0         | 1.0         | No history - default                            |

Note: `linter`, `build_output`, `log` types are excluded from auto-tune -
`threshold_for` returns their (unhalved) base threshold before the auto-tune
multiplier is applied at all, not "always base/2" as an earlier draft of
this doc claimed.

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

`budget_mult` is a smooth linear function of the `x-headroom-budget` request
header, NOT the discrete three-step table an earlier draft of this doc
claimed:

```
budget_mult = clamp(0.50 + (headroom_budget% / 100) * 0.50, 0.50, 1.0)
```

| Budget (fill %) | Multiplier | Effect                              |
| ---------------- | ---------- | ------------------------------------ |
| 0%                | 0.50       | Most aggressive compression allowed  |
| 50%               | 0.75       | Moderate compression                 |
| 100% (or no header/unparseable) | 1.00 | No reduction (default when the header is absent) |

The multiplier never drops below 0.50× regardless of how low the budget
signal is - "semantics and tool chains are worth the tokens" per the code's
own comment - and the default with no budget header supplied is 1.0
(unmodified threshold), not the most aggressive setting.
