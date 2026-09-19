# CCR Lifecycle

CCR (Compress-Cache-Retrieve) is the lossless end-to-end compression path for
LLM proxy traffic: content is hashed, stored, and replaced with a compact
marker in the response, and the original is retrieved by hash on demand. This
document walks the six phases of that lifecycle, from compression to expiry.

## Phase 1: Compress

The chat-completions path rewrites each `choices[].message.content` that is
large enough to be worth compressing:

1. Detect the content type (`proxy::proxy_detect_content_type`).
2. Compute the threshold for that type (base threshold x type multiplier x
   auto-tune factor x budget multiplier - see Thresholds below).
3. If `content.len() > threshold`: compute the BLAKE3 hash, check the CCR
   backend (hit or miss), store on miss, and replace the content with a
   rendered marker. A content block is only replaced when the store actually
   holds the hash - a failed store leaves the content uncompressed rather
   than emitting an unresolvable marker.
4. If the content is above the inline threshold but below the compression
   threshold, store it in the inline cache instead (see
   [Inline](backends/inline.md)).
5. Update the savings counters and the compression-ratio EMA used by
   auto-tune.

`tool_calls[].function.arguments` is never a compression target: it is
client-executable JSON, not model-facing prose, and compressing it would
produce a tool call the client cannot parse. Streaming (`text/event-stream`)
responses bypass this phase entirely.

The direct `POST /ccr/create` endpoint stores content without rendering a
marker in any response, but feeds the same counters and the same
compression-ratio EMA (using a byte-entropy estimate - unique 3-byte
trigrams in the first 4096 bytes - as the effective compressed size).

## Phase 2: Cache

Before storing, the hash is checked against the backend (`ccr_hits` /
`ccr_misses` counters) and against the inline cache (`inline_ccr_hits` /
`inline_ccr_misses`).

A separate LLM response cache avoids repeat upstream round-trips for
identical requests:

| Property        | Detail                                                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Key             | FNV-1a 64-bit over api key, model, messages, tools, tool_choice, temperature, top_p, n, and response_format, in fixed canonical order |
| Capacity        | LRU, 128 entries, 1 MiB per body                                                                                                      |
| TTL             | `ccr_ttl_seconds` (default 3600, env `APHRODITE_CCR_TTL`)                                                                             |
| Scope           | Streaming requests are never cached; successful responses only                                                                        |
| Response header | `X-Aphrodite-Cache: HIT` or `MISS`                                                                                                    |

## Phase 3: Store

Three storage tiers, plus the session-side inline store used by the Hermes
plugin integration:

| Tier             | Threshold                                                 | Backend                          | Capacity             | Expiry                                             |
| ---------------- | --------------------------------------------------------- | -------------------------------- | -------------------- | -------------------------------------------------- |
| Inline (proxy)   | inline threshold (256 B default) to compression threshold | `lru::LruCache`                  | 1,024 entries        | LRU eviction only                                  |
| Inline (session) | any prefetched / compressed blob                          | HashMap + LRU order, byte budget | 500 entries, 256 MiB | LRU + byte-budget eviction                         |
| Cache mode       | > 8 KiB                                                   | `InMemoryCcrStore` (DashMap)     | 10,000 entries       | Sliding idle TTL (default 3600 s), 8x max lifetime |
| Token mode       | > 1 KiB                                                   | `SqliteCcrStore` (SQLite)        | Unlimited (disk)     | Sliding idle TTL (default 3600 s), 8x max lifetime |

Backend details live in [backends](backends/). Thresholds are configurable:
env var > TOML `[compression]` > compiled-in default for cache, token,
inline, and code multiplier.

## Phase 4: Return Marker

Cache mode replaces content with a plain 512-character excerpt plus the
marker line; token mode renders a type-aware summary with metadata. Both use
the same `format_ccr_output` template (see [Marker Format](marker-format.md)).
Compressed responses carry:

- `X-Aphrodite-Compressed: true`
- `X-Aphrodite-Cache: HIT` or `MISS`
- `X-Aphrodite-Fill-Pct: XX.X` (headroom fill derived from the compression
  ratio EMA)

