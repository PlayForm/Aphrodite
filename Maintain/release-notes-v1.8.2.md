**[Compare Aphrodite/v1.8.1...Aphrodite/v1.8.2](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.1...Aphrodite/v1.8.2)**

## Aphrodite v1.8.2 💋 Plugin v2.3.3

### Summary

`1.8.2` closes the last unbounded mutexes on the hook dispatch path. `1.7.0`
gave every session its own state mutex and a bounded (~100 ms) fail-open
acquire, which fixed the shared-gateway wedge; it did not cover the bookkeeping
locks the dispatch path also crosses. Issue #60 documented the result: thread
dumps with `hermes-hook-_dispatch` threads parked inside `_call_json`'s plain
`ctypes` call - a Rust-side block the Python wrapper cannot time out - three
incidents in one day, each ending in a gateway restart. Hermes abandons a hook
worker that outlives `plugins.hook_callback_timeout`; the abandoned worker keeps
whatever mutex it held, forever, and every later hook queues behind it.

Four sites were still unbounded. The registry lock in `session_state()` was held
across `or_insert_with(|| config_template().clone())`, so first init ran
`Config::load()` TOML I/O and the config-watcher spawn **under** the registry
lock. `last_session()` and the core logger's `note_root_session` /
`current_root_session` took their `LAST_SESSION` / `ROOT_SESSION` mutexes
unbounded on every dispatch. And `record_session` held the `SESSION_PARENTS`
guard across `session_chain()`, which re-locks that same map - `std::sync::Mutex`
is not reentrant, so **every subagent `pre_llm_call`** parked its hook thread
permanently on the second lock. That last one is the subagent-shaped wedge:
delegated sessions hit it first, and a parent session looks healthy while its
children never return.

All of them now go through one shared primitive, and each skip degrades to the
cheapest correct answer instead of blocking: a wedged `LAST_SESSION` still
persists `session.current` (the file `last_session()` falls back to), a wedged
flag cache still reads the flag file uncached, a wedged chain map treats the
session as its own root, and a wedged registry hands the caller a throwaway
state seeded from the config template - the same contract `with_shared` already
had.

### Changes

- **Fix (deadlock)**: `record_session` scopes the `SESSION_PARENTS` insert so the
  guard is dropped before `session_chain()` re-locks the map. Regression test
  `test_record_session_with_parent_does_not_deadlock` walks root → child →
  grandchild; before the fix it hangs, which is the failure mode it guards.
- **Fix (bounded locks)**: new `crates/aphrodite/src/sync.rs` exposes
  `aphrodite::sync::bounded` (`try_lock` in a loop behind a ~100 ms wall-clock
  deadline, poison recovered). The bridge's `lock_bounded` delegates to it
  rather than carrying its own copy, and the bridge debug module's
  `LAST_SESSION`, `SESSION_PARENTS`, `FLAG_CACHE` plus the core logger's
  `ROOT_SESSION` all use it.
- **Fix (registry)**: `session_state()` resolves `config_template()` before
  taking the `SESSIONS` lock (idempotent through its `OnceLock`), returns a
  throwaway state on registry contention, and `for_each_session` takes the
  registry read bounded, reporting "keep watching" so a config reload retries on
  the next filesystem event instead of the watcher stopping silently.
- **Version track**: `crates/aphrodite` + `crates/aphrodite-hermes` + the
  `aphrodite-hermes` dependency pin + `package.json` → `1.8.2`; README release
  badge → v1.8.2. `plugin.yaml` `2.3.2 → 2.3.3` with its `install_message`
  header and the plugin README badge. `aphrodite-headroom-core` stays `0.2.1`
  (already on crates.io, so `Publish-Headroom-Core` skips).
- **Docs**: `CHANGELOG.md` gains the `v1.8.2` entry; `SECURITY.md` and
  `Maintain/SECURITY.md` name the `1.8.2` / `2.3.3` pairing.

### What this release does not include

No compression, classifier, preview, store or proxy behaviour changes. The
`1.8.1` marker arithmetic, the compile-time `aphrodite_debug` gate, the `1.8.0`
retention caps and the headroom fork's compression/detection code are carried
over untouched; the fork does not move at all this cycle.

### Infrastructure

- crates.io availability before tagging: `aphrodite` `max_version` 1.8.1,
  `aphrodite-hermes` `max_version` 1.8.1 → `1.8.2` / `1.8.2` free;
  `aphrodite-headroom-core` 0.2.1 already published, so its job short-circuits.
- Toolchain: nightly `1.102.0-nightly (6c709eb42 2026-11-25)`; `rustfmt --check`
  clean on every touched file, so the bump introduces no formatting drift.
- Gates on the final release tree: `cargo check -p aphrodite -p aphrodite-hermes
  --all-targets` ✅ · `cargo build --release -p aphrodite -p aphrodite-hermes` ✅
  1m41s · `cargo clippy --lib -p aphrodite -p aphrodite-hermes -- -D warnings` ✅
  (the CI invocation) · `cargo test -p aphrodite -p aphrodite-hermes` ✅ 580
  passed (500 + 16 + 2 + 62), 0 failed, 1 ignored.
- Release chain: `Aphrodite/v1.8.2` drives `Build.yml` (Release → 4-target
  matrix → `Finalize`, which regenerates the child's in-tree `SHA256SUMS.txt`,
  pushes it to `plugins/aphrodite`, and tags the child `v2.3.3`); `Publish.yml`
  then chains on that run to publish `aphrodite` and `aphrodite-hermes` `1.8.2`
  and bump the parent gitlink.
- `BINARY_VERSION` moves to `1.8.2` after the release assets exist - it names a
  release, so it is set last.

### What Ships

`Build.yml`'s 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.3.3                                                        | Hermes (standalone repo, `v2.3.3` tag)   |
| `aphrodite` / `aphrodite-hermes` 1.8.2                               | crates.io, by `Publish-Aphrodite`        |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.1...Aphrodite/v1.8.2
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Issue**: https://github.com/PlayForm/Aphrodite/issues/60
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
