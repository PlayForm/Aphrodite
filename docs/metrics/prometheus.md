# Prometheus Metrics

All proxy operations expose counters, gauges, and histograms at `/metrics` in
Prometheus text format for monitoring, alerting, and dashboard visualization.

## Endpoint

```
GET /metrics
Content-Type: text/plain; version=0.0.4
```

No auth - intentional for local-only deployments (loopback-enforced). Firewall
or reverse-proxy auth layer recommended for production.

## All 31 Metrics

### Request Counters

| Metric                          | Type    | Labels               | Description                          |
| ---------------------------------- | --------- | ----------------------- | ---------------------------------------- |
| `aphrodite_requests_total`      | counter | `mode` (cache/token) | Total requests received              |
| `aphrodite_requests_compressed` | counter | `mode` (cache/token) | Requests that had content compressed |

### Token Savings

| Metric                   | Type    | Labels | Description                        |
| --------------------------- | --------- | -------- | -------------------------------------- |
| `aphrodite_tokens_saved` | counter | -      | Total tokens saved via compression |

### CCR Operations

| Metric                        | Type    | Labels | Description                  |
| -------------------------------- | --------- | -------- | -------------------------------- |
| `aphrodite_ccr_hits`          | counter | -      | CCR cache hits               |
| `aphrodite_ccr_misses`        | counter | -      | CCR cache misses             |
| `aphrodite_ccr_created`       | counter | -      | New CCR entries created      |
| `aphrodite_ccr_store_entries` | gauge   | -      | Current entries in CCR store |
| `aphrodite_ccr_store_bytes`   | gauge   | -      | Approximate bytes stored     |

### Tool Relay

| Metric                         | Type    | Labels | Description                  |
| --------------------------------- | --------- | -------- | -------------------------------- |
| `aphrodite_tool_relay_calls`   | counter | -      | Total tool relay invocations |
| `aphrodite_tool_relay_success` | counter | -      | Successful tool executions   |
| `aphrodite_tool_relay_failure` | counter | -      | Failed tool executions       |

### Response Cache

| Metric                   | Type    | Labels | Description               |
| --------------------------- | --------- | -------- | --------------------------- |
| `aphrodite_cache_hits`   | counter | -      | LLM response cache hits   |
| `aphrodite_cache_misses` | counter | -      | LLM response cache misses |

### Inline CCR

| Metric                        | Type    | Labels | Description            |
| -------------------------------- | --------- | -------- | ------------------------ |
| `aphrodite_inline_ccr_hits`   | counter | -      | Inline LruCache hits   |
| `aphrodite_inline_ccr_misses` | counter | -      | Inline LruCache misses |

### Notification Callbacks

| Metric                     | Type    | Labels | Description                       |
| ----------------------------- | --------- | -------- | ------------------------------------ |
| `aphrodite_notify_success` | counter | -      | Successful callback notifications |
| `aphrodite_notify_failure` | counter | -      | Failed callback notifications     |

See [Callbacks](../tool-relay/callbacks.md) for how these are triggered.

### Upstream Errors

| Metric                              | Type    | Labels           | Description                   |
| --------------------------------------- | --------- | ------------------ | ---------------------------------- |
| `aphrodite_upstream_errors_total`   | counter | `code` (4xx/5xx) | Upstream HTTP error responses |
| `aphrodite_upstream_timeouts_total` | counter | -                | Upstream connection timeouts  |

### Body Bytes

| Metric                                | Type    | Labels | Description                       |
| ------------------------------------------ | --------- | -------- | -------------------------------------- |
| `aphrodite_request_body_bytes_total`  | counter | -      | Total request body bytes received |
| `aphrodite_response_body_bytes_total` | counter | -      | Total response body bytes sent    |

### Latency

| Metric                                   | Type      | Labels                         | Description                             |
| --------------------------------------------- | ----------- | --------------------------------- | ------------------------------------------ |
| `aphrodite_latency_seconds`              | histogram | `le` (0.001/0.01/0.1/1.0/10.0) | End-to-end request latency distribution |
| `aphrodite_latency_seconds_count`        | counter   | -                              | Total latency observations              |
| `aphrodite_latency_seconds_sum`          | counter   | -                              | Total latency in seconds                |
| `aphrodite_upstream_latency_seconds_sum` | counter   | -                              | Total upstream latency in seconds       |

