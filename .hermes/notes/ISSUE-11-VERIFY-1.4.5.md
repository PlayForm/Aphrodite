# ISSUE-11 VERIFY - PlayForm/Aphrodite-Hermes#11 on 1.4.5

**Date:** 2026-09-17
**Tested artifact:** `target/release/libaphrodite_hermes.dylib`
**Version (from dylib):** `{"version":"1.4.5"}` via `aphrodite_hermes_version`
**Dylib sha256:** `2846b82163adfd25e22971c775fd6e4cadd0e337802e116f68589a081b3b8d0b`
**Dylib mtime:** Sep 17 17:01:49 2026 (newer than newest source edit in `crates/aphrodite-hermes/src`, 16:44 - current build, no rebuild needed)
**Branch:** Development

## Method

Direct dylib FFI via ctypes (authoritative path - same `aphrodite_compress` tool the plugin dispatches to):
`aphrodite_hermes_dispatch_tool("aphrodite_compress", {"content": ..., "type": "tool_result"})`, returning `{hash, type, size, preview, marker}`. Scratch script at `/tmp/issue11_repro.py` (not in repo).

## 1. Exact previews observed (all variants)

| Input                                                                                        | bytes | type        | preview                                                                  |
| -------------------------------------------------------------------------------------------- | ----- | ----------- | ------------------------------------------------------------------------ |
| Issue repro (indented JSON) `{"\n  "success": true,\n  "data": {"web": [{"title": "x"}]}\n}` | 58    | **text**    | **`[text:1L 2B \| ok]`** ← bug                                           |
| Issue repro (compact JSON) `{"success":true,"data":{"web":[{"title":"x"}]}}`                 | 47    | **text**    | **`[text:1L 2B \| ok]`** ← bug                                           |
| `{"success":true}` (1 key)                                                                   | 16    | **text**    | **`[text:1L 2B \| ok]`** ← bug                                           |
| `{"success":false,"data":{"web":[{"title":"x"}]}}`                                           | 48    | tool_result | `[tool_result:1L 48B \| {"success":false,...}]`                          |
| `{"success":true,"data":{"a":1},"meta":"m"}` (3 keys)                                        | 42    | tool_result | `[tool_result:1L 42B \| {"success":true,"data":{"a":1},"meta":"m"}]`     |
| JSON string field containing JSON (flat, 2 keys, no `success`)                               | 71    | tool_result | `[tool_result:1L 71B \| {"kind": "nested", "payload": "{\\"a\\": 1, ...` |
| Large JSON array (~18.9 KB, 200 objects)                                                     | 18931 | tool_result | `[tool_result:1L 18931B \| {"items": [{"id": 0, "name": "item-0", ...`   |
| `=== CRON LIST ===\njob1 line\njob2 line` (issue's non-JSON case)                            | 37    | tool_result | `[tool_result:3L 37B \| === CRON LIST ===]`                              |
| Plain text single line                                                                       | 32    | tool_result | `[tool_result:1L 32B \| hello world ...]`                                |

Confirms the issue's "indented or not both break": the indented AND compact forms of the exact repro both produce `[text:1L 2B | ok]`. Large JSON and nested-string JSON are NOT affected (only the `success:true, ≤2 keys` shape is).

## 2. Full returned dict for the JSON case (issue repro, indented)

```json
{
	"hash": "baa1c9bce421b9d78857bb142f8f700d6b38d5dd",
	"type": "text",
	"size": 58,
	"preview": "[text:1L 2B | ok]",
	"marker": "<<<CCR:baa1c9bce421b9d78857bb142f8f700d6b38d5dd|text|58>>>\n[text:1L 2B | ok]"
}
```

- **type = `text`** (bug: should be `tool_result` - the caller passed `"type": "tool_result"`)
- **preview = `[text:1L 2B | ok]`** (bug: previews literal `"ok"`, claims 2 bytes - the real content is 58B; `size` field is correct at 58 but the preview's byte count is the length of the mangled `"ok"` string)
- Storage unaffected: `size` field 58, retrieve round-trips original content.

## 3. Root cause (from source, crates/aphrodite-hermes/src/tools.rs lines 123-126)

`unwrap_hermes_result()` - the Hermes-envelope unwrapper that runs BEFORE real classification on any content starting with `{`:

```rust
if let Some(ok) = obj.get("success") {
    if ok.as_bool() == Some(true) && obj.len() <= 2 {
        return Some(("ok".to_string(), "text".to_string()));
    }
```

Any JSON object with 1-2 keys including `"success": true` is reclassified as `("ok", "text")` - regardless of what the other key is. The genuine envelope cases (`output`/`exit_code`, `diff`, `error`) are all handled by branches ABOVE this one (lines 81-122), so this branch only ever false-positives on arbitrary user tool results that happen to contain `success: true` plus one other key - exactly the issue's repro shape `{"success": true, "data": {...}}`. Boundary probes confirm it: 3-key objects and `success:false` objects pass through fine.

## 4. VERDICT: **NOT FIXED on 1.4.5** - issue still reproduces exactly

The issue's own repro (both indented and compact) still yields `[text:1L 2B | ok]` with `type='text'` on the current 1.4.5 release dylib. The JSON-preview bug is **still present above 1.3.7**; upgrading to 1.4.5 does NOT close the issue. It requires a code fix: the `success && obj.len() <= 2` branch in `unwrap_hermes_result` must require a known envelope key (or be removed, since `output`/`exit_code`, `diff`, `error`, `total_count`/`matches` are already handled above it) before reclassifying arbitrary JSON.

## Files

- `/tmp/issue11_repro.py` - scratch repro script (all variants + full dict dumps + version probe).
- This report: `.hermes/notes/ISSUE-11-VERIFY-1.4.5.md`.
- No repo source files modified; nothing committed.
