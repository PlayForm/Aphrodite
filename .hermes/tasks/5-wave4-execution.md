# Aphrodite — Unified Execution Plan (Wave 4)
## Merged from 1.md (100 tasks), 2.md (honest gaps), 3.md (audit verification)

### Scorecard
- **25** fully done | **5** partial | **70** not done (from 100)
- **7 immediate** gaps identified for this wave
- **Previous**: 31 releases (v0.5.0 → v0.5.31), current at v0.5.50/v1.59.0

---

## WAVE 4: IMMEDIATE TASKS (7 tasks, ~200 lines)

### Task 78: Remove notify_url/notify_key dead code
**Files**: `proxy.rs` (ProxyConfig:78-79, build_state:295-296, test structs:851-852,940-941, notification spawn:768-798), `config.rs` (Cli:72,76, ProxyConfig:115-116, apply_cli:152-153)  
**Action**: Delete all notify fields. Remove notification callback spawn in handle_ccr_create.  
**Impact**: ~50 lines removed, simpler API surface.

### Task 79: Remove request_history ring buffer
**Files**: `proxy.rs` (AppState:85, stats_json:139, init:301,862,951, record_request:205-217), `main.rs` (history route:144)  
**Action**: Delete the field, init entries, stats_json entry, record_request method, and /history route.  
**Impact**: ~30 lines removed, no memory leak risk.

### Task 80: Stop spawning cache mode (:9797) — single token mode
**Files**: `main.rs` (cache spawn block), `config.rs` (ProxyMode default), `proxy.rs` (cache branches in compress_chat_completion, health_check)  
**Research**: Plugin only starts token (:9798). Cache mode (:9797) is never used. On_start only spawns token. Fallback code in _alive tries cache port but it's vestigial.  
**Action**: Remove cache spawn from main.rs. Simplify ProxyMode defaults. Collapse cache code paths to token-only in compress_chat_completion. Update health_check mode labels.  
**Impact**: ~50 lines removed, single-mode simplicity.

### Task 4: Remove liteLLM from pyproject.toml
**Files**: `vendor/headroom/pyproject.toml` (line 230: litellm, line 363: providers.litellm entry_point)  
**Research**: liteLLM only used by backends/litellm.py (dead) and integrations/litellm_callback.py (already deleted). No other imports.  
**Action**: Remove dependency line + entry point.  
**Impact**: 2 lines, ~200MB savings in dependency tree.

### Task 30: Add linter output detection to detect_content_type
**Files**: `proxy.rs` detect_content_type (line 483)  
**Research**: Already detects 13 types: code, error, build_output, diff, git, json, log, text. Missing: rustc (`error[E`), clippy (`warning:`), mypy (`mypy`), eslint.  
**Action**: Add linter patterns before general error/match.  
**Impact**: ~15 lines, better compression decisions for lint output.

### Task 56: Tag conversation turns by file type
**Files**: `__init__.py` _store_conversation_turn (line 533-587)  
**Research**: Already tracks _referenced_files. Summary only includes user/assistant text.  
**Action**: Extract file extensions from _referenced_files, append [.rs(3) .py(2)] tag to summary.  
**Impact**: ~15 lines, better conversation search/catalog.

### Task 71: Add benchmark mode to aphrodite_test
**Files**: `__init__.py` _test_handler (line 1178-1292), TEST_SCHEMA (line 1294-1303)  
**Research**: Test handler has quick/full/matrix/pipeline modes. No benchmark mode with ratio comparisons per type.  
**Action**: Add "benchmark" mode: compress payloads per type, report original_size, compressed_size, ratio, proxy_alive status. Update schema description.  
**Impact**: ~40 lines, quantitative compression performance measurement.

---

## EXECUTION ORDER
1. 4.md → write plan (this file)
2. Task 78: notify dead code removal
3. Task 79: request_history removal
4. Task 80: cache mode removal
5. Task 4: liteLLM removal
6. Task 30: linter detection
7. Task 56: file-type tagging
8. Task 71: benchmark mode
9. cargo build + test
10. Bump version + commit + push
