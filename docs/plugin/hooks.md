# Plugin Hooks

Origin: The aphrodite Python plugin intercepts 5 Hermes lifecycle hooks to
compress tool and terminal output, inject catalog information before LLM calls,
and store conversation turns.

Source of truth: `plugins/aphrodite/plugin.yaml` (lines 7-12),
`plugins/aphrodite/_hooks/transform.py`, `plugins/aphrodite/_marker/marker.py`,
`plugins/aphrodite/_engine.py`

## Hook Registration

From `plugin.yaml`:

```yaml
provides_hooks:
    - on_session_start
    - transform_tool_result
    - pre_llm_call
    - transform_terminal_output
    - post_llm_call
```

## Lifecycle Order

```
Session Start
  │
  ├─ on_session_start → _inject_session_instruction (first pre_llm_call only)
  │
User Message
  │
  ├─ pre_llm_call → _pre_llm_hook
  │     ├─ Scan conversation for CCR markers
  │     ├─ Auto-expand small tool markers
  │     ├─ Compress old conversation turns to CCR
  │     ├─ Build catalog: markers, files, turn memory, engine stats, git
  │     └─ Inject as ephemeral system message
  │
Tool Call
  │
  ├─ transform_tool_result → _transform_tool_result
  │     ├─ Skip: essential tools, compress/retrieve/stats, existing CCR markers
  │     ├─ Skip: below threshold (< 1024 token, < 8192 cache)
  │     ├─ Skip: above MAX_REQUEST_BODY_SIZE (100MB)
  │     ├─ Classify: _classify_content → metadata
  │     ├─ Compress: proxy (:9798/:9797) → marker
  │     └─ Fallback: inline compression → marker
  │
Terminal Execution
  │
  ├─ transform_terminal_output → _transform_terminal_hook
  │     ├─ Skip: below TERMINAL_THRESHOLD (2048)
  │     ├─ Build detection: collapse repeated lines
  │     ├─ Compress via proxy or inline
  │     └─ Return marker with summary
  │
LLM Response
  │
  └─ post_llm_call → _store_conversation_turn
        ├─ Pack turn (user + assistant)
        ├─ Send to proxy /ccr/create
        └─ Store in _conv_index
```

## 1. on_session_start

Not directly implemented as a hook handler - instead,
`_inject_session_instruction()` fires on the _first_ `pre_llm_call` invocation
(guarded by `_session_instruction_injected` flag).

### Injected Message (Line 115)

```
[APHRODITE] v{PLUGIN_VERSION} active.
  Token proxy :9798 active | engine threshold={pct}% | tools auto-expand inline (<{AUTO_EXPAND_LIMIT})
  Use <<<CCR:hash|type|size>>> markers for compressed context.
  Call aphrodite_retrieve(hash) to fetch content, aphrodite_catalog to list available entries.
  ─ Layer 2: per-turn catalog injected below each turn ─
  ─ Layer 3: load aphrodite-tool-guide skill for full tool reference ─
```

Ephemeral: true - not persisted in conversation.

## 2. transform_tool_result

### Signature (Line 235)

```python
def _transform_tool_result(
    tool_name="", args=None, result="", tool_call_id="", task_id="",
    session_id="", turn_id="", api_request_id="", duration_ms=0,
    status="", error_type="", error_message="", **kwargs
):
```

### Skip Conditions

| Condition                      | Rationale                     |
| ------------------------------ | ----------------------------- |
| `_DEV` (passthrough mode)      | Dev mode disables compression |
| Empty/whitespace result        | Nothing to compress           |
| Tool in skip set               | Essential tools stay raw      |
| Result < threshold             | Below compression minimum     |
| Result > MAX_REQUEST_BODY_SIZE | Above 100MB guard             |
| Existing CCR marker in result  | Already compressed            |

### Skip Set (Lines 271-285)

**When token proxy alive:**

```python
_ESSENTIAL_TOOLS | {"aphrodite_retrieve", "aphrodite_compress", "aphrodite_stats"}
```

Where
`_ESSENTIAL_TOOLS = {"skill_view", "skills_list", "skill_manage", "memory", "session_search", "read_file", "read_terminal"}`

**When only cache proxy alive (or inline):**

```python
_ESSENTIAL_TOOLS | {"execute_code", "patch", "write_file", "search_files", "todo",
                    "aphrodite_retrieve", "aphrodite_compress", "aphrodite_stats"}
```

More tools pass through raw to reduce cache proxy load on frequent operations.

### Compression Threshold (Line 293)

```python
threshold = TOOL_THRESHOLD_TOKEN if token_alive else TOOL_THRESHOLD_CACHE if cache_alive else INLINE_THRESHOLD
```

| Config               | Default | Condition                 |
| -------------------- | ------- | ------------------------- |
| TOOL_THRESHOLD_TOKEN | 1,024   | Token proxy alive         |
| TOOL_THRESHOLD_CACHE | 8,192   | Only cache proxy alive    |
| INLINE_THRESHOLD     | 4,096   | No proxy, inline fallback |

### Compression Priority (Lines 328-388)

1. **Proxy compression**: token (:9798) preferred, cache (:9797) fallback
2. **Inline fallback**: Python-side inline store
3. **Passthrough**: if all fail, return raw

### Metadata Extraction

`_extract_tool_metadata(tool_name, args, result)` at line 148 extracts:

