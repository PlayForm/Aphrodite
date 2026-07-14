# Compression Pipeline

The proxy decides whether to compress each piece of Chat Completions response
content, and if so, how. The pipeline detects content type, computes adaptive
thresholds, checks cache, stores the original content by hash, and tracks
token savings ("compression" here means replacing content with a compact CCR
marker - the stored bytes themselves are kept verbatim, not codec-compressed).
The compression ratio EMA is updated from BOTH the Chat Completions path
(using the rendered marker length) AND the direct `/ccr/create` endpoint
(using a trigram-uniqueness heuristic for the compressed-size estimate).

## What Is Never Compressed

| Exempt content                    | Why                                                                                                                                                                                                                                                  |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tool_calls[].function.arguments` | Client-executable JSON, not model-facing prose - a real OpenAI-tools client (no Aphrodite plugin) can't parse a CCR marker as JSON, so compressing it broke every tool call it made. Only `message.content` is a compression target. Fixed in v1.3.2 |
| SSE streams (`text/event-stream`) | Forwarded chunk-by-chunk, never buffered - see [Architecture: Streaming (SSE)](architecture.md#streaming-sse)                                                                                                                                        |

## Full Pipeline

```
                     Chat Completions Response JSON
                              │
                    ┌─────────▼─────────┐
                    │ For each choice:   │
                    │  message.content   │
                    │  (tool_calls[].    │
                    │   function.args    │
                    │   NEVER touched)   │
                    └─────────┬─────────┘
                              │
                    ┌─────────▼─────────┐
                    │ detect_content_type│
                    │ → "code_rust",      │
                    │   "error", etc.     │
                    └─────────┬─────────┘
                              │
                    ┌─────────▼─────────┐
                    │ threshold_for(ct)  │
                    │ × budget_mult       │
                    └─────────┬─────────┘
                              │
                    ┌─────────▼─────────┐
                    │ len > threshold?   │
                    └────┬──────────┬───┘
                     Yes │          │ No
                         │          │
              ┌──────────▼──┐  ┌────▼──────────┐
              │ compute_key  │  │ len > 256?     │
              │ (BLAKE3, 24) │  └────┬──────┬────┘
              └──────┬───────┘   Yes │      │ No
                     │               │      │
              ┌──────▼──────┐  ┌────▼───┐  │
              │ ccr.get(    │  │inline  │  │
              │   hash)     │  │store   │  │
              └──┬──────┬───┘  └────────┘  │
            hit  │      │ miss             │
                 │      │                  │
                 │  ┌───▼────┐             │
                 │  │ccr.put │             │
                 │  │(zstd)  │             │
                 │  └───┬────┘             │
                 │      │                  │
              ┌──▼──────▼──┐               │
              │ generate    │               │
              │ smart_marker│               │
              └──────┬──────┘               │
                     │                      │
              ┌──────▼──────┐               │
              │ Replace     │               │
              │ content in  │              SKIP
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

## 1. Content Type Detection

`detect_content_type(content: &str) -> &'static str` classifies the content
into one of the supported types. The full type taxonomy is documented in
[../ccr/content-types.md](../ccr/content-types.md).

## 2. Threshold Computation

```rust
fn threshold_for(&self, ct: &str) -> usize {
    let base = self.compress_threshold();  // 1024 (token) or 8192 (cache)

    // Noisy types excluded from auto-tune
    if ct in {"linter", "build_output", "log"} {
        return base / 2;
    }

    // Auto-tune multiplier
    let ratio = self.compression_ratio_ema / 100.0;
    let tune = if ratio > 20.0 {
        2.0   // very aggressive → raise thresholds
    } else if ratio < 3.0 && ratio > 0.0 {
        0.5   // very conservative → lower thresholds
    } else {
        1.0   // default
    };
    let base = (base as f64 * tune) as usize;

    // Per-type multiplier
    match ct {
        "error" => base * 8,
        "code_rust" | "code_python" | "code_go" | "code_js" | "code" => base * 4,
        "diff" | "git" | "text" => base * 2,
        "tool_output" | "json" => base * 1,
        _ => base,
    }
}
```

