# CCR Entry Lifecycle & EMA Threshold State

State machine of a single CCR entry from creation through storage, preview, retrieval, and eviction/decay/GC - plus the separate EMA compression-ratio state that governs the adaptive threshold.

## CCR entry lifecycle

```mermaid
stateDiagram-v2
    [*] --> Classified: content over threshold<br/>(compress_chat_completion / transform_*_inner)
    Classified --> Hashed: compute_key(bytes) → BLAKE3 40-hex

    Hashed --> Stored_inline: len < inline byte budget<br/>(inline_ccr LRU 1024 / AphroditeState inline_store)
    Hashed --> Stored_sqlite: Token mode → SqliteCcrStore.put (TTL rows)
    Hashed --> Stored_lru: Cache mode → InMemoryCcrStore.put (cap 10k + TTL)

    state Stored <<join>>
    Stored_inline --> Stored
    Stored_sqlite --> Stored
    Stored_lru --> Stored

    Stored --> Previewed: marker emitted<br/>preview + &lt;&lt;&lt;CCR:hash|type|size&gt;&gt;&gt;
    Previewed --> Recalled: catalog_summary emits DELTA previews<br/>(+N new compressions this turn - pre_llm_call context)

    Previewed --> Retrieved: /retrieve or aphrodite_retrieve(hash)<br/>byte-exact round-trip
    Retrieved --> Previewed: entry stays stored

    Previewed --> Decayed: turn-age - conv_index keeps last 50 turns<br/>recent_markers keeps last 200
    Previewed --> Evicted: LRU / byte-budget pressure<br/>(inline_store_bytes > 256MB default)
    Previewed --> Expired: TTL elapsed (ccr_ttl_seconds, default 3600s)

    Decayed --> GC: dropped from index/marker ring
    Evicted --> GC: pop_back oldest (inline) / LRU tail
    Expired --> GC: backend TTL sweep on get/len
    Retrieved --> [*]: hot-reload wipes all in-process state<br/>(new dylib image = fresh state)
    GC --> [*]
```

Eviction tiers:

| Store                                 | Bound                                                                               | Eviction                                                      |
| ------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `inline_store` (AphroditeState)       | 500 entries AND 256 MB byte budget                                                  | `evict_over_budget` pops oldest from the back until both hold |
| `inline_ccr` (proxy AppState)         | 1024 entries (LRU)                                                                  | LRU tail                                                      |
| `recent_markers`                      | 200 markers (ring)                                                                  | oldest dropped                                                |
| `conv_index`                          | last 50 turns                                                                       | oldest turn removed                                           |
| `referenced_files`                    | last 100 files                                                                      | oldest dropped                                                |
| `SqliteCcrStore` / `InMemoryCcrStore` | TTL from `ccr_ttl_seconds` (default 3600s); in-memory also capped at 10,000 entries | lazy TTL sweep on get/len                                     |

A dylib hot-reload is a hard reset: a new dylib image has a fresh `OnceLock`/handles, so every prior marker becomes unresolvable at once (not a graceful per-entry transition).

## EMA compression-ratio threshold state

```mermaid
stateDiagram-v2
    [*] --> Seed: ema = 200 (=2.0x) at build_state
    Seed --> Update: each successful compression<br/>ratio = orig/comp*100
    Update --> Update: ema = 0.2*ratio + 0.8*ema (α=0.2)

    state tune <<choice>>
    Update --> tune: threshold_for(ct) reads ema
    tune --> Aggressive: ema/100 > 20 → tune=2.0 (raise bar)
    tune --> Lenient: 0 < ema/100 < 3 → tune=0.5 (lower bar)
    tune --> Neutral: else → tune=1.0

    Aggressive --> Update
    Lenient --> Update
    Neutral --> Update

    note right of Update
      fill_pct = clamp(100 - ema/20, 1..99)*100
      exposed as X-Aphrodite-Fill-Pct + /metrics
      Live-tunable atomics (config hot-reload):
      cache/token/inline thresholds, code_multiplier
    end note
```

## Call sites

| Concern                                                   | Module                                                                        |
| --------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `evict_over_budget` / `inline_store_put`                  | `crates/aphrodite/src/state/`                                                 |
| `record_marker` (cap 200) / `record_tool_event` (cap 200) | `crates/aphrodite/src/state/`                                                 |
| `archive_turn` (conv_index cap 50)                        | `crates/aphrodite/src/session.rs`                                             |
| EMA update / `compute_fill_pct`                           | `crates/aphrodite/src/proxy.rs`                                               |
| Backend TTL: `SqliteCcrStore` / `InMemoryCcrStore`        | `vendor/headroom/crates/headroom-core/src/ccr/backends/{sqlite,in_memory}.rs` |
| inline_ccr LRU (1024)                                     | `crates/aphrodite/src/proxy.rs`                                               |
