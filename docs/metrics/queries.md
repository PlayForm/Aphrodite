# Metrics Queries

PromQL reference for monitoring and alerting on Aphrodite proxy metrics - see
[Prometheus](prometheus.md) for the full metric catalog these queries draw on.

## CCR Cache Performance

```
# CCR hit rate (%)
rate(aphrodite_ccr_hits[5m]) / (rate(aphrodite_ccr_hits[5m]) + rate(aphrodite_ccr_misses[5m])) * 100

# CCR miss rate
rate(aphrodite_ccr_misses[5m])

# CCR entries created per second
rate(aphrodite_ccr_created[5m])
```

## Latency

```
# P50 latency (seconds)  -  requires histogram_quantile
histogram_quantile(0.50, rate(aphrodite_latency_seconds_bucket[5m]))

# P95 latency
histogram_quantile(0.95, rate(aphrodite_latency_seconds_bucket[5m]))

# P99 latency
histogram_quantile(0.99, rate(aphrodite_latency_seconds_bucket[5m]))

# Average latency
rate(aphrodite_latency_seconds_sum[5m]) / rate(aphrodite_latency_seconds_count[5m])
```

## Compression Efficiency

```
# Compression ratio (EMA)
aphrodite_compression_ratio_ema

# Tokens saved per second
rate(aphrodite_tokens_saved[5m])

# Compression rate (% of requests compressed)
rate(aphrodite_requests_compressed[5m]) / rate(aphrodite_requests_total[5m]) * 100
```

## Error Rates

```
# Upstream 4xx rate
rate(aphrodite_upstream_errors_total{code="4xx"}[5m])

# Upstream 5xx rate
rate(aphrodite_upstream_errors_total{code="5xx"}[5m])

# Upstream timeout rate
rate(aphrodite_upstream_timeouts_total[5m])

# Total error rate
rate(aphrodite_upstream_errors_total{code="4xx"}[5m]) +
rate(aphrodite_upstream_errors_total{code="5xx"}[5m]) +
rate(aphrodite_upstream_timeouts_total[5m])
```

## Tool Relay

```
# Tool relay success rate
rate(aphrodite_tool_relay_success[5m]) / rate(aphrodite_tool_relay_calls[5m]) * 100

# Tool relay failure rate
rate(aphrodite_tool_relay_failure[5m])

# Tool relay total rate
rate(aphrodite_tool_relay_calls[5m])
```

## Cache Performance

```
# LLM response cache hit rate
rate(aphrodite_cache_hits[5m]) / (rate(aphrodite_cache_hits[5m]) + rate(aphrodite_cache_misses[5m])) * 100

# Cache hit rate (combined  -  CCR + LLM)
rate(aphrodite_ccr_hits[5m]) + rate(aphrodite_cache_hits[5m])
```

## Store Metrics

```
# CCR store entries (gauge)
aphrodite_ccr_store_entries

# CCR store bytes (gauge)
aphrodite_ccr_store_bytes

# Average bytes per entry
aphrodite_ccr_store_bytes / aphrodite_ccr_store_entries
```

## Throughput

```
# Requests per second
rate(aphrodite_requests_total[5m])

# Response bytes per second
rate(aphrodite_response_body_bytes_total[5m])
```

## Inline CCR

```
# Inline CCR hit rate
rate(aphrodite_inline_ccr_hits[5m]) / (rate(aphrodite_inline_ccr_hits[5m]) + rate(aphrodite_inline_ccr_misses[5m])) * 100
```

## Dashboard Panels

### Key Health Indicators

| Panel            | Query                                                                                                  |
| ---------------- | ------------------------------------------------------------------------------------------------------ |
| Requests/sec     | `rate(aphrodite_requests_total{mode="token"}[1m])`                                                     |
| P95 Latency      | `histogram_quantile(0.95, rate(aphrodite_latency_seconds_bucket[5m]))`                                 |
| Compression Rate | `rate(aphrodite_requests_compressed[5m]) / rate(aphrodite_requests_total[5m]) * 100`                   |
| CCR Hit Rate     | `rate(aphrodite_ccr_hits[5m]) / (rate(aphrodite_ccr_hits[5m]) + rate(aphrodite_ccr_misses[5m])) * 100` |
| Tokens Saved/sec | `rate(aphrodite_tokens_saved[5m])`                                                                     |
| Error Rate       | `rate(aphrodite_upstream_errors_total{code="5xx"}[5m]) + rate(aphrodite_upstream_timeouts_total[5m])`  |

### Alerts

```
# High upstream error rate
rate(aphrodite_upstream_errors_total{code="5xx"}[5m]) > 0.1

# High latency
histogram_quantile(0.95, rate(aphrodite_latency_seconds_bucket[5m])) > 5

# Low CCR hit rate
rate(aphrodite_ccr_hits[5m]) / (rate(aphrodite_ccr_hits[5m]) + rate(aphrodite_ccr_misses[5m])) < 0.5

# Tool relay failures
rate(aphrodite_tool_relay_failure[5m]) > 0
```

## Quick curl (no Prometheus server needed)

Every query above assumes a Prometheus server scraping `/metrics`. If you just
want a number right now, hit the proxy directly:

```bash
# Full metrics dump (Prometheus text format)
curl -s http://0.0.0.0:9798/metrics

# Just CCR-related lines
curl -s http://0.0.0.0:9798/metrics | grep ccr

# Just latency lines
curl -s http://0.0.0.0:9798/metrics | grep latency

# Stats as JSON
curl -s http://0.0.0.0:9798/stats | python3 -m json.tool
```

If you do have a Prometheus server, `PROM=http://localhost:9090/api/v1/query`
and `curl -s "$PROM" --data-urlencode 'query=...'` runs any query from this
page ad hoc, e.g.:

```bash
# CCR entries created, parsed out of the JSON response
curl -s "$PROM" --data-urlencode 'query=aphrodite_ccr_created' | python3 -c "
import sys,json; d=json.load(sys.stdin)
for r in d['data']['result']: print(f\"CCR created: {r['value'][1]}\")
"

# P95 latency in milliseconds
curl -s "$PROM" --data-urlencode \
	'query=histogram_quantile(0.95, rate(aphrodite_latency_seconds_bucket[5m]))' \
	| python3 -c "import sys,json; d=json.load(sys.stdin); [print(f'P95: {float(r[\"value\"][1])*1000:.1f}ms') for r in d['data']['result']]"

# Every aphrodite_* series in one sorted table
curl -s "$PROM" --data-urlencode 'query={__name__=~"aphrodite_.*"}' | python3 -c "
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
