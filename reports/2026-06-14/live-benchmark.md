# HermesCompress — Live Benchmark Report v3

**Date**: 2026-06-14
**Sessions**: 044944, 050301, 050631, 051102, 051905
**Model**: deepseek-v4-pro (1.6T MoE, 49B active/token, 1M context, 384K output)
**Config**: protect_recent=1, min_tokens=100, target_ratio=None

---

## Live Session Benchmarks

### Session 051102 (10 msg, 4 API calls)

```
Call  Turn  Messages  Tokens Saved  Savings  Latency
#1    1      2           694         10.7%    7,224ms (cold)
#2    2      5         9,147         57.5%       72ms
#3    3      7        12,349         63.9%       72ms
#4    4     10        15,166         67.5%       88ms
```

### Session 050631 (24 msg, 8 API calls)

```
Call  Messages  Tokens Saved  Savings  Latency
#1      2           694        10.7%    6,047ms
#2      4         2,091        10.5%      123ms
#3      7        12,579        60.2%      271ms
#4     11        11,968        58.7%       83ms
#5     14        11,280        57.1%       89ms
#6     17        11,276        55.1%      268ms
#7     21        12,403        55.6%      121ms
#8     24        14,734        59.8%      301ms
```

### Session 050301 (66 msg, 14 API calls)

```
Peak: 69.4% at 66 messages — 59,745 tokens saved
```

### Session 051905 (latest)

```
10.7% —   2 msgs,  5,798ms (cold HF download)
15.3% —   5 msgs,     42ms
56.1% —   6 msgs,     65ms
58.6% —   9 msgs,     78ms
61.4% —  12 msgs,    239ms
61.9% —  14 msgs,     85ms
62.3% —  16 msgs,     88ms
61.8% —  18 msgs,     81ms
```

**Every API call across all sessions was compressed. 0 missed calls.**

---

## Full Payload Benchmark (85 msg, standalone)

```bash
.venv/bin/python tests/benchmark_compare.py
```

### Direct (inline shim)

| Metric | Value |
|--------|-------|
| Messages | 85 → 85 |
| Chars | 207,647 → 153,145 |
| **Savings** | **26.3%** |
| Tool outputs | 30, 0 empty |
| CCR markers | 0 |
| Safety guard triggers | 0 |

Per-tool: JSON 31.1%, README dedup 29.8%, new content 0% (protected).

### Proxy (token mode :8788)

| Metric | Value |
|--------|-------|
| Prompt tokens | 148,745 |
| Cache hit | 0 tokens |
| Compression | **0%** |

The proxy does NOT compress Chat Completions (Hermes' format).

---

## Latency Profile

| Call | Latency | Notes |
|------|--------:|-------|
| 1st (cold) | 5-7s | Kompress ONNX model download from HuggingFace |
| 2nd (warm) | 40-120ms | Model loaded, first real pass |
| 3rd+ (hot) | 50-90ms | Cached compression path |
| Sub-100ms steady state | 60-300ms | Depends on message count |

---

## Safety Guard

The `_compress.py` safety guard reverts only empty tool outputs.
Across all benchmarks: **0 empty tool outputs detected**. 0 safety guard triggers.

---

## Comparison: Direct vs Proxy

| | Direct (shim) | Proxy (token :8788) |
|---|---|---|
| Token reduction | **26-67%** | 0% |
| Cache freezing | No | Yes |
| Setup | `pip install -e .` | `./scripts/proxy-start.py` |
| Overhead | 50-300ms/call | 0ms (no compression) |

**The inline shim is the ONLY path that reduces token count for Hermes traffic.**

---

## Sessions Verified

| Session | Date | Outcome |
|---------|------|---------|
| 044944 | 04:49 | 17-65% savings, bug 1+2 fixed |
| 045442 | 04:54 | Shim marker confirmed |
| 050301 | 05:03 | 10-69% savings, debug env added |
| 050631 | 05:06 | 50-59%, bug 6 fixed |
| 051102 | 05:11 | 57-67%, terminal sandbox bug discovered |
| 051905 | 05:19 | Both plugins active, terminal working |
