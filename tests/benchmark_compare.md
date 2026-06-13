# Head-to-Head: Max vs Off vs Moderate

Run this prompt THREE times with different compression configs. Before each run,
set config via: `hermes config set compression.headroom.X Y`

## Config A: MAX (pr=1, tr=0.10, all features)

```bash
hermes config set compression.headroom.enabled true
hermes config set compression.headroom.protect_recent 1
hermes config set compression.headroom.target_ratio 0.10
hermes config set compression.headroom.precompress_tools true
hermes config set compression.headroom.aggressive_kompress true
hermes config set compression.headroom.deduplicate_results true
```

## Config B: OFF (no compression)

```bash
hermes config set compression.headroom.enabled false
```

## Config C: MODERATE (pr=5, tr=null, no extras)

```bash
hermes config set compression.headroom.enabled true
hermes config set compression.headroom.protect_recent 5
hermes config set compression.headroom.target_ratio null
hermes config set compression.headroom.precompress_tools false
hermes config set compression.headroom.aggressive_kompress false
hermes config set compression.headroom.deduplicate_results false
```

## Run (do this for EACH config in a separate session)

1. `headroom_stats` → baseline
2. Search `hermes_compress/` for `class `
3. Read `hermes_compress/_compress.py` (first 40 lines)
4. Run `wc -l hermes_compress/*.py`
5. `headroom_stats` → final

## Report

| Config   | Calls | Saved | Latency | Top Tool |
| -------- | ----- | ----- | ------- | -------- |
| MAX      | N     | N     | Nms     | ...      |
| OFF      | N     | 0     | 0ms     | ...      |
| MODERATE | N     | N     | Nms     | ...      |

**Winner**: config with highest tokens saved per call.
