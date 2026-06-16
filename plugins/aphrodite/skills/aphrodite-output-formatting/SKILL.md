---
name: aphrodite-output-formatting
description: "LLM-native formatting rules for all aphrodite output — CCR previews, catalog, stats, diff, files. No emojis, no decorative separators, compact type-tagged format."
version: 1.0.0
platforms: [macos]
related_skills: [aphrodite-tool-guide, aphrodite-hook-reference]
---

# Aphrodite Output Formatting

All aphrodite output consumed by the LLM MUST use **LLM-native format** — compact, structured, type-tagged — not human-decorative format with emojis and fancy separators. The target audience is the LLM deciding whether to retrieve or act. Every token counts.

Load this skill whenever you work on aphrodite output formatting, CCR preview generation, or tool result display.

---

## CCR Preview Pipeline

Every compressed tool/terminal output carries a structured preview via `_make_ccr_preview()` in `_marker.py`. The classifier (`_classify_content()`) detects the content type, then the formatter produces a compact `[type:key=val ...]` preview:

| Content type | Preview | Meaning |
|-------------|---------|---------|
| git diff | `[diff:3f +12/-3 42L main.rs]` | 3 files, +12/-3, 42 lines |
| build output | `[build:0E 2W 142L]` | errors, warnings, lines |
| Rust error | `[error:E0308 src/main.rs:10 8L]` | error code + location |
| traceback | `[error:AttributeError 'NoneType' ...]` | exception type + message |
| terminal | `[terminal:cargo build exit=0]` | command + exit code |
| git commit | `[commit:afd634b release(aphrodite)]` | short hash + subject |
| search/grep | `[grep:25 matches 30L]` | match count |
| tabular data | `[table:12 rows 15L]` | row count |
| JSON object | `[json:total_items,by_type 30L]` | top-level keys |
| JSON list | `[json:42 items 10L]` | item count |
| process output | `[process:pid=12345 up=2h 10L]` | pid + uptime |
| plain text | `[text:first 110 chars...]` | fallback |

**Format rules:**
- `[type:key=val key=val]` bracket notation — consistent with CCR marker style
- Type tag first for immediate classification
- Space-separated key=value pairs
- Max 120 chars total, pipe-safe (no `|`)

**The absorptive pattern:** new content of the same type automatically gets the same treatment — no manual template writing needed. Add new content types by extending `_classify_content()` in `_marker.py`.

---

## Auto-Formatted Tool Outputs

`_format_aphrodite_output()` in `_hooks.py` intercepts these tools in the `_transform_tool_result` skip path and formats them as clean markdown:

### catalog
```
Catalog: 2 items 4.8KB saved 2 turns 0 files
Types: tool(2)

| Hash | Type | Size | Preview |
|------|------|------|---------|
| 7204304c | tool | 3KB | skill description... |
```

### stats
```
Aphrodite Stats

proxy:
  token: on 1 created 0 hits 37 tokens saved
  cache: on 0 created 0 hits

engine: on 550000 threshold 0 compressions 3/8 protect
inline: 0 entries 0B
```

### diff
```
Turn History: 2 turns

T2: summary of turn 2...
T1: summary of turn 1...
```

### files
```
Referenced Files: 3 files

read_file:
  /path/to/file1.py
  /path/to/file2.rs
```

---

## Anti-Patterns

**NEVER use these in LLM-facing output:**

| ❌ Don't | ✅ Do |
|----------|------|
| `📦` `💋` `📜` `📁` `🔨` `💥` `📝` `🔍` `📊` | No emojis — they waste tokens |
| `·` `—` as separators | Space or `\n` |
| `✅` `❌` for status | `on` / `off` |
| `**bold**` for decoration | Plain text labels |
| `•` bullet points | Indentation or `-` |
| `_italic hints_` | Inline parenthetical |

---

## Implementation Files

- **Classifier:** `plugins/aphrodite/_marker.py` → `_classify_content()` — detects 10+ content types
- **Preview formatter:** `plugins/aphrodite/_marker.py` → `_make_ccr_preview()` — generates `[type:...]`
- **Output formatter:** `plugins/aphrodite/_hooks.py` → `_format_aphrodite_output()` + `_fmt_{catalog,stats,diff,files}()`
- **Hook wiring:** `plugins/aphrodite/_hooks.py` → `_transform_tool_result()` skip path
- **Import chain:** `_hooks.py` imports `_make_ccr_preview` from `_marker.py`
