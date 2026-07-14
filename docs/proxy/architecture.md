# Proxy Architecture

Aphrodite operates in **two modes** - as a reverse proxy between any client and
an LLM API, and as a native Hermes plugin that intercepts output at the hook
level before it reaches the LLM context.

- **Proxy mode**: sits between client and upstream LLM, compresses Chat
  Completions responses via CCR, provides tool relay for bidirectional
  communication.
- **Plugin mode**: Hermes hooks (`transform_tool_result`,
  `transform_terminal_output`, context engine) intercept output directly - no API
  round-trip needed. Broader coverage: file reads, terminal output, search
  results, browser snapshots, and more.

## Two-Listener Model

| Listener | Port  | CCR Backend                             | Compression Threshold           | Tool Relay | Mode             |
| -------- | ----- | --------------------------------------- | ------------------------------- | ---------- | ---------------- |
| Cache    | :9797 | InMemoryCcrStore (DashMap, 10K entries) | >8KB (CACHE_COMPRESS_THRESHOLD) | No         | ProxyMode::Cache |
| Token    | :9798 | SqliteCcrStore (SQLite, persistent)     | >1KB (TOKEN_COMPRESS_THRESHOLD) | Yes        | ProxyMode::Token |

## Data Flow

```
PROXY MODE (any client):
  Client → Aphrodite (:9797/:9798) → Upstream LLM API
              ↓
         compress Chat Completions response
              ↓
  Client ← CCR markers replace raw content

PLUGIN MODE (Hermes only):
  Tool executes → output intercepted by hook
              ↓
         classify → template → store
              ↓
  Agent ← [type:structured preview] (not raw output)
              ↓
         context engine auto-compresses middle turns
         prefetch loads files in background
```

## AppState Structure

30+ AtomicU64 counters, 4 Mutex-protected structures, 1 TaskTracker.

### Core Config

```rust
pub client: HttpClient,           // reqwest pool: 100 idle per host, 90s idle timeout, 60s keepalive
pub api_url: String,
pub model: String,
pub api_key: Secret,               // never logged (Debug/Display → [REDACTED])
pub ccr: Option<Arc<dyn CcrStore>>, // SQLite (token) or InMemory (cache)
pub add_markers: bool,
pub mode: ProxyMode,
pub tool_relay: bool,
pub notify_url: Option<String>,    // Hermes callback URL
pub notify_key: Option<String>,    // Bearer token for callbacks
pub dev: bool,                     // verbose request/response logging
```

### Cache Structures

```rust
pub request_history: Mutex<VecDeque<serde_json::Value>>,  // last 50 requests
pub inline_ccr: Mutex<lru::LruCache<String, String>>,      // 1024 entries, <256B threshold
```

### Primary Counters

```rust
pub requests_total: AtomicU64,
pub requests_compressed: AtomicU64,
pub tokens_saved: AtomicU64,
pub ccr_hits: AtomicU64,
pub ccr_misses: AtomicU64,
pub ccr_created: AtomicU64,
pub tool_relay_calls: AtomicU64,
pub compression_ratio_ema: AtomicU64,  // ×100
pub cache_hits: AtomicU64,
pub cache_misses: AtomicU64,
```

### Latency Tracking

```rust
pub latency_buckets: [AtomicU64; 5],    // <1ms, <10ms, <100ms, <1s, <10s
pub total_latency_micros: AtomicU64,
```

### Error Tracking

```rust
pub last_errors: Mutex<VecDeque<String>>,  // last 100 errors
```

### Compression Tracking

```rust
pub compressions_by_type: Mutex<HashMap<String, u64>>,
```

### Extended Metrics

```rust
pub inline_ccr_hits: AtomicU64,
pub inline_ccr_misses: AtomicU64,
pub tool_relay_success: AtomicU64,
pub tool_relay_failure: AtomicU64,
pub notify_success: AtomicU64,
pub notify_failure: AtomicU64,
pub upstream_errors_4xx: AtomicU64,
pub upstream_errors_5xx: AtomicU64,
pub upstream_timeouts: AtomicU64,
pub upstream_connect_errors: AtomicU64,  // non-timeout transport failures (connect refused, DNS, TLS)
pub sse_stream_errors: AtomicU64,        // mid-stream chunk errors on the SSE relay path
pub ccr_store_entries: AtomicU64,
pub ccr_store_bytes: AtomicU64,
pub request_body_bytes: AtomicU64,
pub response_body_bytes: AtomicU64,
pub upstream_latency_micros: AtomicU64,
```

### Task Tracking

```rust
pub task_tracker: TaskTracker,    // tracks async callbacks for graceful shutdown
```

### Adaptive State

```rust
pub fill_pct: AtomicU64,          // ×100, 0-10000. fill_pct = 100 - (ratio_ema/20), clamped [1..99]
pub response_cache: Mutex<lru::LruCache<u64, Vec<u8>>>,  // 128 entries, FNV-1a hash key
```

## Routing Table

