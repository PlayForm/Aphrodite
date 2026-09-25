---
name: aphrodite-engine-observability
description: "Use when probing, diagnosing, or verifying Aphrodite engine health. Layered runtime checks, cache behavior contract, log locations, telemetry, and probes."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-engine-observability
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, observability, health, probes, cache, telemetry, diagnose]
        related_skills:
            [
                aphrodite-boundaries,
                aphrodite-orientation,
                aphrodite-compression-safety,
                aphrodite-operations,
            ]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - The 5-layer runtime validation model (Process / Configuration / Local engine / Upstream dependency / User-visible behavior)
    - "Proxy healthy definition: local engine availability AND upstream reachability as SEPARATE probes"
    - Cache behavior contract table and its verification probes
    - Log locations and telemetry/counter inventory
    - Layer-3 rule (local health without upstream) and layer-4 degraded-mode rule
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
    - aphrodite-compression-safety (what the engine may compress; retrieval skip facts)
supersedes: []
verification:
    source_of_truth:
        - crates/aphrodite/src/proxy.rs (/health, /health/upstream, counters, thresholds)
        - crates/aphrodite/src/state.rs (inline_store, LRU caches)
        - plugins/aphrodite/__init__.py (_alive 5s TTL, proxy-stderr.log, logger "aphrodite")
        - references/health-check-pattern.md (historical evidence: _wait_alive, dual proxy, backoff)
        - examples/08_health_upstream.py (upstream probe documentation)
mutation_level: read-only
---

# Aphrodite Engine Observability

Canonical owner of runtime validation for the Aphrodite engine. A banner, a
running process, or a single "healthy" string is **not ground truth**;
validation is layered, and each layer has its own probe, expected result, and
failure response. This skill is read-only - it defines what to observe and
how to interpret it, never how to mutate.

## The 5 runtime layers

Validate bottom-up. A layer's failure blocks the layers above it; never
claim a higher layer while a lower one is unverified.

### Layer 1 - Process

- The expected binary/dylib process is present (`aphrodite` binary, the
  plugin dylib loaded).
- Loaded binary version is recorded (ask `aphrodite --version` /
  `aphrodite_rebuild`/`aphrodite_stats`; never trust a hardcoded doc number).
- The process/session was restarted after a change when needed - a stale
  dylib process can mask source changes (AGENTS.md rule). Reinstalling after a
  rebuild goes through `aphrodite setup` (no mtime hot-reload dir exists in the
  runtime home).
- Log path is known and contains no load failure (see Log locations).
- **The dylib has NO tracing subscriber inside the Hermes host** -
  `tracing::warn!`/`info!` are silent no-ops (the host is a Python process that
  never installs a Rust subscriber, and the engine loads config before
  `main()` could install one). Any diagnostic that must be seen needs an
  explicit stderr fallback (`tracing::dispatcher::has_been_set()` ->
  `eprintln!`) or a surfaced state field (e.g. `config_error` for
  found-but-broken `aphrodite.toml`) readable via `aphrodite_stats`. Verify
  subscriber-less behavior with `cargo run --example` in a plain process, not
  with unit tests - the test harness installs a subscriber and masks the gap.

### Layer 2 - Configuration

- Active configuration file and relevant environment values are identified
  (`~/.hermes/aphrodite/aphrodite.toml`; env vars override TOML).
- API key presence is confirmed without printing the secret (presence +
  redacted fingerprint only; `aphrodite-boundaries` secrets model).
- Exactly one compression owner is active: if Aphrodite is active, Hermes
  built-in compression is disabled (`hermes config set compression.enabled
false`). Two independently mutating compressors make "Compacting context"
  un-attributable.
- Required env passthrough values reach the child process.

### Layer 3 - Local engine

**Rule: the local health endpoint succeeds WITHOUT requiring upstream
access.** Verified: `GET /health` never calls the upstream API - it always
returns HTTP 200 and conveys capability in the JSON body:

```json
{ "status": "healthy", "ccr": true, "mode": "token|cache", "version": "1.6.2", "fill_pct": 0.42 }
```

