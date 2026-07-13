# CCR Marker Format

A CCR marker is a compact string that replaces compressed content in an
agent's context, conveying hash, type, size, and a human-readable preview to
whoever receives it - an LLM agent or the `/retrieve` endpoint. This lets the
receiver decide whether to retrieve the full content without having to
decompress anything first.

There are **two real marker layouts** produced by this codebase, one per
compression pipeline (see `docs/ccr/lifecycle.md` for how content reaches
each one). There is no `MODE` field and no single-line
`<<<CCR:HASH|TYPE|SIZE|MODE|preview=...|KEY=VALUE|...>>>` format - earlier
drafts of this doc described a format no Rust code ever emits.

## Hash Format

Every hash is a full **40-character lowercase hex BLAKE3 digest**
(`headroom_core::ccr::compute_key`, vendored in `vendor/headroom`), except
inline-only hashes which use an `i:` prefix followed by 6+ hex characters
(`marker::is_valid_ccr_hash` accepts `[0-9a-f]{24,}` as its lower validity
floor, but every hash actually produced by this codebase is 40 chars).

| Hash kind          | Algorithm             | Format               |
| ------------------- | ---------------------- | --------------------- |
| Standard content hash | BLAKE3, 40 hex chars  | `[0-9a-f]{40}`        |
| Inline-only hash     | `i:` prefix + 6+ hex   | `i:[0-9a-f]{6,}`      |

Every retrieval entry point (`retrieve.rs::handle_retrieve`,
`proxy.rs::execute_tool_relay`'s `"aphrodite_retrieve"` arm, and
`resolve::resolve_one`, via the shared `marker::normalize_hash` helper)
tolerates a hash argument with a trailing `|type|size` marker-body suffix
(as an LLM sometimes echoes a whole marker back) and surrounding whitespace,
stripping both before doing an exact-match lookup. There is no prefix/fuzzy
matching - the full hash (not a truncated display form) must be supplied.
Machine-consumed JSON (catalog, search, prefetch results) always emits the
full 40-char hash for this reason; only the human-readable markdown table
(`catalog::format_catalog_table`) truncates it for display.

## Marker Layout: Proxy Pipeline (`proxy.rs`)

Built by `format_ccr_output` (`crates/aphrodite/src/proxy.rs`):

```
PREVIEW
[TYPE: METADATA;center=CENTER]
<<<CCR:HASH|TYPE|SIZE>>>
```

- `PREVIEW` - `build_preview` (proxy.rs), char-safe truncated to 512 chars in
  cache mode, or a language/type-aware summary in token mode.
- `METADATA` - `generate_metadata` (proxy.rs): type-specific `key=value`
  pairs (`lang=rs`, `fns=main,helper`, `structs=Foo`, etc.), joined with `;`,
  hard-capped at **400 chars** (char-safe truncation, never mid-codepoint).
- `;center=CENTER` - only present when a caller supplies a center
  annotation; omitted otherwise.
