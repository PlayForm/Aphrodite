# CCR Marker Format

CCR markers are compact strings that stand in for compressed content in an
agent's context. Each marker carries the content's hash, type, and size plus
a human-readable preview, so the reader can decide whether to retrieve the
full content without expanding anything first.

## Wire Format

The canonical marker line is the stable contract shared by every pipeline:

```
<<<CCR:hash|type|size>>>
```

| Field | Meaning                                                          |
| ----- | ---------------------------------------------------------------- |
| hash  | BLAKE3 digest of the content, first 40 hex characters (160 bits) |
| type  | Detected content type (see Content Types below)                  |
| size  | Original content size in bytes                                   |

Hashes are lowercase 40-character hex digests computed by
`headroom_core::ccr::compute_key` (BLAKE3, truncated to 160 bits). Inline-only
entries use an `i:` prefix followed by at least 6 hex characters. Every
retrieval entry point tolerates a hash argument that carries a trailing
`|type|size` suffix or surrounding whitespace - an agent sometimes echoes a
whole marker back - and normalizes it before an exact-match lookup. There is
no prefix or fuzzy matching: the full hash must be supplied.

## Marker Layouts

Two pipelines render markers. They share the same vocabulary but differ in
layout: the proxy pipeline leads with the preview, the hook/FFI pipeline
leads with the marker line.

### Proxy Pipeline

Built by `format_ccr_output` (`proxy` module):

```
PREVIEW
[TYPE: METADATA;center=CENTER]
<<<CCR:HASH|TYPE|SIZE>>>
```

- `PREVIEW` is a plain excerpt in cache mode (512 chars) or a type-aware
  summary in token mode.
- `METADATA` is `key=value` pairs joined with `;`, capped at 400 chars with
  char-safe truncation. A `;center=CENTER` segment appears only when a caller
  supplies a center annotation.
- The marker line is always rendered complete - no truncation budget may cut
  into it.

### Hook / FFI Pipeline

Built by `marker::render_marker` (`marker` module):

```
<<<CCR:HASH|TYPE|SIZE>>>
[CENTER:PREVIEW]
[meta:META]
```

- `CENTER` defaults to the content type when no explicit annotation is given.
- The preview line is emitted verbatim when it is already a self-describing
  `[label:...]` preview; bare excerpts are wrapped once as `[CENTER:preview]`.
- The `[meta:META]` line appears only when metadata is non-empty. Metadata is
  `;`-joined `key=value` pairs capped at 300 chars (char-safe, `...` appended
  on truncation).

## Hash Extraction

`marker::extract_hashes` recognizes three delimiter families, opened and
closed independently:

| Style    | Form                          |
| -------- | ----------------------------- |
| Standard | `<<<CCR:...>>>`               |
| Bracket  | `[CCR:...]`                   |
| Unicode  | `⧗CCR:...⧘` (U+2AF7 / U+2AF8) |

The hash capture is anchored to hex characters (plus the `i:` prefix) and
excludes newlines, so an unterminated marker can never capture downstream
text - including other markers - as a bogus multi-line hash. Recursive marker
expansion uses a separate ASCII-only scanner restricted to `<<<CCR:...>>>`
forms.

## Content Types

Three classifiers feed the `type` field.

### Proxy Classifier

First-match-wins over the content (`proxy::proxy_detect_content_type`):

