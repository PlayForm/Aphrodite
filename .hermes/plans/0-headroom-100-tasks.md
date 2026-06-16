# Headroom → Coding-Agent Rewrite Plan

## Objective
Fully adapt headroom for Hermes as a **coding agent** — rip out generic/chat paradigms,
replace with code-specific flows. ~50 tasks in headroom vendor, ~50 in aphrodite plugin/proxy.

## Architecture Map (headroom vendor)

### Python (`vendor/headroom/headroom/`)
```
headroom/
├── __init__.py              # Public API surface
├── _version.py              # Version tracking
├── agent_savings.py         # Cost/savings tracking for agent runs
├── binaries.py              # Binary helper
├── backends/                # LLM provider abstraction (litellm, anyllm)
│   ├── base.py
│   ├── litellm.py
│   └── anyllm.py
├── cache/                   # Compression cache layer
│   ├── compression_cache.py # Caching compressed results
│   ├── compression_feedback.py # Feedback loop for cache optimization
│   ├── compression_store.py # Persistent store
│   └── backends/
├── ccr/                     # Compress-Cache-Retrieve core
│   ├── batch_store.py       # Batch CCR operations
│   ├── context_tracker.py   # Multi-turn context tracking (BUG #34 FIXED)
│   ├── fts5_store.py        # BM25 search backend
│   └── retrieval.py         # Retrieval orchestration
├── compression/             # Compression strategies
│   ├── code/                # Code-aware compression (tree-sitter)
│   ├── json/                # JSON structure handler
│   ├── text/                # Text compression
│   ├── llmlingua/           # LLMLingua integration
│   └── content_router.py    # Routes content to best compressor
├── detector/                # Content type detection
├── integrations/            # LangChain, Agno, etc.
├── memory/                  # Persistent memory system
│   ├── adapters/            # FTS5, embeddings
│   ├── tracker.py           # Component stats
│   └── injector.py          # Context injection
├── signals/                 # Importance scoring
├── transforms/              # Content transforms
├── proxy/                   # Proxy server core
│   ├── server.py            # Main proxy server
│   ├── proxy.py             # Request handling
│   └── ...
├── policies/                # Compression policies
│   └── net_cost_gate.py     # Cost/benefit gating
└── utils/                   # Utilities
```

### Rust (`vendor/headroom/crates/headroom-core/src/`)
```
src/
├── lib.rs                   # Crate root
├── ccr/                     # CCR store implementations
│   ├── mod.rs
│   ├── backends/
│   │   ├── in_memory.rs     # Memory CCR
│   │   └── sqlite.rs        # SQLite CCR
│   └── traits.rs            # CcrStore trait
├── compression/             # Compression engine
│   ├── mod.rs
│   ├── compressors/         # Compression strategies
│   ├── detection.rs         # Content detection
│   ├── routing.rs           # Content routing
│   └── tokenizer.rs         # Token counting
├── relevance/               # Relevance scoring
│   └── embedding.rs         # fastembed-based scorer
├── signals/                 # Signal processors
│   ├── keyword_detector.rs  # Keyword importance
│   ├── line_importance.rs   # Line-level importance
│   └── tiered.rs            # Tiered signal chain
├── transforms/              # Content transforms
│   ├── smart_crusher/       # Smart content compaction
│   │   ├── crusher.rs       # Main crusher
│   │   ├── planning.rs      # Plan generation
│   │   ├── compaction/      # Compaction IR + walker
│   │   ├── field_detect.rs  # Field detection
│   │   └── ...
│   ├── tag_protector.rs     # Tag preservation
│   └── unidiff_detector.rs  # Diff detection
└── utils/                   # Shared utilities
```

---

## PHASE 1: RIP OUT NON-CODING PARADIGMS (headroom Python ~20 tasks)

### 1.1 LangChain/Agno Integrations → DELETE
- `integrations/langchain/` — generic agent framework, not coding agent
- `integrations/agno/` — same
- `integrations/llama_index/` — document RAG, not coding
- **TASK 1**: Remove all integration modules
- **TASK 2**: Strip imports from `__init__.py`

