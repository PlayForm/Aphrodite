# Optimization Clues - Aphrodite v1.3.4 (CCR proxy + Hermes plugin)

Scannable clue index for a later optimization pass. Re-verified at HEAD (2026-07-14) against the fresh bench baseline (`.bench/results/run-2026-07-14/`, corpus 132.82×, cache 9-40ms / token 8-10ms e2e) and the prior deep analysis (`.plans/archive/v1-2026-07-13/18-performance-engineering.md`, `.plans/06-open-items-register.md`). Each clue = one line; cross-refs the plan tasks rather than re-explaining. Format: `[sev/effort] clue - file:line - win (status)`.

## Classification cost (the central perf fact)
- [high/M] **magika ONNX inference runs on EVERY hook-path classification, serialized on a process-wide `Mutex<Session>`** - `hooks.rs:178` + `:291` call `transforms::detect` → `vendor/headroom/.../transforms/detection.rs:64` → `magika_detector.rs:409` `mutex.lock()`+`identify_content_sync` - kill ~2ms/call floor + serialization (status: **still-open from 18-P7 / 06-B7**; decision D-11 = accept heuristic, not yet done)
- [high/M] the semantic short-circuit does NOT avoid the floor: `transforms::detect` runs FIRST (`hooks.rs:178`), `detect_semantic_type` is only a fallback on generic types - magika still pays on every call - `hooks.rs:170-189` - route hook/FFI classify through the heuristic (`proxy.rs` `detect_content_type` or fork's retired `content_detector`) (status: still-open from 18-P7)
- [high/M] same floor on the FFI `"classify"` arm and per-prefetched-file - `lib.rs:632`, `prefetch.rs:78` - same fix as 18-P7 (status: still-open)
- [med/M] proxy hot path already uses the cheap µs heuristic (`proxy.rs` `detect_content_type`) - the split is the point: token-mode e2e is 8-10ms flat, cache-mode 9-40ms; tiny_text 40ms cache outlier smells like first-classify ONNX init (18-P6) (status: proxy side = realized; hook side = open)

## Hot-path allocations
- [med/M] `stage2.rs` `reduce_code`: outer loop over 9 `SigPattern`s × inner loop over ALL lines, `to_lowercase()` per line **per pattern** (9 allocs/line) - `stage2.rs:86` + loop `:81/:120/:149-160` - single-pass, drop lowercase → ≥5× on large code (status: still-open from 18-P12)
- [med/S] `reducer_registry()` rebuilds a HashMap on every `compress_stage2` call - a `match` does it free - `stage2.rs` (registry fn) (status: still-open from 18-P12)
- [med/S] `struct_extract` lowercases every line (`let lower = trimmed.to_lowercase()`) though keywords are case-sensitive in all supported langs - `struct_extract.rs:132` - drop per-line alloc (status: still-open from 18-P13d)
- [low/S] per-request `format!("Bearer {}", api_key)` rebuilt every request - `proxy.rs:1025` - cache `HeaderValue` in AppState at build_state (status: new/low, ~18-P16-adjacent)
- [low/S] `format!("{:.1}", fill_pct)` rebuilt 3× per response - `proxy.rs:997/1229/1278` (status: new/low)
- [low/S] `code_multiplier()` calls `std::env::var` on every `threshold_for` - `proxy.rs` threshold path - cache once (status: still-open from 18-P16)
- [note] `retrieve.rs` pagination `lines[start..end].join("\n")` - already guarded: whole-doc retrieval skips the lossy lines()/join round-trip - `retrieve.rs:216-227` (status: **fixed** F4)
- [note] `flow::build_turn_context` allocates Strings per turn (`flow.rs:52,105,179,197`) but is per-TURN not per-tool; **concurrent agent editing flow.rs/state.rs/hooks.rs - re-verify before touching** (status: watch)

## Locking
- [low/L] single global `Mutex<Connection>` serializes ALL SQLite get/put/del/stats - `vendor/headroom/.../ccr/backends/sqlite.rs:81` - shard by N DB files if contended (status: documented-accepted; open from 18-P10/P11)
- [note] all proxy `std::sync::Mutex` (request_history, inline_ccr, response_cache, health) locked+dropped synchronously, no await held; SQLite wrapped in `spawn_blocking` (`proxy.rs:141-173`) so no lock-across-await - the magika `Mutex<Session>` is the only cross-I/O-ish serialization and it's inference, not I/O (status: **fixed / not-an-issue**)

## Storage
- [low/M] get-before-put per compressed item (round-trip just for a hit/miss counter) - `proxy.rs:2114-2126` - make `put` return inserted-vs-replaced, drop pre-read (status: still-open from 18-P11)
- [med/M] CCR blobs stored **uncompressed** (`original BLOB NOT NULL`); `stats_db` even fakes `total_bytes_compressed = entries*24` - `sqlite.rs:9,121,347` - zstd at-rest for large entries → DB size + page-cache win (status: new / 14-adjacent)
- [note] WAL + `synchronous=NORMAL` + `busy_timeout(5s)` set at open; one connection reused for process lifetime - `sqlite.rs:106,116` (status: **fixed** - do not re-suggest)
- [med/M] inline store `get`/`put`/`contains` are O(n) linear `VecDeque` scans with full-content clone on every get - `state.rs:206,218,249` - hashmap-backed LRU (status: still-open from 18-P14)

## Proxy / IO
- [med/S] request body cloned twice on the no-retry common case: `body.to_vec()` (`proxy.rs:961`) then `req.body(body_vec.clone())` inside `for attempt in 1..=3` (`:1051`) - move on attempt 1, clone only on retry → up to 4 full copies/1MB body under load (status: still-open from 18-P9)
- [med/S] non-SSE `accumulate_body` grows `Vec` via `extend_from_slice` loop with no `with_capacity` up to 64MB cap - `proxy.rs:873` - pre-size from Content-Length (status: new; buffering itself is by-design, JSON must be parsed whole)
- [low/S] cacheable response body copied twice (once for `Body`, once cloned into `response_cache`) - `proxy.rs:1207/1217`, `:1269` - share via `Bytes`/`Arc<[u8]>` (status: still-open from 18-P10)
- [note] SSE path streamed via `bytes_stream()`+`Body::from_stream`, never buffered - `proxy.rs:1102-1138` (status: **fixed**); shared long-lived reqwest clients w/ pooling `proxy.rs:643-660` (status: **fixed**); no per-request regex in proxy (`:1841` hand-rolled) (status: **fixed**)

## Compression-ratio wins
- [med/L] poor-ratio content types in bench: tiny_text 3.00× (120B, below the 40B floor overhead) and json_tool 16.88× (675B) - both are small payloads where the fixed marker overhead dominates; near-dup/delta-chain for repeated reads is the real lever - see 14 (delta previews / repeated-read deflection) (status: planned in 14, open)
- [low] json_tool 16.88×: `reduce_json` parses + re-serializes the value tree at stage2 (`stage2.rs:24,46`) - combined with classify-parse and cache-key-parse, a JSON body is serde-round-tripped up to 4× per lifecycle (18 §2 census) - semantic-reduction headroom is capped by having to reconstruct exact bytes for retrieval (status: open, structural)

## Startup / binary size
- [high/S] **fastembed + ort + tokenizers linked into the dylib but NEVER called from aphrodite/aphrodite-hermes** - declared `vendor/headroom/.../Cargo.toml:131` (fastembed), used only by headroom-core's own unused `EmbeddingScorer` (`relevance/embedding.rs:31`); no call site in `crates/aphrodite*` - dylib is ~29.5MiB vs 12.3MiB proxy (18 §1) - gate ML behind an `ml` cargo feature → strip ~18MiB dead weight (status: still-open from 08-F7 / 06-D-10, decision = gate slim default, not done)
- [med/S] first-classify pays full ONNX session init lazily via `OnceLock` (`magika_detector.rs:352-397`, 5s init timeout) - likely the tiny_text 40ms cache-mode outlier - measure/gate as its own budget line (status: open from 18-P6)

## Top 5 by leverage
1. **Route hook/FFI classification off magika onto the heuristic** (`hooks.rs:178,291`, `lib.rs:632`, `prefetch.rs:78`) - removes the ~2ms/call serialized floor that dominates every Hermes tool result (18-P7 / 06-B7, decision D-11 already made).
2. **Gate the ML stack (fastembed/ort/magika) behind an `ml` cargo feature** (`vendor/headroom/.../Cargo.toml`) - strips ~18MiB dead weight from the hot-loaded dylib; folds in #1's magika removal (08-F7 / D-10).
3. **Fuse `stage2::reduce_code` to single-pass + drop per-line `to_lowercase`** (`stage2.rs:81-160`) and the same in `struct_extract.rs:132` - ~270k transient Strings → ~0 on large code (18-P12/P13).
4. **Kill the per-attempt request-body clone on the no-retry path** (`proxy.rs:961→:1051`) - up to 400MB avoidable transient under 100-way burst (18-P9).
5. **HashMap-back the inline store + zstd CCR blobs at rest** (`state.rs:206-249`, `sqlite.rs:9`) - O(n)→O(1) gets and smaller DB / page-cache (18-P14 + new).

## magika-floor verdict
**STILL-OPEN.** The ~2ms magika-ONNX-behind-`Mutex<Session>` floor is fully present on the hook/FFI path at HEAD - `hooks.rs:178`/`:291` call `transforms::detect` unconditionally before any semantic short-circuit; `detect_semantic_type` only overrides the *reported type/preview*, it never prevents the magika call. Criterion-verified flat 1.89-2.66ms (06-B7). The proxy hot path was never affected (uses the µs heuristic). Fix decision (D-11) is accepted but unimplemented.
