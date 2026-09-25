# Engine assets: shipped config defaults + directive materialization

How the Aphrodite engine's config and directive ASSETS are stored, shipped,
and materialized - needed before touching defaults or directives for a
release, a benchmark, or a restore.

## The shipped-defaults TRIO must agree

Three places carry the same config default; they drift independently:

1. **Shipped template** (`crates/<crate>/templates/aphrodite.toml` + the
   repo-root `aphrodite.toml.example`) - what users copy.
2. **Docs** (`docs/config/env-vars.md` default column) - what the docs
   promise.
3. **Code fallback consts** (`config_loader.rs` `get_usize`/`get_u64`
   defaults, `proxy.rs` consts) - what runs when the config key is ABSENT.

Designed defaults (v0.9.8 baseline, still documented): `engine_threshold_pct
= 45`, `tool_threshold_token = 512`, `tool_threshold_cache = 4096`,
`terminal_threshold = 1024`, `inline_threshold = 2048`, protect 2/5, min_msgs 8. `engine_threshold_pct = 100` = "effectively disable engine compression"
(escape hatch; a persisted disable flag may ship in the template while
the CODE default stays 45 - restore the designed value, keep the 100+
escape documented). When restoring defaults: edit all three, then grep the
code consts too - a template/docs fix alone leaves the code fallback drifted.
Check PER KEY, not per file: after a template+docs restore the code
fallback can still hold a drifted value (config_loader.rs
`tool_threshold_token` fell back to 4096 while docs/template say 512 -
the dylib path's aggressive default) - grep each designed key in
config_loader.rs/proxy.rs, not just the file pair. Hardcoded values inside
`#[cfg(test)]` blocks (e.g. a "90" in `test_override`) are harmless -
confirm the block is a test before treating a value as a production
override.
Verify a hot-reload took effect by the watcher's "config reloaded" line in
the proxy log showing the applied thresholds.

## Session snapshot vs hot-reloaded proxy (config loading semantics)

- The DYLIB reads its config ONCE at session handshake: `aphrodite_stats`
  reports the SESSION-START snapshot (threshold_pct, tool/terminal
  thresholds), not the current file. The standalone proxy hot-reloads via
  the watcher ("config reloaded" log line), so after a config restore the
  proxy and the running session's dylib DISAGREE until the next session
  starts. A stats read showing the old thresholds right after a restore is
  expected, not a failed restore - verify pickup with a FRESH session, not
  the current one.
- **Reset a runtime config to the plugin-provided default**: the runtime
  file is the setup-MATERIALIZED template, so a diff against
  `crates/<crate>/templates/aphrodite.toml` legitimately shows ONLY the
  resolved port placeholders (`127.0.0.1:9797` where the template has
  `{cache_port}`) - everything else should be byte-identical. Keep those
  two resolved lines when restoring; a byte-faithful copy of the template
  over the runtime file would wipe the setup's port resolution.

## Known preview-routing quirks (from example captures)

- `build_error` previews render through the `build_output` template format
  (a rustc E0308 marker shows `[build:1E 0W 30L]`, not the
  `[error:{code} {loc}]` shape) - a real template-routing inconsistency,
  not a classifier failure.
- Content classification is CONTENT-typed, not envelope-typed: a Hermes
  tool_result whose payload is JSON should classify `json` (the json
  preview template exists for that); the caller's explicit `type` hint
  wins by design (caller-hint-wins), so a probe passing `type=tool_result`
  gets `tool_result` - read the code's intent before calling that a bug.

## Directive materialization chain

- **Setup-installed source = the compiled-in builtins**
  (`crates/<crate>/src/builtin_directives/*.md`, `include_str!`),
  materialized into the runtime home (`~/.hermes/aphrodite/directives/`) on
  install. THIS is what users get - put directive content there, not in a
  repo-root `./directives/` folder.
- Repo-root `./directives/` is only a DEV-CWD discovery candidate (checked
  before the runtime home when the binary runs from the repo). It is a
  supplementary source, not the setup-installed set.
- After editing the builtins, the runtime home holds STALE copies (they are
  written once at materialization). Remove the stale files - the engine
  re-materializes them from the new builtins on the next start.
- A merge of directive versions: keep the richer superset, preserve
  formatting conventions (prettier-normalized common-markdown), teach only
  retrieval vocabulary (never the mechanism).
