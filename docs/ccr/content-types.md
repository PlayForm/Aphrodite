# Content Type Taxonomy

Every response payload is classified into a content type so compression
thresholds can adapt per-type - errors stay visible, while verbose logs get
compressed aggressively. Both the Rust proxy and the Python plugin implement
their own classifiers, each with a distinct type registry and detection order.

## Detection Order (Rust)

The Rust classifier returns exactly one type per invocation. Order matters -
first match wins:

```
1. JSON (starts with '{' or '[') → validate JSON
   1a. Contains "exit_code" or "status" → "tool_output"
   1b. Otherwise → "json"
2. Code (lines > 3)
   2a. Rust: fn/pub fn/async fn/impl/struct/enum + (-> or & or use) → "code_rust"
   2b. Python: def + (import/class/from/self.) → "code_python"
   2c. Go: (func/package) + import ( → "code_go"
   2d. JS/TS: (function/const/=> ) + (import/export) → "code_js"
   2e. Generic: fn/def/class/import/pub fn → "code"
3. Error: first_line contains error/Error/ERROR/Traceback/panic/thread ' → "error"
4. Build: first_line starts with Compiling/Finished/running/test → "build_output"
5. Linter: error[E/error:/warning[/warning:/mypy/clippy/eslint/tsc → "linter"
6. Diff: diff --git / @@ - / +++  / ---  → "diff"
7. Git: commit / On branch → "git"
8. Log: [INFO/WARN/ERROR/DEBUG/TRACE/FATAL/PANIC] or timestamp → "log"
9. Default → "text"
```

## Detection Order (Python)

The Python classifier examines the first 5,000 characters:

```
1. Diff: starts with "diff --git" or "---"
2. Rust build errors: "error[E" in trimmed
3. JSON: starts with '{' or '[' → parse:
   3a. total_count + query → "search_results"
   3b. session_id → "process_output"
   3c. exit_code or output → "terminal"
   3d. matches → "search_files"
   3e. Fallback keys → "json"
   3f. List → "json_list"
4. Terminal: exit code pattern in last 5 lines
5. Search: file:line: text pattern (>3 matches, >30% of lines)
6. Tabular: pipe-delimited (>3 pipes, >20% of lines)
7. Fallback → "text"
```

## Complete Type Registry

| Type             | Detection Pattern                 | Threshold Group     | Implementation    |
| ---------------- | --------------------------------- | ------------------- | ----------------- |
| `code_rust`      | Rust syntax                       | Code (×4)           | Rust proxy        |
| `code_python`    | Python syntax                     | Code (×4)           | Rust proxy        |
| `code_go`        | Go syntax with imports            | Code (×4)           | Rust proxy        |
| `code_js`        | JS/TS syntax with imports         | Code (×4)           | Rust proxy        |
| `code`           | Generic programming               | Code (×4)           | Rust proxy        |
| `error`          | First-line error keywords         | Error (×8)          | Rust proxy        |
| `diff`           | diff --git / unified diff headers | Diff (×2)           | Rust proxy        |
| `git`            | git commit/branch output          | Diff (×2)           | Rust proxy        |
| `text`           | Unrecognized content              | Text (×2)           | Rust proxy        |
| `tool_output`    | JSON + exit_code/status           | Default (×1)        | Rust proxy        |
| `json`           | Valid JSON (object/array)         | Default (×1)        | Rust proxy        |
| `build_output`   | cargo build/test output           | Noisy (÷2)          | Rust proxy        |
| `log`            | Structured log lines              | Noisy (÷2)          | Rust proxy        |
| `linter`         | Linter/compiler error output      | Noisy (÷2)          | Rust proxy        |
| `build_error`    | Rust error[E…]                    | n/a (Python only)   | Python plugin     |
| `search_results` | JSON + total_count                | n/a (Python only)   | Python plugin     |
| `process_output` | JSON + session_id                 | n/a (Python only)   | Python plugin     |
| `search_files`   | JSON + matches or file:line:      | n/a (Python only)   | Python plugin     |
| `tabular`        | Pipe-delimited rows               | n/a (Python only)   | Python plugin     |
| `json_list`      | JSON list (Python only)           | n/a (Python only)   | Python plugin     |
| `tool`           | Python tool result                | v (Python plugin)   | Python plugin     |
| `terminal`       | Python terminal output            | v (Python plugin)   | Python plugin     |
| `aphrodite`      | Aphrodite meta-tool output        | v (Python plugin)   | Python plugin     |
| `context`        | Context engine compression        | n/a (engine)         | Python plugin     |
| `build`          | Build output summary              | n/a (Python plugin) | Python plugin     |
| `compress`       | Programmatic CCR create           | n/a (Python plugin) | Python plugin     |

## Threshold Groups

### Noisy Types (÷2, excluded from auto-tune)

```
"linter", "build_output", "log"
```

Always at `base / 2`, regardless of auto-tune state.

### Error Types (×8)

```
"error"
```

Keeps errors visible - the highest threshold tier.

### Code Types (×4)

```
"code_rust", "code_python", "code_go", "code_js", "code"
```

Code should stay in context longer - developer may need to read it.

### Diff/Tracked Types (×2)

```
"diff", "git", "text"
```

Moderate compression - diffs and git output are moderately valuable.

### Default (×1)

```
"tool_output", "json", everything else
```

Standard compression threshold.

## Detection Examples

### Error (matched first)

```
error: could not compile `aphrodite`
```

→ type=error, threshold ×8 (kept in context up to 8× base threshold)

### Rust Code (matched after JSON fail, before error)

```
pub fn main() -> Result<()> {
    let app = AppState::new();
```

→ type=code_rust, threshold ×4

### Build Output (matched after error fail)

```
   Compiling aphrodite v0.5.69
   Compiling headroom-core v1.0.0
    Finished release [optimized] target(s) in 12.34s
```

→ type=build_output, threshold ÷2 (more aggressive compression)

### Linter Output

```
error[E0308]: mismatched types
  --> src/proxy.rs:841:5
```

→ type=linter (matches `error[E` before generic `error`), threshold ÷2

### Log Output

```
[2025-06-16T12:34:56Z INFO  aphrodite] proxy starting on :9798
```

→ type=log, threshold ÷2

## Python Classifier Examples

```python
# Diff
"diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs"
→ {"type": "diff", "ln": "3", "fn": "src/main.rs"}

# Rust build error
"error[E0308]: mismatched types\n --> src/proxy.rs:841:5"
→ {"type": "build_error", "ln": "2", "code": "E0308", "loc": "src/proxy.rs:841:5"}

# Search results
'{"total_count": 15, "query": "fn main", "matches": [...]}'
→ {"type": "search_results", "q": "fn main", "total": "15"}

# Process output
'{"session_id": "abc123", "pid": 12345, "uptime": 3600}'
→ {"type": "process_output", "pid": "12345", "uptime": "3600"}

# Text (fallback)
"Hello, world!"
→ {"type": "text", "ln": "1"}
```
