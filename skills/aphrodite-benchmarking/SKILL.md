---
name: aphrodite-benchmarking
description: "Use when benchmarking the aphrodite compression proxy. Smoke test, type coverage, cache hits, cross-worker behavior, terminal threshold."
version: 1.3.0
platforms: [macos]
tags: [aphrodite, ccr, benchmarking, proxy]
---

# Aphrodite Proxy Benchmarking

Protocol for benchmarking the aphrodite compression proxy: smoke test, content
type coverage, cache hit rates, cross-worker behavior, terminal output
compression, and stats metrics.

## The CCR Marker IS the Proof

When you compress content, the returned marker `<<<CCR:hash|type|size>>>` IS the
verification - you do NOT need to `aphrodite_retrieve` the content back to
confirm it was stored. The hash, type, and size are sufficient evidence.
Retrieve only when you need the actual content for processing, never for
verification - this saves retrieval calls, token cache entries, and context
space.

## Quick Smoke Test

```python
aphrodite_test(mode="full")
```

Expected: `{"mode":"full","status":"ok","passed":3,"total":3,"checks":[...],"proxies":{...}}` -
3 round-trip checks (source_code, build, json_array samples). `mode` accepts
only `quick` (1 sample) or `full` (3 samples) - there is no `matrix`/`pipeline`
mode. `status` is `"ok"` only when every check round-trips.

## Compression Type Matrix

Benchmark each content type independently - no need to retrieve results:

| Type           | Example Content                                 | Center Example             |
| -------------- | ----------------------------------------------- | -------------------------- |
| `code`         | Rust/Python functions, structs, impl blocks     | `code_rust`, `code_python` |
| `log`          | Timestamped log lines, multi-line log output    | `debug`, `verbose`         |
| `diff`         | Unified diff patches with context lines         | `compact`                  |
| `error`        | Stack traces, error messages with tracebacks    | `debug`                    |
| `json`         | Structured data objects, config payloads        | -                          |
| `build_output` | Cargo/rustc compiler output, test runner output | -                          |
| `text`         | Prose, paragraphs, documentation text           | -                          |

Each call returns an instant hash. Verify types are correctly labelled in the
response.

## Cache Hit Verification

Same content → same hash → cache hit (content-addressed dedup):

```python
r1 = aphrodite_compress(content="fn test() {}", type="code")
r2 = aphrodite_compress(content="fn test() {}", type="code")
# r2 has the same hash as r1
```

`aphrodite_compress` returns `{"hash":..., "type":..., "size":..., "preview":..., "marker":...}` -
there is no `source`/`compression_ratio`/`note`/cache-hit flag; dedup is
observed by comparing `hash` across calls. The hash is content-based, not
center-based: same content with different centers still produces a cache hit
(and the center still travels with the marker on the first compression).

## Center Feature Test

```python
aphrodite_compress(
    content="fn center_test() -> bool { true }",
    type="code",
    _ccr_center="code_rust"  # or debug, compact, verbose
)
```

The center string appears in the marker's structure line and survives
retrievals. It does not affect dedup.

## Cross-Worker Cache Behavior

Inline CCR storage is **session-scoped**: content compressed in one worker
session is not visible to another - cross-worker compressions of the same
content produce fresh `stored` entries, not `cache_hit`. Hashes are
deterministic across workers (same content always produces the same hash). The
token proxy cache (HTTP-level) IS shared across sessions - that is the
cross-session cache layer that saves token budget.

## Terminal Output Compression

Large terminal output (>1KB) is auto-compressed into CCR markers:
`<<<CCR:hash|terminal|size>>>` followed by a short preview. To verify the
threshold:

```bash
python3 -c "for i in range(200): print(f'// Section {i}: ' + 'x' * 80)"
```

Expected: output starts with `<<<CCR:hash|terminal|N>>>` and shows preview text.

## Proxy Metrics Tracking

Run `aphrodite_stats` after each benchmark phase:

| Metric               | What It Measures                            |
| -------------------- | ------------------------------------------- |
| `token.created`      | Token cache entries created                 |
| `token.hits`         | Token cache hit count                       |
| `token.tokens_saved` | Total tokens saved by proxy                 |
| `cache.created`      | Content cache entries created               |
| `cache.hits`         | Content cache hit count                     |
| `inline`             | Inline CCR entries (session-scoped)         |
| `engine.compression` | Engine compressions (triggers at threshold) |
| `engine.protect`     | Protect first/last count                    |

The token cache grows with each unique compression; hits increase on repeated
content. The engine fires only when total context exceeds the configured
threshold (default 450K tokens).

## All-Modes Comparison

Full comparative benchmark: Stock Hermes → Stock + Headroom → Aphrodite.

| Mode               | 400K session | Savings  | Reliable |
| ------------------ | ------------ | -------- | -------- |
| Stock Hermes       | 400,000 tok  | baseline | ✓        |
| Stock + HR cache   | 400,000 tok  | 0%       | ✓        |
| Stock + HR token   | ~480,000 tok | −20%     | ✗        |
| Aphrodite cache    | 400,000 tok  | 0%       | ✓        |
| Aphrodite token    | 80,000 tok   | +80%     | ✓        |
| Aphrodite full max | 63,200 tok   | +84%     | ✓        |

Headroom token mode is net-negative (TTL expiry destroys content, forces
re-runs). Aphrodite token adds type+size metadata (agent skips ~80%); full max
gives structured previews (agent skips ~95%).

## Pitfalls

- never trust `aphrodite_catalog` counts for inline entries - it reports 0 when
  the inline store has entries; use `aphrodite_stats` for accurate inline
  counts
- never retrieve to verify compression - the marker IS the proof; retrieval
  wastes tokens and context
- never expect cross-worker inline cache hits - inline storage is session-scoped
  by design; only the token proxy cache is cross-session
- never cite Headroom token-mode "99% savings" - TTL expiry destroys content,
  making the savings fake; only aphrodite has session-lifetime storage with no
  expiry
- never use `body_bytes` for compression rate - it is misleading; use
  `tokens_saved`, `requests.compressed/requests.total`, and
  `compression_ratio_ema`