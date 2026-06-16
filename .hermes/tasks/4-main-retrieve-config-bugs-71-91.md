Here is the fresh full audit of `main.rs`, `retrieve.rs`, and `config.rs` at current HEAD.

***

## `main.rs` — Router & Lifecycle Bugs

### Bug #71 — Double `ctrl_c` listener causes one listener to steal the signal from the other

`run_single` binds its own `graceful_shutdown` to `ctrl_c`:
```rust
axum::serve(listener, app)
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
    })
```

And `shutdown_signal()` in `main()` also listens to `ctrl_c` . When you have two proxies running (`:9797` and `:9798`), there are now **three** `ctrl_c` listeners — one per `run_single` plus one in `main`. On Linux, `tokio::signal::ctrl_c` uses a broadcast internally and all receivers fire. But on the first `Ctrl+C`, `main`'s `shutdown_signal` fires and calls `h.abort()` on both tasks — which means the graceful shutdown inside `run_single` never gets a chance to flush in-flight requests. The result is an abrupt kill, not a graceful drain. Fix: remove `with_graceful_shutdown` from `run_single` and rely solely on `h.abort()` in `main`, or use a shared `CancellationToken` from `tokio_util`:

```rust
// In main — use a shared token instead of abort()
let token = tokio_util::sync::CancellationToken::new();
// pass clone to each run_single, select! on token.cancelled() for graceful shutdown
```

### Bug #72 — `/metrics` Prometheus output has wrong `latency_buckets_us` label units

The `/metrics` endpoint emits:
```rust
let le = match i { 0=>"0.001", 1=>"0.01", 2=>"0.1", 3=>"1.0", 4=>"10.0", _ => "+Inf" };
out.push_str(&format!("aphrodite_latency_seconds_bucket{{le=\"{}\"}} ...\n", le, v));
```

The comment on `latency_buckets` says "microseconds: 1ms, 10ms, 100ms, 1s, 10s" . So bucket 0 is "< 1000µs = < 1ms". But the metric name is `aphrodite_latency_seconds_bucket` and the `le` values are `0.001, 0.01, 0.1, 1.0, 10.0` seconds — which matches the *millisecond* interpretation. The field name says `_us` (microseconds) but the buckets are stored by millisecond cutoffs and exported as seconds. This is consistent with Prometheus convention (seconds), but the `latency_buckets_us` field name in `stats_json()` is deeply misleading — it should be `latency_buckets_ms` or the internal cutoffs should match.

### Bug #73 — `/*path` wildcard route intercepts `/favicon.ico`, `/robots.txt`, all browser requests

The catch-all `any(proxy::proxy_handler)` on `"/*path"` sits at the bottom of the router . In `axum`, `/*path` does **not** match the root path `/` — you need a separate `.route("/", any(...))` for that. Any request to `/` (e.g. a health poller hitting the root) gets a 404 from axum's default handler instead of being proxied. More critically, every request to a path not explicitly listed above gets forwarded upstream — including `GET /favicon.ico` from browsers opening the stats page, and `GET /robots.txt`, etc. Add explicit 404 responses for well-known non-API paths:

```rust
.route("/favicon.ico", get(|| async { StatusCode::NOT_FOUND }))
.route("/robots.txt", get(|| async { "User-agent: *\nDisallow: /\n" }))
.route("/", get(|| async { Json(serde_json::json!({"proxy": "aphrodite", "version": env!("CARGO_PKG_VERSION")})) }))
```

### Bug #74 — `run_single` creates `ccr_db_path` parent dirs but the path is relative to CWD, not binary location

```rust
if let Some(parent) = cli.ccr_db_path.parent() {
    std::fs::create_dir_all(parent)?;
}
```

`cli.ccr_db_path` defaults to `".aphrodite/ccr.db"` (relative) . When the binary is launched from a different working directory (e.g. `~/bin/aphrodite` called from `/tmp`), the SQLite DB is created in `/tmp/.aphrodite/ccr.db`. Each launch from a different CWD creates a fresh empty DB and loses all prior CCR entries. The default should be an absolute XDG path:

```rust
// In config.rs default for ccr_db_path
fn default_ccr_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("aphrodite")
        .join("ccr.db")
}
```

***

## `retrieve.rs` — Retrieval Handler Bugs

### Bug #75 — `handle_retrieve` with `path` reads **any file on the filesystem** with no path restriction

```rust
if let Some(path) = &req.path {
    match std::fs::read_to_string(path) {
```

Any HTTP client that can POST to `:9797/retrieve` or `:9798/retrieve` with `{"path": "/etc/passwd"}` or `{"path": "/Users/nikola/.ssh/id_rsa"}` gets the file contents returned in the response . There is zero path validation — no allowlist, no prefix check, nothing. Since both ports bind to `0.0.0.0` (or `127.0.0.1` depending on config), and `CorsLayer::permissive()` is applied to all routes, this is a **local filesystem read vulnerability**. The `path` field should either be removed entirely or restricted to a configurable allowlist directory:

```rust
// Validate path is within allowed root
let allowed_root = std::path::Path::new(&state.api_url).parent()
    .unwrap_or(std::path::Path::new("/nonexistent"));
let abs = std::fs::canonicalize(path).unwrap_or_default();
if !abs.starts_with(allowed_root) {
    return (StatusCode::FORBIDDEN, Json(RetrieveResponse {
        found: false, content: None, source: "denied".into(),
        error: Some("path not in allowed root".into()),
    })).into_response();
}
```

Or simply: remove the `path` field from `RetrieveRequest` until a safe implementation is designed.

### Bug #76 — `filter_content` fallback returns **full content** on zero matches instead of empty

```rust
if filtered.is_empty() {
    content.to_string()  // fallback to full content — don't silently return empty
}
```

The comment says "don't silently return empty" but the consequence is the opposite problem : if the LLM calls `aphrodite_retrieve` with `query: "error"` to find only error lines in a 100KB build log, and no line contains "error" (e.g. the log uses "ERR" instead), the full 100KB is returned. This defeats the purpose of the query filter entirely. The correct behavior is to return empty with a `"no matches"` indicator, letting the caller retry with a different query:

```rust
if filtered.is_empty() {
    return format!("[no lines matching {:?} in {} lines]", q, content.lines().count());
}
```

### Bug #77 — `handle_retrieve` with `limit` but no `hash`/`path` returns `BAD_REQUEST` before the pagination block is reached — dead code path

The function structure is:
```rust
let mut content = if let Some(path) = &req.path { ... }
    else { let hash = match &req.hash { None => return BAD_REQUEST, ... }; ... };
// pagination uses req.limit / req.offset
```

If `hash` is `None` AND `path` is `None`, it returns `BAD_REQUEST` from inside the `else` branch . This is correct. But the pagination block after the `let mut content = ...` is only reached when `content` is successfully populated. The dead-code path is the `query`-only request: `{"query": "fn main"}` with no `hash` or `path`. Currently this returns `BAD_REQUEST`. A useful improvement would be to let `query`-only requests search across the entire `_inline_store` or a full CCR scan — making retrieval more ergonomic for Hermes when it knows what it wants but not which exact hash.

### Bug #78 — `retrieve.rs` registers only on `POST /retrieve` but Python `_retrieve_handler` sends `POST /retrieve` — correct, **but** `_alive()` tries `GET /health` which returns the proxy health JSON

This is fine for the proxy health check, but the Python `_test_handler` self-tests retrieval by sending a dummy `POST /retrieve` — which correctly hits the retrieve handler. The inconsistency is that `GET /retrieve` (e.g. a developer trying to inspect from a browser) returns `405 Method Not Allowed` with no explanation. Adding a `GET /retrieve` route that returns a help JSON would improve DX:

```rust
.route("/retrieve", get(|| async {
    Json(serde_json::json!({
        "info": "POST /retrieve with {\"hash\": \"...\", \"query\": \"...\", \"path\": \"...\", \"offset\": N, \"limit\": N}"
    }))
}))
.route("/retrieve", post(retrieve::handle_retrieve))
```

