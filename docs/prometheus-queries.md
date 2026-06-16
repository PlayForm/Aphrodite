# Prometheus Queries — Aphrodite Proxy

## Quick curl

```bash
# All metrics raw
curl -s http://0.0.0.0:9798/metrics | sort

# Prometheus API
PROM=http://localhost:9090/api/v1/query
```

## CCR Activity

| What | PromQL | curl |
|------|--------|------|
| **CCR hits** | `aphrodite_ccr_hits_total` | `?query=aphrodite_ccr_hits_total` |
| **CCR misses** | `aphrodite_ccr_misses_total` | `?query=aphrodite_ccr_misses_total` |
| **CCR created** | `aphrodite_ccr_created_total` | `?query=aphrodite_ccr_created_total` |
| **CCR hit rate** | `rate(aphrodite_ccr_hits_total[5m])` | `?query=rate(aphrodite_ccr_hits_total[5m])` |
| **CCR store entries** | `aphrodite_ccr_store_entries` | `?query=aphrodite_ccr_store_entries` |
| **CCR store bytes** | `aphrodite_ccr_store_bytes` | `?query=aphrodite_ccr_store_bytes` |

```bash
# CCR activity snapshot
curl -s "$PROM" --data-urlencode 'query=aphrodite_ccr_created' | python3 -c "
import sys,json; d=json.load(sys.stdin)
for r in d['data']['result']: print(f\"CCR created: {r['value'][1]}\")
"
```

## Requests & Compression

| What | PromQL | curl |
|------|--------|------|
| **Total requests** | `aphrodite_requests_total` | `?query=aphrodite_requests_total` |
| **Compressed requests** | `aphrodite_requests_compressed_total` | `?query=aphrodite_requests_compressed_total` |
| **Request rate (/s)** | `rate(aphrodite_requests_total[1m])` | `?query=rate(aphrodite_requests_total[1m])` |
| **Compression ratio** | `rate(aphrodite_requests_compressed_total[5m]) / rate(aphrodite_requests_total[5m])` | — |
| **Compression EMA** | `aphrodite_compression_ratio_ema` | `?query=aphrodite_compression_ratio_ema` |

```bash
# Request rate per second (last 1m)
curl -s "$PROM" --data-urlencode 'query=rate(aphrodite_requests_total[1m])' | python3 -c "
import sys,json; d=json.load(sys.stdin)
for r in d['data']['result']:
    mode = r['metric'].get('mode','?')
    print(f'{mode}: {float(r[\"value\"][1]):.2f} req/s')
"
```

## Token Savings

| What | PromQL | curl |
|------|--------|------|
| **Tokens saved** | `aphrodite_tokens_saved_total` | `?query=aphrodite_tokens_saved_total` |
| **Savings rate** | `rate(aphrodite_tokens_saved_total[5m])` | `?query=rate(aphrodite_tokens_saved_total[5m])` |
| **Body bytes in** | `aphrodite_request_body_bytes_total` | `?query=aphrodite_request_body_bytes_total` |
| **Body bytes out** | `aphrodite_response_body_bytes_total` | `?query=aphrodite_response_body_bytes_total` |

## Cache

| What | PromQL | curl |
|------|--------|------|
| **LLM cache hits** | `aphrodite_cache_hits_total` | `?query=aphrodite_cache_hits_total` |
| **LLM cache misses** | `aphrodite_cache_misses_total` | `?query=aphrodite_cache_misses_total` |
| **Cache hit rate %** | `aphrodite_cache_hits_total / (aphrodite_cache_hits_total + aphrodite_cache_misses_total) * 100` | `?query=...` |
| **Inline CCR hits** | `aphrodite_inline_ccr_hits_total` | `?query=aphrodite_inline_ccr_hits_total` |
| **Inline CCR misses** | `aphrodite_inline_ccr_misses_total` | `?query=aphrodite_inline_ccr_misses_total` |

## Latency