| Route              | Method | Handler                   | Access                                 |
| ------------------ | ------ | ------------------------- | -------------------------------------- |
| `/health`          | GET    | health_check              | Public (no loopback enforcement)       |
| `/health/upstream` | GET    | upstream probe            | Loopback + mgmt token                  |
| `/version`         | GET    | CARGO_PKG_VERSION         | Loopback + mgmt token                  |
| `/stats`           | GET    | stats_json()              | Loopback + mgmt token                  |
| `/stats/db`        | GET    | ccr.stats_db()            | Loopback + mgmt token                  |
| `/metrics`         | GET    | Prometheus text format    | Loopback only (no auth, by design)     |
| `/history`         | GET    | request_history           | Loopback + mgmt token                  |
| `/retrieve`        | POST   | retrieve::handle_retrieve | Loopback + mgmt token                  |
| `/tool/relay`      | POST   | handle_tool_relay         | Loopback + mgmt token                  |
| `/ccr/create`      | POST   | handle_ccr_create         | Loopback + mgmt token                  |
| `/ccr/list`        | GET    | handle_ccr_list           | Loopback + mgmt token                  |
| `/ccr/{hash}`      | DELETE | handle_ccr_delete         | Loopback + mgmt token                  |
| `/reload`          | POST   | config hot-reload         | Loopback + mgmt token                  |
| `/favicon.ico`     | GET    | 404                       | Loopback only                          |
| `/robots.txt`      | GET    | `Disallow: /`             | Loopback only                          |
| `/`                | GET    | version JSON              | Loopback only                          |
| `/{*path}`         | ANY    | proxy_handler             | Loopback only (no mgmt token - LLM path) |

## Management-Route Authentication

When `APHRODITE_MGMT_TOKEN` is set, every "Loopback + mgmt token" route above
requires `Authorization: Bearer <token>`. This closes a cross-site-write gap:
a hostile local page could previously issue a CORS "simple request" that
lands as a write (seed CCR entries, evict markers via `/reload`) even though
it can't read the reply.

| Property        | Behavior                                                                    |
| --------------- | ---------------------------------------------------------------------------- |
| Unset (default) | Back-compat: any loopback caller accepted; a one-time startup `warn!` fires |
| Set             | Missing/wrong bearer token → 401                                            |
| Exempt          | `/health` (external health checks), `/metrics` (Prometheus scrapers), and the LLM-proxying `/{*path}` route |

## Middleware Stack

| Layer                | Config                                                             |
| -------------------- | ------------------------------------------------------------------ |
| CORS                 | `CorsLayer::permissive()`                                          |
| Body limit           | 1 MB (`DefaultBodyLimit::max(1024 * 1024)`)                        |
| Loopback enforcement | `middleware::from_fn(loopback_only)` - all routes except `/health`; an empty or unparseable `Host` header is rejected (DNS-rebinding hardening), not waved through |

## Streaming (SSE)

`"stream": true` requests and `text/event-stream` upstream responses take a
dedicated pass-through path:

| Aspect          | Behavior                                                                                                            |
| --------------- | -------------------------------------------------------------------------------------------------------------------- |
| Detection       | Request body `"stream": true` selects the streaming HTTP client; response `Content-Type: text/event-stream` (prefix match, charset-tolerant) selects the streaming response path |
| Client          | A separate `stream_client` with **no total timeout** - reqwest's client-level `.timeout()` bounds the whole body stream, which used to sever legitimately slow but progressing streams mid-answer; hang protection comes from `connect_timeout` + `tcp_keepalive` instead |
| Forwarding      | Chunk-by-chunk via `Body::from_stream` - never buffered                                                             |
| Compression     | Skipped entirely - markers can't be spliced into a live stream                                                      |
| Response cache  | Skipped - no cache key is computed for streaming requests                                                           |
| Headers         | Upstream headers propagated; `X-Aphrodite-Streamed: true` added                                                     |
| Metrics         | Streamed bytes count into `response_body_bytes`; mid-stream chunk errors increment `sse_stream_errors` (in `/stats` and `/metrics`) - previously a stream that died mid-flight recorded a 200 with zero signal |

## HTTP Client Config

```rust
HttpClient::builder()
    .timeout(Duration::from_secs(cli.timeout))     // default 300s, max 600s
    .pool_max_idle_per_host(100)
    .pool_idle_timeout(Duration::from_secs(90))
    .tcp_keepalive(Duration::from_secs(60))
    .build()
```

A second `stream_client` is built with the same pool/keepalive tuning but no
total timeout - see [Streaming (SSE)](#streaming-sse) above.

## Shutdown Sequence

1. `shutdown_signal()`: wait for Ctrl+C or SIGTERM
2. `shutdown_tx.send(true)`: broadcast to all proxy listeners
3. `axum::serve.with_graceful_shutdown(shutdown_fut)`: drain connections
4. 5-second drain timeout → abort remaining tasks
5. Second Ctrl+C → force immediate shutdown via abort handles
6. `task_tracker.close(); task_tracker.wait()`: wait for background callbacks

## Multi-Proxy Mode

Config resolution priority: `aphrodite.toml` → CLI args.

Config path: `APHRODITE_CONFIG_PATH` env var or `aphrodite.toml` (CWD).

Each `[[proxies]]` entry spawns its own Tokio task with independent
`run_single()`. A shared `tokio::sync::watch` channel propagates the shutdown
signal to all listeners.

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
