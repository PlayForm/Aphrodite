# Tool-Type Compression Comparison

Test each tool type separately. Start fresh, run headroom_stats after each.

## JSON (search_files)

1. `headroom_stats`
2. Search `hermes_compress/` for `def `
3. `headroom_stats` → note json savings

## Code (read_file)

4. Read `hermes_compress/_compress.py` (first 50 lines)
5. `headroom_stats` → note code savings

## Mixed (terminal)

6. Run `find hermes_compress/ -name "*.py" | xargs wc -l | sort -rn | head -10`
7. `headroom_stats` → note mixed savings

## Prose (web_extract or skill_view)

8. Load skill `hermes-compress`
9. `headroom_stats` → note prose savings

## Report

| Tool Type | Outputs | Tokens Saved | Avg/Output | Latency |
| --------- | ------- | ------------ | ---------- | ------- |
| JSON      | N       | N            | N          | Nms     |
| Code      | N       | N            | N          | Nms     |
| Mixed     | N       | N            | N          | Nms     |
| Prose     | N       | N            | N          | Nms     |

**Total**: N calls, N tokens saved, Nms
