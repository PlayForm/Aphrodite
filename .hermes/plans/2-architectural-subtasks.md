# Aphrodite — Architectural Subtask Map
## Remaining Tasks with Concrete Implementation Plans
### Status: 19 releases (v0.5.0 → v0.5.19), ~40 tasks completed

---

## PHASE 3: CONTEXT TRACKER (headroom vendor) — 15 tasks, 3 subtask groups

### A) In-Process File Index (Tasks 36-39)
The plugin already tracks files via `_referenced_files`. Now build an in-memory project index.

- **36a**: Extend `_track_file_refs()` to record file size + hash from read_file results
- **36b**: Store parsed file metadata in `_file_index = {path: {hash, size, imports, functions, classes}}`
- **37a**: When read_file returns code, run regex extraction: `def (\w+)`, `class (\w+)`, `fn (\w+)`
- **37b**: Store `_symbol_index = {symbol_name: [file_path, ...]}` for cross-referencing
- **38a**: Build directory tree from `_referenced_files` keys — already done in pre_llm_hook
- **38b**: Add `aphrodite_tree` tool that returns the full project file tree as structured JSON
- **39a**: In pre_llm_hook, before building catalog, check if user message mentions a tracked file
- **39b**: If file referenced again, auto-inject `aphrodite_retrieve(hash)` suggestion

### B) Code-Aware Relevance (Tasks 40-44)
- **40a**: Add `_extract_code_symbols(text)` — regex for snake_case, CamelCase, kebab-case identifiers
- **40b**: Weight identifiers by position in message (first mention = highest relevance)
- **41a**: Path-aware split: `src/auth/middleware.py` → tokens `[src, auth, middleware, py]`
- **41b**: Match user query tokens against path tokens with substring scoring
- **42a**: Detect traceback patterns: `File "...", line N, in <function>`
- **42b**: When traceback found, auto-suggest retrieve of the line's file CCR entry
- **43a**: `test_foo.py` ↔ `foo.py` mapping via naming convention
- **43b**: When test file referenced, boost relevance of implementation file and vice versa
- **44a**: Parse Python `import X` and Rust `use X` from read_file results
- **44b**: When a module is referenced, boost relevance of all files importing it

### C) Performance (Tasks 45-50)
- **45a**: Replace `_referenced_files` dict with OrderedDict, add `last_access` timestamp
- **45b**: Eviction: remove smallest files first when at capacity (file-size-weighted LRU)
- **46a**: Dynamic relevance threshold: 0.3 for error context, 0.6 for prose
- **46b**: Expose threshold via `APHRODITE_RELEVANCE_THRESHOLD` env var
- **47a**: Pre-compute all relevance scores in one pass before building catalog
- **47b**: Cache relevance scores per message (skip recomputation for unchanged messages)
- **48a**: Session-scoped cache with TTL: `_session_cache = {session_id: {data, expires_at}}`
- **48b**: Clear cache on session reset (already done for _referenced_files, _conv_index)
- **49a**: Pre-compute BLAKE3 hash of file content on read, store in `_file_index`
- **49b**: O(1) lookup: `hash → file_path` for instant relevance matching
- **50a**: Remove `sample_content[:2000]` from CompressedContext struct — use metadata fields instead
- **50b**: Replace with structured: `{tool_name, tool_args_summary, result_preview_200, file_type}`

---

## PHASE 5: PROXY ARCHITECTURAL REWRITE — 25 tasks, 5 groups

### D) Single-Mode Simplification (Tasks 76-80)
**Architecture**: Collapse cache (:9797) + token (:9798) into single token-only proxy.
- **76a**: Remove cache mode from ProxyMode enum, keep only Token
- **76b**: In main.rs, only spawn one listener on :9798
- **76c**: Remove cache-specific code from compress_chat_completion (the preview-preserving branch)
- **76d**: Update aphrodite.toml to single proxy stanza
- **77a**: Remove /tool/relay endpoint — Python plugin handles tool dispatch directly
- **77b**: Remove ToolRelayRequest/Response types, execute_tool_relay function
- **77c**: Python plugin's _compress_handler already calls /ccr/create directly — no relay needed
- **78a**: Remove notify_url/notify_key from Cli, ProxyConfig, AppState
- **78b**: Remove notification callback spawn in handle_ccr_create
- **79**: Remove request_history ring buffer (already optional, just delete the field + handler)
- **80**: Done — single token mode is the only mode