| What | PromQL | curl |
|------|--------|------|
| **P50 latency** | `histogram_quantile(0.50, rate(aphrodite_latency_seconds_bucket[5m]))` | `?query=...` |
| **P95 latency** | `histogram_quantile(0.95, rate(aphrodite_latency_seconds_bucket[5m]))` | `?query=...` |
| **P99 latency** | `histogram_quantile(0.99, rate(aphrodite_latency_seconds_bucket[5m]))` | `?query=...` |
| **Avg latency** | `aphrodite_latency_seconds_sum / aphrodite_latency_seconds_count` | `?query=...` |
| **Upstream latency** | `aphrodite_upstream_latency_seconds_total` | `?query=aphrodite_upstream_latency_seconds_total` |

```bash
# P95 latency
curl -s "$PROM" --data-urlencode \
  'query=histogram_quantile(0.95, rate(aphrodite_latency_seconds_bucket[5m]))' | \
  python3 -c "import sys,json; d=json.load(sys.stdin); [print(f'P95: {float(r[\"value\"][1])*1000:.1f}ms') for r in d['data']['result']]"
```

## Errors & Reliability

| What | PromQL | curl |
|------|--------|------|
| **Upstream 4xx** | `aphrodite_upstream_errors_total{code="4xx"}` | `?query=...` |
| **Upstream 5xx** | `aphrodite_upstream_errors_total{code="5xx"}` | `?query=...` |
| **Upstream timeouts** | `aphrodite_upstream_timeouts_total` | `?query=aphrodite_upstream_timeouts_total` |
| **Error rate %** | `rate(aphrodite_upstream_errors_total{code="5xx"}[5m]) / rate(aphrodite_requests_total[5m]) * 100` | — |
| **Tool relay success** | `aphrodite_tool_relay_success_total` | `?query=aphrodite_tool_relay_success_total` |
| **Tool relay failure** | `aphrodite_tool_relay_failure_total` | `?query=aphrodite_tool_relay_failure_total` |
| **Notify success** | `aphrodite_notify_success_total` | `?query=aphrodite_notify_success_total` |
| **Notify failure** | `aphrodite_notify_failure_total` | `?query=aphrodite_notify_failure_total` |

## Dashboard Queries (Copy-Paste)

### Overview
```
# Requests by mode
aphrodite_requests_total

# Compression rate (/s)
rate(aphrodite_requests_compressed_total[1m])

# Tokens saved (/s)  
rate(aphrodite_tokens_saved_total[1m])

# Cache hit rate
aphrodite_cache_hits_total / (aphrodite_cache_hits_total + aphrodite_cache_misses_total)

# CCR hit rate
aphrodite_ccr_hits_total / (aphrodite_ccr_hits_total + aphrodite_ccr_misses_total + 1)
```

### Health
```
# Error rate (5xx %)
rate(aphrodite_upstream_errors_total{code="5xx"}[5m]) 
  / rate(aphrodite_requests_total[5m]) * 100

# P95 latency (ms)
histogram_quantile(0.95, rate(aphrodite_latency_seconds_bucket[5m])) * 1000

# Compression ratio EMA
aphrodite_compression_ratio_ema
```

### Full Snapshot (one-liner)
```bash
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query={__name__=~"aphrodite_.*"}' \
  | python3 -c "
import sys,json
d=json.load(sys.stdin)
for r in sorted(d['data']['result'], key=lambda r: r['metric']['__name__']):
    name = r['metric']['__name__']
    labels = ','.join(f'{k}={v}' for k,v in r['metric'].items() if k != '__name__')
    val = r['value'][1]
    print(f'{name:45s} {labels:20s} {val}')
"
```

## Prometheus UI

```
http://localhost:9090                    # Dashboard
http://localhost:9090/targets            # Scrape targets
http://localhost:9090/graph              # Query explorer
http://localhost:9090/alerts             # Alert rules
```

## Raw Proxy Metrics (no Prometheus)

```bash
# Full metrics dump
curl -s http://0.0.0.0:9798/metrics

# Just CCR
curl -s http://0.0.0.0:9798/metrics | grep ccr

# Just latency
curl -s http://0.0.0.0:9798/metrics | grep latency

# Stats JSON
curl -s http://0.0.0.0:9798/stats | python3 -m json.tool
```
