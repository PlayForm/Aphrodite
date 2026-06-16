# Changelog

## Latest - v0.5.61 / 1.62.7 (2026-06-16)

[Full release notes](https://github.com/PlayForm/Aphrodite/compare/v0.5.60...v0.5.61) ·
[All 2026-06-16 releases](https://github.com/PlayForm/Aphrodite/compare/v0.5.39...v0.5.61)

### Summary

| Metric | Value |
|--------|-------|
| **Releases today** | 23 (v0.5.39-v0.5.61) |
| **Plugin versions today** | 1.48.0-1.62.7 (15 increments) |
| **Total bugs fixed** | ~80+ across all time |
| **Critical/High bugs fixed** | ~15+ |
| **Medium/Low bugs fixed** | ~65+ |
| **CCR markers cached** | 253 |
| **Compression ratio** | 25 MB → 6 KB (4167:1) |
| **Median compress time** | 0.9 ms |


- `skills_list`
- `skill_manage`
- `memory`
- `session_search`)
#### v0.5.39 / 1.48.0 — 2026-06-16
- verbose startup debug banner on `APHRODITE_DEBUG=1` showing all thresholds
- engine status
- tools
- proxy config
#### v0.5.30 / 1.39.0 — 2026-06-15
- pipeline mode + feature toggles + regression tracking in `aphrodite_test`
#### v0.5.29 / 1.38.0 — 2026-06-15
- `aphrodite_test` smoke test tool with quick/full/matrix modes
- settings sweep
#### v0.5.28 / 1.37.0 — 2026-06-15
- retrieve caches in `_inline_store` + tracks in `_recent_markers`
- bi-directional compress↔search↔retrieve
#### v0.5.27 / 1.36.0 — 2026-06-15
- content-addressable store
- `compress_handler` checks `_inline_store` before proxy
- falls back inline on error
#### v0.5.26 / 1.35.0 — 2026-06-15
- bi-directional search
- `_inline_store` mirrored in `compress_handler` + `transform_tool_result` hook
#### v0.5.25 / 1.34.0 — 2026-06-15
- search indexes current-turn compressions
- `_recent_markers` updated on CCR + INLINE paths with 200 cap
#### v0.5.24 / 1.33.0 — 2026-06-15
- `aphrodite_search` scans marker catalog using `_recent_markers` cache from `pre_llm_hook`

#### v0.5.22-21 / 1.30.0 — 2026-06-15
- git diff summary in `pre_llm_hook` catalog
- 30 s cached `git diff --stat`
#### v0.5.20 / 1.29.0 — 2026-06-15
- inline CCR store for tiny entries
- `HashMap` for <100 B content
- no round-trip
#### v0.5.19 / 1.28.0 — 2026-06-15
- per-hook timing in debug logs
- every hook decision logged with ms timing
#### v0.5.18 / 1.27.0 — 2026-06-15
- `aphrodite_search` tool
- search CCR entries by keyword + type filter across turns + inline store
#### v0.5.16 / 1.25.0 — 2026-06-15
- stop constant context compression
- `should_compress` 0 = disabled
- default 50% fill
- min 30 messages
#### v0.5.15 / 1.24.0 — 2026-06-15
- `type=` parameter on `aphrodite_compress`
- content type hint for adaptive compression
#### v0.5.14 / 1.23.0 — 2026-06-15
- auto-tune thresholds via compression ratio EMA
- adaptive scaling based on historical ratios
#### v0.5.13 / 1.22.0 — 2026-06-15
- editing-aware context engine
- preserve more context during active edit sessions
#### v0.5.12 / 1.21.0 — 2026-06-15
- file tree injection in catalog + `aphrodite_diff` tool
- show project structure + turn history
#### v0.5.11 / 1.20.0 — 2026-06-15
- build output smart collapse + `aphrodite_files` tool
- deduplicate repeated build lines
#### v0.5.10 / 1.19.0 — 2026-06-15
- deep code-aware tuning
- language-specific content detection
- adaptive thresholds per type
#### v0.5.9 / 1.18.0 — 2026-06-15
- add `aphrodite_list` to tool relay
#### v0.5.5 / 1.14.0 — 2026-06-15
- query parameter on `aphrodite_retrieve`
- filter retrieved content by line-level grep
#### v0.5.3 / 1.12.0 — 2026-06-15
- `context_tracker` deque LRU
- O(1) eviction
#### v0.5.1 / 1.10.0 — 2026-06-15
- `record_latency` in all proxy response paths
#### v0.5.0 / 1.9.0 — 2026-06-15
- `tokens_saved` increment in `handle_ccr_create` + headroom submodule update (relevance_threshold 0.5 + coding stop words)

### Bug Fixes

#### Critical

#### v0.5.56 / 1.62.2 — 2026-06-16
- version bump
- `turn_counter` reset
- engine marker format
- threshold calculation

#### High

#### v0.5.61 / 1.62.7 — 2026-06-16
- headroom fixes: CCR regex
- loopback exempt
- threshold invert
- image auto
- headers passthrough
- savings accumulate
- flush log
- rate limit exempt
#### v0.5.60 / 1.62.6 — 2026-06-16
- binary fixes: build path
- urlretrieve timeout
- version check
- integrity
- marker dedup
- port order fix
#### v0.5.59 / 1.62.5 — 2026-06-16
- inline fixes: key asymmetry
- LRU eviction
- base64 size
#### v0.5.56 / 1.62.2 — 2026-06-16
- `recent_markers` cap 200
- launch both proxies
- sentinel after turn archive
- `should_compress` -1 = always / 0 = disabled
#### v0.5.8 / 1.17.0 — 2026-06-15
- wave 4 fixes: retrieve pagination offset/limit
- `x-headroom-*` header pass-through
- `Secret api_key`
#### v0.5.7 / 1.16.0 — 2026-06-15
- wave 3 fixes: remove tool inject from response
- dead `retry_with_backoff`
- absolute DB path via `dirs` crate
#### v0.5.6 / 1.15.0 — 2026-06-15
- wave 2 fixes: CCR hits/misses in compress path
- marker ASCII standardization
- health always 200

#### Medium

#### v0.5.61 / 1.62.7 — 2026-06-16
- deep integration: canonical serialize
- telemetry passthrough
- SSE buffer coordination
- compression failure handling
- lazy key read
- body size guard
#### v0.5.57 / 1.62.3 — 2026-06-16
- all 18+ medium+low bugs fixed: schema hints
- preview cache
- tool content
- dedup
- round-trip
- `__all__`
- regex
- CCR warning
- START.md
- cache_port
- marker logging
- docstrings + inline prefix
- version guard
- preview base64
- PID+lock
- bare except
- dual proxy
- `tokens_saved`
- `pathlib`
- `RUST_LOG`
- metrics doc
- config path
- unwrap safety
- key chain
- lock docs
#### v0.5.52 / 1.61.0 — 2026-06-16
- 8 bugs fixed: mode warning
- listen optional
- first-turn skip
- `threshold_tokens`
- wildcard routes
- `filter_content` zero-match
- compress size
#### v0.5.51 / 1.60.0 — 2026-06-16
- 13 bugs fixed: `cache_alive` crash
- `_recent_markers` shadow
- EMA ratio
- health check
- double detect
- false Rust+
- body read
- double elapsed
- port 9797 default
- XDG DB path
- path read security
#### v0.5.50 / 1.59.0 — 2026-06-16
- restore engine fallback + dedup in catalog when `update_from_response` not called
#### v0.5.49 / 1.58.0 — 2026-06-16
- engine defaults to `context_length` tokens when unknown
- always compresses on threshold
#### v0.5.48 / 1.57.0 — 2026-06-16
- engine `should_compress` falls back to 1 token minimum when `update_from_response` not called
#### v0.5.47 / 1.56.0 — 2026-06-16
- `should_compress` uses `self.last_prompt_tokens` as fallback
- engine actually compresses
#### v0.5.45 / 1.54.0 — 2026-06-16
- `saturating_sub` on `tokens_saved`
- prevents overflow panic when hash > content
#### v0.5.38 / 1.47.0 — 2026-06-15
- `_extract_preview` split on `>>>` not `]`
- fixes `|tool|token>>>` fragments
#### v0.5.37 / 1.46.0 — 2026-06-15
- marker preview extraction uses `finditer().end()`
- fixes broken reversed find
#### v0.5.36 / 1.45.0 — 2026-06-15
- harden against `CCR:{}` ghost entries
- `str()` cast + skip on empty/None/dict hashes
#### v0.5.35 / 1.44.0 — 2026-06-15
- filter empty hashes from `_parse_ccr_markers`
- prevents `CCR:{}` ghost entries in catalog

#### Low

#### v0.5.23 / 1.32.0 — 2026-06-15
- Prometheus label format fix

### Infrastructure

#### v0.5.61 / 1.62.7 — 2026-06-16
- `Procfile.dev`
- SIGTERM handler
- body size guard
- `x-headroom-bypass` header support
#### v0.5.60 / 1.62.6 — 2026-06-16
- build path fix
- urlretrieve timeout
- version check
- integrity checks
- async binary download
- CC0-1.0 license
- UTF-8 guard
#### v0.5.59 / 1.62.5 — 2026-06-16
- proxy PID in `BINARY_DIR`
- engine marker state
- dynamic port tool relay
- rewritten README with benchmarks
- `.env.sh` profiles
#### v0.5.58 / 1.62.4 — 2026-06-16
- 440-line proxy benchmark (19/19 pass
- sub-ms)
- `rust-toolchain.toml`
- CHANGELOG.md
- `api_key` moved to env var fallback
#### v0.5.54 / 1.61.0 — 2026-06-16
- remove duplicate shared-state definitions from `_hooks`/`_tools`/`_resolve`
#### v0.5.53 / 1.61.0 — 2026-06-16
- consolidate shared state into `_core.py`
- break circular imports
- add ruff + pyright config
#### v0.5.34 / 1.43.0 — 2026-06-15
- linter detection + turn file-type tags + liteLLM removal + honest-gaps assessment
#### v0.5.31 / 1.40.0 — 2026-06-15
- move summary before file save in test handler
- `.test-results.json` includes summary + regression
#### v0.5.17 / 1.26.0 — 2026-06-15
- detach context engine
- only register when `APHRODITE_CONTEXT_ENGINE=1`
#### v0.5.4 / 1.13.0 — 2026-06-15
- debug decision logging in `_transform_tool_result` + `_transform_terminal_hook`
#### v0.5.2 / 1.11.0 — 2026-06-15
- rip out non-coding headroom integrations (-5
- 354 lines)

## Headroom (v0.7.x)

#### v0.7.12 — 2026-06-14
- winged sandal logo + headroom kwargs passthrough
#### v0.7.11 — 2026-06-14
- bump version 0.7.10 → 0.7.11
#### v0.7.6 — 2026-06-13
- regenerate reports
- freeze-cache tuning
- aggressive config at 58.8%
#### v0.7.4 — 2026-06-13
- template-based report
- cumulative benchmarks
- linear arrows
- pipeline docs

## Legacy (v0.2.x - v0.4.x)

#### v0.4.1 — 2026-06-15
- conversation memory via CCR
- 5 hooks (session_start
- transform_tool_result
- pre_llm_call
- transform_terminal_output
- post_llm_call)
#### v0.4.0 — 2026-06-15
- benchmarks
- clean docs
- version bump
#### v0.3.0 — 2026-06-15
- initial save commit
#### v0.2.0 — 2026-06-15
- stats proxy name 'aphrodite'
- modes: cache + token

## Plugin Version History

| Binary Version | Plugin Version | Date       |
|----------------|----------------|------------|
| v0.5.61        | 1.62.7         | 2026-06-16 |
| v0.5.60        | 1.62.6         | 2026-06-16 |
| v0.5.59        | 1.62.5         | 2026-06-16 |
| v0.5.58        | 1.62.4         | 2026-06-16 |
| v0.5.57        | 1.62.3         | 2026-06-16 |
| v0.5.56        | 1.62.2         | 2026-06-16 |
| v0.5.52-55     | 1.61.0         | 2026-06-16 |
| v0.5.51        | 1.60.0         | 2026-06-16 |
| v0.5.50        | 1.59.0         | 2026-06-16 |
| v0.5.49        | 1.58.0         | 2026-06-16 |
| v0.5.48        | 1.57.0         | 2026-06-16 |
| v0.5.47        | 1.56.0         | 2026-06-16 |
| v0.5.46        | 1.55.0         | 2026-06-16 |
| v0.5.45        | 1.54.0         | 2026-06-16 |
| v0.5.44        | 1.53.0         | 2026-06-16 |
| v0.5.43        | 1.52.0         | 2026-06-16 |
| v0.5.42        | 1.51.0         | 2026-06-16 |
| v0.5.41        | 1.50.0         | 2026-06-16 |
| v0.5.40        | 1.49.0         | 2026-06-16 |
| v0.5.39        | 1.48.0         | 2026-06-16 |
| v0.5.38        | 1.47.0         | 2026-06-15 |
| v0.5.37        | 1.46.0         | 2026-06-15 |
| v0.5.36        | 1.45.0         | 2026-06-15 |
| v0.5.35        | 1.44.0         | 2026-06-15 |
| v0.5.34        | 1.43.0         | 2026-06-15 |
| v0.5.31        | 1.40.0         | 2026-06-15 |
| v0.5.30        | 1.39.0         | 2026-06-15 |
| v0.5.29        | 1.38.0         | 2026-06-15 |
| v0.5.28        | 1.37.0         | 2026-06-15 |
| v0.5.27        | 1.36.0         | 2026-06-15 |
| v0.5.26        | 1.35.0         | 2026-06-15 |
| v0.5.25        | 1.34.0         | 2026-06-15 |
| v0.5.24        | 1.33.0         | 2026-06-15 |
| v0.5.23        | 1.32.0         | 2026-06-15 |
| v0.5.21-22     | 1.30.0         | 2026-06-15 |
| v0.5.20        | 1.29.0         | 2026-06-15 |
| v0.5.19        | 1.28.0         | 2026-06-15 |
| v0.5.18        | 1.27.0         | 2026-06-15 |
| v0.5.17        | 1.26.0         | 2026-06-15 |
| v0.5.16        | 1.25.0         | 2026-06-15 |
| v0.5.15        | 1.24.0         | 2026-06-15 |
| v0.5.14        | 1.23.0         | 2026-06-15 |
| v0.5.13        | 1.22.0         | 2026-06-15 |
| v0.5.12        | 1.21.0         | 2026-06-15 |
| v0.5.11        | 1.20.0         | 2026-06-15 |
| v0.5.10        | 1.19.0         | 2026-06-15 |
| v0.5.9         | 1.18.0         | 2026-06-15 |
| v0.5.8         | 1.17.0         | 2026-06-15 |
| v0.5.7         | 1.16.0         | 2026-06-15 |
| v0.5.6         | 1.15.0         | 2026-06-15 |
| v0.5.5         | 1.14.0         | 2026-06-15 |
| v0.5.4         | 1.13.0         | 2026-06-15 |
| v0.5.3         | 1.12.0         | 2026-06-15 |
| v0.5.2         | 1.11.0         | 2026-06-15 |
| v0.5.1         | 1.10.0         | 2026-06-15 |
| v0.5.0         | 1.9.0          | 2026-06-15 |

> **Note:** Tags v0.5.32, v0.5.33, v0.5.34-39 (1.41.0-1.42.0, 1.45.0-1.48.0) and v0.5.22 (1.30.0 dup) exist as binary-only releases without plugin version bumps - their `__init__.py` did not increment `PLUGIN_VERSION`. The gaps above reflect that: the plugin stayed at 1.40.0 from v0.5.31 to v0.5.34 (1.43.0), and at 1.30.0 from v0.5.21 through v0.5.22.
