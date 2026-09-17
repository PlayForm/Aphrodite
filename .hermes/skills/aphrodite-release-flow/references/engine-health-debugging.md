# Engine Health & Debugging (live engine, on-disk)

Companion to `plugin-lifecycle.md` (install layouts, key sourcing). This
file covers verifying a RUNNING engine and diagnosing why pieces are down.

## Probe battery (in order)

**API tools first:** `aphrodite_stats` → `aphrodite_test` (quick, then full
if quick passes) → `aphrodite_rebuild` → `aphrodite_catalog` →
`aphrodite_diff` → `aphrodite_files` → `aphrodite_directive list`.

- `catalog` populated + `diff` empty is EXPECTED: the turn-history layer
  tracks conversation-turn compression only; test-generated entries store
  at turn 1 but never register as turns. Not a compression failure.
- `files` empty is expected until the session reads a file.
- `directive list`: compare `available` against the last known set - a new
  directive in the list means a version bump added engine features.

**Terminal probes** (what the user means by "debug" - real system probing,
not just the API tools):

- `ps aux | grep -i aphrodite` - proxy processes; none + stats
  `alive:false` = proxies simply not running.
- `ls -la ~/.hermes/aphrodite/` and `find ~/.hermes/aphrodite -type f` -
  the data dir contents are the first truth about what exists.
- Read `proxy-stderr.log` - it records the proxy spawn failure reason.
- `find ~/.hermes ~/Library/'Application Support' /tmp -name 'ccr.db'`,
  then `sqlite3 <db> '.tables'` + row counts per table.
- `env | grep -i aphrodite` - APHRODITE_API_KEY, APHRODITE_HOME
  (Python-side data-dir override only), APHRODITE_DIRECTIVES_DIR.

## On-disk map (~/.hermes/aphrodite/)

- `hotreload/libaphrodite_hermes.dylib.<pid>.<n>` - the hot-reloaded Rust
  engine. A version bump swaps this file and restarts the engine; session
  counters reset to zero, which is normal.
- `ccr.db` - created lazily on first write. **0 bytes while catalog/stats
  show entries = inline entries are session-scoped in-memory**; without the
  proxy layer nothing flushes to disk. Never promise cross-session
  retrieval in inline-only mode.
- `proxy-stderr.log` - proxy spawn failures ("no API key configured" =
  key missing; sourcing in plugin-lifecycle.md).
- `aphrodite` binary + `aphrodite.toml` - README-documented auto-downloaded
  binary and config; their absence = proxies cannot start → inline
  fallback. Missing binary also results from `cargo clean` dangling a dev
  symlink (plugin then re-downloads the released binary).
- Config lives at `~/.hermes/aphrodite/aphrodite.toml`, NOT
  `~/.hermes/aphrodite.toml` (that path is doc drift). No config file →
  engine runs on code defaults; the threshold_pct in stats reveals which
  defaults were in effect.

## Failure-chain decision table

| Symptom | Meaning |
|---|---|
| stats `proxies.alive:false` + no processes | proxies never launched; read proxy-stderr.log for why |
| proxy-stderr.log "no API key configured" | token proxy needs APHRODITE_API_KEY / toml `api_key` (key sourcing in plugin-lifecycle.md) |
| `aphrodite` binary missing | never downloaded, or dev symlink broken by cargo clean |
| catalog has entries, ccr.db 0 bytes | inline memory-only mode (proxies down) - expected; entries vanish on restart |
| `~/Library/Application Support/aphrodite/ccr.db` (large, old mtime) | orphaned legacy db from an old standalone install - not the live store; leave it |
| `aphrodite_rebuild` version ≠ stats version | stale dylib; rebuild via `cargo build --release -p aphrodite`, hot-reloads on mtime change |

## Path resolution

- `_data_dir()` honors `APHRODITE_HOME` override, else `~/.hermes/aphrodite`
  - Python side only. The Rust proxy/dylib keep resolving `aphrodite.toml`
  and `ccr.db` on their own; overrides set for the Python side do not move
  the Rust paths.
