# Metrics Endpoint

`GET /metrics` exposes proxy counters, gauges, and a latency histogram in the
Prometheus text exposition format for scraping, alerting, and dashboards. It
is always available on loopback without authentication - intentional for
local-only deployments.

## Endpoint

| Method | Path       | Access   | Auth |
| ------ | ---------- | -------- | ---- |
| GET    | `/metrics` | Loopback | None |

## Access

Loopback only (subject to `loopback_only` middleware, which also validates the
`Host` header). No auth: `/metrics` stays token-exempt even when
`APHRODITE_MGMT_TOKEN` gates the other management routes, so Prometheus
scrapers keep working unmodified. In production, add a reverse-proxy auth
layer or firewall this endpoint if it must be exposed.

## Content-Type

```
text/plain; version=0.0.4
```

## Format

Prometheus text exposition format - one metric per line, optional `mode`,
`code`, and `le` labels. All values are read from atomic counters with relaxed
ordering, so no locking is involved. The output is built from the same stats
object as `/stats`.

## Emitted Metrics

28 distinct metric names. Latency buckets are cumulative (see below), so the
5 bucket lines share one name:

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

Latency is tracked in a 5-element atomic counter array with the boundaries
`<1ms`, `<10ms`, `<100ms`, `<1s`, and everything else. Output buckets are
cumulative, as the format requires; the last bucket is labeled `+Inf`, not
`10.0` - it catches every sample at or above 1s, including long outliers, and
the explicit `+Inf` bucket is what makes `histogram_quantile()` work:

```
aphrodite_latency_seconds_bucket{le="0.001"} <1ms count
aphrodite_latency_seconds_bucket{le="0.01"}  <10ms cumulative count
aphrodite_latency_seconds_bucket{le="0.1"}   <100ms cumulative count
aphrodite_latency_seconds_bucket{le="1.0"}   <1s cumulative count
aphrodite_latency_seconds_bucket{le="+Inf"}  total count
aphrodite_latency_seconds_count              total count
aphrodite_latency_seconds_sum                total seconds (float, 6dp)
```

See [Prometheus Metrics](../metrics/prometheus.md) for the full catalog with
types, labels, and example queries.