### E) Streaming Compression (Tasks 94-96)
**Architecture**: Replace buffered compression with streaming progressive compression.
- **94a**: Change `response.bytes().await` to `response.chunk()` stream in proxy_handler
- **94b**: Accumulate chunks into a rolling buffer of N bytes
- **94c**: When buffer reaches compress_threshold, compress the accumulated content
- **94d**: Forward compressed marker immediately, start new buffer for remaining chunks
- **94e**: On stream end, compress final buffer if any content remains
- **95a**: For content < 100 bytes, compute hash but store in-memory HashMap instead of CCR store
- **95b**: `_inline_ccr: HashMap<String, String>` with 1000-entry LRU cap
- **95c**: retrieve handler checks inline_ccr first, then CCR store
- **96a**: Add `_ccr_write_queue: Vec<(hash, content)>` to AppState
- **96b**: Spawn background task that drains queue every 100ms or when 10 entries accumulated
- **96c**: Use `tokio::spawn` with a `tokio::sync::mpsc` channel for batched writes

### F) Compression Intelligence (Tasks 81-86)
**Architecture**: Content-type-aware compression with conversation memory.
- **81**: Done — `threshold_for()` with 13 content types already implemented
- **82**: Done — code types get ×4 multiplier
- **83**: Done — build_output/log get ÷2 multiplier
- **84**: Done — error gets ×8 multiplier
- **85a**: In detect_content_type, detect unified diff hunks: `@@ -N,M +N,M @@`
- **85b**: For diff content, compress only the context lines (keep @@ headers + changed lines)
- **86a**: Add `_conversation_topics: Vec<String>` tracking recent discussion themes
- **86b**: Extract topics via keyword extraction from user messages (top 10 nouns)
- **86c**: When topic shifts, lower thresholds temporarily (more aggressive compression of old topic)

### G) Stats & Monitoring (Tasks 87-91)
- **87**: Done — compressions_by_type in stats_json
- **88a**: Add `tokens_saved_realtime` that resets per session (separate from lifetime counter)
- **88b**: Expose via `/stats/realtime` endpoint
- **89**: Done — /health returns version + status + mode
- **90a**: Add `/metrics` endpoint returning Prometheus text format
- **90b**: Metrics: `aphrodite_requests_total`, `aphrodite_ccr_hits`, `aphrodite_tokens_saved`, `aphrodite_compression_ratio`
- **91a**: Track compression ratio EMA over time (already done via compression_ratio_ema)
- **91b**: If ratio drops below 1.5× for 10 consecutive compressions, log warning
- **91c**: Expose `compression_health: ok|degraded|warning` in /health

---

## PHASE 4: PLUGIN REMAINING — 5 tasks

### H) Hook + Tool Polish (Tasks 55-57, 63-64, 71, 73, 75)
- **55a**: Add `_git_diff_summary()` — runs `git diff --stat` via subprocess, caches for 30s
- **55b**: Inject in pre_llm_hook: `[GIT] 3 files changed, 45 insertions(+), 12 deletions(-)`
- **57a**: Register `pre_tool_call` hook that fires before read_file/write_file/patch
- **57b**: When tool_name is read_file, check if related files exist (same dir, imports)
- **57c**: Pre-cache those related files by reading them into `_file_dependency_cache`
- **63**: Done — path parameter on retrieve
- **64a**: Add `function=` filter to retrieve handler — filter returned content to function body
- **64b**: Implement with regex: extract lines between `def func_name(` and next `def ` or `class `
- **71a**: `aphrodite_benchmark` tool — compresses test payloads of each type, returns ratios
- **71b**: Benchmark results: `{code: 23.4x, json: 8.1x, log: 45.2x, error: 1.2x}`
- **73**: Done — debug decision logging in all hooks
- **75a**: `aphrodite_health` endpoint combining proxy stats + engine stats + inline store
- **75b**: JSON dashboard: `{proxy_alive, ccr_entries, tokens_saved, compression_ratio, session_turns}`

---

## EXECUTION ORDER (by impact)

1. **Phase 5-D**: Single-mode simplification (removes cache complexity, ~200 lines removed)
2. **Phase 5-E**: Streaming compression (architectural change, enables progressive forward)
3. **Phase 3-A**: In-process file index (enables smart pre-expansion)
4. **Phase 5-G**: Prometheus metrics (production monitoring)
5. **Phase 4-H**: Git diff + pre-cache hooks (coding workflow improvements)
6. **Phase 5-F**: Conversation memory (smarter thresholds)
7. **Phase 3-B/C**: Relevance scoring + performance (polish)

Total: ~50 subtasks across 7 execution groups.
