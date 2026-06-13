# Compression Benchmark — Full Pipeline

Run these steps sequentially. After each step, report `headroom_stats`.

## Steps

1. `headroom_stats` — record baseline: calls, saved, by_tool
2. Search `hermes_compress/` for `def ` → triggers JSON compression
3. `headroom_stats` — record delta
4. Read `hermes_compress/_compress.py` (first 30 lines) → triggers code compression
5. `headroom_stats` — record delta
6. Run `wc -l hermes_compress/*.py` → triggers terminal compression
7. `headroom_stats` — record delta
8. Read `hermes_compress/__init__.py` (first 30 lines) → another code call
9. `headroom_stats` — record delta

## Report

Output a table:

| Step | Calls | Saved | by_tool | Latency |
|------|-------|-------|---------|---------|
| baseline | N | N | ... | Nms |
| after search | N | N | ... | Nms |
| after read | N | N | ... | Nms |
| after wc | N | N | ... | Nms |
| after read2 | N | N | ... | Nms |

Then: total tokens saved, average per call, top tool type.
