---
name: aphrodite-benchmarking
description: "Use when benchmarking the aphrodite compression proxy. Smoke test, type coverage, cache hits, cross-worker behavior, terminal threshold, reproducible experiment records."
version: 2.0.0
platforms: [macos]
tags: [aphrodite, ccr, benchmarking, proxy, experiment-record]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - The benchmark experiment record schema and required report fields
    - Benchmark boundary rules (what may and may not be compared)
    - The hash-and-cache verification contract (normal vs resilience)
    - The deterministic preview fixture suite and its 6 per-fixture assertions
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
supersedes: []
verification:
    source_of_truth:
        - crates/aphrodite/src/proxy.rs (config serialization, thresholds)
        - ~/.hermes/aphrodite/aphrodite.toml (live config values)
        - aphrodite_stats output (runtime counters)
mutation_level: read-only
---

# Aphrodite Benchmarking

Protocol for benchmarking the aphrodite compression proxy: smoke test, content
type coverage, cache hit rates, cross-worker behavior, terminal output
compression, and stats metrics - all under a reproducibility contract.

This skill is `mutation_level: read-only`: benchmarking never mutates the
repository, Git state, or configuration. It exercises the runtime and records
observations. Run the `aphrodite-orientation` orientation gate before any
session; keep scratch artifacts in `.hermes/tmp/`.

A benchmark result without an experiment record is a snapshot, not a
benchmark - and a snapshot is never citable as a savings claim.

## Experiment record - no result is citable without one

Write this record **before** the run begins and attach it to every reported
result, so results stay comparable across versions and cannot be silently
mixed across warm states, threshold settings, or storage scopes.

```yaml
run_id: <timestamp-and-commit>
aphrodite_commit: <sha>
plugin_commit: <sha>
hermes_version: <version>
binary_version: <reported version>
platform: <os-arch>
runtime_mode: source | installed
config_fingerprint: <redacted hash>
engine_enabled: true | false
proxy_state: healthy | degraded | unavailable
fixtures_version: <fixture suite version>
```

`config_fingerprint` is a **redacted** hash of the normalized active config
(e.g. `sha256` of the config with secrets and values that identify the user
removed) - never raw config values, never secrets. Record `binary_version`
from the runtime probe (`aphrodite --version`), never from a hardcoded doc
number.

Every run must then report:

- Number of source bytes and tokens per fixture
- Compression mode/type chosen
- Marker size and preview length
- Compression latency percentiles
- Retrieval latency percentiles when retrieval is required
- Inline-store, shared-cache, and token-cache observations **separately**
- Error and unresolved-marker counts
- Whether the run used a warm or cold process/cache

A run missing any of these fields is incomplete; do not publish its
percentages.

## Verification levels: normal vs resilience (hash and cache boundaries)

A returned CCR marker is proof of **key determinism and marker validity**, not
proof of persistence, storage scope, or retrievability. Choose the
verification level by what the test must establish:

| Test purpose                                            | Verification required                                                                    |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Throughput / type testing                               | Marker validity plus stats may suffice                                                   |
| Persistence, eviction, race, resolver, or release tests | Retrieve the payload and compare it with the source - marker validity is not enough      |
| Cache-scope questions                                   | Distinguish deterministic key calculation from storage scope; observe both, never assume |

Boundary rules:

- **Never infer a shared inline store from identical hashes.** Two workers
  compressing the same content produce the same hash (deterministic key
  calculation) while each writes a fresh session-scoped inline entry. Identical
  hashes prove key determinism only.
- Inline CCR storage is **session-scoped**; the token proxy cache (HTTP-level)
  is **cross-session**. Document which layer an observation belongs to.

### Cache contract table

| Property            | Required behavior                                                                |
| ------------------- | -------------------------------------------------------------------------------- |
| Key determinism     | Same normalized bytes produce same key                                           |
| Content type impact | Must be explicitly defined; never assumed                                        |
| Storage scope       | Session, worker, process, or shared cache - observed and documented              |
| Cache hit proof     | Compare documented metrics or behavior, not guessed response fields              |
| Resolver fallback   | Local inline store before remote/proxy fallback, if the architecture supports it |
| Eviction semantics  | Explicit TTL/capacity policy with a user-visible degraded result                 |
| Security            | Keys reveal no raw secret content; logs avoid payload leakage                    |

## Quick Smoke Test

```python
aphrodite_test(mode="full")
```

Expected: `{"mode":"full","status":"ok","passed":3,"total":3,"checks":[...],"proxies":{...}}` -
3 round-trip checks (source_code, build, json_array samples). `mode` accepts
only `quick` (1 sample) or `full` (3 samples) - there is no `matrix`/`pipeline`
mode. `status` is `"ok"` only when every check round-trips.

## Compression Type Matrix

Benchmark each content type independently. For throughput/type testing, marker
validity plus stats is sufficient verification (see verification levels);
verify the type label is correctly carried in the response.

