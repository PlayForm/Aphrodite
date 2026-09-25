# Engine Health & Debugging (decision-tree runbook)

Companion to `plugin-lifecycle.md` (install layouts, key sourcing). This file
is the decision tree for verifying a RUNNING engine and diagnosing why pieces
are down: symptom → evidence to collect → bounded likely causes → safe repair
→ exit criteria. Troubleshooting begins with ONE observed symptom, never a
suspected root cause.

## Bounded-investigation progression (fixed order)

1. **Classify the symptom** - pick one class from the tree below before any
   repair.
2. **Collect immutable evidence** - read-only probes only (see Probe
   battery).
3. **State the expected contract** - one sentence: "For input X, component Y
   must produce output Z without invoking subsystem W."
4. **Run the smallest discriminating test** - change ONE dimension (never
   env vars, thresholds, source, branch, and plugin install together).
5. **Choose one bounded repair** - name exact files/state modified, expected
   new observation, reversal method, and validation that succeeds before the
   next repair.
6. **Update the contract** - if source behavior differed from documented
   behavior, update the canonical owner skill + test matrix; never an
   unstructured note in a random reference.

## Probe battery (in order)

**API tools first:** `aphrodite_stats` → `aphrodite_test` (quick, then full
if quick passes) → `aphrodite_rebuild` → `aphrodite_catalog` →
`aphrodite_diff` → `aphrodite_files` → `aphrodite_directive list`.

- `catalog` populated + `diff` empty is EXPECTED: the turn-history layer
  tracks conversation-turn compression only; test-generated entries store at
  turn 1 but never register as turns. Not a compression failure.
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
- `find ~/.hermes ~/Library/'Application Support' /tmp -name 'ccr.db'`, then
  `sqlite3 <db> '.tables'` + row counts per table.
- `env | grep -i aphrodite` - APHRODITE_API_KEY, APHRODITE_HOME
  (Python-side data-dir override only), APHRODITE_DIRECTIVES_DIR.

## Symptom decision tree

### S1 - Proxy down (`proxies.alive:false`, no processes)

**Evidence to collect**

- `ps aux | grep -i aphrodite` (any process?).
- `proxy-stderr.log` contents (the spawn failure reason).
- `env | grep -i aphrodite` (key presence, redacted).

**Bounded likely causes (discriminating test → safe repair)**

1. Proxies never launched - no processes + `alive:false` → read
   `proxy-stderr.log` for the reason.
2. Key missing - log says "no API key configured" → the proxy needs
   `APHRODITE_API_KEY` or toml `api_key`; Hermes' provider config is NOT
   reused (sourcing: `plugin-lifecycle.md`).
3. Binary missing - never downloaded, or `cargo clean` dangled a dev symlink
   → `_ensure_binaries` re-downloads the released binary; or re-create the
   dev symlink.

**Safe repair**

- Set the key from a defined source; re-link or re-download the binary;
  restart the session. Verify only presence + redacted fingerprint, never
  print the key.

**Exit criteria**

- `proxies.alive:true`; `proxy-stderr.log` free of spawn errors;
  `aphrodite_test` round trip passes.

**Prohibited**

