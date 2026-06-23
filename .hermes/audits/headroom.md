# Headroom Vendor Lean Audit

**Purpose:** Identify every line of `vendor/headroom/` that can be removed,
unified, or simplified in the aphrodite integration context. Our plugin
(`plugins/aphrodite/`, ~2,400 LOC Python + ~1,900 LOC Rust) already handles most
of what headroom's full SDK provides - the vendor tree carries ~298,000 LOC of
dead weight.

**Date:** 2026-06-16 **Scope:** `vendor/headroom/` READ ONLY - report only, no
modifications.

---

## 1. What to Remove (Dead Paths)

These headroom subsystems are **not referenced** by the aphrodite plugin or the
Rust proxy. Removing them cuts the vendored tree by ~95%.

### 1a. Full Transform Pipeline (~18,000 LOC)

| File                                | LOC    | Why Dead                                                                                                                                                           |
| ----------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `transforms/content_router.py`      | 2,976  | Aphrodite uses proxy-based `ccr/create` with hash addressing, not content-typed routing. The Rust `detect_content_type` is called by the proxy binary, not Python. |
| `transforms/smart_crusher.py`       | ~2,500 | Aphrodite's Rust binary handles JSON array compression natively. Python SmartCrusher is never invoked.                                                             |
| `transforms/content_detector.py`    | 435    | Python regex-based detection (magika→unidiff→PlainText chain). Rust binary has its own detection; this Python fallback is dead.                                    |
| `transforms/code_compressor.py`     | ~1,500 | AST-aware code compression via tree-sitter. Aphrodite doesn't do code-specific compression - all content goes through proxy CCR.                                   |
| `transforms/diff_compressor.py`     | ~1,200 | Specialized git-diff compressor. Not used by aphrodite.                                                                                                            |
| `transforms/log_compressor.py`      | ~800   | Build/test log compressor. Not used.                                                                                                                               |
| `transforms/search_compressor.py`   | ~600   | grep/ripgrep result compressor. Not used.                                                                                                                          |
| `transforms/html_extractor.py`      | ~500   | trafilatura-based HTML extraction. Not used.                                                                                                                       |
| `transforms/kompress_compressor.py` | ~500   | ML-based text compression (ModernBERT). Requires torch/transformers. Heavy. Not used.                                                                              |
| `transforms/anchor_selector.py`     | ~1,200 | Information-theoretic item selection for SmartCrusher. Dead without SmartCrusher.                                                                                  |
| `transforms/cache_aligner.py`       | ~800   | Dynamic content detection for cache prefix stability. Aphrodite's Rust proxy handles Anthropic prompt caching natively.                                            |
| `transforms/adaptive_sizer.py`      | ~400   | Compressed-output-size optimizer. Not used.                                                                                                                        |
| `transforms/pipeline.py`            | ~500   | `TransformPipeline` orchestrator. Not used.                                                                                                                        |
| `transforms/compression_policy.py`  | 284    | Per-auth-mode flags (`live_zone_only`, `max_lossy_ratio`, etc.). Multiple AuthMode enums. Entirely for headroom's proxy server.                                    |
| `transforms/compression_summary.py` | 243    | Categorical summary of dropped items. Not used.                                                                                                                    |
| `transforms/read_lifecycle.py`      | ~500   | Read lifecycle config. Not used.                                                                                                                                   |
| `transforms/error_detection.py`     | ~200   | Error indicators in content. Not used.                                                                                                                             |
| `transforms/base.py`                | ~200   | Abstract `Transform` base class. Dead without pipeline.                                                                                                            |
| `transforms/tag_protector.py`       | ~200   | XML/HTML tag protection during compression. Not used.                                                                                                              |
| `transforms/observability.py`       | ~200   | Transform-level logging wrapper. Not used.                                                                                                                         |
| `transforms/compression_units.py`   | ~200   | Unit-based compression tracking. Not used.                                                                                                                         |

