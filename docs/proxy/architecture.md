# Proxy Architecture

Origin: Aphrodite is a reverse proxy that sits between Hermes (the LLM agent) and the upstream LLM API (DeepSeek/OpenAI-compatible). It intercepts Chat Completions responses, compresses tool outputs via CCR, and provides tool relay for bidirectional communication.

Source of truth: `crates/aphrodite/src/main.rs`, `crates/aphrodite/src/proxy.rs:build_state()` (line 426)

## Two-Listener Model

| Listener | Port | CCR Backend | Compression Threshold | Tool Relay | Mode |
|----------|------|-------------|----------------------|------------|------|
| Cache | :9797 | InMemoryCcrStore (DashMap, 10K entries) | >8KB (CACHE_COMPRESS_THRESHOLD) | No | ProxyMode::Cache |
| Token | :9798 | SqliteCcrStore (SQLite, persistent) | >1KB (TOKEN_COMPRESS_THRESHOLD) | Yes | ProxyMode::Token |

## Data Flow

```
Hermes Agent → aphrodite proxy (:9797/:9798) → Upstream LLM API
                                                     ↓
                                            Chat Completions response
                                                     ↓
                                            compress_chat_completion()
                                                     ↓
                                       CCR markers replace content
                                                     ↓
Hermes Agent ← compressed response ←───────────────┘
```

## AppState Structure

30+ AtomicU64 counters, 4 Mutex-protected structures, 1 TaskTracker.

### Core Config (proxy.rs:117-129)
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

### Cache Structures (proxy.rs:136-140)
```rust
pub request_history: Mutex<VecDeque<serde_json::Value>>,  // last 50 requests
pub inline_ccr: Mutex<lru::LruCache<String, String>>,      // 1024 entries, <256B threshold
```

### Primary Counters (proxy.rs:157-170)
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

### Latency Tracking (proxy.rs:144-146)
```rust
pub latency_buckets: [AtomicU64; 5],    // <1ms, <10ms, <100ms, <1s, <10s
pub total_latency_micros: AtomicU64,
```

### Error Tracking (proxy.rs:151)
```rust
pub last_errors: Mutex<VecDeque<String>>,  // last 100 errors
```

### Compression Tracking (proxy.rs:154)
```rust
pub compressions_by_type: Mutex<HashMap<String, u64>>,
```

### Extended Metrics (proxy.rs:182-195)
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

### Adaptive State (proxy.rs:174-179)
```rust
pub fill_pct: AtomicU64,          // ×100, 0-10000. fill_pct = 100 - (ratio_ema/20), clamped [1..99]
pub response_cache: Mutex<lru::LruCache<u64, Vec<u8>>>,  // 128 entries, FNV-1a hash key
```

## Routing Table

From `main.rs:run_single()` (lines 190-353):

| Route | Method | Handler | Access |
|-------|--------|---------|--------|
| `/health` | GET | health_check | Public (no loopback enforcement) |
| `/health/upstream` | GET | upstream probe | Loopback only |
| `/version` | GET | CARGO_PKG_VERSION | Loopback only |
| `/stats` | GET | stats_json() | Loopback only |
| `/stats/db` | GET | ccr.stats_db() | Loopback only |
| `/metrics` | GET | Prometheus text format | Loopback only (no auth) |
| `/history` | GET | request_history | Loopback only |
| `/retrieve` | POST | retrieve::handle_retrieve | Loopback only |
| `/tool/relay` | POST | handle_tool_relay | Loopback only |
| `/ccr/create` | POST | handle_ccr_create | Loopback only |
| `/ccr/list` | GET | handle_ccr_list | Loopback only |
| `/ccr/{hash}` | DELETE | handle_ccr_delete | Loopback only |
| `/favicon.ico` | GET | 404 | Loopback only |
| `/robots.txt` | GET | `Disallow: /` | Loopback only |
| `/` | GET | version JSON | Loopback only |
| `/{*path}` | ANY | proxy_handler | Loopback only |

## Middleware Stack

| Layer | Config |
|-------|--------|
| CORS | `CorsLayer::permissive()` |
| Body limit | 1 MB (`DefaultBodyLimit::max(1024 * 1024)`) |
| Loopback enforcement | `middleware::from_fn(loopback_only)`  -  all routes except `/health` |

## HTTP Client Config

From `proxy.rs:build_state()` (line 429):
```rust
HttpClient::builder()
    .timeout(Duration::from_secs(cli.timeout))     // default 300s, max 600s
    .pool_max_idle_per_host(100)
    .pool_idle_timeout(Duration::from_secs(90))
    .tcp_keepalive(Duration::from_secs(60))
    .build()
```

## Shutdown Sequence

From `main.rs:run()` (lines 107-148):

1. `shutdown_signal()`: wait for Ctrl+C or SIGTERM
2. `shutdown_tx.send(true)`: broadcast to all proxy listeners
3. `axum::serve.with_graceful_shutdown(shutdown_fut)`: drain connections
4. 5-second drain timeout → abort remaining tasks
5. Second Ctrl+C → force immediate shutdown via abort handles
6. `task_tracker.close(); task_tracker.wait()`: wait for background callbacks

## Multi-Proxy Mode

From `main.rs:run()` (line 46):

Priority: aphrodite.toml → CLI args

Config path: `APHRODITE_CONFIG_PATH` env var or `aphrodite.toml` (CWD).

Each `[[proxies]]` entry spawns its own Tokio task with independent `run_single()`. A shared `tokio::sync::watch` channel propagates the shutdown signal to all listeners.

## Worker Threads

From `main.rs:main()` (line 28):
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

Version info from env vars set by `build.rs`:
```rust
env!("CARGO_PKG_VERSION")
option_env!("APHRODITE_VERSION")
option_env!("APHRODITE_GIT_HASH")
option_env!("APHRODITE_PROFILE")
option_env!("APHRODITE_BUILD_DATE")
option_env!("APHRODITE_TARGET")
```
