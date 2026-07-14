# 02 - Chat-Completion Compression (the core value path)

The token/cache proxy's central flow: a `/v1/chat/completions` response comes
back from upstream, and `compress_chat_completion` classifies each
`message.content`, applies an EMA-tuned per-type threshold (scaled by the
`x-headroom-budget` header), BLAKE3-hashes over-threshold content, stores it,
and replaces the text with a self-describing preview + `<<<CCR:hash|type|size>>>`
marker. `tool_calls[].function.arguments` is a sibling field the loop never
visits, so it always passes through untouched.

> NOTE (verified in code): the HTTP proxy path does **not** call
> `transforms::detect`, `stage2::compress_stage2`, or
> `struct_extract::extract_code_structure`. Those run on the Hermes FFI hook path
> (see `04-hook-ffi.md`). The proxy classifies via `proxy_detect_content_type`
> (proxy.rs:1316) and builds previews via `proxy_build_preview` (proxy.rs:1919).

## Response compression sequence

```mermaid
sequenceDiagram
    autonumber
    participant Client as OpenAI/Anthropic client
    participant PH as proxy_handler (proxy.rs:913)
    participant Cache as response_cache (LRU, 1MB cap)
    participant Up as upstream LLM API
    participant CC as compress_chat_completion (proxy.rs:2075)
    participant CT as proxy_detect_content_type (proxy.rs:1316)
    participant TH as threshold_for + EMA (proxy.rs:470,503)
    participant Store as CcrStore / inline_ccr LRU
    participant MK as smart_marker/cache_marker (proxy.rs:2060)

    Client->>PH: POST /v1/chat/completions {messages,tools,...}
    PH->>PH: requests_total++; is_chat_completion = path==/v1/chat/completions
    PH->>Cache: cache_key_from_body(body, api_key)  (skips if stream:true)
    alt cache HIT
        Cache-->>PH: cached bytes
        PH-->>Client: 200 X-Aphrodite-Cache: HIT (early return)
    else miss
        PH->>Up: forward (stream_client if body stream:true, else client) · ≤3 conn retries
        Up-->>PH: response (headers + body)
        alt content-type is text/event-stream
            PH-->>Client: passthrough stream (NO compression - see 06-sse-streaming)
        else non-streaming
            PH->>PH: accumulate_body (cap 64MB)
            alt is_chat_completion && state.ccr.is_some()
                PH->>CC: compress_chat_completion(state, body, x-headroom-budget)
                CC->>CC: base_threshold = compress_threshold() (mode dispatch)
                CC->>CC: budget_mult = clamp(0.50 + hdr/100*0.50, 0.50..1.0)
                loop each choice.message.content (string)
                    CC->>CT: proxy_detect_content_type(content)
                    CT-->>CC: ct (tool_output|code_rust|error|diff|json|text|…)
                    CC->>TH: threshold = threshold_for(ct).max(base) * budget_mult
                    alt content.len() > threshold  (COMPRESS)
                        CC->>CC: hash = compute_key(bytes) - BLAKE3, 40 hex
                        CC->>Store: ccr_get(hash)?  (dedup)
                        alt exists
                            Store-->>CC: hit → ccr_hits++
                        else
                            CC->>Store: ccr_put(hash, content) → ccr_created++
                        end
                        CC->>MK: Cache→cache_marker / Token→smart_marker
                        MK-->>CC: "preview\n[ct: meta]\n<<<CCR:hash|ct|size>>>"
                        CC->>TH: update_compression_ratio(orig,marker) → EMA
                        CC->>CC: replace content with marker; did_compress=true
                    else content.len() > inline_ccr_threshold (INLINE only)
                        CC->>Store: inline_ccr.put(hash, content)  (no marker swap)
                    else below all thresholds
                        Note over CC: leave content untouched
                    end
                end
                Note over CC: tool_calls[].function.arguments NEVER visited<br/>(loop reads message.content only - 02-F3)
                CC-->>PH: Some(response) if did_compress else None
                alt Some(compressed)
                    PH->>Cache: write (success + ≤1MB)
                    PH-->>Client: 200 X-Aphrodite-Compressed: true, X-Aphrodite-Fill-Pct
                else None
                    PH-->>Client: raw body passthrough, X-Aphrodite-Cache: MISS
                end
            else not chat / no CCR
                PH-->>Client: raw passthrough
            end
        end
    end
```

## Per-type threshold (EMA auto-tune → fixed multiplier)

```mermaid
flowchart TD
    A["threshold_for(ct) (proxy.rs:470)"] --> B{"ct in linter/build_output/log?"}
    B -->|yes| Z["return base (no tune, no mult)"]
    B -->|no| C["ratio = compression_ratio_ema/100"]
    C --> D{"ratio"}
    D -->|">20"| E["tune = 2.0 (compressing well → raise bar)"]
    D -->|"0<ratio<3"| F["tune = 0.5 (weak → lower bar)"]
    D -->|else| G["tune = 1.0"]
    E --> H["base' = base*tune"]
    F --> H
    G --> H
    H --> I{"fixed per-type multiplier"}
    I -->|error| J["base'*8"]
    I -->|"code_*"| K["base' * code_multiplier (default 3.0)"]
    I -->|diff/git/text| L["base'*2"]
    I -->|tool_output/json| M["base'"]

    subgraph EMA["update_compression_ratio (proxy.rs:503)"]
      N["ratio = orig/comp * 100"] --> O["new = 0.2*ratio + 0.8*old  (α=0.2)"]
      O --> P["fill_pct = clamp(100 - ema/20, 1..99)*100"]
    end
```

## Key call sites
- `proxy_handler` request/response orchestration - `crates/aphrodite/src/proxy.rs:913`
- `compress_chat_completion` - `crates/aphrodite/src/proxy.rs:2075`
- `proxy_detect_content_type` (classify) - `crates/aphrodite/src/proxy.rs:1316`
- `AppState::threshold_for` / `update_compression_ratio` / `compute_fill_pct` - `crates/aphrodite/src/proxy.rs:470,503,520`
- `compute_key` (BLAKE3, 40-hex) - `vendor/headroom/crates/headroom-core/src/ccr/mod.rs:86`
- `smart_marker` / `cache_marker` / `proxy_format_ccr_output` - `crates/aphrodite/src/proxy.rs:2060,2068,1906`
- tool_calls pass-through rationale + test - `crates/aphrodite/src/proxy.rs:2169`, test `:3409`
