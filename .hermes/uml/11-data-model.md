# 11 - Core Data Model (Class Diagram)

The principal data structures across `state.rs`, `session.rs`, `marker.rs`,
`flow.rs`, `directives.rs`, and the vendored `CcrStore` trait. Two distinct
runtime states exist: `AphroditeState` (FFI/Hermes per-session) and `AppState`
(proxy per-listener) - they share the marker/CCR concepts but not the struct.

```mermaid
classDiagram
    class AphroditeState {
        +HashMap~String,String~ inline_store
        +VecDeque~String~ inline_order
        -usize inline_store_bytes
        -usize inline_store_byte_budget
        +Vec~MarkerEntry~ recent_markers
        +HashMap~usize,(String,String,usize)~ conv_index
        +VecDeque~(String,String)~ referenced_files
        +usize turn_counter
        +usize scanned_msg_idx
        +Vec~String~ file_tools
        +String api_url
        +String model
        +u64 engine_threshold_pct
        +usize tool_threshold
        +usize terminal_threshold
        +bool context_engine_enabled
        +String catalog_mode
        +HashMap~String,Directive~ directives
        +Vec~String~ active_directives
        +Vec~ActiveDirective~ ephemeral_directives
        +usize flow_budget_chars
        +String session_inject
        +Option~usize~ manual_directive_turn
        +VecDeque~ToolEvent~ tool_events
        +VecDeque~BgTask~ bg_tasks
        +bool poll_worker_enabled
        +bool chain_split_enabled
        +usize chain_split_min_segments
        +usize chain_split_floor
        +usize chain_split_max_segments
        +VecDeque~SplitEvent~ split_events
        +HashMap~String,usize~ split_segment_map
        +usize last_emitted_marker_count
        +usize last_emitted_file_count
        +inline_store_put()
        +inline_store_get()
        +record_marker()
        +record_tool_event()
    }

    class MarkerEntry {
        +String hash
        +String ccr_type
        +usize size
        +String preview
        +usize turn
        +Option~String~ center
        +Option~HashMap~ meta
    }

    class ToolEvent {
        +usize turn
        +String tool
        +u64 sig
        +bool ok
        +Option~u64~ error_sig
        +usize bytes
        +Option~String~ wrote_path
    }

    class SplitEvent {
        +usize id
        +usize turn
        +usize produced
        +usize retrieved
    }

    class ActiveDirective {
        +String name
        +Option~String~ inline
        +Option~usize~ expires_after_turn
    }

    class Directive {
        +String name
        +String content
    }

    class WindowStats {
        +usize reads
        +usize writes
        +usize searches
        +usize errors
        +usize distinct_error_sigs
        +usize new_files
        +usize total_calls
    }

    class AppState {
        +Option~Arc~dyn CcrStore~~ ccr
        +bool add_markers
        +ProxyMode mode
        +Mutex~LruCache~ inline_ccr
        +AtomicU64 compression_ratio_ema
        +AtomicU64 fill_pct
        +AtomicUsize cache_compress_threshold
        +AtomicUsize token_compress_threshold
        +AtomicUsize inline_ccr_threshold
        +AtomicU64 code_multiplier_x100
        +Mutex~LruCache~ response_cache
        +threshold_for()
        +update_compression_ratio()
    }

    class CcrStore {
        <<trait>>
        +put(hash, payload) bool
        +get(hash) Option~String~
        +len() usize
        +del(hash) bool
        +stats_db() Option~Value~
    }
    class SqliteCcrStore
    class InMemoryCcrStore
    class RedisCcrStore

    class ResolvedThresholds {
        +usize cache
        +usize token
        +usize inline
        +f64 code_multiplier
    }

    AphroditeState "1" o-- "*" MarkerEntry : recent_markers
    AphroditeState "1" o-- "*" ToolEvent : tool_events
    AphroditeState "1" o-- "*" SplitEvent : split_events
    AphroditeState "1" o-- "*" ActiveDirective : ephemeral_directives
    AphroditeState "1" o-- "*" Directive : directives
    ToolEvent ..> WindowStats : aggregated by turn_window()
    AppState "1" o-- "0..1" CcrStore : ccr backend
    CcrStore <|.. SqliteCcrStore
    CcrStore <|.. InMemoryCcrStore
    CcrStore <|.. RedisCcrStore
    AppState ..> ResolvedThresholds : seeded by resolve_thresholds()
```

Relationships / invariants:

- `MarkerEntry.hash` is the BLAKE3 `compute_key` (40 hex) that also keys
  `inline_store`; `conv_index[turn] = (hash, summary, size)` is the last marker
  of that turn, written by `archive_turn`.
- `inline_store` is now a `HashMap` (O(1) get/put - was an O(n) `VecDeque`
  linear scan per op, bug 18-P14) with `inline_order: VecDeque<String>`
  preserving LRU recency so `evict_over_budget` drops the least-recent entry
  from the back; bounded by `INLINE_MAX` (500 entries) AND
  `DEFAULT_INLINE_BYTE_BUDGET` (256 MB).
- `ToolEvent.sig` = `normalize_args_sig` (FNV-1a, volatile keys stripped);
  `error_sig` = `error_sig(error_type, first line)`. `turn_window(n)` folds the
  ring into `WindowStats` for phase/error-loop detection.
- `SplitEvent` is the Tier-1 teaching-loop ledger: one entry per chain-split
  event (`produced` segment markers vs `retrieved` distinct hashes); the
  retrieval ratio adapts `chain_split_min_segments` within
  `[chain_split_floor, chain_split_max_segments]`. Ring capped at
  `SPLIT_EVENT_CAP` (16).
- `ActiveDirective` with `inline=Some(..)` renders as a `[nudge:…]`; `name` set
  keys into `directives`. `expires_after_turn=None` is permanent.
- `catalog_summary` is delta-only: it emits `+N new compressions this turn`
  (or a cache-stable "no change" line) using `last_emitted_marker_count` /
  `last_emitted_file_count`, not a fixed "recent 5" listing.
- `AppState` (proxy) holds the live-tunable threshold atomics + the CCR backend
  trait object; `AphroditeState` (FFI) holds the directive/turn/telemetry spine
  plus the chain-split and poll-worker machinery. They are **separate structs
  in separate processes**.

## Key call sites

- `AphroditeState`, `MarkerEntry`, `ToolEvent`, `ActiveDirective`, `SplitEvent` - `crates/aphrodite/src/state.rs:27,193,160,183,210`
- `Directive` / `build_directive_context` - `crates/aphrodite/src/directives.rs:56,157`
- `WindowStats` / `normalize_args_sig` / `turn_window` - `crates/aphrodite/src/flow.rs:240,199,251`
- `AppState` / `ResolvedThresholds` - `crates/aphrodite/src/proxy.rs:189,113`
- `CcrStore` trait + backends - `vendor/headroom/crates/headroom-core/src/ccr/mod.rs:40` (+ `backends/`)
- `archive_turn` / `catalog_summary` / `conv_index` - `crates/aphrodite/src/session.rs:38,66`
