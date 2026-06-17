# CCR Marker Format

Origin: Compressed content must convey its hash, type, size, mode, preview, and
structured metadata to the receiver (LLM agent or retrieve endpoint) in a single
parseable string. The marker allows the receiver to decide whether to retrieve
the full content without having to decompress it first.

Source of truth: `plugins/aphrodite/_marker/marker.py:_ccr_marker()` (line 180),
`crates/aphrodite/src/proxy.rs:smart_marker()` (line 1342)

## Base Format

```
<<<CCR:HASH|TYPE|SIZE|MODE|preview=PREVIEW|KEY=VALUE|...>>>
```

## Field Definitions

| Field     | Required | Format                                                                     | Description                                                  |
| --------- | -------- | -------------------------------------------------------------------------- | ------------------------------------------------------------ |
| HASH      | Yes      | 24 lowercase hex chars (`[0-9a-f]{24}`) or `i:` prefix (6+ hex for inline) | Content-addressable hash of original content                 |
| TYPE      | Yes      | Enum string (see type enum below)                                          | Content classification for adaptive retrieval                |
| SIZE      | Yes      | Integer (bytes)                                                            | Original uncompressed content size                           |
| MODE      | No       | `token`, `cache`, `inline`, `engine`, `?`                                  | Proxy mode that stored this entry                            |
| preview   | No       | Pipe-safe string (max varies by headroom budget)                           | First ~120 bytes of content for LLM to decide retrieval      |
| KEY=VALUE | No       | Multiple pipe-delimited pairs                                              | Structured metadata (e.g., `lang=rs`, `fns=main`, `files=3`) |

## Hash Format

### Proxy hashes (token/cache mode)

- Algorithm: BLAKE3, first 24 hex chars (96 bits)
- Regex: `[0-9a-f]{24}`
- Source: `vendor/headroom/crates/headroom-core/src/ccr/mod.rs:compute_key()`
  (line 86)

### Inline hashes (Python plugin fallback)

- Algorithm: SHA-256, first 24 hex chars
- Regex: `[0-9a-f]{24}`
- Source: `plugins/aphrodite/_tools.py:_compress_handler()` (line 84)

### Short inlines (i: prefix)

- Prefix: `i:` followed by 6+ hex chars
- Validated by: `_marker/marker.py:_is_valid_ccr_hash()` via `_VALID_HASH_RE`
  regex `^(?:[0-9a-f]{24,}|i:[0-9a-f]{6,})$`

## Type Enum

All values extracted from `crates/aphrodite/src/proxy.rs:detect_content_type()`
(line 841) and `plugins/aphrodite/_marker/marker.py:_classify_content()` (line
53):

| Type             | Detection                                                                           | Threshold Multiplier |
| ---------------- | ----------------------------------------------------------------------------------- | -------------------- |
| `error`          | First line contains `error`, `Error`, `ERROR`, `Traceback`, `panic`, `thread '`     | ×8                   |
| `code_rust`      | `fn `, `pub fn `, `async fn `, `impl `, `struct `, `enum ` + `-> ` or `&` or `use ` | ×4                   |
| `code_python`    | `def ` + (`import `, `class `, `from `, `self.`)                                    | ×4                   |
| `code_go`        | (`func ` or `package `) + `import (`                                                | ×4                   |
| `code_js`        | (`function ` or `const ` or `=> `) + (`import ` or `export `)                       | ×4                   |
| `code`           | Generic: `fn `, `def `, `class `, `import `, `pub fn`                               | ×4                   |
| `diff`           | `diff --git `, `@@ -`, `+++ `, `--- `                                               | ×2                   |
| `git`            | `commit `, `On branch `                                                             | ×2                   |
| `text`           | Unrecognized content                                                                | ×2                   |
| `tool_output`    | Valid JSON + contains `exit_code` or `"status"`                                     | ×1                   |
| `json`           | Valid JSON (starts with `{` or `[`)                                                 | ×1                   |
| `build_output`   | `Compiling `, `Finished`, `running `, `test `                                       | ÷2 (half base)       |
| `log`            | `[INFO]`, `[WARN]`, `[ERROR]`, `[DEBUG]` patterns, timestamps                       | ÷2 (half base)       |
| `linter`         | `error[E`, `error: `, `warning[`, `warning: `, `mypy`, `clippy`, `eslint`           | ÷2 (half base)       |
| `context`        | Context engine compression                                                          | n/a (engine)         |
| `aphrodite`      | Aphrodite meta-tool outputs                                                         | Auto-expanded inline |
| `build`          | Terminal build output                                                               | Terminal threshold   |
| `terminal`       | Terminal command output                                                             | Terminal threshold   |
| `search_results` | JSON with `total_count` + `query`                                                   | ×1                   |
| `process_output` | JSON with `session_id`                                                              | ×1                   |
| `search_files`   | JSON with `matches`, or file:line: pattern                                          | ×1                   |
| `tabular`        | Pipe-delimited rows                                                                 | ×1                   |
| `compress`       | Programmatic CCR create                                                             | ×1                   |

