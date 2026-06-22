# Aphrodite Documentation

Aphrodite compresses context before it hits the LLM - through a reverse proxy
for any OpenAI-compatible client, or as a native Hermes plugin with hook-level
interception. Covers tool output, terminal output, file reads, search results,
browser snapshots, build logs, and more. CCR (Compress-Cache-Retrieve) storage,
28-type classifier, context engine, and prefetch pipeline - all under 1ms.

## Index

### Aphrodite & Headroom

- [Comparison: Aphrodite vs Headroom](APHRODITE-HEADROOM.md) - What Aphrodite
  adds on top of our Headroom fork, what we rewrote, how they ship together
- [Fork Divergence Analysis](HEADROOM-FORK-DIFF.md) - Every commit, deletion,
  and modification between upstream Headroom and the PlayForm fork

### CCR (Compress-Cache-Retrieve)

- [Marker Format](ccr/marker-format.md) - `<<<CCR:hash|type|size>>>` schema with
  SHA-256 hash, 28 content types, TOML‑driven preview templates, and metadata
  encoding rules
- [Lifecycle](ccr/lifecycle.md) - Full 6-phase flow: compress
  (detect→threshold→hash→cache→store→marker), retrieve
  (inline→CCR→resolve→filter→paginate), expire (TTL+LRU+debounce). Includes all
  threshold tables per type and mode
- [Content Types](ccr/content-types.md) - Complete taxonomy of 28 content types
  with detection order, threshold groups, and examples from both Rust
  (`detect_content_type`) and Python (`_classify_content`)
- **Backends**
    - [SQLite](ccr/backends/sqlite.md) - Schema (`ccr_entries` table), WAL mode,
      upsert semantics, lazy TTL purge (debounced 60s), poison resilience,
      stats_db schema
    - [In-Memory](ccr/backends/in-memory.md) - DashMap + VecDeque architecture,
      capacity 10,000, lazy TTL + capacity eviction, queue compaction,
      TOCTOU-safe `remove_if`, soft-cap race documentation
    - [Inline](ccr/backends/inline.md) - `lru::LruCache<String, String>`, 1024
      entries, <256B threshold, dedup via `contains()`, lock-safety pattern
      (drop before await). Includes Python `_CappedStore` comparison

### Proxy

- [Architecture](proxy/architecture.md) - Two-listener model (:9797 cache +
  :9798 token), full AppState with 30+ AtomicU64 counters, routing table (16
  routes), middleware stack, HTTP client config, multi-proxy mode, worker
  threads, shutdown sequence
- [Handlers](proxy/handlers.md) - 7 handlers: proxy_handler (catch-all forward),
  handle_tool_relay, handle_ccr_create/list/delete, health_check,
  handle_retrieve. Full request/response schemas and flow
- [Retry](proxy/retry.md) - 3 attempts, exponential backoff (100ms × 2^(n-1)),
  jitter 0.75–1.25×, transport-only retries, 502 on final failure
- [Compression](proxy/compression.md) - Full pipeline: detect→threshold
  (per-type × auto-tune × headroom budget)→hash→cache→marker→EMA→tokens_saved.
  Auto-tune state machine with fill_pct feedback loop

### Metrics

- [Prometheus](metrics/prometheus.md) - All 31 metrics with types, labels,
  wiring locations. Latency histogram (5 buckets). stats_json() schema. /metrics
  endpoint output format
- [Queries](metrics/queries.md) - PromQL reference: CCR hit rate, latency
  percentiles (P50/P95/P99), error rates, tool relay, cache performance,
  throughput. Dashboard panels and alert rules

### Configuration

- [aphrodite.toml](config/aphrodite-toml.md) - Full schema: [[proxies]] (name,
  listen, mode, tool_relay, timeout, retry, ccr_db_path), [defaults] (api_url,
  model, api_key, ccr_ttl). Resolution chain and API key fallback (5 levels)
- [Environment Variables](config/env-vars.md) - All 20+ env vars: API key chain,
  proxy operation, Python thresholds (engine, tool, terminal, inline), limits,
  passthrough. Production and development presets

### Tool Relay

- [Tools](tool-relay/tools.md) - 12 tools with full JSON schemas:
  aphrodite_retrieve, compress, stats, rebuild, files, diff, search, test,
  catalog, reclassify, prefetch, prefetch_status. All delegate to Rust dylib
- [Callbacks](tool-relay/callbacks.md) - Async tool relay + CCR create
  notifications. SSRF protection (https only), Bearer token auth, 5s timeout,
  TaskTracker lifecycle, metrics

### Plugin

- [Hooks](plugin/hooks.md) - 5 hooks: lifecycle order, on_session_start (inject
  instruction), transform_tool_result (proxy→inline→passthrough), pre_llm_call
  (catalog injection), transform_terminal_output (build collapse), post_llm_call
  (conversation store). Skip sets, thresholds per hook, headroom feedback loop
- [Context Engine](plugin/context-engine.md) - AphroditeContextEngine: compress
  middle messages→CCR, protect head/tail, editing detection, orphan sweep.
  Threshold semantics (-1/0/>0), mutual exclusion, hooks, session lifecycle

### API

- [Health](api/health.md) - GET /health →
  `{status, ccr, mode, version, fill_pct}` (public, no loopback)
- [Metrics](api/metrics-endpoint.md) - GET /metrics → Prometheus text, 31
  metrics, 5 latency buckets
- [Retrieve](api/retrieve.md) - POST /retrieve `{hash, query?, offset?, limit?}`
  → `{found, content, source}`
- [CCR Endpoints](api/ccr-endpoints.md) - POST /ccr/create, GET /ccr/list,
  DELETE /ccr/:hash

## Source of Truth

All schemas, formats, and values are extracted verbatim from:

### Rust (core compression engine — binary + dylib)

- `crates/aphrodite/src/proxy.rs` — proxy handler, compression pipeline, tool relay
- `crates/aphrodite/src/resolve.rs` — CCR marker resolution (nested, recursive, cycle-safe)
- `crates/aphrodite/src/stage2.rs` — Semantic reduction (JSON, build, diff, code)
- `crates/aphrodite/src/struct_extract.rs` — Code structure extraction (Rust, Python, Go, JS/TS)
- `crates/aphrodite/src/hooks.rs` — transform_tool_result, transform_terminal_output
- `crates/aphrodite/src/state.rs` — Session state, inline store, LRU eviction
- `crates/aphrodite/src/marker.rs` — CCR marker generation, parsing
- `crates/aphrodite/src/catalog.rs` — Full catalog with by-type grouping
- `crates/aphrodite/src/session.rs` — Session lifecycle, turn tracking
- `crates/aphrodite/src/prefetch.rs` — Background file loading
- `crates/aphrodite/src/config_loader.rs` — TOML config with env override
- `crates/aphrodite/src/lib.rs` — 17 C ABI functions for dylib loading
- `crates/aphrodite-hermes/src/` — Hermes integration: tools, schemas, skills
- `vendor/headroom/crates/headroom-core/src/` — Compression engine (PlayForm fork)

### Python (thin loader — Hermes integration)

- `plugins/aphrodite/__init__.py` — Thin loader: dylib loading, hook/tool registration

## Conventions

- Every file includes "Source of truth:" pointing to the specific file and line
  number
- Every file includes "Origin:" explaining the design rationale
- Schemas use tables, not prose
- No placeholder content - every schema is verified against the actual source
  code