- Offering proxy restarts as the default fix (proxies-down + inline-only is
  the user's ACCEPTED state); printing or logging the key.

### S2 - Inline-only mode (entries vanish on restart)

**Evidence to collect**

- `aphrodite_catalog` entry count vs `ccr.db` byte size.
- `proxy-stderr.log` (why the proxy layer is absent).

**Bounded likely causes**

1. Inline entries are session-scoped in-memory: `ccr.db` is 0 bytes while
   catalog/stats show entries. Without the proxy layer nothing flushes to
   disk.

**Safe repair**

- None needed - expected degraded mode. Document it.

**Exit criteria**

- User accepts inline-only; never promise cross-session retrieval in this
  mode.

**Prohibited**

- Treating the empty db as data loss; restarting proxies against the user's
  accepted state.

### S3 - Version mismatch (`aphrodite_rebuild` version ≠ stats version)

**Evidence to collect**

- `aphrodite_stats` loaded version; `aphrodite_rebuild` expected-symbol
  check; `ls -la ~/.hermes/aphrodite/hotreload/` (which dylib copies exist).

**Bounded likely causes (discriminating test → safe repair)**

1. Stale dylib/process - the copied-into-place new dylib only takes effect
   on the next load; the live session holds the OLD dylib until Hermes
   restarts → restart the session, re-probe.
2. Symbol drift - a removed `#[no_mangle]` export left in the expected-
   symbol list makes `aphrodite_rebuild`/dlsym die with "missing an expected
   symbol" against the fresh build → grep for the symbol name in the
   tooling/check code, not just the crate's lib.rs; update the expected list.

**Safe repair**

- `cargo build --release -p aphrodite -p aphrodite-hermes` (hot-reloads on
  mtime change), restart the process, re-probe.

**Exit criteria**

- A FRESH process reports the NEW version; `aphrodite_rebuild` cross-check
  matches.

**Prohibited**

- Concluding behavior from the old process; hand-editing generated bindings.

### S4 - Marker does not resolve / raw `<<<CCR:` marker

**Evidence to collect**

- The exact marker string; whether retrieval is inline (S2) or proxied;
  `aphrodite_search` hits.

**Bounded likely causes (discriminating test → safe repair)**

1. Retrieval path issue - resolve the marker through the canonical retrieval
   route once (never re-read the source file behind it); truncated hashes do
   not resolve (exact match only).
2. Inline-only mode with a session-scoped entry - expected (S2).
3. Malformed/unknown marker - resolver must return a readable non-destructive
   diagnostic, never arbitrary content.

**Exit criteria**

- Marker resolves to normalized source, or the failure is classified as one
  of the bounded causes above.

**Prohibited**

- Re-compressing a retrieval response (marker-resolution loop); treating a
  malformed marker as a valid retrieval key.

### S5 - Orphan legacy db

**Evidence to collect**

- `find ~/.hermes ~/Library/'Application Support' -name 'ccr.db'` with
  sizes and mtimes.

**Bounded likely causes**

1. `~/Library/Application Support/aphrodite/ccr.db` (large, old mtime) - a
   legacy standalone install's store, NOT the live store.

**Safe repair**

- Leave it alone.

**Exit criteria**

- The live store (`~/.hermes/aphrodite/ccr.db`) is the one being written.

## On-disk map (~/.hermes/aphrodite/)

- `hotreload/libaphrodite_hermes.dylib.<pid>.<n>` - the hot-reloaded Rust
  engine. A version bump swaps this file and restarts the engine; session
  counters reset to zero, which is normal.
- `ccr.db` - created lazily on first write. **0 bytes while catalog/stats
  show entries = inline entries are session-scoped in-memory** (S2); without
  the proxy layer nothing flushes to disk.
- `proxy-stderr.log` - proxy spawn failures ("no API key configured" = key
  missing; sourcing in `plugin-lifecycle.md`).
- `aphrodite` binary + `aphrodite.toml` - auto-downloaded binary + config;
  their absence = proxies cannot start → inline fallback. Missing binary also
  results from `cargo clean` dangling a dev symlink (the plugin then
  re-downloads the released binary).
- Config lives at `~/.hermes/aphrodite/aphrodite.toml`, NOT
  `~/.hermes/aphrodite.toml` (that path is doc drift). No config file → the
  engine runs on code defaults; the `threshold_pct` in stats reveals which
  defaults were in effect.

## Path resolution

- `_data_dir()` honors the `APHRODITE_HOME` override, else
  `~/.hermes/aphrodite` - Python side only. The Rust proxy/dylib keep
  resolving `aphrodite.toml` and `ccr.db` on their own; overrides set for the
  Python side do not move the Rust paths.

## Failure-chain table (compact)

| Symptom                                                             | Meaning                                                                                    |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| stats `proxies.alive:false` + no processes                          | proxies never launched; read proxy-stderr.log for why                                      |
| proxy-stderr.log "no API key configured"                            | token proxy needs APHRODITE_API_KEY / toml `api_key` (sourcing in plugin-lifecycle.md)     |
| `aphrodite` binary missing                                          | never downloaded, or dev symlink broken by cargo clean                                     |
| catalog has entries, ccr.db 0 bytes                                 | inline memory-only mode (proxies down) - expected; entries vanish on restart               |
| `~/Library/Application Support/aphrodite/ccr.db` (large, old mtime) | orphaned legacy db from an old standalone install - not the live store; leave it           |
| `aphrodite_rebuild` version ≠ stats version                         | stale dylib; rebuild via `cargo build --release -p aphrodite`, hot-reloads on mtime change |