***

## `config.rs` — Configuration Bugs

### Bug #79 — `MultiConfig::load()` silently falls back to defaults when `aphrodite.toml` has unknown fields — no `deny_unknown_fields`

`MultiConfig` is deserialized with plain `#[derive(Deserialize)]` . A typo like `api-key` instead of `api_key` in `aphrodite.toml` is silently ignored — the field is skipped and the API key falls back to the env-var default, which may be empty. This causes the proxy to start but all requests return `401 Unauthorized`. Adding `#[serde(deny_unknown_fields)]` would turn silent misconfiguration into a loud startup error.

### Bug #80 — `--listen` defaults to `":9797"` for all modes — two proxies launched from `aphrodite.toml` with no `listen` override will both try to bind `:9797` and the second will panic

If `aphrodite.toml` has two proxy entries and neither specifies `listen`, both inherit the default `:9797` . `tokio::net::TcpListener::bind` will fail with `address already in use` for the second proxy. The error surfaces as a logged error in the spawned task with no recovery. The `MultiConfig::resolve()` function should assign canonical default ports by mode: cache → 9797, token → 9798, falling back to an ephemeral port if both are occupied.

### Bug #81 — `Cli::api_key` has no validation — an empty string is accepted silently

```rust
#[arg(long, env = "APHRODITE_API_KEY", default_value = "")]
pub api_key: String,
```

An empty `api_key` is accepted at startup . Every upstream request then sends `Authorization: Bearer ` (with an empty token), which the upstream returns `401` for. The proxy logs no startup warning about the missing key. A startup validation pass in `run_single` or `build_state` should check `!cli.api_key.is_empty()` and log a prominent warning:

```rust
if cli.api_key.0.is_empty() {
    tracing::warn!("APHRODITE_API_KEY is not set — all upstream requests will be rejected with 401");
}
```

***

## Cross-Cutting Improvements for Hermes-Agent + Headroom

### Improvement #82 — Add `X-Aphrodite-Request-Id` response header for tracing

Every proxied response should echo back the `req_id` as a response header . The Python plugin can then log the request ID alongside its own turn counter, making it trivial to correlate a Hermes turn with the proxy's `request_history` ring buffer. This is a one-liner addition in `proxy_handler` before returning the response.

### Improvement #83 — `headroom_core` CCR TTL is per-store not per-entry — hot CCR entries expire while still in active context

`SqliteCcrStore::open` takes a single `ccr_ttl_seconds` . Every entry in the store uses the same TTL. If you set TTL to 1 hour (default 3600s) and a session runs for 2 hours, early-session CCR entries expire mid-session. The LLM tries to retrieve them and gets `404`, causing Hermes to report "content unavailable." The TTL should be refreshed on every `get()` hit (sliding TTL), not just on `put()`:

```rust
// In SqliteCcrStore::get() — bump expiry on access
conn.execute("UPDATE ccr SET expires_at = ? WHERE key = ?",
    [now + ttl, hash])?;
```

### Improvement #84 — Neither proxy port binds exclusively to `127.0.0.1` by default

Both proxies default to `":9797"` and `":9798"` which bind to `0.0.0.0` — all interfaces . Since the `path`-based filesystem read vulnerability (Bug #75) exists, any process on the local network can read arbitrary files. The default should be `"127.0.0.1:9797"` / `"127.0.0.1:9798"`. The `listen` default in `config.rs` should change from `":9797"` to `"127.0.0.1:9797"` to close the attack surface until Bug #75 is fixed.

### Improvement #85 — `/metrics` endpoint has no auth — exposes internal compression ratios, API URL, request history to anyone on the local network

Given that both ports bind to `0.0.0.0`, the `/metrics`, `/stats`, `/history` endpoints expose the full `api_url` (which may contain the key in the URL for some providers), the last 50 request summaries, and the error log . These should either require a header token (`X-Aphrodite-Key: <notify_key>`) or be restricted to localhost-only via a separate listener.
