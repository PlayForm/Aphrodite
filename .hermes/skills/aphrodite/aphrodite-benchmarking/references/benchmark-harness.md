# Aphrodite Benchmark Harness - worked instances, probes, matrices

Supporting probes for `aphrodite-benchmarking`. Each section is a worked
instance: the exact command or fixture, the expected observation, and the
claim it establishes. The rules and boundary contracts live in SKILL.md; this
file holds the probes that exercise them.

## Runtime identity - historical snapshot, not a standing truth

The version numbers below were read at one point in time. They are finished
observations, not standing truths. Re-read them with `aphrodite --version`
and record `binary_version` from that probe, never from this file or any doc
number.

Historical values: binary 1.6.2 (`crates/aphrodite` + `crates/aphrodite-hermes`),
plugin 2.2.2, `BINARY_VERSION` 1.6.2.

Runtime layout:

- `~/.hermes/plugins/aphrodite` holds only the loader (`plugin.yaml` +
  `__init__.py`: 5 hooks + 13 CCR tools).
- Every runtime artifact (binaries, dylib, `aphrodite.toml`, `ccr.db`, logs)
  lives under `~/.hermes/aphrodite/`.

Release/CI context (historical): Build.yml builds a 4-target matrix producing
12 assets; Publish.yml publishes headroom-core → aphrodite → aphrodite-hermes
under the `Aphrodite/v*` tag scheme.

## Quick smoke test

```python
aphrodite_test(mode="full")
```

Expected: `{"mode":"full","status":"ok","passed":3,"total":3,"checks":[...],"proxies":{...}}` -
3 round-trip checks (source_code, build, json_array samples). `mode` accepts
only `quick` (1 sample) or `full` (3 samples) - there is no `matrix`/`pipeline`
mode. `status` is `"ok"` only when every check round-trips.

## Compression type matrix

Benchmark each content type independently. Verify the type label is correctly
carried in the response.

| Type           | Example Content                                 | Center Example             |
| -------------- | ----------------------------------------------- | -------------------------- |
| `code`         | Rust/Python functions, structs, impl blocks     | `code_rust`, `code_python` |
| `log`          | Timestamped log lines, multi-line log output    | `debug`, `verbose`         |
| `diff`         | Unified diff patches with context lines         | `compact`                  |
| `error`        | Stack traces, error messages with tracebacks    | `debug`                    |
| `json`         | Structured data objects, config payloads        | -                          |
| `build_output` | Cargo/rustc compiler output, test runner output | -                          |
| `text`         | Prose, paragraphs, documentation text           | -                          |

## Cache hit verification probe

1. Compress the same content twice.
2. Compare `hash` across the two calls.
3. Same content producing the same hash is a deterministic key
   (content-addressed dedup).

```python
r1 = aphrodite_compress(content="fn test() {}", type="code")
r2 = aphrodite_compress(content="fn test() {}", type="code")
# r2 has the same hash as r1
```

`aphrodite_compress` returns
`{"hash":..., "type":..., "size":..., "preview":..., "marker":...}` - there is
no `source`/`compression_ratio`/`note`/cache-hit flag; dedup is observed by
comparing `hash` across calls. The hash is content-based, not center-based:
same content with different centers still produces the same hash (and the
center still travels with the marker on the first compression). An identical
hash across calls proves key determinism; it does not by itself prove a
shared store - see the cache contract table in SKILL.md.

## Center feature probe

```python
aphrodite_compress(
    content="fn center_test() -> bool { true }",
    type="code",
    _ccr_center="code_rust"  # or debug, compact, verbose
)
```

The center string appears in the marker's structure line and survives
retrievals. It does not affect dedup.

## Cross-worker cache behavior - worked instance

Inline CCR storage is session-scoped: content compressed in one worker
session is not visible to another. Cross-worker compressions of the same
content produce fresh `stored` entries, not `cache_hit`. Hashes are
deterministic across workers (same content always produces the same hash).
The token proxy cache (HTTP-level) IS shared across sessions - that is the
cross-session cache layer that saves token budget. Record observations for
the two layers separately; never merge them into one "cache" number.

## Terminal output threshold probe

Large terminal output above the terminal threshold is auto-compressed into CCR
markers: `<<<CCR:hash|terminal|size>>>` followed by a short preview. The
threshold is a config property (`[compression] terminal_threshold`, env
override `APHRODITE_TERMINAL_THRESHOLD`) - read the active value at probe
time and print it before asserting anything. Never put a threshold literal
in an expected outcome; the shipped default snapshot (~512 bytes) is not a
constant. The T-1/T/T+1 transition procedure is owned by
`aphrodite-auto-expand-testing`; use it to verify threshold edges.

```bash
python3 -c "for i in range(200): print(f'// Section {i}: ' + 'x' * 80)"
```

Expected: output starts with `<<<CCR:hash|terminal|N>>>` and shows preview
text, where `N` is compared against the live-read threshold.

## Proxy metrics - worked reference

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

Observations: the token cache grows with each unique compression; hits
increase on repeated content. The engine fires only when total context
exceeds `engine_threshold_pct` (live TOML: 100 = effectively disabled; source
default 45%) - read both live, never from memory. `aphrodite_stats` is the
accurate source for inline counts; `aphrodite_catalog` reports 0 when the
inline store has entries.

## Scenario-scoped comparison - historical snapshot, not a guarantee

The table below is a historical snapshot captured during an earlier,
uncontrolled run. It is scenario-specific measurement, not a product
guarantee. Regenerate it through a controlled experiment with a full
experiment record before citing any of its numbers as current; until then it
may only be referenced as historical context.

| Mode               | 400K session | Savings  | Reliable |
| ------------------ | ------------ | -------- | -------- |
| Stock Hermes       | 400,000 tok  | baseline | ✓        |
| Stock + HR cache   | 400,000 tok  | 0%       | ✓        |
| Stock + HR token   | ~480,000 tok | −20%     | ✗        |
| Aphrodite cache    | 400,000 tok  | 0%       | ✓        |
| Aphrodite token    | 80,000 tok   | +80%     | ✓        |
| Aphrodite full max | 63,200 tok   | +84%     | ✓        |

Headroom token mode is net-negative (TTL expiry destroys content, forces
re-runs - and per the boundaries in SKILL.md, TTL-expired content is not
savings). Aphrodite token adds type+size metadata (agent skips ~80%); full
max gives structured previews (agent skips ~95%).

## Deterministic preview fixtures

Preview quality is checked with fixtures, not subjective inspection - a
misleading preview is a functional failure, not a cosmetic issue. Maintain
the fixture suite under a recorded `fixtures_version`; every run names the
version in its experiment record. Fixtures:

- Empty text
- One-line text
- Multiline text
- Unicode text near byte boundaries
- Large JSON object
- JSON array
- Source code
- Binary-like/base64 content
- Payload with an existing CCR-looking substring
- Payload just below and just above each compression threshold
- Payload whose retrieval result is itself larger than the threshold

For each fixture, assert:

- Marker grammar is valid when compressed.
- Hash and stated size are internally consistent.
- Preview does not claim a false line count or byte size.
- Retrieval returns the original normalized content.
- Re-running the transform does not produce nested markers.
- A malformed marker never resolves arbitrary content.
