---
name: aphrodite-benchmarking
description: "Comprehensive benchmark protocol for aphrodite proxy - compression
    throughput, cache hit rates, cross-worker behavior, terminal threshold
    verification, and all-type coverage. Current as of v0.8.43."
version: 1.2.0
platforms: [macos]
related_skills:
    [aphrodite-context-efficiency, aphrodite-center-testing, aphrodite-auto-expand-testing]
---

# Aphrodite Proxy Benchmarking

Protocol for running comprehensive benchmarks of the aphrodite compression
proxy. Covers smoke tests, all 28 content types, center feature, cache hit
rates, cross-worker behavior, and terminal output compression.

## The CCR Marker IS the Proof

**Core principle**: When you compress content, the returned marker
`<<<CCR:hash|type|size>>>` IS the verification. You do NOT need to
`aphrodite_retrieve` the content back to confirm it was stored. The hash, type,
and size are sufficient evidence. Retrieve only when you need the actual content
for processing, not for verification.

This saves retrieval calls, token cache entries, and context space. The marker
structure encodes everything needed to prove compression worked.

## Quick Smoke Test

```python
aphrodite_test(mode="full")
```

Expected (current Rust `aphrodite_test` tool, `crates/aphrodite-hermes/src/tools.rs`):
`{"mode":"full","status":"ok","passed":3,"total":3,"checks":[...],"proxies":{...}}` -
3 round-trip checks (source_code/build/json_array samples), not 13. There is
no `matrix`/`pipeline` mode; any `mode` value other than `"quick"` runs the
same 3-check `full` set.

## Compression Type Matrix

Benchmark all 6 compression types. Issue them independently - no need to
retrieve results.

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

Same content → same hash → cache hit:

```python
# First compression
r1 = aphrodite_compress(content="fn test() {}", type="code")
# Repeat exactly
r2 = aphrodite_compress(content="fn test() {}", type="code")
# r2 will have the same hash as r1 (content-addressed)
```

Expected shape (current Rust `aphrodite_compress` tool): `{"hash":...,
"type":..., "size":..., "preview":..., "marker":...}`. There is no
`source`/`compression_ratio`/`note` field on this tool's response - dedup is
observed by comparing `hash` across calls, not by a cache-hit flag.

Cache hits also work when the center parameter varies - the hash is
content-based, not center-based. Same content with different centers still
produces a cache hit (and the center still travels with the marker on the first
compression).

## Center Feature Test

Compress with `_ccr_center` parameter:

```python
aphrodite_compress(
    content="fn center_test() -> bool { true }",
    type="code",
    _ccr_center="code_rust"  # or debug, compact, verbose
)
```

The center string appears in the marker's structure line. It survives
retrievals. The hash is content-based, so the center doesn't affect dedup.

## Cross-Worker Cache Behavior

Important architectural property: inline CCR storage is **session-scoped**.
Content compressed in one worker session is not visible to another worker
session. Cross-worker compressions of the same content produce fresh `stored`
entries, not `cache_hit`.

The token proxy cache (HTTP-level) IS shared across sessions - this is the
cross-session cache layer that saves token budget.

To test:

1. Worker A compresses content → gets hash `abc123`, `stored`
2. Worker B compresses same content → gets same hash `abc123`, but `stored` not
   `cache_hit`
3. Hashes are deterministic across workers - same content always produces same
   hash

## Terminal Output Compression

Large terminal output (>1KB) is auto-compressed into CCR markers. The terminal
output appears as `<<<CCR:hash|terminal|size>>>` followed by a short preview.

To verify threshold works:

```bash
# Produce >5KB of output
python3 -c "for i in range(200): print(f'// Section {i}: ' + 'x' * 80)"
```

Expected: output starts with `<<<CCR:hash|terminal|N>>>` and shows preview text.
The actual content is retrievable via the hash.

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

The token cache grows with each unique compression. Hits increase on repeated
content. The engine fires only when total context exceeds the configured
threshold (default 450K tokens).

## References

- `references/benchmark-session-results.md` - Concrete examples from the
  v0.5.124 production benchmark

## All-Modes Comparison (v0.8.43)

Full comparative benchmark: Stock Hermes → Stock + Headroom → Aphrodite.

| Mode               | 400K session | Savings  | Reliable |
| ------------------ | ------------ | -------- | -------- |
| Stock Hermes       | 400,000 tok  | baseline | ✓        |
| Stock + HR cache   | 400,000 tok  | 0%       | ✓        |
| Stock + HR token   | ~480,000 tok | −20%     | ✗        |
| Aphrodite cache    | 400,000 tok  | 0%       | ✓        |
| Aphrodite token    | 80,000 tok   | +80%     | ✓        |
| Aphrodite full max | 63,200 tok   | +84%     | ✓        |

Headroom token mode is net-negative: TTL expiry destroys content, forces re-runs.
Aphrodite token gives type+size metadata (agent skips ~80%).
Aphrodite full max gives structured previews (agent skips ~95%).
Benchmark CCR: `4b47d220c805679cc964f66f264896dbba76f51d`

## Pitfalls

- **aphrodite_catalog reports 0** when inline store has entries. Use
  `aphrodite_stats` for accurate inline counts.
- **Don't retrieve to verify** - the CCR marker IS the proof. Retrieval wastes
  tokens and context.
- **Cross-worker cache** is not shared for inline storage - this is by design.
  Only the token proxy cache is cross-session.
- **Stock Headroom token mode is broken** - never cite its "99% savings" as
  real. TTL expiry destroys content, making savings fake. Only Aphrodite has
  session-lifetime storage with no expiry.
- **body_bytes is misleading** for compression rate. Use `tokens_saved`,
  `requests.compressed/requests.total`, and `compression_ratio_ema` instead.
