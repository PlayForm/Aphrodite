> **Draft** - planned 1.6.0 binary release note; `{VERSION}` / `{PLUGIN_VERSION}`
> placeholders mark values to fill at release time.

**[Compare Aphrodite/v1.5.1...Aphrodite/{VERSION}](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.5.1...Aphrodite/{VERSION})**

## Aphrodite 1.6.0 💋 Plugin v{PLUGIN_VERSION}

### Summary

1.6.0 makes config failure visible and config changes live. A found-but-broken
`aphrodite.toml` used to fall back to defaults with zero indication (issue
#38): the parse-failure warnings existed but `tracing::warn!` is a silent
no-op in the Hermes dylib path (the Python host installs no Rust tracing
subscriber) and the engine binary loaded config before initializing its own.
That failure class - config problems indistinguishable from config not set -
is now diagnosed on every surface. The release also fixes the real
`is_char_boundary` panic behind the report (a byte-slice of a UTF-8 header
value in the proxy's dev-mode request log) and adds opt-in config
auto-reload for the dylib.

### Changes

- **Fix (issue #38, config diagnostics)**: parse-failure warnings now fall
  back to **stderr** when no tracing subscriber exists (the Hermes dylib
  path), so a broken config is visible in the host log / proxy-stderr
  instead of only in the binary's string table. `aphrodite_stats` now
  reports **`config_error`** - e.g.
  `"/path/aphrodite.toml: TOML parse error at line 1…"` - so "defaults in
  effect (parse failed)" is one call away from being told apart from
  "config not set". Regression tests cover the broken-file warn + record
  path and lock in that multibyte TOML comments parse (an em dash in a
  comment is valid TOML and was never the trigger). Fixes #38.
- **Fix (char-boundary panic)**: the proxy's dev-mode header logging sliced
  header values at byte 80 (`&val[..80]`), panicking with
  `is_char_boundary` on multibyte header values - the literal panic string
  reported in #38. Truncation is now char-boundary-safe
  (`truncate_char_safe`).
- **Fix (silent reload)**: `aphrodite_reload` discarded a found-but-broken
  TOML silently (`.parse().ok()`); it now warns on a real surface, records
  `config_error` on the state, and echoes it in the response.
- **Feature (opt-in config auto-reload)**: `[compression] auto_reload =
true` (default off, env `APHRODITE_AUTO_RELOAD`) watches `aphrodite.toml`
  (same notify-based watcher as the engine binary) and re-applies config
  fields live on save - thresholds, flow budget, poll-worker, chain-split
  knobs. Only config fields mutate: session CCR state, directive selection,
  and telemetry are preserved; a broken edit keeps the previous values and
  surfaces in `config_error`; setting the key back to `false` stops the
  watcher.
- **Chore (release prep)**: README badges moved to the 1.6.0 / plugin
  v2.2.0 values; the shipped `aphrodite.toml` template gains the
  `auto_reload` key; embedded-template / shim drift-guards pass.

### Infrastructure

- Tests: `cargo test -p aphrodite --lib` ✅ (398 passed, 0 failed, 1
  ignored)
- Tests: `cargo test -p aphrodite-hermes --lib` ✅ (55 passed)
- Lint: `cargo clippy -p aphrodite -p aphrodite-hermes --lib` ✅ (only
  pre-existing manifest notes)
- FFI: build.rs "header unchanged" - committed `_bindings.py` matches the
  ABI (no C-surface drift)
- Docs: `npx prettier --check` on edited `.hermes/**/*.md` ✅
- Live probe (dylib): `aphrodite_stats` dispatch reports
  `auto_reload: true, config_error: null` from the installed dylib;
  watcher round-trip verified (threshold re-applied on file change, ~2s)

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v{PLUGIN_VERSION}                                             | Hermes (standalone repo)                 |

`aphrodite-headroom-core` stays at 0.1.3 (published with 1.5.1; the fork
holds 2 dependency-update commits post-0.1.3 - fastembed→5, crate version
refresh - carried in the fork's RELEASE-CYCLE ledger, no version bump in
this release).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.5.1...Aphrodite/{VERSION}
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
