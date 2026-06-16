# Coding Deep-Tune Patterns

Patterns for optimizing aphrodite for coding-agent workflows. Apply when user says "more", "deeply fine tune", or "fix more".

## Content Type Detection

`detect_content_type()` in `crates/aphrodite/src/proxy.rs` identifies 13 types from first-line heuristics:

| Type | Detection | Threshold Multiplier |
|------|-----------|---------------------|
| `code_rust` | `fn ` + (`-> `, `impl `, `struct `, `pub `) | ×4 |
| `code_python` | `def ` + (`import `, `class `, `from `, `self.`) | ×4 |
| `code_go` | `func `/`package ` + `import (` | ×4 |
| `code_js` | `function `/`const `/`=>` + (`import `, `export `) | ×4 |
| `code` | Generic: `fn `, `def `, `class `, `import `, `pub fn` | ×4 |
| `error` | First line: `error`, `Traceback`, `panic`, `thread '` | ×8 |
| `diff` | `diff --git `, `@@ -`, `+++ `, `--- ` | ×2 |
| `git` | `commit `, `On branch ` | ×2 |
| `tool_output` | JSON with `exit_code` or `"status"` | ×1 |
| `json` | Starts with `{` or `[` | ×1 |
| `build_output` | `Compiling `, `Finished`, `running `, `test ` | ÷2 |
| `log` | >5 lines, multi-line, no code patterns | ÷2 |
| `text` | Everything else | ×1 |

## Adaptive Thresholds

`threshold_for(ct)` in `AppState` applies multipliers. The base threshold is auto-tuned by an EMA of compression ratios:
- Ratio > 20×: raise thresholds 2× (too aggressive — preserve more)
- Ratio < 3×: lower thresholds 0.5× (too conservative — compress more)
- Ratio 3-20×: neutral

## Build Output Collapse

Python `_transform_terminal_hook` detects build/test output (>20 lines) and collapses repeated lines:
- Deduplicates consecutive repeats, counts unique patterns
- Summarizes with `[build: N lines, M unique patterns] | errors: ... | warnings: ...`
- Full output stored in CCR, summary shown inline

## File Tracking + Tree Injection

- `_track_file_refs()` captures file paths from `read_file`, `write_file`, `patch`, `search_files` tool calls
- `_pre_llm_hook` injects file tree grouped by directory when >5 files referenced
- `aphrodite_files` tool exposes the tracking: count, by_tool grouping, all paths sorted
- Cleared on session reset

## Context Engine Editing Awareness

`AphroditeContextEngine.compress()`:
- Detects editing sessions by scanning last 10 messages for write/patch keywords
- During edits: `protect_last_n` bumped to 8 (keep more recent context)
- Tool-chain safety: backtracks to include assistant `tool_calls` owning orphan `tool_result` messages
- Progressive: first pass keeps more, subsequent passes can be more aggressive

## New Tool Addition Pattern

1. Define handler function + schema dict at module level
2. Register with `ctx.register_tool(name=..., schema=..., handler=..., toolset="aphrodite")` in `register()`
3. Add to `provides_tools` list in `plugin.yaml`
4. Add to `install_message` version string
5. Update docstring tool count in `__init__.py`
6. Clear any session state in `on_session_reset()`

## Debug Decision Logging

When `APHRODITE_DEBUG=1`, hooks log every decision:
- `transform_tool_result`: SKIP, BELOW, GUARD, CCR, INLINE, PASSTHROUGH, PROXY FAIL
- `transform_terminal_hook`: BELOW, GUARD, BUILD collapse, CCR, INLINE, PASSTHROUGH
- All decisions include tool_name, size, threshold, compression ratio where applicable

## Versioning

- `BIN_VERSION` in `__init__.py` must match `Cargo.toml` `version` (prefixed with `v`)
- `PLUGIN_VERSION` tracks API changes independently
- `plugin.yaml` version, description, install_message all in sync
- 6 locations to update per release (see `aphrodite-iterate-release` skill)
