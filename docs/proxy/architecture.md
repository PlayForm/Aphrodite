# Proxy Architecture

Aphrodite ships a local reverse proxy that sits between any OpenAI-compatible
client and an upstream LLM API. It forwards Chat Completions traffic,
compresses eligible response content into CCR markers, serves identical
requests from a response cache, relays tools on request, and exposes
management endpoints for stats, retrieval, and CCR control. This page covers
the listener model, shared state, routing, middleware, streaming, and
lifecycle; the individual handlers are documented in
[Handlers](https://github.com/PlayForm/Aphrodite/tree/Development/docs/proxy/handlers.md).

## Two-Listener Model

A single binary can run two listeners with different roles - a cache proxy
and a token proxy - each with its own `AppState` and CCR backend. Both bind to
loopback by default.

| Listener | Default port | CCR backend                       | Compression threshold | Tool relay  | Mode               |
| -------- | ------------ | --------------------------------- | --------------------- | ----------- | ------------------ |
| Cache    | :9797        | InMemoryCcrStore (10,000 entries) | 8192 bytes            | Config flag | `ProxyMode::Cache` |
| Token    | :9798        | SqliteCcrStore (`ccr.db`)         | 1024 bytes            | Config flag | `ProxyMode::Token` |

- Default ports are configurable (`APHRODITE_CACHE_PORT`, `APHRODITE_TOKEN_PORT`,
  or the TOML `listen` per proxy).
- The token proxy persists entries to SQLite at `~/.hermes/aphrodite/ccr.db`
  (override with `APHRODITE_DB`); the cache proxy holds entries in memory only.
- Tool relay is an independent per-listener flag (`--tool-relay` or
  `tool_relay = true` in the TOML), not implied by the mode.
- Compression thresholds are live values, resolved as env var > TOML
  `[compression]` > compiled default, and hot-reloaded by `POST /reload` and
  the config-file watcher.

## Request Lifecycle

```
CLIENT → Aphrodite (:9797/:9798) → upstream LLM API
              │
              ├─ SSE response  → forwarded chunk-by-chunk, never compressed
              │
              └─ JSON response → Chat Completions?
                    │
                    ├─ cache hit  → replay buffered response (X-Aphrodite-Cache: HIT)
                    │
                    ├─ compressible message.content
                    │     → detect type → threshold check → CCR store
                    │     → replace with marker (X-Aphrodite-Compressed: true)
                    │
                    └─ otherwise  → pass through untouched
CLIENT ← response with upstream headers + X-Aphrodite-* headers
```

## AppState

`AppState` is the shared per-listener state, wrapped in `Arc` and cloned into
every handler. It holds the upstream client configuration, the CCR backend,
and all counters and caches used by the hot paths.

| Group                 | Contents                                                                                                                                                                                                                                           |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Client config         | `client` (bounded) + `stream_client` (no total timeout), `api_url`, `model`, `api_key` (redacted in logs), `dev` flag                                                                                                                              |
| CCR                   | `ccr: Option<Arc<dyn CcrStore>>`, `add_markers`, `mode`                                                                                                                                                                                            |
| Tool relay / notify   | `tool_relay`, `notify_url`, `notify_key`                                                                                                                                                                                                           |
| Counters              | 33 `AtomicU64` counters (requests, compressed, ccr hits/misses/created, tokens_saved, cache hits/misses, tool relay, notify, upstream 4xx/5xx/timeouts/connect errors, SSE stream errors, store entries/bytes, body bytes, latency, fill_pct, ...) |
| Live thresholds       | 3 `AtomicUsize` (cache/token/inline thresholds) + `code_multiplier_x100` (×100)                                                                                                                                                                    |
| Mutex-protected state | 6 `Mutex` fields: `request_history` (last 50), `last_errors` (last 100), `compressions_by_type`, `inline_ccr` (1024-entry LRU), `response_cache` (128-entry LRU, TTL-stamped), `upstream_health_cache`                                             |
| Task tracking         | `TaskTracker` for background callbacks (tool relay, CCR notifications)                                                                                                                                                                             |

`tokens_saved` accumulates raw bytes saved (original minus replacement), never
a token estimate; the name is kept for API compatibility.

## Routing Table

| Route              | Method | Handler                     | Access                                   |
| ------------------ | ------ | --------------------------- | ---------------------------------------- |
| `/health`          | GET    | `health_check`              | Public (no loopback enforcement)         |
| `/health/upstream` | GET    | upstream probe (60s cache)  | Loopback + mgmt token                    |
| `/version`         | GET    | `CARGO_PKG_VERSION`         | Loopback + mgmt token                    |
| `/stats`           | GET    | `stats_json()`              | Loopback + mgmt token                    |
| `/stats/db`        | GET    | `ccr.stats_db()`            | Loopback + mgmt token                    |
| `/metrics`         | GET    | Prometheus text format      | Loopback only (mgmt-token exempt)        |
| `/history`         | GET    | request history             | Loopback + mgmt token                    |
| `/retrieve`        | POST   | `retrieve::handle_retrieve` | Loopback + mgmt token                    |
| `/tool/relay`      | POST   | `handle_tool_relay`         | Loopback + mgmt token                    |
| `/ccr/create`      | POST   | `handle_ccr_create`         | Loopback + mgmt token                    |
| `/ccr/list`        | GET    | `handle_ccr_list`           | Loopback + mgmt token                    |
| `/ccr/{hash}`      | DELETE | `handle_ccr_delete`         | Loopback + mgmt token                    |
| `/reload`          | POST   | config hot-reload           | Loopback + mgmt token                    |
| `/favicon.ico`     | GET    | 404                         | Loopback + mgmt token                    |
| `/robots.txt`      | GET    | `Disallow: /`               | Loopback + mgmt token                    |
| `/`                | GET    | version JSON                | Loopback + mgmt token                    |
| `/{*path}`         | ANY    | `proxy_handler` catch-all   | Loopback only (LLM path - no mgmt token) |

## Management-Route Authentication

When `APHRODITE_MGMT_TOKEN` is set, every "Loopback + mgmt token" route above
requires `Authorization: Bearer <token>`. This closes a cross-site-write gap:
a hostile local page could previously issue a CORS "simple request" that lands
as a write (seed CCR entries, evict markers via `/reload`) even though it
cannot read the reply.

| Property        | Behavior                                                                                                    |
| --------------- | ----------------------------------------------------------------------------------------------------------- |
| Unset (default) | Back-compat: any loopback caller accepted; a one-time startup `warn!` fires                                 |
| Set             | Missing or wrong bearer token → 401                                                                         |
| Exempt          | `/health` (external health checks), `/metrics` (Prometheus scrapers), and the LLM-proxying `/{*path}` route |

Loopback enforcement adds a second layer beyond the peer-IP check: the `Host`
header must name a loopback address (`localhost`, `127.0.0.1`, `[::1]`, `::1`).
A missing or unparseable `Host` is rejected, not waved through - this blocks
DNS-rebinding, where an attacker's hostname resolves to 127.0.0.1 and the
browser genuinely is a loopback peer.

## Middleware Stack

| Layer                | Behavior                                                                                             |
| -------------------- | ---------------------------------------------------------------------------------------------------- |
| CORS                 | None - the proxy is consumed by non-browser HTTP clients, so no permissive CORS layer exists         |
| Body limit           | 1 MB on management routes; 64 MB on the catch-all `/{*path}` (large agent conversations exceed 1 MB) |
| Loopback enforcement | `loopback_only` middleware on all routes except `/health`; peer IP + Host-header validation          |
| Management auth      | `require_mgmt_token` bearer gate on the restricted router; `/metrics` exempt by path                 |

## Streaming (SSE)

Requests whose body sets `"stream": true`, and upstream responses with
`Content-Type: text/event-stream`, take a dedicated pass-through path:

| Aspect         | Behavior                                                                                                                                                                                                                                         |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Detection      | Request body `"stream": true` selects the streaming HTTP client; response content type `text/event-stream` (prefix match, charset-tolerant) selects the streaming response path                                                                  |
| Client         | A separate `stream_client` with no total timeout - reqwest's client-level `.timeout()` bounds the whole body stream, which used to sever slow-but-progressing streams mid-answer; hang protection comes from `connect_timeout` + `tcp_keepalive` |
| Forwarding     | Chunk-by-chunk via a body stream, never buffered                                                                                                                                                                                                 |
| Compression    | Skipped entirely - markers cannot be spliced into a live stream                                                                                                                                                                                  |
| Response cache | Skipped - streaming requests produce no cache key, so they never hit or populate the response cache                                                                                                                                              |
| Headers        | Upstream headers propagated (hop-by-hop ones stripped); `X-Aphrodite-Streamed: true` added                                                                                                                                                       |
| Metrics        | Streamed bytes count into `response_body_bytes`; mid-stream chunk errors increment `sse_stream_errors` (previously a stream that died mid-flight recorded a 200 with zero signal)                                                                |

## HTTP Client Config

```rust
HttpClient::builder()
    .timeout(Duration::from_secs(cli.timeout))  // default 300s, clamped to max 600s
    .connect_timeout(Duration::from_secs(10))
    .pool_max_idle_per_host(100)
    .pool_idle_timeout(Duration::from_secs(90))
    .tcp_keepalive(Duration::from_secs(60))
    .build()
```

A second `stream_client` is built with the same pool/keepalive tuning but no
total timeout - see [Streaming (SSE)](#streaming-sse) above. The client is
built without gzip/brotli auto-decompression, so `Accept-Encoding: gzip` from
a caller is stripped rather than forwarded.

## Response Cache

Chat Completions requests are cached so an identical request is served
without a second upstream round-trip.

| Property  | Behavior                                                                                                        |
| --------- | --------------------------------------------------------------------------------------------------------------- |
| Key       | FNV-1a 64-bit over api_key + model + messages + tools + tool_choice + temperature + top_p + n + response_format |
| Size      | 128-entry LRU, entries capped at 1 MB                                                                           |
| TTL       | `response_cache_ttl`, reusing `ccr_ttl_seconds` (default 3600s); expired entries are evicted on the hit path    |
| Store     | Successful (2xx) responses only, compressed or raw                                                              |
| Streaming | Never cached - a `"stream": true` request yields no cache key                                                   |
| Hit       | `X-Aphrodite-Cache: HIT`, whole cached body length added to `tokens_saved`                                      |
| Miss      | `X-Aphrodite-Cache: MISS` on the forwarded response                                                             |

## Shutdown Sequence

1. `shutdown_signal()`: wait for Ctrl+C or SIGTERM.
2. `shutdown_tx.send(true)`: broadcast to every proxy listener.
3. `axum::serve.with_graceful_shutdown(shutdown_fut)`: drain connections.
4. 5-second drain timeout → abort remaining tasks.
5. Second Ctrl+C → force immediate shutdown.
6. `task_tracker.close(); task_tracker.wait()`: wait for background callbacks.

## Multi-Proxy Mode

Configuration resolution: env var > TOML (`[[proxies]]` entry over `[defaults]`)

> CLI defaults. Config path resolution: `APHRODITE_CONFIG_PATH` →
> `./aphrodite.toml` (CWD) → `~/.hermes/aphrodite/aphrodite.toml`.

Each `[[proxies]]` entry spawns its own Tokio task with an independent
`run_single()`. All listeners are bound before any server task spawns - a bind
failure aborts startup loudly instead of leaving a silently dead listener. A
shared `tokio::sync::watch` channel propagates the shutdown signal to all
listeners, and a config-file watcher (500 ms debounce) applies `[compression]`
threshold changes to every live `AppState`.

## Worker Threads

```rust
let worker_threads = std::env::var("APHRODITE_WORKER_THREADS")
    .ok()
    .and_then(|v| v.parse::<usize>().ok())
    .unwrap_or_else(|| {
        let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
        (cpus * 4).max(32)
    });
```

Default: 4× CPU cores, minimum 32. Override via `APHRODITE_WORKER_THREADS`.

## Build Info

Version info comes from env vars set by the build script:

```rust
env!("CARGO_PKG_VERSION")
option_env!("APHRODITE_VERSION")
option_env!("APHRODITE_GIT_HASH")
option_env!("APHRODITE_PROFILE")
option_env!("APHRODITE_BUILD_DATE")
option_env!("APHRODITE_TARGET")
```
