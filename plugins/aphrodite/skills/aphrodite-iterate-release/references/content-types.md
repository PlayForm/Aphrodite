# Content Type Taxonomy for Adaptive Compression

detect_content_type() in `crates/aphrodite/src/proxy.rs` identifies 13 categories.

## Detection Priority

1. **json**: starts with `{` or `[`, no tool output markers → JSON data
2. **tool_output**: JSON with `exit_code` or `"status"` fields → structured tool result
3. **error**: first line contains `error`/`Error`/`ERROR`/`Traceback`/`panic`/`thread '` → always preserved
4. **build_output**: first line starts with `Compiling `, `Finished`, `running `, `test ` → cargo/go/npm/test
5. **diff**: first line starts with `diff --git `, `@@ -`, `+++ `, `--- ` → git diffs
6. **git**: first line starts with `commit `, `On branch ` → git log output
7. **code_rust**: contains `fn ` + (`-> ` or `impl ` or `struct ` or `pub `) → Rust source
8. **code_python**: contains `def ` + (`import ` or `class ` or `from ` or `self.`) → Python source
9. **code_go**: contains (`func ` or `package `) + `import (` → Go source
10. **code_js**: contains (`function ` or `const ` or `=> `) + (`import ` or `export `) → JS/TS source
11. **code**: generic code signals (`fn `, `def `, `class `, `import `, `pub fn`) → unknown language
12. **log**: multiline (>5 lines) without code patterns → terminal/log output
13. **text**: everything else → plain text

## Adaptive Thresholds

`threshold_for(ct)` multiplies the base threshold per mode:

| Type | Multiplier | Token (1KB base) | Cache (8KB base) | Rationale |
|------|-----------|------------------|------------------|-----------|
| error | ×8 | 8KB | 64KB | Errors must always be visible |
| code_* / code | ×4 | 4KB | 32KB | Preserve function-level code |
| diff / git | ×2 | 2KB | 16KB | Moderate diff compression |
| tool_output / json | ×1 | 1KB | 8KB | Default |
| build_output / log | ÷2 | 512B | 4KB | Aggressive log compression |

## Build Output Smart Collapse (Python plugin)

In `_transform_terminal_hook`, build output (>20 lines, first line matches build patterns) gets:
- Consecutive duplicate lines collapsed to count
- Unique error/warning lines extracted into summary
- Full output stored in CCR, summary returned inline
- Pattern: `[build: N lines, M unique patterns] | errors: ... | warnings: ...`

## When to Add New Types

Add a new type when:
- A new content pattern appears frequently in tool output (>10% of sessions)
- The pattern would benefit from a different compression threshold
- The pattern has distinct structural characteristics

To add: extend `detect_content_type()`, add entry to `threshold_for()`, and add a new branch to `_transform_terminal_hook` if terminal-specific.
