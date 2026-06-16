# Metrics Endpoint

Origin: Prometheus-compatible metrics exposition for monitoring and dashboarding. Always available on loopback without authentication (intentional for local-only deployments).

Source of truth: `crates/aphrodite/src/main.rs` `/metrics` handler (lines 251-325)

## Endpoint

```
GET /metrics
```

## Access

Loopback only  -  subject to `loopback_only` middleware. No auth.

## Content-Type

```
text/plain; version=0.0.4
```

## Format

Prometheus text exposition format  -  one metric per line with optional labels.

## Metrics Output

31 metrics in total:

```
aphrodite_requests_total{mode="token"} N
aphrodite_requests_compressed{mode="token"} N
aphrodite_tokens_saved N
aphrodite_ccr_hits N
aphrodite_ccr_misses N
aphrodite_ccr_created N
aphrodite_tool_relay_calls N
aphrodite_cache_hits N
aphrodite_cache_misses N
aphrodite_latency_seconds_bucket{le="0.001"} N
aphrodite_latency_seconds_bucket{le="0.01"} N
aphrodite_latency_seconds_bucket{le="0.1"} N
aphrodite_latency_seconds_bucket{le="1.0"} N
aphrodite_latency_seconds_bucket{le="10.0"} N
aphrodite_latency_seconds_count N
aphrodite_latency_seconds_sum N.NNNNNN
aphrodite_compression_ratio_ema N.NN
aphrodite_inline_ccr_hits N
aphrodite_inline_ccr_misses N
aphrodite_tool_relay_success N
aphrodite_tool_relay_failure N
aphrodite_notify_success N
aphrodite_notify_failure N
aphrodite_upstream_errors_total{code="4xx"} N
aphrodite_upstream_errors_total{code="5xx"} N
aphrodite_upstream_timeouts_total N
aphrodite_ccr_store_entries N
aphrodite_ccr_store_bytes N
aphrodite_request_body_bytes_total N
aphrodite_response_body_bytes_total N
aphrodite_upstream_latency_seconds_sum N.NNNNNN
```

## Latency Histogram

Buckets are cumulative:
```
aphrodite_latency_seconds_bucket{le="0.001"} <1ms_count
aphrodite_latency_seconds_bucket{le="0.01"}  <10ms_count
aphrodite_latency_seconds_bucket{le="0.1"}   <100ms_count
aphrodite_latency_seconds_bucket{le="1.0"}   <1s_count
aphrodite_latency_seconds_bucket{le="10.0"}  <10s_count
aphrodite_latency_seconds_count               total count
aphrodite_latency_seconds_sum                 total seconds (float)
```

Source: `proxy.rs:latency_buckets`  -  5-element AtomicU64 array with ranges <1ms, <10ms, <100ms, <1s, <10s.

## Build Logic

From main.rs:254-325:
```rust
let stats = s.stats_json();
let mut out = String::new();
// ... for each metric: push_str(&format!(...))
return (StatusCode::OK, [(CONTENT_TYPE, "text/plain; version=0.0.4")], out)
```

Values from `AppState` AtomicU64 counters read with `Ordering::Relaxed`  -  no locking.

## Security Note

No authentication  -  intentional for local-only deployments with loopback enforcement. In production, add a reverse-proxy auth layer or firewall this endpoint if exposed.