### 1.2 Backend Abstractions → SIMPLIFY
- `backends/litellm.py` — 50+ model routing, we only need DeepSeek
- `backends/anyllm.py` — generic any-model interface
- **TASK 3**: Replace with single `DeepSeekProvider` class
- **TASK 4**: Remove liteLLM dependency from pyproject.toml
- **TASK 5**: Hard-code tokenizer mappings for deepseek-chat/r1/v4/v4-pro

### 1.3 Cache Layer → CODE-SPECIFIC
- `cache/compression_cache.py` — generic TTL cache
- `cache/compression_feedback.py` — generic feedback loop
- **TASK 6**: Replace with per-file-type cache (Rust .rs, Python .py, TypeScript .ts, etc.)
- **TASK 7**: Add AST fingerprint cache — skip re-compressing identical code structures
- **TASK 8**: Add diff-aware cache — only re-compress changed functions

### 1.4 Compression Strategies → CODE-ONLY
- `compression/text/` — generic prose compression → DELETE
- `compression/llmlingua/` — LLM-based compression → DELETE (too slow for coding)
- `compression/json/` → KEEP but optimize for tool outputs, not arbitrary JSON
- `compression/code/` → KEEP and expand
- **TASK 9**: Remove text compression module
- **TASK 10**: Remove LLMLingua dependency
- **TASK 11**: Optimize JSON handler for tool output patterns (exit_code, stdout, error, etc.)
- **TASK 12**: Add language-specific code compressors (Python, Rust, JS/TS, Go)

### 1.5 Policies → CODE-CONTEXT-AWARE
- `policies/net_cost_gate.py` — generic cost/benefit
- **TASK 13**: Add "code importance" scoring — function/class definitions > import blocks
- **TASK 14**: Add "error relevance" boost — error traces always high priority
- **TASK 15**: Add "recency decay" — older code context decays faster

### 1.6 Detector → SPECIALIZE
- **TASK 16**: Prioritize code detection (`.py`, `.rs`, `.ts`, `.go`, `.js`)
- **TASK 17**: Add `git diff` output detection as high-priority
- **TASK 18**: Add test output detection (pytest, cargo test, jest patterns)

### 1.7 Memory → CODE WORKSPACE MEMORY
- `memory/` — generic memory system
- **TASK 19**: Replace with code workspace memory — remember project structure
- **TASK 20**: Add file-modification tracking across turns

---

## PHASE 2: CODE-SPECIFIC COMPRESSION (headroom ~15 tasks)

### 2.1 AST-Aware Code Compression
- **TASK 21**: Tree-sitter parser for Python (already have `tree-sitter-python`)
- **TASK 22**: Tree-sitter parser for Rust (add `tree-sitter-rust`)
- **TASK 23**: Tree-sitter parser for TypeScript/JavaScript (`tree-sitter-typescript`)
- **TASK 24**: Function signature preservation — never compress function/class signatures
- **TASK 25**: Import block compression — collapse imports to `[X imports]`
- **TASK 26**: Docstring/comment tiered compression (keep TODO/FIXME/HACK, drop boilerplate)

### 2.2 Tool Output Compression
- **TASK 27**: Terminal output patterns — detect `cargo build` output, `pytest` output
- **TASK 28**: File read output — detect line-numbered output, preserve structure
- **TASK 29**: Git output — detect `git diff`, `git status`, `git log` patterns
- **TASK 30**: Linter output — detect `rustc`, `clippy`, `mypy`, `eslint` patterns

