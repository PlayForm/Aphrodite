# 05 - CCR Entry Lifecycle & EMA Threshold State

State machine of a single CCR entry from creation through storage, preview,
retrieval, and eviction/decay/GC - plus the separate EMA compression-ratio
state that governs the adaptive threshold.

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
    Previewed --> Recalled: catalog_summary lists recent 5<br/>(pre_llm_call context)

    Previewed --> Retrieved: /retrieve or aphrodite_retrieve(hash)<br/>byte-exact round-trip
    Retrieved --> Previewed: entry stays stored

    Previewed --> Decayed: turn-age - conv_index keeps last 50 turns<br/>recent_markers keeps last 200
    Previewed --> Evicted: LRU / byte-budget pressure<br/>(inline_store_bytes > 256MB default)
    Previewed --> Expired: TTL elapsed (ccr_ttl_seconds, default 3600s)

    Decayed --> GC: dropped from index/marker ring
    Evicted --> GC: pop_back oldest (inline) / LRU tail
    Expired --> GC: backend TTL sweep on get/len
    Retrieved --> [*]: (hot-reload wipes ALL - new dylib image = fresh state)
    GC --> [*]
```

Notes on eviction tiers (from `state.rs` + backends):
- **inline_store** (AphroditeState): dual bound - `INLINE_MAX = 500` entries
  AND `DEFAULT_INLINE_BYTE_BUDGET = 256MB`; `evict_over_budget` pops oldest
  from the back until both hold (state.rs:187).
- **inline_ccr** (proxy AppState): `lru::LruCache` capped at 1024 entries.
- **recent_markers**: ring capped at 200 (state.rs:228); **conv_index**: last
  50 turns (session.rs:39); **referenced_files**: last 100.
- **SqliteCcrStore / InMemoryCcrStore**: TTL from `ccr_ttl_seconds`; in-memory
  also capped at 10,000 entries.
- **Hot-reload** (see `08-dylib-hotreload.md`) is a hard reset: a new dylib
  image has a fresh `OnceLock`/`HANDLES`, so every prior marker becomes
  unresolvable at once (not a graceful per-entry transition).

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

## Key call sites
- inline_store eviction (`evict_over_budget`, `inline_store_put`) - `crates/aphrodite/src/state.rs:187,203`
- `record_marker` (cap 200) / `record_tool_event` (cap 200) - `crates/aphrodite/src/state.rs:228,239`
- `archive_turn` (conv_index cap 50) - `crates/aphrodite/src/session.rs:39`
- EMA update / fill_pct - `crates/aphrodite/src/proxy.rs:503,520`
- backend TTL: `SqliteCcrStore` / `InMemoryCcrStore` - `vendor/headroom/crates/headroom-core/src/ccr/backends/{sqlite.rs,in_memory.rs}`
- inline_ccr LRU (1024) - `crates/aphrodite/src/proxy.rs:213`
