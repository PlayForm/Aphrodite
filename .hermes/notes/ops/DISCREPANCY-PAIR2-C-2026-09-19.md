# DISCREPANCY-PAIR2-C-2026-09-19

Append-only log of verified-stale claims found while rewriting docs/ccr
(marker-format.md, lifecycle.md, backends/sqlite.md, backends/in-memory.md,
backends/inline.md). Child C of PAIR-2.

- [2026-09-19] docs/ccr/lifecycle.md: inline tier described as the only
  inline store being `lru::LruCache` 1,024 entries; session-side inline store
  is HashMap + LRU VecDeque capped at 500 entries with a 256 MiB byte budget
  (crates/aphrodite/src/state.rs:13,17,79-82,95-99); proxy inline cache IS
  still lru::LruCache(1024) (crates/aphrodite/src/proxy.rs:722).
- [2026-09-19] docs/ccr/lifecycle.md: claimed "Python Plugin Inline Store
  `_CappedStore` (OrderedDict, 500 entries)"; no such store exists in
  plugins/aphrodite (grep for _CappedStore/OrderedDict/inline_store: 0 hits);
  inline storage is Rust-side only.
- [2026-09-19] docs/ccr/lifecycle.md: claimed inline hash format "BLAKE3, 24
  hex" for Rust inline; compute_key is BLAKE3 truncated to 40 hex chars
  (vendor/headroom/crates/headroom-core/src/ccr/mod.rs:113-121).
- [2026-09-19] docs/ccr/lifecycle.md: response-cache key claimed
  FNV-1a(api_key + model + messages) only; actual key covers api_key, model,
  messages, tools, tool_choice, temperature, top_p, n, response_format in
  canonical order (crates/aphrodite/src/proxy.rs:798-816).
- [2026-09-19] docs/ccr/backends/sqlite.md: schema claimed
  hash/original/created_at/ttl_seconds with purge `WHERE created_at +
ttl_seconds <= now`; actual schema adds last_accessed (schema v2) and
  expiry is `last_accessed + ttl_seconds < now OR created_at + max_lifetime
< now` (sliding idle window, max lifetime 8x TTL), purge runs from get AND
  put debounced 60s (vendor/headroom/crates/headroom-core/src/ccr/backends/sqlite.rs:16-25,158-177,204-206).
- [2026-09-19] docs/ccr/backends/in-memory.md: claimed DEFAULT_TTL 300s
  (5 min); actual DEFAULT_TTL is 1,800s (30 min) with a sliding idle window
  and absolute max lifetime 8x TTL, and `get` refreshes last_accessed
  (vendor/headroom/crates/headroom-core/src/ccr/mod.rs:86-95;
  backends/in_memory.rs:106-112,222-228).
- [2026-09-19] docs/ccr/backends/inline.md: claimed inline store is only
  `lru::LruCache` 1,024 entries; session-side inline store is HashMap +
  VecDeque LRU with INLINE_MAX 500 and 256 MiB byte budget
  (crates/aphrodite/src/state.rs:13,17,79-82).