`status` is always `"healthy"` (liveness); `ccr` is the capability flag. The
historical `status: healthy|degraded` dichotomy in
`references/health-check-pattern.md` is STALE - a live upstream call was
removed from `/health` precisely so it stays local-only.

Layer 3 also verifies: the cache/store accepts and returns a known payload;
a CCR marker parses and resolves locally (`aphrodite_compress` +
`aphrodite_retrieve` round trip with byte comparison).

### Layer 4 - Upstream dependency

- Upstream health is probed SEPARATELY: `GET /health/upstream` (deepseek/
  provider API probe), TTL-cached server-side (F19) so a monitor polling it
  does not hammer the upstream. Documented example:
  `examples/08_health_upstream.py`.
- Python-side `_alive()` caches the result with a 5-second TTL to avoid
  per-turn socket overhead; `_wait_alive()` retries bounded: 10 × 0.3 s.
- **Rule: upstream failure results in an explicitly supported degraded mode,
  not an assumed recovery action.** Supported degraded modes: inline-only
  compression (local store still resolves), hooks-only mode (context engine
  not selected - no partial "context engine active"), raw pass-through with
  degraded status exposed. Never silently retry forever or fall back to an
  undocumented behavior. Bounded retry (3 attempts, exponential backoff
  100·2^n ms) then degrade - verified in
  `references/health-check-pattern.md` and current proxy code.

### Layer 5 - User-visible behavior

- Compress a representative payload; retrieve it; search/catalog it.
- Verify the preview is truthful enough to guide retrieval.
- Verify no retrieval result is immediately re-compressed (owner:
  `aphrodite-compression-safety`).

## Cache behavior contract

| Property            | Required behavior                                                            | Verified current state                                                                                                |
| ------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Key determinism     | Same normalized bytes → same key                                             | BLAKE3 first-40 hex; deterministic (tested)                                                                           |
| Content type impact | Explicitly defined, never assumed                                            | `threshold_for(ct)` + classifier-dependent previews                                                                   |
| Storage scope       | Session, worker, process, or shared - observed and documented                | Inline store = session-scoped Rust state (per-image OnceLock); token store = SQLite `ccr.db`; cache store = in-memory |
| Cache hit proof     | Compare documented metrics, not guessed response fields                      | `ccr_hits`/`ccr_misses`, `cache_hits`/`cache_misses`, `inline_ccr_hits`/`misses` counters                             |
| Resolver fallback   | Local inline store before remote/proxy fallback, if architecture supports it | `resolve_one`: inline store first; `i:` prefix = inline-only                                                          |
| Eviction semantics  | Explicit TTL/capacity policy with user-visible degraded result               | `inline_ccr` LRU 1024; `response_cache` LRU 128, TTL 3600 s; SQLite idle-TTL with max-lifetime multiplier (8×)        |
| Security            | Keys reveal no raw secret content; logs avoid payload leakage                | Content-addressed hashes; secrets model in `aphrodite-boundaries`                                                     |

Beware: identical hashes across workers do NOT prove a shared inline store
(inline is session-scoped); token caching may be cross-session. Verify
storage scope per cache before drawing conclusions.

## Log locations

| What             | Where                                                                                                               | Verified                             |
| ---------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| Plugin logger    | `logging.getLogger("aphrodite")` → Hermes logging                                                                   | `plugins/aphrodite/__init__.py:22`   |
| Proxy stderr     | `~/.hermes/aphrodite/proxy-stderr.log` (runtime home root - the dylib stderr fallback)                              | loader `__init__.py` (log_dir)       |
| SQLite CCR store | `~/.hermes/aphrodite/ccr.db`                                                                                        | AGENTS.md                            |
| Plugin loader    | `~/.hermes/plugins/aphrodite/` holds ONLY `plugin.yaml` + `__init__.py` (hooks-only layout)                         | verified layout                      |
| Runtime home     | `~/.hermes/aphrodite/` (binaries/, aphrodite.toml, BINARY_VERSION pin, ccr.db, directives/, proxy-stderr.log)       | AGENTS.md                            |
| Rebuild evidence | `BINARY_VERSION` pin + `aphrodite_rebuild` dylib version; no mtime hot-reload dir - reinstall via `aphrodite setup` | `~/.hermes/aphrodite/BINARY_VERSION` |

