# Metrics Endpoint

Exposes Prometheus-compatible metrics for monitoring and dashboarding. It's
always available on loopback without authentication, which is intentional for
local-only deployments.

## Endpoint

```
GET /metrics
```

## Access

Loopback only - subject to `loopback_only` middleware. No auth - `/metrics`
stays exempt even when `APHRODITE_MGMT_TOKEN` gates the other management
routes, so Prometheus scrapers keep working unmodified.

## Content-Type

```
text/plain; version=0.0.4
```

## Format

Prometheus text exposition format - one metric per line with optional labels.

## Metrics Output

28 metrics in total (verified against a live `curl /metrics` response):

```
aphrodite_requests_total{mode="token"} N
aphrodite_requests_compressed_total{mode="token"} N
aphrodite_tokens_saved_total N
aphrodite_ccr_hits_total N
aphrodite_ccr_misses_total N
aphrodite_ccr_created_total N
aphrodite_tool_relay_calls_total N
aphrodite_cache_hits_total N
aphrodite_cache_misses_total N
aphrodite_latency_seconds_bucket{le="0.001"} N
aphrodite_latency_seconds_bucket{le="0.01"} N
aphrodite_latency_seconds_bucket{le="0.1"} N
aphrodite_latency_seconds_bucket{le="1.0"} N
aphrodite_latency_seconds_bucket{le="+Inf"} N
aphrodite_latency_seconds_count N
aphrodite_latency_seconds_sum N.NNNNNN
aphrodite_compression_ratio_ema N.NN
aphrodite_inline_ccr_hits_total N
aphrodite_inline_ccr_misses_total N
aphrodite_tool_relay_success_total N
aphrodite_tool_relay_failure_total N
aphrodite_notify_success_total N
aphrodite_notify_failure_total N
aphrodite_upstream_errors_total{code="4xx"} N
aphrodite_upstream_errors_total{code="5xx"} N
aphrodite_upstream_timeouts_total N
aphrodite_upstream_connect_errors_total N
aphrodite_sse_stream_errors_total N
aphrodite_ccr_store_entries N
aphrodite_ccr_store_bytes N
aphrodite_request_body_bytes_total N
aphrodite_response_body_bytes_total N
aphrodite_upstream_latency_seconds_total N.NNNNNN
```

## Latency Histogram

Buckets are cumulative; the last bucket is unbounded (`+Inf`), not a `10.0`
label - it catches every sample >= 1s, including 30s+ outliers, so labeling
it `10.0` would have made Prometheus consumers assume everything in it is
<= 10s and mis-compute quantiles:

```
aphrodite_latency_seconds_bucket{le="0.001"} <1ms_count
aphrodite_latency_seconds_bucket{le="0.01"}  <10ms_count
aphrodite_latency_seconds_bucket{le="0.1"}   <100ms_count
aphrodite_latency_seconds_bucket{le="1.0"}   <1s_count
aphrodite_latency_seconds_bucket{le="+Inf"}  everything_count (total, satisfies histogram_quantile())
aphrodite_latency_seconds_count               total count
aphrodite_latency_seconds_sum                 total seconds (float)
```

The buckets are backed by a 5-element atomic counter array covering ranges
<1ms, <10ms, <100ms, <1s, and everything else (labeled `+Inf`).

## Build Logic

```rust
let stats = s.stats_json();
let mut out = String::new();
// ... for each metric: push_str(&format!(...))
return (StatusCode::OK, [(CONTENT_TYPE, "text/plain; version=0.0.4")], out)
```

Values come from atomic counters read with relaxed ordering, so there's no
locking involved.

## Security Note

No authentication - intentional for local-only deployments with loopback
enforcement. In production, add a reverse-proxy auth layer or firewall this
endpoint if exposed.