## Mode Values

| Value    | Meaning                              | Store                                 |
| -------- | ------------------------------------ | ------------------------------------- |
| `token`  | Stored via token proxy (:9798)       | SQLite                                |
| `cache`  | Stored via cache proxy (:9797)       | In-memory                             |
| `inline` | Stored in Python plugin inline store | `_CappedStore` (OrderedDict, max 500) |
| `engine` | Created by ContextEngine             | CCR backend                           |
| `?`      | Unknown mode (parse fallback)        | -                                     |

## Preview Truncation by Headroom Budget

From `_marker/marker.py:_ccr_marker()` (line 201):

| Budget            | Preview Max              |
| ----------------- | ------------------------ |
| < 25              | 30 chars                 |
| < 50              | 60 chars                 |
| < 75              | 100 chars                |
| ≥ 75 or no budget | Unlimited (full preview) |

## Metadata Encoding Rules

From `_marker/marker.py:_ccr_marker()` (line 214) and
`proxy.rs:generate_metadata()` (line 978):

- Format: `KEY=VALUE|KEY=VALUE|...` (flat, pipe-delimited)
- Pipe `|` in values replaced with `/`
- Newlines replaced with space
- Control characters stripped
- Max 200 chars total (truncated: `...`)
- Common keys: `lang`, `fns`, `structs`, `classes`, `imports`, `files`, `adds`,
  `dels`, `trace`, `msg`, `ln`, `keys`, `entries`, `status`, `level`, `N_errors`

## Regex for Parsing

From `plugins/aphrodite/_core/config.py` (line 69):

```python
_CCR_RE = re.compile(r'(?:\[|<<<|⫷)CCR:([^|\\>⫸]+)(?:\|[^\\\]]*?)?(?:\]|>>>|⫸)')
```

Supports three delimiter styles:

- Standard: `<<<CCR:...>>>`
- Legacy bracket: `[CCR:...]`
- Unicode: `⫷CCR:...⫸`

## Markers in Rust Proxy

### Cache mode marker (proxy.rs:1406)

```
<<<CCR:HASH|TYPE|SIZE>>>
PREVIEW_TEXT
```

Preview is first 512 bytes of content appended after the marker on a new line.
No structured metadata.

### Token mode marker (proxy.rs:1408, smart_marker)

```
<<<CCR:HASH|TYPE|SIZE|METADATA>>>
```

Full structured metadata generated by `generate_metadata()` with type-specific
keys.

## Python Plugin Markers

### Tool output marker (hooks.py:353)

```
<<<CCR:HASH|tool|SIZE|token|preview=PREVIEW|lang=rs|fns=main|ln=42>>>
```

### Terminal output marker (hooks.py:1152)

```
<<<CCR:HASH|terminal|SIZE>>> PREVIEW…(use aphrodite_retrieve)
```

### Build output marker (hooks.py:1122)

```
<<<CCR:HASH|build|SIZE>>> SUMMARY…(use aphrodite_retrieve)
```

### Context engine marker (engine.py:232)

```
<<<CCR:HASH|context|SIZE|engine>>>
```
