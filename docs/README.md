# Aphrodite Documentation

Aphrodite is a reverse proxy with CCR (Compress-Cache-Retrieve) compression for LLM Chat Completions APIs. It sits between Hermes (the agent) and upstream LLM providers (DeepSeek, OpenAI-compatible), intercepting responses, compressing tool outputs, and providing a tool relay for bidirectional communication.

## Index

### CCR (Compress-Cache-Retrieve)

- [Marker Format](ccr/marker-format.md)  -  `<<<CCR:hash|type|size|mode|preview=PREVIEW|KEY=VALUE|...>>>` schema with exact hash format (BLAKE3, 24 hex), type enum (20 types), mode values, preview truncation by headroom budget, and metadata encoding rules
- [Lifecycle](ccr/lifecycle.md)  -  Full 6-phase flow: compress (detect→threshold→hash→cache→store→marker), retrieve (inline→CCR→zstd→filter→paginate), expire (TTL+LRU+debounce). Includes all threshold tables per type and mode
- [Content Types](ccr/content-types.md)  -  Complete taxonomy of 25 content types with detection order, threshold groups (×8, ×4, ×2, ×1, ÷2), and examples from both Rust (`detect_content_type`) and Python (`_classify_content`)
- **Backends**
  - [SQLite](ccr/backends/sqlite.md)  -  Schema (`ccr_entries` table), WAL mode, upsert semantics, lazy TTL purge (debounced 60s), poison resilience, stats_db schema
  - [In-Memory](ccr/backends/in-memory.md)  -  DashMap + VecDeque architecture, capacity 10,000, lazy TTL + capacity eviction, queue compaction, TOCTOU-safe `remove_if`, soft-cap race documentation
  - [Inline](ccr/backends/inline.md)  -  `lru::LruCache<String, String>`, 1024 entries, <256B threshold, dedup via `contains()`, lock-safety pattern (drop before await). Includes Python `_CappedStore` comparison

### Proxy

- [Architecture](proxy/architecture.md)  -  Two-listener model (:9797 cache + :9798 token), full AppState with 30+ AtomicU64 counters, routing table (16 routes), middleware stack, HTTP client config, multi-proxy mode, worker threads, shutdown sequence
- [Handlers](proxy/handlers.md)  -  7 handlers: proxy_handler (catch-all forward), handle_tool_relay, handle_ccr_create/list/delete, health_check, handle_retrieve. Full request/response schemas and flow
- [Retry](proxy/retry.md)  -  3 attempts, exponential backoff (100ms × 2^(n-1)), jitter 0.75–1.25×, transport-only retries, 502 on final failure
- [Compression](proxy/compression.md)  -  Full pipeline: detect→threshold (per-type × auto-tune × headroom budget)→hash→cache→marker→EMA→tokens_saved. Auto-tune state machine with fill_pct feedback loop

### Metrics

- [Prometheus](metrics/prometheus.md)  -  All 31 metrics with types, labels, wiring locations. Latency histogram (5 buckets). stats_json() schema. /metrics endpoint output format
- [Queries](metrics/queries.md)  -  PromQL reference: CCR hit rate, latency percentiles (P50/P95/P99), error rates, tool relay, cache performance, throughput. Dashboard panels and alert rules

### Configuration

- [aphrodite.toml](config/aphrodite-toml.md)  -  Full schema: [[proxies]] (name, listen, mode, tool_relay, timeout, retry, ccr_db_path), [defaults] (api_url, model, api_key, ccr_ttl). Resolution chain and API key fallback (5 levels)
- [Environment Variables](config/env-vars.md)  -  All 20+ env vars: API key chain, proxy operation, Python thresholds (engine, tool, terminal, inline), limits, passthrough. Production and development presets

### Tool Relay

- [Tools](tool-relay/tools.md)  -  9 tools with full JSON schemas: aphrodite_retrieve, compress, stats, rebuild, files, diff, search, test, catalog. Handlers, proxy support table, content-type hints
- [Callbacks](tool-relay/callbacks.md)  -  Async tool relay + CCR create notifications. SSRF protection (https only), Bearer token auth, 5s timeout, TaskTracker lifecycle, metrics

### Plugin

- [Hooks](plugin/hooks.md)  -  5 hooks: lifecycle order, on_session_start (inject instruction), transform_tool_result (proxy→inline→passthrough), pre_llm_call (catalog injection), transform_terminal_output (build collapse), post_llm_call (conversation store). Skip sets, thresholds per hook, headroom feedback loop
- [Context Engine](plugin/context-engine.md)  -  AphroditeContextEngine: compress middle messages→CCR, protect head/tail, editing detection, orphan sweep. Threshold semantics (-1/0/>0), mutual exclusion, hooks, session lifecycle

### API

- [Health](api/health.md)  -  GET /health → `{status, ccr, mode, version, fill_pct}` (public, no loopback)
- [Metrics](api/metrics-endpoint.md)  -  GET /metrics → Prometheus text, 31 metrics, 5 latency buckets
- [Retrieve](api/retrieve.md)  -  POST /retrieve `{hash, query?, offset?, limit?}` → `{found, content, source}`
- [CCR Endpoints](api/ccr-endpoints.md)  -  POST /ccr/create, GET /ccr/list, DELETE /ccr/:hash

## Source of Truth

All schemas, formats, and values are extracted verbatim from:
- `crates/aphrodite/src/proxy.rs` (1988 lines)  -  proxy handler, compression pipeline, tool relay, CCR management, health check, AppState, content detection
- `crates/aphrodite/src/retrieve.rs` (169 lines)  -  retrieve endpoint with zstd decompression, query filtering, pagination
- `crates/aphrodite/src/config.rs` (222 lines)  -  CLI args, MultiConfig, ProxyConfig, resolution chain
- `crates/aphrodite/src/main.rs` (414 lines)  -  routing, middleware, metrics handler, multi-proxy spawning, shutdown
- `vendor/headroom/crates/headroom-core/src/ccr/mod.rs` (133 lines)  -  CcrStore trait, compute_key (BLAKE3), marker_for
- `vendor/headroom/crates/headroom-core/src/ccr/backends/sqlite.rs` (334 lines)  -  SQLite schema, WAL, upsert, lazy purge, stats_db
- `vendor/headroom/crates/headroom-core/src/ccr/backends/in_memory.rs` (428 lines)  -  DashMap, FIFO eviction, queue compaction, TOCTOU safety
- `plugins/aphrodite/_core.py` (196 lines)  -  constants, thresholds, CCR regex, inline store, trigram index
- `plugins/aphrodite/_marker.py` (330 lines)  -  CCR marker generation, content classification, proxy compression, marker parsing
- `plugins/aphrodite/_tools.py` (149 lines)  -  retrieve/compress handlers with JSON schemas
- `plugins/aphrodite/_hooks.py` (1686 lines)  -  5 hook handlers, 7 additional tools, catalog builder
- `plugins/aphrodite/_engine.py` (289 lines)  -  AphroditeContextEngine: compress, editing detection, orphan sweep
- `plugins/aphrodite/plugin.yaml` (32 lines)  -  hook/tool/engine registration, install message

## Conventions

- Every file includes "Source of truth:" pointing to the specific file and line number
- Every file includes "Origin:" explaining the design rationale
- Schemas use tables, not prose
- No placeholder content  -  every schema is verified against the actual source code
