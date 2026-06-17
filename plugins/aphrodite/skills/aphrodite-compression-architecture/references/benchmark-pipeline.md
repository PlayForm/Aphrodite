# Benchmark Pipeline

Combined performance + correctness pipeline for aphrodite compression. Run after any code change to verify proxy health, latency distribution, compression ratios, and regression status.

## Pipeline

```bash
# Step 1: HTTP performance benchmark
python3 scripts/benchmark.py

# Step 2: Correctness + feature toggles (from within Hermes)
# aphrodite_test mode=pipeline
```

Or from the same run:
```bash
python3 scripts/benchmark.py && echo "---PIPELINE---"
# Then run aphrodite_test(mode="pipeline") in Hermes
```

## Step 1: HTTP Benchmark (`scripts/benchmark.py`)

Direct HTTP calls against `:9798`. No Hermes plugin needed — just the proxy binary.

**Phases:**
1. **Proxy health + stats** — latency, mode, request/compression counts
2. **Compression across sizes × types** — 5 sizes (1KB, 10KB, 50KB, 100KB, 500KB) × 3 types (text, code, json) = 15 variants, 3–5 iterations each
3. **Retrieve** — 10 random hashes from phase 2
4. **Catalog** — entry count

**Output:**
- `benchmark-<ts>.json` — full run data with per-test latencies, ratios
- `benchmark-history.jsonl` — cumulative run history for trend comparison
- Prints Δ vs previous run (compress latency, retrieve latency)

**Metrics collected:**
- Compress: mean, p50, p95 latency per size/type
- Compress: ratio (original → hash)
- Retrieve: mean, p50, p95 latency
- Summary: compress avg (min/max), retrieve avg (min/max), ratio range

**Example output:**
```
APHRODITE BENCHMARK v2
Proxy: http://127.0.0.1:9798

-- Phase 1: Proxy --
  health    12.4ms  cache=? token=?
  stats      0.4ms  mode=token requests=457 compressed=229

-- Phase 2: Compression --
  compress/1KB/text     avg=0.2ms  ratio=25.6x
  compress/500KB/json   avg=2.0ms  ratio=12800.4x

-- Phase 3: Retrieve --
  retrieve x10          avg=1.4ms  p50=0.2ms  p95=11.9ms  found=10/10

-- Phase 4: Catalog --
  catalog               latency=0.2ms  entries=370

SUMMARY
  Results:      19/19 passed
  Compress avg: 0.9ms  min=0.2ms  max=2.7ms
  Retrieve avg: 1.4ms
  Ratio range:  26x - 12800x (median 1280x)
```

## Step 2: Smoke Test Pipeline (`aphrodite_test mode=pipeline`)

Runs inside Hermes with the plugin loaded. Tests the full compression pipeline end-to-end.

**Tests (9):**
- `compress_json`, `compress_code`, `compress_cache_hit` — compression + dedup
- `retrieve_roundtrip` — compress → retrieve → verify
- `stats` — proxy/cache health metrics
- `files_empty`, `diff_empty` — tool output format
- `proxy_health`, `proxy_metrics` — endpoint verification

**Feature toggles (4):**
- `debug_on` / `debug_off` — APHRODITE_DEBUG env
- `engine_on` / `engine_off` — APHRODITE_CONTEXT_ENGINE env

**Output:**
- `.hermes/aphrodite/.test-results.json` — full results with regression delta
- Regression status: OK (no degradation) or DEGRADED (fewer passes than previous)

## When to Run

- After any Rust proxy code change → run both steps
- After Python plugin change → step 2 only
- After threshold/tuning change → step 1 to verify latency impact
- Pre-release → both steps, verify no regression
