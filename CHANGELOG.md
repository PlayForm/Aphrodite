# Changelog

## Latest

#### v0.5.62 / 1.62.8 - 2026-06-16
- **Rust fixes R1-R12**: DefaultHasher-cache-stable, inline_ccr→LRU, task_tracker tests, UTF-8 safe slice, detect_content_type log fix, retrieve-inline-check, cache-retrieve route, EMA marker length fix, content-type charset, task_tracker spawn, auto-tune noisy exclude, shutdown close-wait
- **Build monitor**: dedicated wezterm cli agent, 5s poll, writes `.hermes/build-status.json`
- **Skills v3.0**: hermes-z-execution, plan-then-delegate, dev-metrics, build-monitor, execution-blocks - all with build monitor pattern + git rule
- **CHANGELOG**: proper markdown hierarchy, no single-line entries, every release has `####` heading with date + version pair

#### v0.5.61 / 1.62.7 - 2026-06-16
- headroom fixes: workspace header (`x-headroom-workspace`) preserved in proxy
- deep integration with headroom subsystems (disk cache, benchmark, token modes)

## Aphrodite Binary

### Features and Enhancements

#### v0.5.55 / 1.61.0 - 2026-06-16
- headroom cache/benchmark/token modes
- 1-8 workers
- Hermes default provider
- `--no-telemetry`

#### v0.5.46 / 1.55.0 - 2026-06-16
- auto-expand cached CCR markers under 10KB
- LLM never sees `aphrodite_retrieve` for small cached items

#### v0.5.42 / 1.51.0 - 2026-06-16
- debug info injected into `[APHRODITE]` catalog block
- version, mode, thresholds shown in conversation

### Bug Fixes

#### Critical

##### v0.5.56 / 1.62.2 - 2026-06-16
- 20+ Rust binary bugs fixed (DefaultHasher, inline_ccr, task_tracker, EMA, UTF-8 slicing, content-type, shutdown)
- symlink chain fixed - single source via symlink chain, never copy
- CHANGELOG restructured to proper markdown hierarchy

##### v0.5.52 / 1.61.0 - 2026-06-16
- mode warning now respects `--mode`
- `--listen` made optional
- first-turn skip for compression
- `threshold_tokens` clamping fix
- wildcard route matching on `/*`
- `filter_content` zero-match edge case
- `compress` size calculation fix

##### v0.5.51 / 1.60.0 - 2026-06-16
- `cache_alive` crash on empty response
- `_recent_markers` shadowing bug
- EMA ratio calculation fix
- health check endpoint fix
- double `detect_content_type` call
- false Rust `+` detection in version strings
- body read exhaustion in streaming
- double elapsed time in metrics
- port 9797 as the single default
- XDG database path resolution
- path read security hardening

#### High

##### v0.5.62 / 1.62.8 - 2026-06-16
- **R1**: DefaultHasher seeded per-process - cache keys stable across restarts
- **R2**: EMA uses correct marker length instead of summary length
- **R3**: `inline_ccr` replaced with LRU cache
- **R4**: `task_tracker` called from test constructors
- **R5**: UTF-8 safe string slicing at byte boundaries
- **R6**: content-type charset normalised (strip `+json` suffixes for matching)
- **R7**: `task_tracker.spawn()` used for background callbacks
- **R8**: `detect_content_type` log false-positive eliminated
- **R9**: noisy content types excluded from auto-tune
- **R10**: `/retrieve` checks inline_ccr before falling through
- **R11**: `/retrieve` route registered on cache-mode router
- **R12**: `task_tracker.close()` called before `wait()` to prevent hang

##### v0.5.61 / 1.62.7 - 2026-06-16
- headroom workspace headers preserved in proxy
- deep integration: disk cache mode, benchmark mode, token mode

##### v0.5.59 / 1.62.5 - 2026-06-16
- inline marker eviction fixes
- proxy state consistency fixes

##### v0.5.50 / 1.59.0 - 2026-06-16
- restore engine fallback when no engine configured
- dedup markers in catalog output
- `context_length` needed when `update_from_response` not called

##### v0.5.49 / 1.58.0 - 2026-06-16
- engine defaults to `context_length` tokens when unknown
- always compresses on threshold regardless of engine state

##### v0.5.48 / 1.57.0 - 2026-06-16
- engine `should_compress` falls back to 1 token minimum
- works even when `update_from_response` not called

##### v0.5.47 / 1.56.0 - 2026-06-16
- `should_compress` uses `self.last_prompt_tokens` as fallback
- engine actually compresses now

