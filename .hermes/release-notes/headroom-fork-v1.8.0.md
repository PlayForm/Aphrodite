> **Released with `Aphrodite/v1.8.0`** - fork commit `724da7bf` on `Current`
> (the parent gitlink floats there). Two steps remain outside this note's
> scope and outside CI: publishing `aphrodite-headroom-core` `0.2.0` to
> crates.io, and creating the fork tag `aphrodite-v0.11.0`.

## Headroom Fork: aphrodite-headroom-core 0.2.0 · fork tag aphrodite-v0.11.0

### Summary

The PlayForm/Headroom fork syncs with upstream and ships its first functional
change since 0.1.3.

The sync: `headroomlabs-ai/main` was merged into the fork's `Current`
(merge-base `c365c7ff` → `bb8d18a5`), bringing upstream's fixes across
`transforms`, `ccr`, `hf_cache`, `relevance`, `rollout` and `tokenizer`. What
this crate version carries is the **core** delta - 66 files, +9,539/−1,318
across 77 commits - not the fork's proxy work, which Aphrodite does not link.

The feature: the SQLite CCR store gains a **space** bound and stops charging
retrieval for store size. `SqliteCcrStore::open_with_limits` takes optional
entry and byte caps and evicts least-recently-accessed rows past either one;
`get` no longer runs the expiry sweep on every call, and the sweep is now
index-driven. Measured `get` on a missing hash over a 300k-row store:
**37.40 ms → 0.22 ms**. The parent release consumes this crate through the same
path pin, so the retention knobs land there as
`[compression] store_max_entries` / `store_max_bytes`.

### Changes

- **Feature (upstream sync)**: merge of `headroomlabs-ai/main` (`bb8d18a5`).
  Core-side fixes carried in: `smart_crusher` keeps mixed-array dict rows and
  the exact drop count with `factor_out_constants`; `search_compressor` keeps
  kept rows byte-for-byte and in input order; diff hunk headers are renumbered
  after context trimming; the tag protector keeps the enclosing tag open after
  a nested close; `kompress` runs the CPU and ONNX Runtime loader guards before
  building a session; the in-memory order queue no longer evicts re-stored
  entries; SQLite CCR TTLs are clamped; the `Bearer` scheme is matched
  case-insensitively; the Redis URL is redacted in logs.
- **Feature (retention)**: `open_with_limits(path, default_ttl_seconds,
max_entries, max_bytes)`; `None`/`0` = unlimited, normalised inside the
  constructor. Past either cap the store evicts by `last_accessed ASC,
created_at ASC` in batches of 256 rows, with exact freed rows/bytes from
  `DELETE … RETURNING LENGTH(original)`. An already-oversized DB is trimmed at
  open, not only on the next `put`.
- **Feature (telemetry)**: `max_entries()`, `max_bytes()`, `evicted_total()`
  accessors; `stats_db()` gained `max_entries`, `max_bytes`, `evicted_total`,
  `freelist_bytes` alongside the existing entry/byte counts.
- **Fix (retrieval cost)**: `get` shares the 60 s purge debounce instead of
  sweeping per call; the lookup predicate still refuses expired rows, so
  skipping a sweep delays reclamation but never serves stale content.
- **Fix (schema)**: `user_version` 2 → 3 with `ccr_entries_expiry` (expression
  index on `last_accessed + ttl_seconds`), `ccr_entries_created`,
  `ccr_entries_lru`; the sweep predicate is written as `created_at < cutoff`
  so SQLite's OR-optimisation drives both disjuncts from an index.
- **Perf (internal)**: maintained entry/byte counters make `len()` O(1) and
  keep cap enforcement off a `COUNT(*)` per write. They are per-process, so
  caps are approximate when several processes share one DB file; `stats_db`
  stays exact.
