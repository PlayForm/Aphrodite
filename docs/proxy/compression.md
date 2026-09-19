# Compression Pipeline

The proxy decides whether to compress each piece of Chat Completions response
content, and if so, how. The pipeline detects content type, computes adaptive
thresholds, stores the original content by hash, replaces it with a compact
CCR marker, and tracks token savings. "Compression" here means the marker
substitution in the model-facing response - the stored bytes are kept verbatim,
not codec-compressed.

The compression-ratio EMA is updated from two sources: the Chat Completions
path (using the rendered marker length) and the direct `/ccr/create` endpoint
(using a trigram-uniqueness heuristic for the compressed-size estimate), so
`/stats` reflects the compressibility of everything flowing through the proxy.

## What Is Never Compressed

| Exempt content                    | Why                                                                                                                                                                                                                                 |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tool_calls[].function.arguments` | Client-executable JSON, not model-facing prose - a real OpenAI-tools client (no Aphrodite plugin) can't parse a CCR marker as JSON, so compressing it broke every tool call it made. Only `message.content` is a compression target |
| SSE streams (`text/event-stream`) | Forwarded chunk-by-chunk, never buffered - markers can't be spliced into a live stream. See [Architecture: Streaming (SSE)](https://github.com/PlayForm/Aphrodite/tree/Current/docs/proxy/architecture.md#streaming-sse)            |

## Full Pipeline

```
                    Chat Completions Response JSON
                             │
                   ┌─────────▼─────────┐
                   │ For each choice:   │
                   │  message.content   │
                   │  (tool_calls[].    │
                   │   function.arguments│
                   │   NEVER touched)   │
                   └─────────┬─────────┘
                             │
                   ┌─────────▼─────────┐
                   │ detect_content_type│
                   │ → "code_rust",     │
                   │   "error", etc.    │
                   └─────────┬─────────┘
                             │
                   ┌─────────▼─────────┐
                   │ threshold_for(ct) │
                   │  × budget_mult    │
                   └─────────┬─────────┘
                             │
                   ┌─────────▼─────────┐
                   │ len > threshold?  │
                   └────┬──────────┬───┘
                    Yes │          │ No
                        │          │
             ┌──────────▼──┐  ┌────▼─────────────┐
             │ compute_key  │  │ len > inline_    │
             │ (BLAKE3, 40) │  │ threshold (256)? │
             └──────┬───────┘  └────┬───────┬─────┘
                    │            Yes │       │ No
                    │               │       │
             ┌──────▼──────┐   ┌────▼───┐   │
             │ ccr.get(    │   │ inline │   │
             │   hash)     │   │ store  │   │
             └──┬──────┬───┘   └────────┘   │
           hit  │      │ miss               │
                │      │                    │
                │  ┌───▼────┐               │
                │  │ccr.put │               │
                │  │(store) │               │
                │  └───┬────┘               │
                │      │                    │
             ┌──▼──────▼──┐                │
             │ generate   │                │
             │ marker     │               SKIP
             └──────┬──────┘                │
                    │                       │
             ┌──────▼──────┐               │
             │ Replace     │               │
             │ content in  │               │
             │ JSON        │               │
             └──────┬──────┘               │
                    │                      │
             ┌──────▼──────┐               │
             │ update_     │               │
             │ compression_│               │
             │ ratio EMA   │               │
             └──────┬──────┘               │
                    │                      │
             ┌──────▼──────┐               │
             │ record_     │               │
             │ compression │               │
             │ (per type)  │               │
             └─────────────┘               │
```

A failed store put never produces a marker: the content is left uncompressed
rather than swapped for an unresolvable hash.

## 1. Content Type Detection

`detect_content_type` classifies the content into one of the supported types
(JSON/tool output, language-specific code, error, build output, linter, diff,
git, log, text, plus semantic detection for git status / git log / ls / test /
grep shapes). The full type taxonomy is documented in
[CCR: Content Types](https://github.com/PlayForm/Aphrodite/tree/Current/docs/ccr/content-types.md).

## 2. Threshold Computation

```rust
fn threshold_for(&self, ct: &str) -> usize {
    let base = self.compress_threshold();  // 8192 (cache) or 1024 (token)

    // Noisy types stay at the base threshold - coding sessions need build
    // output visible
    if ct in {"linter", "build_output", "log"} {
        return base;
    }

    // Auto-tune multiplier from the compression-ratio EMA
    let ratio = self.compression_ratio_ema / 100.0;
    let tune = if ratio > 20.0 {
        2.0   // very aggressive → raise thresholds, preserve more content
    } else if ratio < 3.0 && ratio > 0.0 {
        0.5   // very conservative → lower thresholds, compress more
    } else {
        1.0   // default
    };
    let base = (base as f64 * tune) as usize;

    // Per-type multiplier
    match ct {
        "error" => base * 8,
        "code_rust" | "code_python" | "code_go" | "code_js" | "code" => {
            (base as f64 * self.code_multiplier()) as usize  // default 3.0
        },
        "diff" | "git" | "text" => base * 2,
        "tool_output" | "json" => base,
        _ => base,
    }
}
```

The code multiplier is live-configurable (`[compression] code_multiplier` in
the TOML, `APHRODITE_CODE_MULTIPLIER` env var, default 3.0) and applies to all
code types.

### Headroom Budget Adjustment

The `x-headroom-budget` header (sent by Hermes from its own context-fill
tracking) adjusts the threshold further. It is a smooth linear curve, never a
step function:

```rust
let budget_mult = headroom_budget
    .and_then(|b| {
        let val: f64 = b.parse().ok()?;
        // 0.50 + (fill% * 0.50), clamped to [0.50, 1.0]
        Some((0.50 + (val / 100.0) * 0.50).clamp(0.50, 1.0))
    })
    .unwrap_or(1.0);
