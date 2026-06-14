# BENCHMARK.md - HermesCompress

## Quick Test (standalone)

```bash
.venv/bin/python tests/shim_hermes_compress.py --test
```

Output: 36 messages → 36 messages, 10 tool outputs, 0 CCR markers, ✓ ready.

## Live Hermes Session Benchmark

Start Hermes with debug and observe compression logs:

```bash
HERMES_COMPRESS_DEBUG=1 hermes
# ... use normally for 10+ turns ...
# In another terminal:
grep "hermes-compress: saved" ~/.hermes/logs/agent.log | tail -5
```

### Results (2026-06-14, pane 6 session)

```
10.7% -   2 msgs,  5,798ms  (cold: Kompress ONNX model download from HF)
15.3% -   5 msgs,     42ms
56.1% -   6 msgs,     65ms
58.6% -   9 msgs,     78ms
61.4% -  12 msgs,    239ms
61.9% -  14 msgs,     85ms
62.3% -  16 msgs,     88ms
61.8% -  18 msgs,     81ms
```

**Steady state: ~62% savings at 10+ messages with ~80ms overhead.**

Previous session (050301) hit 69.4% at 66 messages - 59,745 tokens saved.

## Full Payload Benchmark (85 messages, 5 tool types)

```bash
.venv/bin/python tests/benchmark_compare.py
```

### Direct (inline shim)

| Metric | Value |
|--------|-------|
| Messages | 85 → 85 (no truncation) |
| Chars | 207,647 → 153,145 |
| **Savings** | **26.3%** |
| Latency | 42,764ms (all 85 msgs in one pass) |
| Tool outputs | 30, 0 empty |
| CCR markers | 0 |

Per-type breakdown in `reports/2026-06-14/live-benchmark.md`.

### Proxy (token mode :8788)

| Metric | Value |
|--------|-------|
| Prompt tokens | 148,745 |
| Cache hit | 0 tokens (first request) |
| Compression | **0%** (Chat Completions pass through) |

**Key insight**: The proxy does NOT compress Chat Completions (Hermes' format).
Only the inline shim reduces actual token count.

## Comparison: Direct vs Proxy

| | Direct (shim) | Proxy (token :8788) |
|---|---|---|
| Token reduction | **26-62%** | 0% |
| Cache freezing | No | Yes (DeepSeek $0.0036/M) |
| Hermes transparent | Yes | Yes |
| Requires proxy process | No | Yes |
| Setup | `pip install -e .` | `./scripts/proxy-start.py` |

The shim wins on token savings. The proxy wins on cache-hit pricing.
They're complementary - use the shim for always-on compression, add the proxy
when you want prefix-cache cost savings.

## Tuning Sweep

```bash
.venv/bin/python tests/tune.py
```

Tests protect_recent and min_tokens parameter combinations.
Optimal config from sweep: `protect_recent=1, min_tokens=100, target_ratio=None`.
