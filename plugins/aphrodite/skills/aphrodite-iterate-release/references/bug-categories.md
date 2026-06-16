# Common Aphrodite Bug Categories (Discovered Wave 1 + 2 Audits)

## Marker Consistency (Bug 19)
- Unicode vs ASCII marker format: `marker_for()` uses `⫷...⫸`, `smart_marker()` uses `<<<CCR:...>>>`
- Fix: standardize ALL paths to ASCII `<<<CCR:hash|type|size>>>`
- Check: Python regex, Rust marker_for, smart_marker, tool relay compress response

## Stats Counters Not Incremented (Bugs 7, 17, 22)
- `tokens_saved` in handle_ccr_create: never incremented
- `ccr_hits`/`ccr_misses` in compress_chat_completion: never checked
- `request_history`: never written
- Pattern: field exists in AppState + stats_json, but no path calls it
- Fix: add increment in every code path that produces the event

## Health/Status Misreporting (Bugs 3, 8, 20)
- JSON parse fragile: substring check `"status":"healthy"` vs serde's `"status": "healthy"`
- Upstream API call on every health check: burns rate limits
- 503 when CCR disabled: misleading, CCR is optional
- Fix: json.loads() for Python, always 200 with JSON body flags, decouple upstream check

## Tool Injection Wrong Array (Bug 18)
- inject_tool pushed tool definition into response `tool_calls` array
- tool_calls is for model-initiated calls, not tool definitions
- Fix: remove response injection entirely, plugin registers the tool

## Dead Code / Unused Imports (Bugs 9, 23)
- retry_with_backoff defined but never called (inline retry loop exists)
- marker_for imported but only ASCII format used
- no_ccr_inject_tool CLI flag + field dead after tool injection removed
- Fix: delete dead code, remove unused imports, strip CLI fields

## Borrow Checker Pattern (Rust)
- When adding state method calls after `*args = ...` mutation: `args_str` borrow from `args.as_str()` is still alive
- Pattern: `let owned = args_str.to_string();` then use `&owned` throughout
- Alternative: wrap computations in block `{ let ct = detect(); let compressed = marker(); (compressed, len) }` to drop borrows

## Version Sync Checklist
Must update ALL 6 locations in lockstep:
- plugins/aphrodite/__init__.py: BIN_VERSION
- plugins/aphrodite/__init__.py: PLUGIN_VERSION
- plugins/aphrodite/__init__.py: docstring version
- plugins/aphrodite/plugin.yaml: version
- plugins/aphrodite/plugin.yaml: install_message version
- crates/aphrodite/Cargo.toml: version
