# 11 - Core Data Model (Class Diagram)

The principal data structures across `state.rs`, `session.rs`, `marker.rs`,
`flow.rs`, `directives.rs`, and the vendored `CcrStore` trait. Two distinct
runtime states exist: `AphroditeState` (FFI/Hermes per-session) and `AppState`
(proxy per-listener) - they share the marker/CCR concepts but not the struct.

```mermaid
classDiagram
    class AphroditeState {
        +VecDeque~(String,String)~ inline_store
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
        +usize tool_threshold
        +usize terminal_threshold
        +bool context_engine_enabled
        +HashMap~String,Directive~ directives
        +Vec~String~ active_directives
        +Vec~ActiveDirective~ ephemeral_directives
        +usize flow_budget_chars
        +Option~usize~ manual_directive_turn
        +VecDeque~ToolEvent~ tool_events
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
- `ToolEvent.sig` = `normalize_args_sig` (FNV-1a, volatile keys stripped);
  `error_sig` = `error_sig(error_type, first line)`. `turn_window(n)` folds the
  ring into `WindowStats` for phase/error-loop detection.
- `ActiveDirective` with `inline=Some(..)` renders as a `[nudge:…]`; `name` set
  keys into `directives`. `expires_after_turn=None` is permanent.
- `AppState` (proxy) holds the live-tunable threshold atomics + the CCR backend
  trait object; `AphroditeState` (FFI) holds the directive/turn/telemetry spine.
  They are **separate structs in separate processes**.

## Key call sites
- `AphroditeState`, `MarkerEntry`, `ToolEvent`, `ActiveDirective` - `crates/aphrodite/src/state.rs:20,93,116,126`
- `Directive` - `crates/aphrodite/src/directives.rs:17`
- `WindowStats` / `normalize_args_sig` / `turn_window` - `crates/aphrodite/src/flow.rs:205,164,216`
- `AppState` / `ResolvedThresholds` - `crates/aphrodite/src/proxy.rs:180,104`
- `CcrStore` trait + backends - `vendor/headroom/crates/headroom-core/src/ccr/mod.rs:40` (+ `backends/`)
- `archive_turn` / `conv_index` - `crates/aphrodite/src/session.rs:39`