| Type           | Detection                                                                                                       |
| -------------- | --------------------------------------------------------------------------------------------------------------- |
| `tool_output`  | Valid JSON starting with `{` or `[` and containing `exit_code` or `"status"`                                    |
| `json`         | Valid JSON starting with `{` or `[`                                                                             |
| `code_rust`    | A line starts with `fn `/`pub fn `/`async fn `/`impl `/`struct `/`enum ` AND content has `-> `, `&`, or `use `  |
| `code_python`  | Contains `def ` AND one of `import `/`class `/`from `/`self.`                                                   |
| `code_go`      | Contains (`func ` or `package `) AND `import (`                                                                 |
| `code_js`      | Contains (`function ` or `const ` or `=> `) AND (`import ` or `export `)                                        |
| `code`         | Fallback: contains `fn `, `def `, `class `, `import `, or `pub fn`                                              |
| `error`        | First line contains `error`/`Error`/`ERROR`/`Traceback`/`panic` or starts with `thread '`                       |
| `build_output` | First line starts with `Compiling `/`  Compiling`, contains `Finished`, or starts with `running `/`test `       |
| `linter`       | First line starts with `error[E`/`error: `/`warning[`/`warning: `, contains `                                   | `+ error/warning, or mentions`mypy`/`clippy`/`eslint`/`tsc ` |
| `diff`         | First line starts with `diff --git `/`@@ -`/`+++ `/`--- `                                                       |
| `git`          | First line starts with `commit ` or `On branch `                                                                |
| `log`          | A line has a bracketed `INFO`/`WARN`/`ERROR`/`DEBUG`/`TRACE`/`FATAL`/`PANIC` marker, or a timestamp-shaped line |
| `text`         | Fallback: none of the above matched                                                                             |

Code detection only runs on content with more than 3 lines. Invalid JSON
starting with `{` or `[` is treated as `text`.

### Semantic Detector

`preview::detect_semantic_type` upgrades generic buckets
(`text`/`terminal`/`log`) to a high-signal shape using conservative
line-prefix and majority-vote rules: `json`, `test`, `diff`, `code`, `table`,
`markdown`, `yaml`, `html`, `xml`, `csv`, `build`, `git`, `ls`, `grep`,
`gitlog`. Hermes wrapper envelopes (objects carrying `output`/`exit_code`/
`error`/`success`/`total_count`/`content`/`result`/`found`/`preview`-style
keys) are deliberately excluded so their raw-JSON previews survive.

### Headroom Classifier

The hook/FFI pipeline classifies via `headroom_core::transforms`, which
yields exactly one of: `json_array`, `source_code`, `search`, `build`,
`diff`, `html`, `text`. These strings are what appear as the `type` field in
hook-pipeline markers.

## Previews

Previews are self-describing `[type:...]` strings built by
`preview::build_preview`, shared by the proxy and hook paths. They carry
decision-relevant facts rather than generic counts: error and warning lines
are tallied line-by-line (never substring matches), the first real error
message is surfaced, diffs name the changed files, test output shows
pass/fail/ignored tallies, and JSON previews show item or key counts.

### Preview Length Cap

The rendered preview is capped in characters when configured:

| Source                              | Behavior                    |
| ----------------------------------- | --------------------------- |
| `APHRODITE_PREVIEW_MAX_CHARS` env   | Highest precedence          |
| TOML `[previews] preview_max_chars` | Shipped default 120         |
| Key absent                          | Unlimited (legacy behavior) |

Truncation is char-safe and preserves the closing `]` of a self-describing
preview so downstream parsing keeps working.

### Headroom-Budget Truncation

The hook pipeline's `headroom_budget` parameter, when supplied, truncates the
preview further:

| Budget        | Preview max      |
| ------------- | ---------------- |
| < 25          | 30 chars         |
| < 50          | 60 chars         |
| < 75          | 100 chars        |
| >= 75 or none | Left as supplied |

### Sanitization

Both pipelines sanitize preview and metadata text: newlines become spaces,
control characters are stripped, and `|` in metadata values is replaced with
`/`.

## Metadata Encoding Rules

| Rule                     | Proxy pipeline                              | Hook / FFI pipeline       |
| ------------------------ | ------------------------------------------- | ------------------------- |
| Format                   | `KEY=VALUE;KEY=VALUE` (flat, `;`-delimited) | same                      |
| `                        | ` in values                                 | replaced with `/`         | replaced with `/` |
| Newlines / control chars | stripped                                    | stripped                  |
| Total cap                | 400 chars                                   | 300 chars                 |
| Truncation               | char-safe                                   | char-safe, `...` appended |

Common keys: `lang`, `fns`, `structs`, `traits`, `impls`, `classes`,
`types`, `path` (prefetch), `center`.