### 1b. Tokenizer System (~2,000 LOC)

| File                             | LOC  | Why Dead                                                                                                                   |
| -------------------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------- |
| `tokenizers/tiktoken_counter.py` | ~200 | Only tiktoken-backed counter. In theory usable, but aphrodite never calls it - uses byte size + proxy-side token counting. |
| `tokenizers/estimator.py`        | 198  | Character-based estimation. Dead.                                                                                          |
| `tokenizers/huggingface.py`      | ~200 | HuggingFace tokenizer wrapper. Not used.                                                                                   |
| `tokenizers/mistral.py`          | ~100 | Mistral tokenizer. Not used.                                                                                               |
| `tokenizers/registry.py`         | ~100 | Tokenizer registry. Not used.                                                                                              |
| `tokenizers/base.py`             | ~100 | Abstract base.                                                                                                             |
| `tokenizer.py`                   | 80   | Wrapper. Dead.                                                                                                             |

### 1c. Memory System (~3,000+ LOC)

| Module                         | LOC         | Why Dead                                                                                                                                                                                         |
| ------------------------------ | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `headroom/memory/`             | ~2,000      | `HierarchicalMemory`, `MemoryConfig`, `with_memory` decorator, hnswlib/sqlite-vec/sentence-transformers. Aphrodite has CCR catalog (`_conv_index`, `_recent_markers`, `aphrodite_catalog` tool). |
| `headroom/relevance/`          | ~1,000      | BM25, EmbeddingScorer, HybridScorer. Not used.                                                                                                                                                   |
| `headroom/memory-stack` extras | (deps only) | mem0ai, qdrant-client, neo4j. Not used.                                                                                                                                                          |

### 1d. Telemetry / Observability (~3,000 LOC)

| Module           | LOC    | Why Dead                                                                                            |
| ---------------- | ------ | --------------------------------------------------------------------------------------------------- |
| `telemetry/`     | ~1,500 | TOIN (pattern learning), OTel metrics, Langfuse tracing, telemetry beacons. None used by aphrodite. |
| `observability/` | ~500   | OTel + Langfuse config. Not used.                                                                   |

### 1e. Proxy Utilities (~2,000 LOC)

| File                                  | LOC   | Why Dead                                                                                                                      |
| ------------------------------------- | ----- | ----------------------------------------------------------------------------------------------------------------------------- |
| `proxy/savings_tracker.py`            | 1,157 | Durable JSON persistence of savings, LiteLLM cost estimation, display session tracking. Aphrodite has `aphrodite_stats` tool. |
| `proxy/rate_limiter.py`               | 117   | Token bucket rate limiter. Headroom proxy server feature - aphrodite uses its own Rust proxy.                                 |
| `proxy/forwarded_headers.py`          | 319   | Trusted-gateway X-Forwarded-For/CIDR allow-list. Not relevant.                                                                |
| `proxy/loopback_guard.py`             | 197   | FastAPI debug endpoint guard. Not relevant.                                                                                   |
| `proxy/image_compression_decision.py` | 143   | Image compression gate. Not relevant.                                                                                         |

### 1f. CCR Subsystem (~700+ LOC)

