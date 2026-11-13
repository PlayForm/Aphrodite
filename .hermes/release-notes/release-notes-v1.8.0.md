> **Released** - `Aphrodite/v1.8.0`, tag on `Current` at `8eb2d95f`, with plugin
> `v2.3.1` and `aphrodite-headroom-core` `0.2.0`. The tag was re-cut from
> `b3fcd565` after the publish-chain fix described under Infrastructure; the
> release source is identical apart from `.github/workflows/Publish.yml`.

**[Compare Aphrodite/v1.7.0...Aphrodite/v1.8.0](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.7.0...Aphrodite/v1.8.0)**

## Aphrodite v1.8.0 💋 Plugin v2.3.1

### Summary

`1.8.0` is two releases in one: the CCR store gains a **space** bound, and the
vendored headroom core is resynced with upstream.

The store half: until now `ccr_ttl_seconds` was the only bound, so a burst of
compression inside one TTL window could grow `ccr.db` without limit - and
because the expiry sweep ran un-debounced on every `get`, a missing-hash
`POST /retrieve` cost **37.40 ms** on a 300k-row store behind the store's single
connection mutex. The sweep is now debounced and index-driven (**0.22 ms** on
the same data), `[compression] store_max_entries` / `store_max_bytes` evict
least-recently-accessed rows past either cap, and two metrics that operators
read as store occupancy turned out to be fiction: `fill_pct` is compression
headroom (90.0 on a fresh process with an _empty_ store) and `/stats`'
`ccr_store.entries` came from atomics nothing incremented. Both are now labelled
or made live.

The sync half: `headroomlabs-ai/main` was merged into the fork and
`aphrodite-headroom-core` moves `0.1.3 → 0.2.0`. What reaches this binary is
`headroom-core` only - 66 files, +9,539/−1,318 across 77 commits - so the
compression, CCR and detection paths carry upstream's fixes while upstream's
_proxy_ work stays out of scope (see "What this release does not include").

This release answers the first half of the store-growth report in #53 (bounded
retention); per-session store isolation - its second ask - is not in `1.8.0` and
is tracked separately.

### Changes

- **Feature (headroom sync, `aphrodite-headroom-core` 0.2.0)**: the fork merged
  upstream `main` (merge-base `c365c7ff` → `bb8d18a5`) and carries the fixes
  that matter to a compression-only consumer - `smart_crusher` keeps
  mixed-array dict rows and the exact drop count with `factor_out_constants`,
  `search_compressor` keeps kept rows byte-for-byte and in input order, diff
  hunk headers are renumbered after context trimming, the tag protector keeps
  the enclosing tag open after a nested close, `kompress` runs the CPU and ONNX
  Runtime loader guards before building a session, the in-memory order queue no
  longer evicts re-stored entries, SQLite CCR TTLs are clamped, the `Bearer`
  scheme is matched case-insensitively, and the Redis URL is redacted in logs.
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
- **Fix (telemetry, `/health`)**: `fill_pct` is
  `100 - compression_ratio_ema/20` clamped to `[1, 99]` - compression headroom,
  pinned near 90-99 by construction, not store occupancy. `/health` now returns
  `fill_pct_kind: "compression_headroom"` and `store_entries` beside it, and
  `/stats/db` carries the real capacity picture (`total_entries`,
  `total_bytes_original`, `database_size_bytes`, `freelist_bytes`,
  `max_entries`, `max_bytes`, `evicted_total`).
- **Fix (telemetry, `/stats`)**: `ccr_store.entries` / `bytes_approx` were
  backed by two `AtomicU64`s that nothing ever incremented - a full store
  reported `entries: 0`. The fields now read the live backend.
- **Feature (fork API)**: `SqliteCcrStore::open_with_limits` plus
  `max_entries()` / `max_bytes()` / `evicted_total()`; exact freed-row/byte
  accounting via `DELETE ... RETURNING LENGTH(original)`; maintained entry/byte
  counters so `len()` is O(1).
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

### What this release does not include