### Compression Ratio

| Metric                            | Type  | Labels | Description                                     |
| -------------------------------------- | ------- | -------- | ---------------------------------------------------- |
| `aphrodite_compression_ratio_ema` | gauge | -      | Exponential moving average of compression ratio |

## Latency Bucket Boundaries

Latency is tracked in 5 fixed buckets, each holding a cumulative count:

| Bucket Index | le value | Range        |
| -------------- | ---------- | -------------- |
| 0            | 0.001    | < 1ms        |
| 1            | 0.01     | 1ms - 10ms   |
| 2            | 0.1      | 10ms - 100ms |
| 3            | 1.0      | 100ms - 1s   |
| 4            | 10.0     | 1s - 10s     |

Buckets are cumulative in the Prometheus output, as required by the format:
each bucket's count includes all observations from lower buckets.

## Example Output

```
aphrodite_requests_total{mode="token"} 15423
aphrodite_requests_compressed{mode="token"} 12001
aphrodite_tokens_saved 15432000
aphrodite_ccr_hits 8945
aphrodite_ccr_misses 3056
aphrodite_ccr_created 3056
aphrodite_tool_relay_calls 423
aphrodite_cache_hits 1200
aphrodite_cache_misses 14223
aphrodite_latency_seconds_bucket{le="0.001"} 1000
aphrodite_latency_seconds_bucket{le="0.01"} 8000
aphrodite_latency_seconds_bucket{le="0.1"} 12000
aphrodite_latency_seconds_bucket{le="1.0"} 15000
aphrodite_latency_seconds_bucket{le="10.0"} 15423
aphrodite_latency_seconds_count 15423
aphrodite_latency_seconds_sum 245.123456
aphrodite_compression_ratio_ema 8.50
aphrodite_inline_ccr_hits 567
aphrodite_inline_ccr_misses 234
aphrodite_tool_relay_success 412
aphrodite_tool_relay_failure 11
aphrodite_notify_success 8901
aphrodite_notify_failure 23
aphrodite_upstream_errors_total{code="4xx"} 15
aphrodite_upstream_errors_total{code="5xx"} 3
aphrodite_upstream_timeouts_total 2
aphrodite_ccr_store_entries 3056
aphrodite_ccr_store_bytes 45234000
aphrodite_request_body_bytes_total 250000000
aphrodite_response_body_bytes_total 187000000
aphrodite_upstream_latency_seconds_sum 180.000000
```

## stats_json() Schema

`/metrics` is built from the same underlying stats as `/stats`. Full JSON
schema:

```json
{
    "mode": "token" | "cache",
    "proxy": "aphrodite",
    "ccr_backend": "enabled" | "none",
    "tool_relay": bool,
    "requests": {
        "total": u64,
        "compressed": u64
    },
    "tokens_saved": u64,
    "ccr": {
        "hits": u64,
        "misses": u64,
        "created": u64
    },
    "tool_relay_calls": u64,
    "cache": {
        "hits": u64,
        "misses": u64
    },
    "latency_buckets_us": [u64; 5],
    "total_latency_micros": u64,
    "compressions_by_type": {"code_rust": u64, ...},
    "compression_ratio_ema": f64,
    "last_errors": ["error string", ...],         // last 5, most recent first
    "request_history": [                              // last 50
        {"id": "uuid8", "method": "POST", "path": "/v1/...", "status": 200, "compressed": true, "elapsed_ms": 1234}
    ],
    "inline_ccr": {"hits": u64, "misses": u64},
    "tool_relay": {"total": u64, "success": u64, "failure": u64},
    "notify": {"success": u64, "failure": u64},
    "upstream_errors": {"4xx": u64, "5xx": u64, "timeouts": u64},
    "ccr_store": {"entries": u64, "bytes_approx": u64},
    "body_bytes": {"request": u64, "response": u64},
    "upstream_latency_micros": u64
}
```

## Endpoint: /stats

Returns the JSON above directly. Loopback only.

## Endpoint: /stats/db

Returns database-level stats, only available for the SQLite backend:

```json
{
	"total_entries": 3056,
	"total_bytes_original": 45234000,
	"total_bytes_compressed": 73344,
	"oldest_entry_age_seconds": 3421,
	"database_size_bytes": 52428800
}
```