## Phase 5: Retrieve

The HTTP `/retrieve` endpoint resolves a hash to its original content:

1. Normalize the hash argument (strip a trailing `|type|size` suffix, trim).
2. Check the inline cache first; on a hit, return immediately.
3. Fall back to the CCR backend; on a miss, return 404 `NOT_FOUND`.
4. Apply an optional `query` filter (case-insensitive line match, query
   capped at 512 chars) and optional pagination (`offset` + `limit`; an
   explicit `limit` is clamped to 10,000 lines, `limit: 0` returns the full
   document).
5. Windowed results are prefixed with a `[lines a-b/total]` header and
   flagged via the `truncated` field.

Full-document retrieval is byte-identical - including a trailing newline -
so the returned body hashes back to the marker's own hash.

The Hermes `aphrodite_retrieve` tool is a separate path that resolves against
the session's in-process inline store:

- Recursive expansion of nested markers up to 5 levels deep; at the depth
  limit the raw content is returned rather than a placeholder.
- Workspace-bounded file-path reads (10 MiB cap).
- Inline-store fallback; never writes expanded content back over the stored
  entry, preserving the content-address invariant.

## Phase 6: Expire

| Backend          | Expiry behavior                                                                                                                                          |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SQLite           | Lazy purge on `get` and `put`, debounced to once per 60 seconds; rows expire when idle past TTL or older than the 8x max lifetime. No background thread. |
| In-Memory        | Lazy TTL check on `get`; expired entries evicted via an atomic check-and-remove; queue compaction when the eviction order grows past twice capacity.     |
| Inline (proxy)   | LRU eviction at capacity (1,024). No TTL.                                                                                                                |
| Inline (session) | LRU eviction at 500 entries and/or over the 256 MiB byte budget. No TTL.                                                                                 |

## Thresholds

### Base Thresholds

| Constant       | Default | Mode  |
| -------------- | ------- | ----- |
| Cache compress | 8,192   | Cache |
| Token compress | 1,024   | Token |
| Inline         | 256     | All   |

Each is resolved as env var > TOML `[compression]` value > compiled-in
default. The code multiplier defaults to 3.0.

### Per-Type Multipliers

| Type                                                 | Multiplier                      |
| ---------------------------------------------------- | ------------------------------- |
| `error`                                              | x8                              |
| `code_rust`/`code_python`/`code_go`/`code_js`/`code` | x code multiplier (default 3.0) |
| `diff`, `git`, `text`                                | x2                              |
| `tool_output`, `json`                                | x1 (base)                       |
| `linter`, `build_output`, `log`                      | x1 (base, never discounted)     |

`linter`, `build_output`, and `log` are pinned at the base threshold before
any multiplier is applied, so build output stays visible in coding sessions.

### Auto-Tune

The compression-ratio EMA (x100) adjusts every threshold:

| EMA ratio   | Tune factor | Effect                           |
| ----------- | ----------- | -------------------------------- |
| > 20.0      | 2.0         | Raise thresholds (compress less) |
| 3.0 .. 20.0 | 1.0         | Default                          |
| < 3.0       | 0.5         | Lower thresholds (compress more) |
| 0.0         | 1.0         | No history - default             |

### Budget Multiplier

An `x-headroom-budget` request header (fill percentage 0-100) scales the
effective threshold on a smooth linear curve, never below 0.5x:

```
budget_mult = clamp(0.50 + (budget% / 100) * 0.50, 0.50, 1.0)
```

| Budget (fill %) | Multiplier |
| --------------- | ---------- |
| 0%              | 0.50       |
| 50%             | 0.75       |
| 100% or absent  | 1.00       |

## Session Catalog Emission

Per-turn catalog summaries are emitted delta-only: the renderer remembers
`last_emitted_marker_count` and `last_emitted_file_count` and reports
`+N new compressions this turn` / `+N new files` only when new items
arrived, with a stable "no change" line otherwise. Counters reset on session
start.