### 2.3 Smart Crusher Coding Extensions
- **TASK 31**: Crusher → add code structure awareness (don't crush between `{` `}`)
- **TASK 32**: Crusher → preserve error/warning lines at 100% fidelity
- **TASK 33**: Crusher → add language detection before crushing
- **TASK 34**: Crusher → add diff-mode (only show changed hunks)
- **TASK 35**: Field detection → add code-specific fields (function, class, module, error, warning)

---

## PHASE 3: CONTEXT TRACKER REWRITE (headroom ~15 tasks)

### 3.1 Code Context Tracking
- **TASK 36**: Track which files have been read/edited per turn
- **TASK 37**: Track which functions/classes have been referenced
- **TASK 38**: Build project structure index from file reads
- **TASK 39**: Auto-expand when same file referenced again

### 3.2 Relevance for Coding Queries
- **TASK 40**: Code-specific keyword extraction (snake_case, CamelCase, kebab-case)
- **TASK 41**: Path-aware matching (`src/auth/middleware.py` matches "auth middleware")
- **TASK 42**: Error-aware matching (traceback patterns → expand relevant code)
- **TASK 43**: Test-awareness (test function ↔ implementation function linking)
- **TASK 44**: Import graph tracking (expand dependency when function used)

### 3.3 Performance
- **TASK 45**: LRU with file-size-weighted eviction (small files stay longer)
- **TASK 46**: Adaptive relevance threshold (lower for errors, higher for prose)
- **TASK 47**: Batch relevance scoring (score all contexts in single pass)
- **TASK 48**: Workspace-scoped cache with session TTL
- **TASK 49**: Pre-compute code fingerprints for O(1) relevance lookup
- **TASK 50**: Remove the `sample_content[:2000]` truncation in tracker — use structured metadata instead

---

## PHASE 4: APHRODITE PLUGIN REWRITE (~25 tasks)

### 4.1 Hooks Optimization
- **TASK 51**: `_transform_tool_result` → add code type detection (detect .py/.rs output)
- **TASK 52**: `_transform_tool_result` → never compress file reads < 50KB (LLM needs code)
- **TASK 53**: `_transform_terminal_hook` → detect build output, compress only repeated lines
- **TASK 54**: `_pre_llm_hook` → inject project file tree when >20 files referenced
- **TASK 55**: `_pre_llm_hook` → inject recent git diff summary
- **TASK 56**: `_store_conversation_turn` → tag by file type for better retrieval
- **TASK 57**: Add `pre_tool_call` hook to pre-cache file dependencies

### 4.2 Context Engine Rewrite
- **TASK 58**: Should compress logging output aggressively, code output conservatively
- **TASK 59**: Detect "editing session" mode — keep edited files uncompressed
- **TASK 60**: Add "task boundary" detection — new task starts, flush old context
- **TASK 61**: Progressive compression — first pass light, deeper passes for older content
- **TASK 62**: Never compress the last 3 tool_call → tool_result pairs (active work)

### 4.3 Tool Enhancements
- **TASK 63**: `aphrodite_retrieve` → add `file=filename.py` filter
- **TASK 64**: `aphrodite_retrieve` → add `function=func_name` filter
- **TASK 65**: `aphrodite_retrieve` → add `grep=pattern` filter
- **TASK 66**: `aphrodite_compress` → add `type=code|log|diff` parameter
- **TASK 67**: `aphrodite_stats` → add per-file-type breakdown
- **TASK 68**: Add `aphrodite_files` tool — list all files referenced in CCR
- **TASK 69**: Add `aphrodite_diff` tool — show what changed between turns

### 4.4 Dev/Test Improvements
- **TASK 70**: `APHRODITE_DEV=1` → also log which decisions were made (compress? threshold?)
- **TASK 71**: Add `aphrodite_benchmark` tool for compression ratio stats
- **TASK 72**: Add per-hook timing (profile hook overhead)
- **TASK 73**: Add compression decision log (why was X compressed? why not Y?)
- **TASK 74**: Auto-tune thresholds based on session patterns
- **TASK 75**: Add session health dashboard (compression ratio, retrieval hit rate, token savings)

---

## PHASE 5: APHRODITE PROXY REWRITE (~25 tasks)

### 5.1 Proxy Streamlining
- **TASK 76**: Remove generic proxy pass-through → only Chat Completions API
- **TASK 77**: Remove tool relay (Hermes has its own tool dispatch)
- **TASK 78**: Remove notification callback system
- **TASK 79**: Remove request history ring buffer (memory leak in long sessions)
- **TASK 80**: Simplify to single-mode (token-only, drop cache mode complexity)

### 5.2 Compression Intelligence
- **TASK 81**: Add file-type-aware compression thresholds
- **TASK 82**: Code files: threshold 50KB (keep function-level code in context)
- **TASK 83**: Terminal output: threshold 2KB (compress aggressively)
- **TASK 84**: Error output: threshold 500B (keep errors always visible)
- **TASK 85**: Add diff detection in stream — compress merged diffs more
- **TASK 86**: Add "conversation memory" — tracks what was discussed → smarter threshold

### 5.3 Stats & Monitoring
- **TASK 87**: Per-file-type compression stats (code vs log vs diff vs json)
- **TASK 88**: Real-time token savings (update every N compressions)
- **TASK 89**: Health endpoint with uptime + total savings
- **TASK 90**: Prometheus metrics endpoint (for Grafana dashboards)
- **TASK 91**: Alert on compression ratio anomalies (something is wrong)

### 5.4 Performance
- **TASK 92**: Async CCR reads → don't block proxy on retrieval
- **TASK 93**: Connection pooling for DeepSeek upstream
- **TASK 94**: Response streaming with progressive compression
- **TASK 95**: Inline small CCR entries (no round-trip for < 100B)
- **TASK 96**: Batch CCR writes (flush every 100ms or 10 entries)

### 5.5 Config / CI
- **TASK 97**: `aphrodite.toml` → add `[coding]` section with language-specific thresholds
- **TASK 98**: CI test for compression ratio regression
- **TASK 99**: CI test for retrieval round-trip fidelity
- **TASK 100**: Generate compression benchmark report on each push

---

## Audit Findings — Issues in Current Code

### Already Fixed (confirmed)
1. ✅ Typo in env-var name (INLINE_THRESHOLD) — now `APHRODITE_INLINE_THRESHOLD`
2. ✅ Duplicate inline_store/INLINE_THRESHOLD declarations
3. ✅ _alive() fragile health-check → JSON parse
4. ✅ _rebuild_handler hardcoded path → os.path.dirname(__file__)
5. ✅ _download_binary platform detection
6. ✅ CCR marker glyphs → ASCII `<<<CCR:>>>`
7. ✅ tokens_saved in compress_chat_completion (NOW ALSO in handle_ccr_create)
8. ✅ Health check decoupled from upstream API call
9. ✅ Proxy launch retry loop (was fixed 0.5s sleep)
10. ✅ _alive() cache with 5s TTL
11. ✅ should_compress() threshold check
12. ✅ _resolve_one tries both ports
13. ✅ compress() no truncation [:2000]
14. ✅ tokens_saved increment in handle_ccr_create — JUST FIXED
15. ✅ relevance_threshold 0.3→0.5 — JUST FIXED
16. ✅ Coding stop words added — JUST FIXED

### Still Pending
- Headroom fork rebase for upstream JSON/code compression fixes
- Bug #34: further tracker optimization beyond threshold (see tasks 45-50)
- BIN_VERSION/Cargo.toml sync (already at v0.5.0)
- AphroditeContextEngine tool-chain safety (already fixed at boundary check)

---

## Priority Order

1. **IMMEDIATE**: Phase 1 tasks 1-5 (rip out non-coding integrations)
2. **IMMEDIATE**: Phase 2 tasks 21-26 (AST-aware code compression)
3. **HIGH**: Phase 3 tasks 36-44 (code context tracking)
4. **HIGH**: Phase 4 tasks 51-62 (hooks + context engine)
5. **MEDIUM**: Phase 5 tasks 76-85 (proxy streamlining)
6. **LATER**: Phase 5 tasks 87-100 (stats, perf, CI)