- **Fix (fork contracts preserved through the sync)**: package names stay
  `aphrodite-*` with `headroom-core` aliased by `package =`; the `ml` feature
  stays default-OFF with `transforms::kompress` / `magika_detector` gated; the
  parity comparator keeps its `#[cfg(feature = "ml")]` gate and
  `ml = ["headroom-core/ml"]`; `compute_key` stays at **40 hex chars** (160
  bits) to match Aphrodite's documented marker format, where upstream
  truncates to 24 for its Python tool-injection regex; the Go
  `(string, error)` classifier fix is restored (the sync had kept upstream's
  `i < 4` error-confidence check against upstream's two-pattern list);
  `headroom-ffi` is back in `[workspace] members` (upstream had orphaned it);
  edition-2024 `set_var`/`remove_var` calls route through documented
  `test_support` shims; nightly style lints are suppressed at the crate root.
- **Chore (deps, carried from post-0.1.3)**: `fc58ee39` (fastembed → 5) and
  `41e3602b` (crate-version refresh), both `Cargo.toml` only.
- **Fix (fork workspace build)**: `headroom-parity` used
  `headroom_core::transforms::kompress`, which headroom-core gates behind
  `ml` (default-OFF), so a plain `cargo build` in `vendor/headroom` had failed
  since 2026-08-07 with `could not find kompress in transforms` - invisible to
  CI, which only builds `headroom-core` through the parent's path dependency.
  The comparator is now `#[cfg(feature = "ml")]` and parity declares
  `ml = ["headroom-core/ml"]`, so the fork workspace builds without the ML
  stack and the kompress leg is opt-in.

### Verification

- Fork range: `aphrodite-v0.10.0` (`02706ea1`) .. `724da7bf` - the two
  dependency commits, the CCR-store change, the upstream sync (`bb8d18a5`),
  and the merge repairs.
- `cargo test -p aphrodite-headroom-core` ✅ 996 lib tests + every integration
  target green, including 9 store tests: entry-cap LRU order, byte-cap
  eviction, overwrite-is-not-a-new-row, trim-at-open, index-plan assertion,
  stats caps.
- `cargo clippy -- -D warnings` ✅ for `headroom-core` and `headroom-parity` in
  the fork workspace (the MSRV lint that blocked it is fixed in this cycle);
  `cargo fmt --check` ✅ `headroom-core`. `headroom-parity` remains
  non-conformant with the inherited `hard_tabs` config - pre-existing, whole
  file, deliberately not rewritten here.
- `cargo build` ✅ fork workspace default-members, now including
  `headroom-ffi`.
- Pins: 11 exact requirements - `ort =2.0.0-rc.12`, `fastembed =5.17.3` (a
  caret there resolves 5.17.4, which demands `ort rc.13` and makes the graph
  unresolvable) and the nine `tree-sitter*` grammars that must match the
  Python reference wheels. `cargo metadata --features ml` resolves cleanly.
- Parent-side proof: 300k-row store trimmed to 5000 with
  `evicted_total 295000`; median `POST /retrieve` 0.22 ms.

### What Ships

- `aphrodite-headroom-core` **0.2.0** - the crate version this release pins.
  Two steps are still outstanding and neither is automated by CI:
  **publishing it to crates.io** (the `Publish-Headroom-Core` step is gated on a
  `workflow_dispatch` input that no longer exists, so the chain never publishes
  the fork) and **creating the fork tag `aphrodite-v0.11.0`**. The parent
  gitlink already floats to `724da7bf`, the commit carrying 0.2.0.
- The main `Aphrodite/v1.8.0` release ships the 4-target binary matrix - see
  [`release-notes-v1.8.0.md`](release-notes-v1.8.0.md).

### Links

- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **Fork tag compare**: https://github.com/PlayForm/Headroom/compare/aphrodite-v0.10.0...aphrodite-v0.11.0
- **crates.io**: https://crates.io/crates/aphrodite-headroom-core
- **Parent release note**: [`release-notes-v1.8.0.md`](release-notes-v1.8.0.md)