- **read_file**: `fn`, `ext`, `lines`, `names` (def/class/fn/struct/trait names)
- **search_files**: `q` (pattern), `files` (count)
- **terminal**: `exit` (exit code), `last` (last line)
- Returns `None` for other tools

### Marker Type

```python
marker_type = "aphrodite" if tool_name.startswith("aphrodite_") else "tool"
```

Aphrodite meta-tools get auto-expanded inline; regular tool results stay as
markers.

## 3. pre_llm_call

### Signature (Line 514)

```python
def _pre_llm_hook(conversation_history=None, user_message=None, **kwargs):
```

### Flow (Simplified)

1. **Refresh alive cache**: probe both proxy ports
2. **Headroom feedback**: query proxy fill_pct, set budget
3. **Inject session instruction** (first call only)
4. **Pass x-headroom-\* headers** to proxy
5. **Scan for CCR markers** (incremental - only new tool/system messages)
6. **Auto-expand small tool markers** (< AUTO_EXPAND_LIMIT, type=aphrodite) -
   replace in-line
7. **Compress old conversation turns** (turns > 6) → CCR
8. **Build catalog**:
    - AUTO line: build status, git status, proxy health
    - Debug banner (full/debug mode)
    - Git diff summary
    - Compression wrapping summary (by type)
    - Per-turn hint with counts
    - Engine stats
    - Turn archive link
    - Full CCR catalog (grouped by type)
    - Conversation memory (last 3 turns)
    - Referenced file tree
    - Context size warning (>100 msgs)
    - Read-intent detection
9. **Inject catalog as ephemeral system message**

### Catalog Modes

| Mode      | Output                                                                       |
| --------- | ---------------------------------------------------------------------------- |
| `full`    | Full catalog: all markers with previews, file tree, turn memory, debug info  |
| `compact` | By-type summary: `{N} items ({size} saved) - {n} [code_rust] {n} [error]...` |
| `tool`    | Minimal: `{N} items compressed`. Early-returns when no markers               |

### Read-Intent Detection (Line 944)

```python
_READ_KEYWORDS = {"read", "show", "view", "get", "cat", "display", "retrieve",
                   "fetch", "look", "see", "open", "inspect", "check", "print",
                   "dump", "output"}
```

If user message contains any keyword + markers exist → inject recent CCR hashes
for direct retrieval.

### Headroom Feedback Loop (Line 554)

```
pre_llm_hook → query proxy /stats → read fill_pct →
  set x-headroom-budget on outbound headers → proxy uses budget to adjust thresholds

Fill calculation (proxy.rs:319):
  ratio_ema = compression_ratio_ema
  pct = 100 - (ratio_ema / 20), clamped [1..99]

Budget mapping (proxy.rs:1358):
  < 25  → 0.25× (aggressive compression)
  < 50  → 0.50×
  < 75  → 0.75×
  ≥ 75  → 1.00× (default)
```

## 4. transform_terminal_output

### Signature (Line 1026)

```python
def _transform_terminal_hook(command="", output="", returncode=0, **kwargs):
```

### Threshold

```python
TERMINAL_THRESHOLD = 2_048  # from APHRODITE_TERMINAL_THRESHOLD env var
```

### Build Output Detection (Line 1061)

If first line starts with `Compiling`, `Finished`, `error:`, `warning:`,
`Running`, `PASSED`, `FAILED`, `test result:`:

- Collapse repeated consecutive lines
- Extract unique error/warning patterns
- Store full output in CCR
- Return summary: `<<<CCR:hash|build|size>>> [build: N lines, M unique patterns]
  | errors: ...

### Regular Terminal Output

Same compression priority as tool results: proxy → inline → passthrough.

### Marker Format

```
<<<CCR:hash|terminal|size>>> PREVIEW…(use aphrodite_retrieve)
```

## 5. post_llm_call

### Signature (Line 433)

```python
def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
```

### Flow

1. Check proxy alive (token preferred)
2. Pack turn: `{turn, user: last_user, assistant: capped_resp[:4096]}`
3. POST to proxy `/ccr/create`
4. Store in `_conv_index[tnum] = (hash, summary, size)`
5. Cap at 100 turns (LRU eviction)

### Turn Summary Format

```
T{tnum}: {user_msg_first_chars}… → {assistant_response_first_200_chars} [{file_tags}]
```

## Compression Thresholds Per Hook

| Hook                        | Threshold Config                            | Default     | Bypass                                             |
| --------------------------- | ------------------------------------------- | ----------- | -------------------------------------------------- |
| transform_tool_result       | TOOL_THRESHOLD_TOKEN / TOOL_THRESHOLD_CACHE | 1024 / 8192 | \_DEV, skip set, < threshold, >100MB, existing CCR |
| transform_terminal_output   | TERMINAL_THRESHOLD                          | 2048        | \_DEV, < threshold, existing CCR                   |
| pre_llm_call (turn archive) | ctx_len > 30, turns > 6, packed > 500B      | -           | No proxy                                           |
| pre_llm_call (catalog)      | Always (if markers/files/engine)            | -           | quiet_mode=1                                       |

## Dev Mode

Set `APHRODITE_PASSTHROUGH=1` or `HERMES_DEV=1`:

- `_transform_tool_result`: returns result unchanged
- `_transform_terminal_hook`: returns output unchanged
- `_pre_llm_hook`: returns early
- `_store_conversation_turn`: returns early
