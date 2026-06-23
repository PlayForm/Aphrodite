# Aphrodite Tool Validation

## 2026-06-16 | v0.5.60 / v1.62.6

| Tool      | Endpoint         | Status | Notes                                  |
| --------- | ---------------- | ------ | -------------------------------------- |
| Health    | :9798/health     | ✅     | v0.5.60, token mode, CCR enabled       |
| Health    | :9797/health     | ✅     | v0.5.60, cache mode, CCR enabled       |
| Stats     | :9798/stats      | ✅     | In-memory counters                     |
| Stats/DB  | :9798/stats/db   | ✅     | 323 entries, 31MB→8KB, persistent      |
| Compress  | POST /ccr/create | ✅     | Hash returned, ratio varies by content |
| Retrieve  | POST /retrieve   | ✅     | Round-trip verified                    |
| Benchmark | benchmark.py     | ✅     | 19/19 pass, sub-ms latencies           |
| Metrics   | :9798/metrics    | ✅     | Prometheus format                      |