The upstream sync brought 1575 commits into the **fork repository**, but
Aphrodite links only `headroom-core`. Everything upstream added to
`headroom-proxy` and the Python proxy - `headroom wrap droid`, unified
multi-upstream routing (Anthropic Cloud + local oMLX), model-router shadow mode,
per-request `cost_usd` in `/stats recent_requests`, DeepSeek as its own
dashboard provider, Redis store availability in `/health` and `/metrics` - lives
in the fork but does **not** reach this binary. Read the fork's changelog for
those; they ship when the proxy itself is consumed.

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ✅ 1m39s on
  `aarch64-apple-darwin`; the CI 4-target matrix is built by `Build.yml` on the
  tag and attaches all 12 assets.
- Release chain: the tag drives `Build.yml` → `Publish.yml` with no manual
  step. Two failures were cleared on the way. The first `Build` run could not
  check out the plugin gitlink (`upload-pack: not our ref 0558bc47` - the child
  commit was not yet on `PlayForm/Aphrodite-Hermes`). `Publish-Aphrodite` then
  failed on `aphrodite-headroom-core = "^0.2.0"` because `Publish-Headroom-Core`
  reported success in 18s while publishing nothing: its publish step was gated
  on `github.event_name == 'workflow_dispatch' && inputs.publish_crates`, which
  is never true on a `workflow_run`. The step now runs on
  `steps.check.outputs.published == 'false'` alone - the job is already gated on
  the release tag and on `Test` - and the tag was re-cut to carry the fix.
- Check / lint / format on the release line (`Current`): `cargo check
--workspace --release` ✅ · `cargo clippy -p aphrodite -p aphrodite-hermes --
-D warnings` ✅ 0 errors · `cargo fmt --all -- --check` ✅ · `pyright 1.1.411`
  on `plugins/aphrodite` ✅ 0 errors (1 pre-existing wildcard-import warning in
  the generated `_bindings.py`).
- Tests: `cargo test -p aphrodite --lib` ✅ 500 passed / 0 failed / 1 ignored ·
  `cargo test -p aphrodite-headroom-core --lib` ✅ 996 passed / 0 failed / 1
  ignored · `cargo ci-test` on `Development` ✅ 1695 passed across 31 targets ·
  `ruff check plugins/aphrodite/` ✅ · `pnpm run Check:FFI` ✅ 0 violations.
- Store probe (release binary, 300k-row store): `/health` → `fill_pct 90.0`,
  `store_entries 300000`; `--store-max-entries 5000` trims to 5000 rows with
  `evicted_total 295000`; median `POST /retrieve` 0.22 ms (pre-fix 37.40 ms on
  the same file).
- Shipped-artifact smoke test (installed `1.8.0 (b3fcd565, release)`, built
  before the publish-workflow fix and identical in source otherwise): `/health`
  → `version 1.8.0`, `fill_pct 90.0`, `fill_pct_kind compression_headroom`,
  `store_entries 0`; `/stats/db` → `max_entries 100` (the launched cap),
  `max_bytes 536870912`, `freelist_bytes 0`, `evicted_total 0`; `POST /retrieve`
  for a missing hash → 404 in 1.05 ms; listeners shut down gracefully.
- crates.io availability check before tagging: `aphrodite` / `aphrodite-hermes`
  `max_version` 1.7.0, `aphrodite-headroom-core` 0.1.3 - `1.8.0` / `1.8.0` /
  `0.2.0` free.
- Version track: crates + `aphrodite-hermes` pin + `package.json` → `1.8.0`;
  `plugin.yaml` + `install_message` → `2.3.1`; fork crate + parent pin →
  `0.2.0`; README badges v1.8.0 / v2.3.1. `BINARY_VERSION` moves to `1.8.0`
  only after the release assets exist - it names a release, so it is set last.
- Known pre-existing reds, not introduced here: `pnpm run FormatCheck` flags two
  untouched files (`.githooks/post-checkout`, `.githooks/pre-commit` - the
  CRLF/eol trap for extensionless hooks); the plugin test
  `test_data_dir_honours_aphrodite_home` fails identically at the pre-change
  tree.

### What Ships

`Build.yml`'s 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.3.1                                                        | Hermes (standalone repo)                 |
| `aphrodite-headroom-core` 0.2.0                                      | crates.io, by `Publish-Headroom-Core`    |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.7.0...Aphrodite/v1.8.0
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Store-growth report**: [#53](https://github.com/PlayForm/Aphrodite/issues/53)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