| Type           | Example Content                                 | Center Example             |
| -------------- | ----------------------------------------------- | -------------------------- |
| `code`         | Rust/Python functions, structs, impl blocks     | `code_rust`, `code_python` |
| `log`          | Timestamped log lines, multi-line log output    | `debug`, `verbose`         |
| `diff`         | Unified diff patches with context lines         | `compact`                  |
| `error`        | Stack traces, error messages with tracebacks    | `debug`                    |
| `json`         | Structured data objects, config payloads        | -                          |
| `build_output` | Cargo/rustc compiler output, test runner output | -                          |
| `text`         | Prose, paragraphs, documentation text           | -                          |

## Cache Hit Verification

Same content → same hash → deterministic key (content-addressed dedup):

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
hash across calls proves key determinism; it does **not** by itself prove a
shared store - see the cache contract table.

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
cross-session cache layer that saves token budget. Record observations for the
two layers separately; never merge them into one "cache" number.

## Terminal Output Compression (threshold live-read)

Large terminal output above the terminal threshold is auto-compressed into CCR
markers: `<<<CCR:hash|terminal|size>>>` followed by a short preview. The
threshold is a config property (`[compression] terminal_threshold`, env
override `APHRODITE_TERMINAL_THRESHOLD`) - **read the active value at probe
time and print it before asserting anything**. Never put a threshold literal
in an expected outcome; the shipped default snapshot (~512 bytes) is not a
constant. The T-1/T/T+1 transition procedure is owned by
`aphrodite-auto-expand-testing`; use it to verify threshold edges.

```bash
python3 -c "for i in range(200): print(f'// Section {i}: ' + 'x' * 80)"
```

Expected: output starts with `<<<CCR:hash|terminal|N>>>` and shows preview
text, where `N` is compared against the live-read threshold.

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
content. The engine fires only when total context exceeds
`engine_threshold_pct` (live TOML: 100 = effectively disabled; source default
45%) - read both live, never from memory. `aphrodite_stats` is the accurate
source for inline counts; `aphrodite_catalog` reports 0 when the inline store
has entries.

## Scenario-Scoped Comparison - historical snapshot, not a guarantee

The table below is a **historical snapshot** captured during an earlier,
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
re-runs - and per the boundaries below, TTL-expired content is not savings).
Aphrodite token adds type+size metadata (agent skips ~80%); full max gives
structured previews (agent skips ~95%).

## Benchmarks Need Boundaries

- Never report a percentage without workload, configuration, model/context
  budget, cache state, and sample size.
- Never compare systems with different retrieval requirements as though they
  consumed equal context.
- Never treat TTL expiration or unretrievable content as successful savings.
- Separate token savings from latency, correctness, retrieval frequency, and
  user-visible completeness.
- A benchmark **fails** if any payload cannot be resolved when the test
  requires retrieval, even if the reported token savings are high.

## Deterministic Preview Fixtures

Preview quality is verified with fixtures, not subjective inspection - a
misleading preview is a functional failure, not a cosmetic issue. Maintain the
fixture suite under a recorded `fixtures_version`; every run names the version
in its experiment record. Fixtures:

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

## Pitfalls

- never trust `aphrodite_catalog` counts for inline entries - it reports 0
  when the inline store has entries; use `aphrodite_stats`
- never retrieve to verify compression in throughput/type tests - marker
  validity plus stats suffices; in resilience tests you MUST retrieve and
  compare with the source
- never expect cross-worker inline cache hits - inline storage is
  session-scoped by design; only the token proxy cache is cross-session
- never cite Headroom token-mode "savings" - TTL expiry destroys content, and
  TTL-expired content is not savings under the boundaries rules
- never use `body_bytes` for compression rate - it is misleading; use
  `tokens_saved`, `requests.compressed/requests.total`, and
  `compression_ratio_ema`
- never report a percentage without the full boundary context
  (workload/config/model/cache-state/sample-size)
- never put a threshold literal in an expected outcome - live-read the active
  config and print the value first

## Local claim-to-test matrix

| Claim                                              | Evidence source                                  | Test                                                                      | Pass condition                              | Failure response                                  |
| -------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------- |
| Experiment record is complete                      | This skill                                       | Run a benchmark; check all 10 record fields + 8 report fields are present | Record complete; fingerprint redacted       | Halt reporting; regenerate with a complete record |
| Marker validity suffices for throughput/type tests | `aphrodite_test` / `aphrodite_compress` response | Compress one payload per type; validate marker grammar + stats            | Valid markers; stats move as expected       | Retrieve + compare (resilience path)              |
| Resilience verification catches storage loss       | `aphrodite_retrieve`                             | Store payload, restart process, retrieve                                  | Payload equals normalized source            | Benchmark FAILS; investigate store scope/eviction |
| Identical hashes do not imply shared store         | Cross-worker compress                            | Two sessions compress identical content                                   | Same hash; fresh session-scoped entries     | Update storage-scope documentation                |
| Cache contract holds                               | `aphrodite_stats`                                | Repeated content; TTL probe                                               | Hits increment only on the documented scope | Fix cache-hit proof; stop savings claims          |
| Preview fixtures are truthful                      | Fixture suite                                    | Run the 6 assertions per fixture                                          | All assertions pass                         | Fix preview machinery; stop benchmark             |
| Savings claims are bounded                         | Experiment record                                | Cite any % with workload/config/model/cache-state/sample-size             | All context present                         | Retract the claim; regenerate under the record    |
