> **Draft** - fork release note for the `v1.8.0` release cycle. The fork DOES
> bump this cycle (`aphrodite-headroom-core` 0.1.3 → 0.2.0); `{PENDING}` marks
> the fork commit/tag once the release task creates them.

## Headroom Fork: aphrodite-headroom-core 0.2.0 · fork tag aphrodite-v0.11.0

### Summary

The PlayForm/Headroom fork ships its first functional change since 0.1.3: the
SQLite CCR store gains a **space** bound and stops charging retrieval for store
size. `SqliteCcrStore::open_with_limits` takes optional entry and byte caps and
evicts least-recently-accessed rows past either one; `get` no longer runs the
expiry sweep on every call, and the sweep is now index-driven. Measured `get`
on a missing hash over a 300k-row store: **37.40 ms → 0.22 ms**. The parent
release (`Aphrodite/v1.8.0`) consumes this crate through the same path pin, so
the retention knobs land there as `[compression] store_max_entries` /
`store_max_bytes`.

This crate version must be **published to crates.io before the parent tag**:
the parent's `Publish-Aphrodite` leg resolves its `path + version` dependency
with `path` stripped, and the CI chain never publishes the fork itself - a fork
version that is not live makes that leg fail.

### Changes

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

- Fork range: `aphrodite-v0.10.0` (`02706ea1`) .. `20be515b` - the two
  dependency commits plus the CCR-store change.
- `cargo test -p aphrodite-headroom-core --lib` ✅ 900 passed / 0 failed / 1
  ignored, including 9 store tests: entry-cap LRU order, byte-cap eviction,
  overwrite-is-not-a-new-row, trim-at-open, index-plan assertion, stats caps.
- `cargo clippy -- -D warnings` ✅ for `headroom-core` and `headroom-parity` in
  the fork workspace (the MSRV lint that blocked it is fixed in this cycle);
  `cargo fmt --check` ✅ `headroom-core`. `headroom-parity` remains
  non-conformant with the inherited `hard_tabs` config - pre-existing, whole
  file, deliberately not rewritten here.
- `cargo build` ✅ fork workspace default-members (the parity `ml` gate fix).
- Parent-side proof (`Aphrodite/v1.8.0`): 300k-row store trimmed to 5000 with
  `evicted_total 295000`; median `POST /retrieve` 0.22 ms.

### What Ships

- `aphrodite-headroom-core` **0.2.0** on crates.io, published manually before
  the parent tag (the CI chain cannot publish it), fork tag
  `aphrodite-v0.11.0` on the commit carrying 0.2.0, parent gitlink floated to
  that commit.
- The main `Aphrodite/v1.8.0` release ships the 4-target binary matrix (see
  `vNEXT-draft.md`).

### Links

- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **Fork tag compare**: https://github.com/PlayForm/Headroom/compare/aphrodite-v0.10.0...aphrodite-v0.11.0
- **crates.io**: https://crates.io/crates/aphrodite-headroom-core
- **Parent release note**: [`vNEXT-draft.md`](vNEXT-draft.md)