- The final line is always a complete, unsliced `<<<CCR:HASH|TYPE|SIZE>>>` -
  no truncation budget is ever allowed to cut into this line (see
  `regression_13_marker_terminator_never_truncated` in proxy.rs's tests).

Cache mode (`ProxyMode::Cache`) uses the same `format_ccr_output` template but
with an empty metadata string and a plain 512-char preview (`cache_marker`);
token mode (`ProxyMode::Token`) additionally calls `generate_metadata`
(`smart_marker`).

## Marker Layout: Hook/Dylib Pipeline (`marker.rs`)

Built by `marker::render_marker` (via `marker::ccr_marker`,
`crates/aphrodite/src/marker.rs`):

```
<<<CCR:HASH|TYPE|SIZE>>>
[CENTER:PREVIEW]
[meta:META]
```

- The marker line comes first here (opposite order from the proxy layout
  above) - the two pipelines share a vocabulary, not a byte-for-byte format.
- `CENTER` defaults to `TYPE` when no explicit center annotation is given
  (`center.unwrap_or(ccr_type)`).
- `PREVIEW` is sanitized (`|` → `-`, newlines → space, control chars
  stripped) and, when a `headroom_budget` is supplied, truncated to 30/60/100
  chars for budgets `<25`/`<50`/`<75` respectively (see "Preview Truncation"
  below); otherwise left at whatever the caller-supplied preview already is.
- The optional `\n[meta:META]` line is only emitted when metadata is
  non-empty; `META` is `;`-joined `key=value` pairs, capped at 300 chars
  (char-safe truncation via `struct_extract::floor_boundary`, `...`
  appended).
- Note: when the caller's `preview` argument already carries its own
  `[type:...]`-shaped prefix (as built by `crate::build_preview`), the
  rendered marker visibly doubles the type tag (e.g.
  `[json_array:[json:412items 1L]]`). This is flagged as possibly-intentional
  in `.plans/05-compression-pipeline.md` and is a deliberate open question
  for the user, not a bug fixed by this doc pass.

## Type Enum

Values are produced by two independent classifiers that do NOT share a
vocabulary: the proxy's hand-rolled `detect_content_type`
(`crates/aphrodite/src/proxy.rs`) and the Headroom-boundary
`transforms::detect` consumed by the hook pipeline
(`headroom_core::transforms`, which yields exactly
`json_array|source_code|search|build|diff|html|text`).

### Proxy classifier (`detect_content_type`, first-match-wins)

| Type           | Detection                                                                                | Threshold                        |
| -------------- | ----------------------------------------------------------------------------------------- | --------------------------------- |
| `tool_output`  | Valid JSON + contains `exit_code` or `"status"`                                          | base                               |
| `json`         | Valid JSON (starts with `{` or `[`)                                                      | base                               |
| `code_rust`    | A line starts with `fn `/`pub fn `/`async fn `/`pub async fn `/`impl `/`struct `/`enum ` (or `pub` variants), AND the content contains `-> `, `&`, or `use ` | ×code multiplier (config, default 4) |
| `code_python`  | Contains `def ` AND one of `import `/`class `/`from `/`self.`                            | ×code multiplier                   |
| `code_go`      | Contains (`func ` or `package `) AND `import (`                                          | ×code multiplier                   |
| `code_js`      | Contains (`function ` or `const ` or `=> `) AND (`import ` or `export `)                 | ×code multiplier                   |
| `code`         | Generic fallback: contains `fn `, `def `, `class `, `import `, or `pub fn`                | ×code multiplier                   |
| `error`        | First line contains `error`/`Error`/`ERROR`/`Traceback`/`panic`, or starts with `thread '` | ×8                                 |
| `build_output` | First line starts with `Compiling `/`   Compiling `, contains `Finished`, or starts with `running `/`test ` | base (NOT halved - see below)      |
| `linter`       | First line starts with `error[E`/`error: `/`warning[`/`warning: `, or contains `\|` + error/warning, or mentions `mypy`/`clippy`/`eslint`/`tsc ` | base (NOT halved)                  |
| `diff`         | (see proxy.rs `detect_content_type` for the full diff/git branch)                        | ×2                                 |
| `text`         | Fallback: none of the above matched                                                       | ×2                                 |

**Important correction:** `linter`, `build_output`, and `log` are explicitly
pinned at the BASE threshold (`threshold_for` returns `base` for these three
types before any multiplier or auto-tune is applied) - they are NOT halved
("÷2"). An earlier draft of this doc claimed a ÷2 discount for noisy types;
the actual code keeps them at base specifically because "coding sessions need
build output visible" (see the comment at `proxy.rs::threshold_for`).

Thresholds are further scaled by an auto-tune multiplier (0.5×-2× based on
the exponential moving average of the historical compression ratio) and,
per-request, by a `headroom_budget` multiplier - see "Budget Curve" below.

### Hook/dylib classifier (Headroom boundary)

`headroom_core::transforms::detect` returns one of exactly:
`json_array`, `source_code`, `search`, `build`, `diff`, `html`, `text`.
These strings are what actually appear as the `TYPE` field in hook-pipeline
markers - NOT the proxy's `code_rust`/`code_python`/etc. vocabulary.

## Budget Curve

`compress_chat_completion` (`proxy.rs`) scales the effective compression
threshold by a `budget_mult` derived from an optional `headroom_budget`
request header:

```
budget_mult = clamp(0.50 + (headroom_budget% / 100) * 0.50, 0.50, 1.0)
```

This is a smooth linear interpolation from **0.50× at 0% fill to 1.0× at
100% fill**, never dropping below 0.5× (semantics and tool chains are worth
the tokens even under heavy budget pressure). An earlier draft of this doc
described a discrete 0.25/0.50/0.75 three-step curve; the real code has no
such steps.

## Preview Truncation by Headroom Budget (hook/dylib pipeline only)

`marker::ccr_marker`'s `headroom_budget` parameter, when supplied, truncates
the preview:

| Budget            | Preview Max              |
| ----------------- | ------------------------- |
| < 25              | 30 chars                  |
| < 50              | 60 chars                  |
| < 75              | 100 chars                 |
| ≥ 75 or no budget | Left as supplied (no cap applied here) |

This table applies to `marker::ccr_marker` specifically; the proxy pipeline's
`generate_metadata` has its own independent 400-char cap (see above), and
`build_preview` (proxy.rs) previews are capped at 512 chars in cache mode.

## Metadata Encoding Rules

Both pipelines follow the same sanitization rules, with different length
caps:

- Format: `KEY=VALUE;KEY=VALUE;...` (flat, `;`-delimited - NOT the
  pipe-delimited `KEY=VALUE|KEY=VALUE` an earlier draft of this doc claimed)
- Pipe `|` in values replaced with `/`
- Newlines replaced with space
- Control characters stripped
- Proxy pipeline (`generate_metadata`): max 400 chars total, char-safe
  truncation (never mid-codepoint)
- Hook/dylib pipeline (`marker::ccr_marker`): max 300 chars total, char-safe
  truncation
- Common keys: `lang`, `fns`, `structs`, `traits`, `impls`, `classes`,
  `types`, `path` (prefetch), `center`

## Regex for Parsing (`marker::extract_hashes`)

```rust
static HASH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:<<<|\[|\u{2af7})CCR:([0-9a-fA-F:i]{6,64})(?:\|[^\]>\n]*?)?(?:\]|>>>|\u{2af8})").unwrap()
});
```

Supports three delimiter styles, opened and closed independently:

- Standard: `<<<CCR:...>>>`
- Legacy bracket: `[CCR:...]`
- Unicode: `⫷CCR:...⫸` (`⫷` = U+2AF7 opener, `⫸` = U+2AF8 closer - both ends
  are recognized; an earlier version of this regex accepted `⫸` as a closer
  but never recognized `⫷` as an opener, so the Unicode-glyph style was never
  actually extracted)

The hash capture is anchored to hex digits (plus `i:` for inline hashes) and
excludes newlines, so a marker whose closing delimiter never appears cannot
capture arbitrary downstream text (including other markers) as a bogus
multi-line "hash".

`resolve::find_markers` (`crates/aphrodite/src/resolve.rs`) is a second,
separate ASCII-only scanner used specifically for recursive marker expansion
(ASCII `<<<CCR:...>>>` only, no bracket/Unicode forms, no regex) - it is not
governed by this regex.