##### v0.5.45 / 1.54.0 - 2026-06-16
- `saturating_sub` on `tokens_saved`
- prevents overflow panic when hash > content length

##### v0.5.44 / 1.53.0 - 2026-06-16
- liveness filter on catalog
- only show markers with retrievable content, skip ghosts

##### v0.5.43 / 1.52.0 - 2026-06-16
- hex validation on CCR hash filter
- requires ≥8 hex chars, removes `abc123` placeholders

#### Medium / Low

##### v0.5.57 / 1.62.3 - 2026-06-16
- all medium severity bugs fixed
- all low severity bugs fixed

### Infrastructure

#### v0.5.62 / 1.62.8 - 2026-06-16
- build monitor: dedicated wezterm cli agent polls every 5s, writes `.hermes/build-status.json`
- skills v3.0: hermes-z-execution, plan-then-delegate, dev-metrics, build-monitor, execution-blocks
- CHANGELOG restructured to proper markdown hierarchy (no single-line entries)
- git convention codified: `git add`, `git commit` only (no reset, force-push, rebase)
- release workflow standardised: build → bump → commit → push → tag → gh release → profile sync

#### v0.5.60 / 1.62.6 - 2026-06-16
- binary fixes for edge-case crashes
- medium severity bug fixes
- CC0 license switch

#### v0.5.59 / 1.62.5 - 2026-06-16
- README documentation
- benchmark lint fixes
- `.env.sh` setup script

#### v0.5.58 / 1.62.4 - 2026-06-16
- 440-line proxy benchmark suite
- `rust-toolchain.toml` pinned
- CHANGELOG.md created

#### v0.5.54 - 2026-06-16
- remove duplicate shared-state definitions from `_hooks` / `_tools` / `_resolve`

#### v0.5.53 - 2026-06-16
- consolidate shared state into `_core.py`
- break circular imports

## Headroom

### v0.7.12 - 2026-06-14
- feat: winged sandal logo
- headroom kwargs passthrough

### v0.7.11 - 2026-06-14
- chore: bump version 0.7.10 → 0.7.11

### v0.7.6 - 2026-06-13
- chore: regenerate reports
- freeze-cache tuning
- save snapshots

### v0.7.4 - 2026-06-13
- feat: template-based report
- cumulative benchmarks
- linear arrows visualization

## Plugin Version History

| Plugin Version | Date       | Description                              |
|----------------|------------|------------------------------------------|
| v0.7.12        | 2026-06-14 | Winged sandal logo + headroom kwargs     |
| v0.7.11        | 2026-06-14 | Version bump 0.7.10 → 0.7.11            |
| v0.7.6         | 2026-06-13 | Regenerate reports, freeze-cache tuning  |
| v0.7.4         | 2026-06-13 | Template report, cumulative benchmarks   |
| v0.5.62        | 2026-06-16 | Rust fixes R1-R12 + build monitor + skills v3.0 |
| v0.5.61        | 2026-06-16 | Headroom fixes + deep integration        |
| v0.5.60        | 2026-06-16 | Binary fixes, medium bugs, CC0 license   |
| v0.5.59        | 2026-06-16 | Inline fixes, README, benchmark lint     |
| v0.5.58        | 2026-06-16 | Benchmark + toolchain + changelog        |
| v0.5.57        | 2026-06-16 | All medium+low bugs fixed                |
| v0.5.56        | 2026-06-16 | Critical + high bug fixes                |
| v0.5.55        | 2026-06-16 | Headroom cache/benchmark/token modes     |
| v0.5.54        | 2026-06-16 | Remove duplicate shared-state            |
| v0.5.53        | 2026-06-16 | Consolidate shared state into `_core.py` |
| v0.5.52        | 2026-06-16 | 8 bugs fixed                             |
| v0.5.51        | 2026-06-16 | 13 bugs fixed                            |
| v0.5.50        | 2026-06-16 | Engine fallback + dedup                  |
| v0.5.49        | 2026-06-16 | Engine context_length fallback           |
| v0.5.48        | 2026-06-16 | Engine should_compress fallback          |
| v0.5.47        | 2026-06-16 | should_compress last_prompt_tokens       |
| v0.5.46        | 2026-06-16 | Auto-expand CCR markers <10KB            |
| v0.5.45        | 2026-06-16 | saturating_sub on tokens_saved           |
| v0.5.44        | 2026-06-16 | Liveness filter on catalog               |
| v0.5.43        | 2026-06-16 | Hex validation on CCR hash               |
| v0.5.42        | 2026-06-16 | Debug info in catalog block              |