### Headroom Budget Adjustment

The `x-headroom-budget` header adjusts the threshold further:

```rust
let budget_mult = match budget_value {
    val < 25.0  => 0.25,  // aggressive
    val < 50.0  => 0.50,
    val < 75.0  => 0.75,
    _           => 1.00,  // default
};
let threshold = (threshold_for(ct).max(base) as f64 * budget_mult) as usize;
```

Budget values come from Hermes' own context fill tracking, creating a feedback
loop: the more full the agent's context, the more aggressively the proxy
compresses.

## 3. Content-Addressable Hashing

```rust
pub fn compute_key(payload: &[u8]) -> String {
    let h = blake3::hash(payload);
    h.to_hex().as_str()[..40].to_string()
}
```

BLAKE3, first 40 hex chars (160 bits). Deterministic - same content always
yields the same hash. An earlier draft of this doc said 24 chars; the shipped
`headroom_core::ccr::compute_key` has a test pinning 40.

## 4. CCR Cache

| Path | Steps                                                                                                                                                                  |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hit  | `ccr_hits` incremented; marker generated, content replaced; no store operation                                                                                         |
| Miss | `ccr_misses` incremented; `ccr.put(hash, content)` stored; `ccr_created` incremented; `tokens_saved` += content.len() - hash.len(); marker generated, content replaced |

### Storage Is Verbatim (no zstd)

CCR backends store the original content bytes as-is - the "compression" is
the marker substitution in the LLM-facing response, not a byte-level codec.
An earlier draft of this doc (and of `retrieve.rs` itself) described a zstd
encode/decode step; that branch was dead code and has been removed - see
[CCR Lifecycle](../ccr/lifecycle.md#phase-5-retrieve) for the full
correction.

## 5. Marker Generation

| Mode  | Format                                                                                                                                                                            |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cache | `<<<CCR:{hash}\|{type}\|{size}>>>` followed by the first 512 bytes of content as a preview - simpler format, no structured metadata                                               |
| Token | `<<<CCR:{hash}\|{type}\|{size}\|{metadata_flat}>>` via `smart_marker`, where metadata includes type-specific keys: `lang=rs`, `fns=main,init`, `ln=42`, `keys=status,error`, etc. |

## 6. EMA Update

```rust
fn update_compression_ratio(&self, original_len: usize, compressed_len: usize) {
    let ratio = (original_len as f64 / compressed_len as f64 * 100.0) as u64;
    let old = self.compression_ratio_ema.load(Ordering::Relaxed);
    let new = ((ratio as f64 * 0.2) + (old as f64 * 0.8)) as u64;
    self.compression_ratio_ema.store(new, Ordering::Relaxed);
    self.compute_fill_pct();  // side-effect: updates fill_pct
}
```

Exponential moving average with α=0.2 (20% weight on new observation).

### Initial Value

```rust
compression_ratio_ema: AtomicU64::new(200),  // 2.0x  -  conservative, avoids startup scale-up
```

### Fill Percentage

```rust
fn compute_fill_pct(&self) {
    let ratio_ema = self.compression_ratio_ema;
    let pct = 100u64.saturating_sub(ratio_ema / 20);
    self.fill_pct.store(pct.clamp(1, 99) * 100);  // ×100 for precision
}
```

Higher compression ratio → lower fill → more headroom available.

## 7. Token Savings Tracking

```rust
state.tokens_saved.fetch_add(
    (original_len - marker_len) as u64,
    Ordering::Relaxed
);
```

Also for LLM response cache hits:

```rust
state.tokens_saved.fetch_add(cached_body.len() as u64 / 4, Ordering::Relaxed);
// ~4 bytes per token heuristic
```

## 8. Inline Store (Below Threshold, Above 256B)

Content that's too small for compression but above 256B goes to `inline_ccr`:

```rust
let hash = compute_key(content.as_bytes());
if map.contains(&hash) {
    inline_ccr_hits++;
} else {
    inline_ccr_misses++;
    map.put(hash, content);
}
```

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
