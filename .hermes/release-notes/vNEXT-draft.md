> **Draft** - planned `v1.8.0` binary release note. Finalized copy is staged as
> `release-notes-v1.8.0.md` in this directory at tag time; this draft tracks
> the same content for the next release cycle. `{PENDING}` marks a value that
> only exists after the release task runs.

**[Compare Aphrodite/v1.7.0...Aphrodite/v1.8.0](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.7.0...Aphrodite/v1.8.0)**

## Aphrodite v1.8.0 💋 Plugin v2.3.1

### Summary

`1.8.0` bounds the CCR store in **space**, not just in time, and stops
retrieval from paying for store size. Until now `ccr_ttl_seconds` was the only
bound: a burst of compression inside one TTL window could grow `ccr.db` without
limit, and because the expiry sweep ran un-debounced on every `get`, a
missing-hash `POST /retrieve` cost **37.40 ms** on a 300k-row store - behind
the store's single connection mutex. The sweep is now debounced and
index-driven (**0.22 ms** on the same data), `[compression] store_max_entries`
/ `store_max_bytes` evict least-recently-accessed rows past either cap, and two
metrics that operators were reading as store occupancy turned out to be
fiction: `fill_pct` is compression headroom (90.0 on a fresh process with an
_empty_ store) and `/stats`' `ccr_store.entries` came from atomics nothing
incremented. Both are now labelled or made live.

This release answers the first half of the store-growth report in #53 (bounded
retention); per-session store isolation - its second ask - is not in `1.8.0`
and is tracked separately.

### Changes

- **Feature (store retention)**: `[compression] store_max_entries` (default
  `100000`) and `store_max_bytes` (default `536870912`; payload bytes, i.e.
  `SUM(LENGTH(original))`, not file size) evict the least-recently-accessed
  rows past either cap, batched at 256 rows per statement. `0` = unlimited.
  Env: `APHRODITE_STORE_MAX_ENTRIES` / `APHRODITE_STORE_MAX_BYTES`. Caps apply
  when the store opens - an already-oversized DB is trimmed at open - and are
  not hot-reloaded. An evicted hash degrades to a retrieval miss, exactly like
  an expired one.
- **Fix (store, retrieval cost)**: the expiry sweep no longer runs per `get`.
  It shares the 60-second debounce `put` already had (the lookup predicate
  still refuses expired rows, so correctness is unchanged) and is index-driven
  via `ccr_entries_expiry` - an expression index on
  `last_accessed + ttl_seconds` - plus `ccr_entries_created`; the sweep
  predicate was rewritten as `created_at < cutoff` so SQLite's
  OR-optimisation can drive both disjuncts from an index. Schema
  `user_version` 2 → 3.
- **Fix (telemetry, `/health`)**: `fill_pct` is `100 -
compression_ratio_ema/20` clamped to `[1, 99]` - compression headroom, pinned
  near 90-99 by construction, not store occupancy. `/health` now returns
  `fill_pct_kind: "compression_headroom"` and `store_entries` beside it, and
  `/stats/db` carries the real capacity picture (`total_entries`,
  `total_bytes_original`, `database_size_bytes`, `freelist_bytes`,
  `max_entries`, `max_bytes`, `evicted_total`).
- **Fix (telemetry, `/stats`)**: `ccr_store.entries` / `bytes_approx` were
  backed by two `AtomicU64`s that nothing ever incremented - a full store
  reported `entries: 0`. The fields now read the live backend.
- **Feature (headroom fork, `aphrodite-headroom-core` 0.2.0)**:
  `SqliteCcrStore::open_with_limits` plus `max_entries()` / `max_bytes()` /
  `evicted_total()`; exact freed-row/byte accounting via
  `DELETE … RETURNING LENGTH(original)`; maintained entry/byte counters so
  `len()` is O(1).
- **Fix (plugin loader, 2.3.1)**: a missing `aphrodite.toml` is now reported
  instead of inferred. With no config the binary falls back to a single
  token-mode listener on the first port - the second proxy never binds - and
  every `[compression]` knob, including the new retention caps, is absent.
- **Docs**: retention block in the shipped `aphrodite.toml` template; new rows
  in `docs/config/aphrodite-toml.md` and `docs/config/env-vars.md`; `/health`
  and `/stats/db` semantics in `docs/proxy/handlers.md`; corrected counter
  inventory in `docs/proxy/architecture.md`; plugin README config sample.
- **Credit**: the store-growth report in #53 (filed by @p1ksntded-oss) is what
  put the retention gap and the misleading `fill_pct` reading on the map.

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ✅ (host
  `aarch64-apple-darwin`; `{PENDING}` for the 4-target CI matrix at tag time)
- Check: `cargo check -p aphrodite -p aphrodite-hermes` ✅ 0 errors
- Tests: `cargo test -p aphrodite --lib` ✅ 500 passed / 0 failed / 1 ignored;
  `cargo test -p aphrodite-headroom-core --lib` ✅ 900 passed / 0 failed / 1
  ignored
- Lint/format: `cargo clippy -p aphrodite -p aphrodite-headroom-core --lib --
-D warnings` ✅; `cargo fmt --check` ✅ both crates; `ruff check
plugins/aphrodite/` ✅; `pnpm run Check:FFI` ✅ 0 violations
- Store probe (release binary, 300k-row store): `/health` → `fill_pct 90.0`,
  `store_entries 300000`; `--store-max-entries 5000` trims to 5000 rows with
  `evicted_total 295000`; median `POST /retrieve` 0.22 ms (pre-fix 37.40 ms on
  the same file)
- crates.io availability check: `aphrodite` / `aphrodite-hermes` `max_version`
  1.7.0, `aphrodite-headroom-core` 0.1.3 - `1.8.0` / `1.8.0` / `0.2.0` free
- Version track: crates + `aphrodite-hermes` pin + `package.json` → `1.8.0`;
  `plugin.yaml` + `install_message` → `2.3.1`; fork crate + parent pin →
  `0.2.0`; README badges v1.8.0 / v2.3.1. `BINARY_VERSION` stays at `1.7.0`
  until the `Aphrodite/v1.8.0` assets exist - bump LAST.
- Known pre-existing reds (not introduced here): `pnpm run FormatCheck` flags
  two untouched files (`.githooks/post-checkout`, `.githooks/pre-commit` - the
  CRLF/eol trap for extensionless hooks); the plugin test
  `test_data_dir_honours_aphrodite_home` fails identically at the pre-change
  tree.

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.3.1                                                        | Hermes (standalone repo)                 |

`aphrodite-headroom-core` ships **0.2.0**, which must be published to
crates.io _before_ the parent tag: `Publish-Aphrodite` resolves its
`path + version` dependency by stripping `path` on publish, so a fork version
that is not live makes that leg fail - and the CI chain never publishes the
fork itself. Fork tag `aphrodite-v0.11.0` goes on the fork commit carrying
0.2.0, and the parent gitlink must float to it before the tag.

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.7.0...Aphrodite/v1.8.0
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Store-growth report**: [#53](https://github.com/PlayForm/Aphrodite/issues/53)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