## Telemetry (verified counter inventory)

`aphrodite_stats` and the proxy expose: `requests_total`,
`requests_compressed`, `tokens_saved`, `ccr_hits`, `ccr_misses`,
`ccr_created`, `cache_hits`, `cache_misses`, `inline_ccr_hits`,
`inline_ccr_misses`, `tool_relay_calls`/`success`/`failure`, `notify_success`
`/`failure`, `upstream_errors_4xx`, `upstream_errors_5xx`,
`upstream_timeouts`, `upstream_connect_errors`, `sse_stream_errors`,
`ccr_store_entries`, `ccr_store_bytes`, `compression_ratio_ema`,
`compressions_by_type`, `fill_pct`(source:`AppState`fields in`crates/aphrodite/src/proxy.rs`). A health claim should cite the relevant
counters, not a banner.

## 'Proxy healthy' - two independent probes

| Probe                 | Endpoint / tool        | Answers                                     | Failure response                                        |
| --------------------- | ---------------------- | ------------------------------------------- | ------------------------------------------------------- |
| Local engine          | `GET /health`          | Is the proxy up, CCR enabled, correct mode? | Process/config/layer-1-2 diagnosis                      |
| Upstream reachability | `GET /health/upstream` | Can the provider API be reached?            | Explicit degraded mode (inline-only / raw pass-through) |

They have different endpoints and different remediation paths - never
collapse them into one "proxy healthy" boolean.

## Reference evidence

- `references/health-check-pattern.md` - historical evidence: `_alive()` 5 s
  TTL, `_wait_alive()` bounded retry, dual-proxy resolve, Rust backoff loop.
  Note its stale `status: healthy|degraded` claim (see Layer 3) - the body is
  evidence, the current `/health` contract wins.

## Local test matrix

| Claim                                                 | Evidence source                       | Test                                                                  | Pass condition                                              | Failure response                            |
| ----------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------- |
| `/health` works without upstream access               | `proxy.rs:2480-2501`                  | Stop/blackhole upstream; call `/health`                               | HTTP 200, JSON body with `ccr` flag                         | Layer-3 diagnosis; fix local path           |
| `/health/upstream` is a separate probe with TTL cache | F19 cache in `proxy.rs`               | Call it twice; observe only one upstream hit within TTL               | Cached second result; no upstream flood                     | Fix TTL cache                               |
| Local engine round trip works                         | `aphrodite_compress`/`retrieve`       | Compress known payload, retrieve, byte-compare                        | Identical bytes; marker parses                              | Stop; resolver/store diagnosis              |
| Upstream failure yields a supported degraded mode     | Layer-4 rule                          | Force upstream error; observe engine state                            | Degraded mode is explicit (inline-only / raw), not invented | Add the missing degraded mode               |
| Preview truthfully guides retrieval                   | `preview/builders/`                   | Fixture-driven preview assertions (multiline/binary/JSON/code)        | No false line count or byte size                            | Fix class builder                           |
| Cache hit proof uses documented counters              | Counter inventory                     | Warm-cache compress/retrieve; read counters                           | Hit counters increment; no guessed fields                   | Fix telemetry mapping                       |
| Logs contain load failures when they occur            | Log locations table                   | Corrupt dylib, start session, read logs                               | Load failure visible in proxy-stderr.log or plugin logger   | Fix log wiring                              |
| No tracing subscriber inside the host                 | `tracing::dispatcher::has_been_set()` | Run a diagnostic in a subscriber-less process (`cargo run --example`) | `has_been_set()` false; stderr fallback prints              | Add stderr fallback or surfaced state field |
| One compression owner per session                     | Layer-2 rule                          | Check `compression.enabled` config with Aphrodite active              | Hermes built-in compression disabled                        | Disable built-in compression; recheck       |