let threshold = (threshold_for(ct).max(base) as f64 * budget_mult) as usize;
```

Budget values come from Hermes' own context-fill tracking, creating a feedback
loop: the fuller the agent's context, the more aggressively the proxy
compresses. The multiplier never drops below 0.5× - semantics and tool chains
are worth the tokens.

## 3. Content-Addressable Hashing

```rust
pub fn compute_key(payload: &[u8]) -> String {
    let h = blake3::hash(payload);
    h.to_hex().as_str()[..40].to_string()
}
```

BLAKE3, first 40 hex chars (160 bits). Deterministic - same content always
yields the same hash, so identical content is stored once and shared across
requests and listeners.

## 4. CCR Cache

| Path | Steps                                                                                                                                                                          |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Hit  | `ccr_hits` incremented; marker generated, content replaced; no store operation                                                                                                 |
| Miss | `ccr_misses` incremented; `ccr.put(hash, content)` stored; `ccr_created` incremented; `tokens_saved` += content length minus marker length; marker generated, content replaced |

### Storage Is Verbatim (no codec)

CCR backends store the original content bytes as-is - the "compression" is the
marker substitution in the LLM-facing response, not a byte-level codec. See
[CCR: Lifecycle](https://github.com/PlayForm/Aphrodite/tree/Current/docs/ccr/lifecycle.md) for the store/retrieve contract.

## 5. Marker Generation

Both modes render a three-line block: the preview line, a structure line
(`[type: metadata]`), and the marker line. Cache mode uses a simpler form with
no metadata; token mode carries structured metadata plus an enriched preview.

```
{preview line}
[{type}: {metadata}]
<<<CCR:{hash}|{type}|{size}>>>
```

| Mode  | Preview                                  | Metadata                                                                                                |
| ----- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Cache | First 512 bytes of the content, raw      | None (empty structure line)                                                                             |
| Token | Enriched, type-aware preview (see below) | `lang=rs`, `fns=main,init`, `ln=42`, `keys=status,error`, ... (pipe-safe `key=value;key=value`, capped) |

Metadata is generated per content type: line counts (`ln=`), language tags
(`lang=rs|py|go|js|gen`), function/struct names for code, JSON keys for JSON
content, and more. Markers on their own line are easy for the model to scan
and to pass back to `aphrodite_retrieve` verbatim.

## Enriched Preview Catalog

Every marker carries a compact, human-readable preview built by `build_preview`
(`crates/aphrodite/src/preview.rs`). These enriched previews are **automatic
and default** - no flag - and are emitted identically on both the proxy path
and the Hermes hook/FFI path (both funnel through the same `build_preview`
function).

When the classifier only reaches a generic bucket (`text`/`terminal`/`log`/
empty), a semantic detector (`detect_semantic_type`) upgrades the arm to a
high-signal shape (git status, git log, grep, ls, test) before the preview is
built. Detection is conservative (line-prefix / marker patterns, majority
votes) so ordinary prose is never mis-tagged. An explicit non-generic type
from the classifier is always honored as-is.

| Content type                         | Enriched preview shape                                               |
| ------------------------------------ | -------------------------------------------------------------------- |
| build / build_output / build_error   | `[build:1E 1W 142L                                                   | error[E0432]: unresolved import ...]` (first error msg)     |
| diff                                 | `[diff:2F +7/-3 12L                                                  | src/main.rs Cargo.toml +N more]` (first file names)         |
| git status (`git`)                   | `[git:2M 2A 1D 3??                                                   | src/x.rs src/y.rs +5 more]` (code tallies + paths)          |
| git log (`gitlog`)                   | `[gitlog:2 commits                                                   | abc1234 fix… → def5678 feat…]` (first→last hash/subj)       |
| directory listing (`ls`)             | `[ls:3 files 2 dirs                                                  | .rs×2 .md×1]` (file/dir counts + top extensions)            |
| test output (`test`)                 | `[test:220 pass 0 fail 1 ignored                                     | 0.31s]`(or`                                                 | FAIL name` on failure) |
| grep / ripgrep (`grep`)              | `[grep:4 hits in 3 files                                             | src/preview.rs:12 …]` (hits, files, first loc)              |
| source_code / code_* (`code`)        | `[code:3fns                                                          | 2structs fn main() 414L]` (structure map + first signature) |
| search (`search`)                    | `[search:Nhits ML]`                                                  |
| json_array (`json`)                  | `[json:5items 3L]`                                                   |
| terminal (`terminal`)                | `[terminal:14L exit code: 0]` (exit-code / last-output-line context) |
| plain text / unrecognized (fallback) | `[text:3L 50B                                                        | some unrecognizable prose here]` (first non-empty line)     |

The rendered preview length is capped by `[previews] preview_max_chars`
(`APHRODITE_PREVIEW_MAX_CHARS` env var, applied end-to-end including
hot-reload). See also
[CCR: Content Types](https://github.com/PlayForm/Aphrodite/tree/Current/docs/ccr/content-types.md) for the underlying type taxonomy.

## 6. EMA Update

```rust
fn update_compression_ratio(&self, original_len: usize, compressed_len: usize) {
    if original_len == 0 || compressed_len == 0 {
        return;
    }
    let ratio = (original_len as f64 / compressed_len as f64 * 100.0) as u64;
    let old = self.compression_ratio_ema.load(Ordering::Relaxed);
    let new = ((ratio as f64 * 0.2) + (old as f64 * 0.8)) as u64;
    self.compression_ratio_ema.store(new, Ordering::Relaxed);
    self.compute_fill_pct();  // side-effect: updates fill_pct
}
```

Exponential moving average with α=0.2 (20% weight on the new observation).

### Initial Value

```rust
compression_ratio_ema: AtomicU64::new(200),  // 2.0x - conservative, avoids startup scale-up
```

### Fill Percentage

```rust
fn compute_fill_pct(&self) {
    let ratio_ema = self.compression_ratio_ema.load(Ordering::Relaxed);
    let pct = if ratio_ema == 0 {
        99u64
    } else {
        let raw = 100u64.saturating_sub(ratio_ema / 20);
        raw.clamp(1, 99)
    };
    self.fill_pct.store(pct * 100, Ordering::Relaxed); // ×100 for precision
}
```

Higher compression ratio → lower fill → more headroom available.

## 7. Token Savings Tracking

Savings are raw bytes throughout (the counter keeps its `tokens_saved` name
for API compatibility):

```rust
state.tokens_saved.fetch_add(
    (original_len - marker_len) as u64,
    Ordering::Relaxed
);
```

The subtractee is the rendered marker length (hundreds of chars), never the
bare 40-char hash - the marker is far longer than the hash, so subtracting
`hash.len()` would overstate savings. Response-cache hits add the whole cached
body length:

```rust
state.tokens_saved.fetch_add(cached_body.len() as u64, Ordering::Relaxed);
```

## 8. Inline Store (Below Threshold, Above Inline Threshold)

Content that is below the compression threshold but above the inline threshold
(256 bytes by default, live-configurable) goes into `inline_ccr` - a 1024-entry
in-memory LRU - so later retrievals find small entries without a backend
round-trip:

```rust
let hash = compute_key(content.as_bytes());
if map.contains(&hash) {
    inline_ccr_hits++;
} else {
    inline_ccr_misses++;
    map.put(hash, content);
}
```

The tool-relay `aphrodite_compress` path stores inline content to the durable
backend as well, so the entry survives inline-LRU eviction and proxy restarts.

## Auto-Tune State Machine

```
                ┌──────────┐
    startup     │  EMA=200 │  (2.0×, conservative)
       →        │  fill=90 │
                └────┬─────┘
                     │
         ratio > 20.0 (very compressed)
                     │
                ┌────▼─────┐
                │  tune=2.0│  raise thresholds
                │ (relax)  │  preserve more content
                └────┬─────┘
                     │
         ratio drops below 20
                     │
                ┌────▼─────┐
                │  tune=1.0│  default
                └────┬─────┘
                     │
         ratio < 3.0 (barely compressing)
                     │
                ┌────▼─────┐
                │ tune=0.5 │  lower thresholds
                │(compress)│  compress more
                └──────────┘
```

Initial values at startup:

| Field                   | Value         |
| ----------------------- | ------------- |
| `compression_ratio_ema` | 200 (2.0×)    |
| `fill_pct`              | 9000 (90.00%) |

## Chain-Split (Hermes Plugin Side)

Chain-split is a Hermes plugin-side feature, not a proxy one: `pre_tool_call`
rewrites chained shell commands (`cd x && cargo build && cargo test`) by
echoing segment markers to stderr, and `transform_tool_result` splits the
produced output into per-segment pieces so each segment is compressed
independently (`[chain:3 | build ...]` previews). It is opt-in
(`chain_split = false` by default), and constructs that would corrupt control
flow (heredocs, loops spanning segments) are left untouched. The proxy itself
only ever sees Chat Completions responses and never performs chain splitting.
