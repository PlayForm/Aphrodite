# Changelog

All notable changes to **@playform/hermes-compress** are documented in this file.

---

## 0.7.14 - headroom-retrieve v0.2.0 standalone plugin + CCR recompression detection

- **headroom-retrieve v0.2.0**: New standalone plugin (`plugins/headroom-retrieve/`)
  that registers ONLY the `headroom_retrieve` tool - no compression, no hooks,
  no cross-session interference. Safe to leave enabled globally.
- **CCR recompression detection**: `_looks_like_ccr()` detects when the token-mode
  proxy's `/v1/retrieve` endpoint returns another CCR marker instead of original
  content (proxy re-compression bug). Treats this as a cache miss and falls
  back to local file read.
- **`path` parameter now required**: Forces the LLM to always include the original
  file path, enabling the fallback to work reliably.
- **Three-provider config**: `deepseek-proxy-cache` (:8787, default),
  `deepseek-proxy-token` (:8788, `--provider` flag), `deepseek-direct` (fallback).
  No more config swapping - just `hermes --provider deepseek-proxy-token`.

## 0.7.13 - Inline Compression Always-On, Double-Wrap Fix, headroom_retrieve Fix
  skipped local compression when a local headroom proxy was detected. The
  cache-mode proxy is transparent for Chat Completions (Hermes' API format) -
  only Anthropic Messages / OpenAI Responses get proxy compression. Inline
  compression now runs alongside the proxy, providing 50-78% token savings
  while the proxy provides prefix-freeze cost savings.
- **Double-compression (N-layer wrapping) fixed**: `_patch_loop()` was
  re-wrapping forwarders on every turn, creating N layers of wrappers after
  N turns. Each layer called `headroom.compress()` on already-compressed
  messages (0% savings, wasted CPU). Fixed by saving originals once and
  wrapping one-time only via `_saved_origins` dict.
- **headroom_retrieve TypeError fixed**: Handler signature `_handle_headroom_retrieve(args: dict)`
  rejected Hermes' `task_id` kwarg. Changed to `**kwargs` passthrough.
  Now returns proper "Content expired" messages instead of crashing.
- **Startup message corrected**: No longer prints misleading "proxy-active
  (no local compression)" - now says "inline compression active".

## 0.7.0 - Pipeline Reorder: Headroom-First

- **Critical fix**: Reordered compression pipeline so headroom runs BEFORE
  pre-processing. Previously, pre-processing (ANSI strip, whitespace
  normalization, path optimization) ran first, which confused headroom's
  ContentRouter - messages were incorrectly marked as `protected:user_message`
  instead of being compressed. This cost 15-30% in lost savings.
- **Benchmark-verified**: Full pipeline now matches or exceeds headroom-only
  compression on all tool types. Overall savings increased from 25.8% to
  59-64% on real sessions.
- **DeepSeek optimization**: CacheAligner is correctly disabled for DeepSeek
  models (headroom auto-detects the provider and skips Anthropic-specific
  prefix caching heuristics). All other compressors (SmartCrusher,
  CodeCompressor, Kompress) remain active.
- **Cold-start warning**: First API call of any session now emits a prominent
  `WARNING` log informing users of the 10-15 second Kompress ONNX model load
  time. Subsequent calls run at 8-80ms.
- **Post-processing only**: After headroom compression, only CCR markers are
  stripped from the compressed output. Full pre-processing is skipped on
  already-compressed content.

## 0.6.0 - Auto-Update, Per-Tool Strategies, Smart Truncation, Dedup

- **Auto-update**: `_update.py` - checks GitHub releases on plugin load.
  `check_for_updates()` returns `UpdateResult` with version comparison.
  `install_update()` runs `pip install --upgrade` from the GitHub repo.
  Cached for 1 hour. Disable with `HERMES_COMPRESS_NO_UPDATE=1`.
- **Per-tool compression strategies**: `_strategies.py` - six tiers
  (aggressive, balanced, code, prose, minimal, skip) mapped to each Hermes
  tool based on content type. Aggressive for JSON tools (SmartCrusher, 40-60%
  savings), code tier for `read_file`/`patch` (CodeCompressor, 30-50%),
  balanced for mixed content, skip for tiny tools. Strategies merge with
  global config: more aggressive `protect_recent` and lower
  `min_tokens_to_compress` win.
- **Smart truncation**: `_truncate.py` - three strategies for outputs over
  50K chars. Head-and-tail for code/content, JSON-aware for structured data
  (truncates arrays to first 100 items, preserves structure), line-based for
  logs/terminal output. All truncation is lossy but preserves the most
  informative parts.
- **Message deduplication**: LRU cache of the last 50 tool results. When a
  tool returns identical content to a previous call, the message is replaced
  with a short reference (`[Duplicate of previous search_files result]`).
  Saves 100% on duplicate results.
- **Plugin hot-reload**: When `HERMES_COMPRESS_DEV=1`, the compressor checks
  file modification times before each call. If any `.py` file in the plugin
  directory changed, it auto-reloads the affected modules - no restart needed.
- **Zero-fidelity optimization pass**: `_optimize.py` - whitespace
  normalization (tabs→spaces, trailing whitespace, blank line collapse), JSON
  number rounding (high-precision floats→4 decimal places), path normalization
  (home dir→`~`, backslash→forward slash), timestamp shortening
  (ISO→compact), boilerplate stripping (standard tool headers and footers).
  All transforms preserve semantic meaning - only formatting changes.
- **New CompressOption fields**: `PrecompressTools`, `AggressiveKompress`,
  `DeduplicateResults`, `VerboseStats` - all configurable via `config.yaml`
  or `CompressOption` constructor. Install patcher reads all fields from
  `compression.headroom` config.
- **Smart re-patch**: Install patcher now detects outdated patches (marker
  present but content differs) and restores from `.bak` backup before
  re-applying the new patch. No more stale installations.
- **Conversation loop injection**: `install` patches `conversation_loop.py`
  to pass `api_messages` through the compressor before every API call.
  `agent_init.py` reads all `compression.headroom.*` config keys. The
  `headroom_compression.py` wrapper delegates to the plugin's `Compress`
  class - all logic lives in the plugin, not the patched core.
- **HeadroomCompressor wrapper**: `_headroom_compression.py` now translates
  `agent_init` kwargs to `CompressOption`, accepts all advanced fields, and
  delegates via `__getattr__` to the underlying `Compress` instance. Zero
  duplication - all compression logic is in the plugin.

## 0.5.0 - Pre-Processing Pipeline + Dev Mode

- **Pre-processing pipeline**: Strips waste before headroom compression.
  ANSI escape codes (terminal output), repeated log lines (collapses N
  identical lines to 2 + count), debug-level noise (tracebacks, verbose npm,
  Docker layers), repeated pattern compression.
- **Pre-compress tool outputs**: Double-pass compression - each large tool
  result (>500 chars) is individually compressed before the full message
  list pass. Gives 5-15% extra savings on JSON and log-heavy sessions.
- **Dev mode**: `HERMES_COMPRESS_DEV=1` enables per-message stats collection
  (`StatsCollector`), dry-run mode (compress without modifying messages),
  feature flags via `HERMES_COMPRESS_FLAGS`, simulated backpressure testing,
  and stats replay/analysis.
- **Cold start handling**: `protect_recent=1` (was 4) increases savings by
  79%. Combined with CCR marker stripping, average savings rose from 25.8%
  to 50.8%.

## 0.4.0 - Post-Install Patcher

- **`hermes-compress install`**: Patches `hermes-agent` core files
  (`conversation_loop.py`, `agent_init.py`, `agent_runtime_helpers.py`) to
  inject compression into the API call loop. Creates `.bak` backups for all
  patched files.
- **`hermes-compress uninstall`**: Restores all patched files from `.bak`
  backups. Fully reversible - no permanent changes.
- **`hermes-compress status`**: Checks which files are currently patched and
  whether backups exist.

## 0.3.0 - Standalone Package + CLI

- **Standalone pip package**: `pip install hermes-compress`. Works in any
  Python app without Hermes. `from hermes_compress import Compress`.
- **CLI**: `hermes-compress proxy`, `hermes-compress compress`,
  `hermes-compress install`, `hermes-compress uninstall`, `hermes-compress
  status`.
- **Proxy mode**: `hermes-compress proxy --port 8787` starts a headroom
  proxy server. Zero code changes needed - point provider `base_url` at it.

## 0.2.0 - Proxy Mode + Generalization

- **Proxy class**: Start/stop/health-check the headroom proxy server from
  Python. `Proxy(port=8787).start()`.
- **Mode switching**: `mode="inline"` (library) vs `mode="proxy"` (HTTP).
  Identical `compress(messages)` API for both.

## 0.1.0 - Initial Release

- **Headroom integration**: In-process library call via
  `headroom.compress()`.
- **25-60% token savings** in real benchmarks (11 API calls, DeepSeek).
- **Warm latency 50-80ms**, cold start 7-10s (Kompress ONNX model load).
