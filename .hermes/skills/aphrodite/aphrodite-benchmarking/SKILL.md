---
name: aphrodite-benchmarking
description: "Use when benchmarking the aphrodite compression proxy. Smoke test, type coverage, cache hits, cross-worker behavior, terminal threshold, reproducible experiment records."
version: 2.2.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-benchmarking
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, ccr, benchmarking, proxy, benchmark, measure, smoke-test]
        related_skills: [aphrodite-boundaries, aphrodite-orientation, aphrodite-auto-expand-testing]
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

This skill is `mutation_level: read-only`. Benchmarking never mutates the
repository, Git state, or configuration; it exercises the runtime and records
observations. Run the `aphrodite-orientation` orientation gate before any
session; keep scratch artifacts in `.hermes/tmp/`.

A benchmark result without an experiment record is a snapshot. A snapshot is
not a benchmark, and a snapshot is never citable as a savings claim.

## Stop if / Recovery

- Stop if the proxy is not healthy: `proxy_state` must be `healthy` in the
  experiment record. Recovery: confirm `APHRODITE_API_KEY` is actually
  exported and not commented out (a missing or commented-out
  `APHRODITE_API_KEY` fails the proxy loudly at startup), then re-run the
  orientation gate.
- Stop if the experiment record is missing any required field. Recovery:
  regenerate the record completely before reporting any percentage.

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

## Runtime identity - record from live probes, never from this doc

The runtime layout is stable: `~/.hermes/plugins/aphrodite` holds only the
loader (`plugin.yaml` + `__init__.py`: 5 hooks + 13 CCR tools); every runtime
artifact (binaries, dylib, `aphrodite.toml`, `ccr.db`, logs) lives under
`~/.hermes/aphrodite/`. Read `binary_version` with `aphrodite --version` at
benchmark time; a version written in this file is not evidence. Fill
`binary_version`, `plugin_commit`, and `hermes_version` in the experiment
record from live probes, never from memory. Historical version observations,
the release/CI layout (Build.yml 4-target matrix producing 12 assets;
Publish.yml publishing headroom-core → aphrodite → aphrodite-hermes under the
`Aphrodite/v*` tag scheme), and every probe script live in
`references/benchmark-harness.md`.

Before benchmarking, confirm the proxy is healthy: a missing or commented-out
`APHRODITE_API_KEY` fails the proxy loudly at startup. Verify the env var is
actually exported, then set `proxy_state: healthy` in the experiment record.

## Verification levels: normal vs resilience (hash and cache boundaries)

A returned CCR marker is proof of **key determinism and marker validity**. It
is not proof of persistence, storage scope, or retrievability. Choose the
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

## Protocol steps

1. Smoke test: run the quick smoke test; the command and its expected shape
   are in `references/benchmark-harness.md`.
2. Type coverage: benchmark each content type independently; verify the type
   label is correctly carried in the response. Matrix in
   `references/benchmark-harness.md`.
3. Cache hit: compress identical content twice and compare `hash`; the probe
   is in `references/benchmark-harness.md`.
4. Center feature: compress with `_ccr_center`; the center string appears in
   the marker's structure line and survives retrievals; it does not affect
   dedup. Probe in `references/benchmark-harness.md`.
5. Cross-worker: run two worker sessions on identical content; observe fresh
   `stored` entries, not `cache_hit`; hashes stay deterministic. Worked
   instance in `references/benchmark-harness.md`.
6. Terminal threshold: read `[compression] terminal_threshold` (env override
   `APHRODITE_TERMINAL_THRESHOLD`) live and print the value, then run the
   probe. Probe in `references/benchmark-harness.md`.
7. Stats: run `aphrodite_stats` after each phase; metric meanings are in
   `references/benchmark-harness.md`.
8. Record: fill the experiment record above and verify every field before
   reporting.

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
- never conclude "no logs = no activity" for dylib-side behavior: tracing has no
  subscriber inside the Hermes host - read `aphrodite_stats`/`aphrodite_rebuild`
  surfaced state, or add a stderr fallback (check tracing::dispatcher::has_been_set())
- when parallel benchmark workers on free-tier providers hit intermittent HTTP
  401/429, treat it as a dispatch-size problem: re-dispatch failed clusters
  smaller instead of aborting the benchmark

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