| File                     | LOC | Why Dead                                                                                                                                                                                 |
| ------------------------ | --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ccr/context_tracker.py` | 687 | `ContextTracker` with `ExpansionRecommendation`, `CompressedContext`, proactive relevance-based expansion. Aphrodite's `_conv_index` + `_recent_markers` fulfill same role in ~30 lines. |

### 1g. Config System (~700 LOC)

| File                        | LOC  | Why Dead                                                                                                                                               |
| --------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `config.py`                 | 697  | Dataclass-based `HeadroomConfig`, `HeadroomMode`, `CacheAlignerConfig`, `SmartCrusherConfig`, etc. Aphrodite uses env vars (`_cfg_int`, `os.environ`). |
| `headroom/config/models.py` | ~200 | ML model defaults. Not used.                                                                                                                           |

### 1h. Remaining Dead

| Module               | LOC    | Why Dead                                                                                                                  |
| -------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------- |
| `providers/`         | ~3,000 | OpenAI, Anthropic, Google, LiteLLM provider wrappers. Aphrodite delegates to Hermes LLM provider, not headroom providers. |
| `cache/`             | ~2,000 | `SemanticCache`, `CacheOptimizerRegistry`, `AnthropicCacheOptimizer`, `OpenAICacheOptimizer`. Not used.                   |
| `reporting/`         | ~500   | Jinja2 report generator. Not used.                                                                                        |
| `cli/`               | ~500   | `headroom` CLI entry point. Not used.                                                                                     |
| `integrations/`      | ~1,000 | Langchain, MCP, agno, strands wrappers. Not used.                                                                         |
| `mcp_registry/`      | ~200   | MCP server installer. Not used.                                                                                           |
| `utils.py` (markers) | 251    | `<headroom:tool_digest ...>` marker format. Dead. Aphrodite uses `<<<CCR:...>>>`.                                         |
| `paths.py`           | ~100   | Path resolution. Partially used by savings_tracker.                                                                       |

### 1i. Tests (~144,000 LOC - 402 files)

99% are unit tests for dead modules above. Only ~6 tests reference CCR or proxy
concepts applicable to aphrodite (test_mcp_registry, test_tokenizers,
test_storage, test_telemetry context). The rest exercise SmartCrusher,
transforms, savings tracking, WS sessions, TOIN, and the headroom proxy server -
none of which aphrodite uses.

### 1j. Examples (~6,500 LOC - 23 files)

Only the 5 `examples/hermes_demo/` files are relevant. The rest are headroom SDK
demos.

---

## 2. What to Unify (Shared Format)

### 2a. CCR Marker Format

**Current state - DRIFT:**

| Side                           | Format                                             | Used Where                                                 |
| ------------------------------ | -------------------------------------------------- | ---------------------------------------------------------- |
| **aphrodite**                  | `<<<CCR:hash\|type\|size\|mode\|preview=BASE64>>>` | `_marker.py`, `_hooks.py`, `_core.py` (ASCII triple-angle) |
| **headroom**                   | `<headroom:tool_digest sha256="...">`              | `utils.py` (XML-like tags)                                 |
| **headroom's `_CCR_RE` regex** | matches `[...]`, `<<<...>>>`, `⫷...⫸`              | In headroom's Rust core + Python `_CCR_RE`                 |

**Fix:** Strip `<headroom:...>` marker format - nothing produces it; nothing
consumes it. The aphrodite `<<<CCR:hash|type|size>>>` ASCII format is the only
live format. The `_CCR_RE` regex currently also accepts `[...]` (legacy bracket)
and `⫷...⫸` (Unicode chevrons). These can be dropped to a single ASCII format,
matching the memory note.

### 2b. Threshold Calculation

| Side          | Approach                                                                                                                                                                     | LOC  |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- |
| **aphrodite** | `ENGINE_THRESHOLD_PCT` (single env var, `_cfg_int`) + proxy `threshold` config                                                                                               | ~20  |
| **headroom**  | `CompressionPolicy` with `live_zone_only`, `max_lossy_ratio`, `volatile_token_threshold`, `net_mutation_gain()`, `break_even_reads()`, `should_mutate_deep()` - per AuthMode | ~284 |

**Fix:** Keep aphrodite's simple percentage-based approach. Headroom's
`CompressionPolicy` is built for a SaaS proxy with multiple auth tiers -
irrelevant for a single-user Hermes plugin.

### 2c. Content Detection

| Side          | Approach                                                                                 | LOC    |
| ------------- | ---------------------------------------------------------------------------------------- | ------ |
| **aphrodite** | Size-based thresholds (1KB/2KB/4KB/8KB) + single `detect_content_type` via Rust          | ~5     |
| **headroom**  | 435-line Python `content_detector.py` + 2,976-line `content_router.py` with 9 strategies | ~3,411 |

**Fix:** Keep size-based. The headroom content-type routing is sophisticated but
unused - all content goes to the same `ccr/create` endpoint regardless of type.

---

## 3. What to Simplify (Config Contract)

### 3a. Remove headroom's separate config system

Headroom's `config.py` (697 LOC) defines 20+ dataclasses (`HeadroomConfig`,
`HeadroomMode`, `SmartCrusherConfig`, `CacheAlignerConfig`,
`CacheOptimizerConfig`, `RelevanceScorerConfig`, `Block`, `CachePrefixMetrics`,
etc.). Aphrodite reads everything from env vars via `_cfg_int()` and
`aphrodite.toml`. Delete the headroom config layer.

### 3b. Single env var namespace

Current two-way bridge:

- `HEADROOM_SSE_BUFFER_MAX_BYTES` → bumps `INLINE_THRESHOLD` to 1MB
  (`_core.py:37-38`)
- Aphrodite ignores all other `HEADROOM_*` vars
- Headroom ignores all `APHRODITE_*` vars

**Fix:** Replace `HEADROOM_SSE_BUFFER_MAX_BYTES` check with
`APHRODITE_SSE_BUFFER_MAX_BYTES`. Headroom's env vars
(`HEADROOM_PROXY_AUTH_MODE_POLICY_ENFORCEMENT`,
`HEADROOM_PROXY_TRUSTED_GATEWAY_CIDRS`, `HEADROOM_SAVINGS_PATH`) are never set -
delete the env-var reading code.

### 3c. Port config

Aphrodite defines `PORTS = {"cache": 9797, "token": 9798}` in `_core.py`.
Headroom's proxy server reads port from its own environment/config. In our
context, headroom's proxy server is never launched - only aphrodite's Rust proxy
runs. So headroom port config is dead.

---

## 4. Top 5 Simplifications

### #1: Strip transforms/ directory

**What:** Remove `transforms/` (18 modules, ~18,000 LOC) - content router,
content detector, SmartCrusher, CodeCompressor, DiffCompressor, LogCompressor,
SearchCompressor, HtmlExtractor, KompressCompressor, CacheAligner,
AnchorSelector, AdaptiveSizer, CompressionPolicy, Pipeline, ReadLifecycle,
ErrorDetection, TagProtector, Observability, CompressionSummary,
CompressionUnits.

**Why:** Aphrodite delegates all compression to the Rust proxy binary
(`ccr/create` endpoint). Content-type routing, AST-preserving code compression,
git-diff awareness, cache alignment, and error detection are handled by the Rust
binary, not Python.

**LOC cut:** ~18,000

### #2: Strip telemetry/, observability/, reporting/

**What:** Remove `telemetry/` (TOIN, OTel, beacons), `observability/` (tracing),
`reporting/`.

**Why:** Aphrodite has no telemetry dependency. Metrics come from
`aphrodite_stats` tool.

**LOC cut:** ~3,000

### #3: Strip providers/, cache/, memory/, relevance/

**What:** Remove `providers/` (OpenAI/Anthropic/Google/LiteLLM wrappers),
`cache/` (semantic cache, cache optimizers), `memory/` (hierarchical memory,
embeddings), `relevance/` (BM25, embedding scorers).

**Why:** Aphrodite delegates LLM interaction to Hermes. No need for provider
wrappers, semantic caching, or memory backends - CCR inline store + proxy
addressable store suffice.

**LOC cut:** ~8,000

### #4: Strip proxy utility modules

**What:** Remove `proxy/savings_tracker.py`, `proxy/rate_limiter.py`,
`proxy/forwarded_headers.py`, `proxy/loopback_guard.py`,
`proxy/image_compression_decision.py`.

**Why:** These are headroom's own proxy server utilities. Aphrodite runs its own
Rust proxy; these PowerScale would never be called.

**LOC cut:** ~2,000

### #5: Strip config system + `ccr/context_tracker.py` + old markers

**What:** Remove `config.py` (697 LOC), `ccr/context_tracker.py` (687 LOC),
`<headroom:...>` marker format from `utils.py`, and all dead tokenizers.

**Why:** Aphrodite has its own env-var config, its own conv_index/markers, its
own marker format.

**LOC cut:** ~1,800

---

## 5. Estimated Total LOC Reduction

| Category                  | Current LOC  | After       | Cut                |
| ------------------------- | ------------ | ----------- | ------------------ |
| headroom/ source          | ~145,000     | ~10,000     | **~135,000**       |
| headroom/ tests           | ~144,000     | ~500        | **~143,500**       |
| headroom/ examples        | ~6,500       | ~1,000      | **~5,500**         |
| headroom/ (Rust + config) | ~2,500       | ~0          | **~2,500**         |
| **Total**                 | **~298,000** | **~11,500** | **~286,500 (96%)** |

**Retained after pruning (~11,500 LOC):**

- `headroom/__init__.py` - stub (312 LOC, but needed for namespace)
- `headroom/_version.py` - version string
- `headroom/proxy/` - server.py + helpers used by Rust proxy integration (~8,000
  LOC)
- `headroom/ccr/` - mcp_server.py used by tool integration (~500 LOC)
- `headroom/tokenizers/tiktoken_counter.py` - if needed by proxy (~200 LOC)
- Tests: only integration tests that exercise proxy CCR endpoints (~500 LOC)
- Examples: only `examples/hermes_demo/` (~1,000 LOC)

---

## Summary Table

| #         | Simplification                                     | LOC Cut  | Effort                                                    | Risk                                 |
| --------- | -------------------------------------------------- | -------- | --------------------------------------------------------- | ------------------------------------ |
| 1         | Strip transforms/                                  | ~18,000  | Low - no import in aphrodite plugin                       | Low - Rust proxy handles all         |
| 2         | Strip telemetry/observability/reporting/           | ~3,000   | Low                                                       | Low                                  |
| 3         | Strip providers/cache/memory/relevance/            | ~8,000   | Medium - verify no transitive imports                     | Low                                  |
| 4         | Strip proxy utils                                  | ~2,000   | Low                                                       | Low - headroom proxy server not used |
| 5         | Strip config + context_tracker + old markers       | ~1,800   | Low                                                       | Low                                  |
| 6         | Strip tokenizers (keep tiktoken)                   | ~1,500   | Low                                                       | Low                                  |
| 7         | Strip CLI, integrations, mcp_registry              | ~1,700   | Low                                                       | Low                                  |
| 8         | Strip tests (402→~5 files)                         | ~143,500 | Medium - need to identify which tests exercise proxy CRUD | Low                                  |
| 9         | Strip examples (23→~5 files)                       | ~5,500   | Low                                                       | Low                                  |
| 10        | Unify marker format (drop `[...]` + Unicode)       | ~5       | Low - regex change                                        | Low - already ASCII-only in practice |
| 11        | Unify env var namespace (`HEADROOM_`→`APHRODITE_`) | ~5       | Low                                                       | Low - single bridge var              |
| 12        | Remove `CompressionPolicy` duplication             | ~284     | Low                                                       | Low - aphrodite uses simple pct      |
| **Total** | **~286,500**                                       |          |                                                           |                                      |

## Verification Checklist (Post-Cleanup)

1. `import headroom` succeeds (stub `__init__.py` + Rust `_core` extension)
2. `aphrodite` plugin imports resolve (plugin only imports `headroom._core` Rust
   extension)
3. Rust proxy binary builds and runs (no Python headroom deps)
4. CCR marker format is ASCII-only (`<<<CCR:hash|type|size>>>`)
5. Test suite passes for retained tests
