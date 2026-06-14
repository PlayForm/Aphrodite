# HermesCompress — Session Handoff Plan (updated v0.7.10+)

## Current State (branch Current, post-fix)

### What was fixed this session

| Issue | Status | Detail |
|-------|--------|--------|
| Monkey-patch broken | ✅ Fixed | Targets `agent._interruptible_api_call` + `_streaming` (run_agent.py:4012,4183) instead of nonexistent `_call_llm_with_retry` |
| headroom-ai missing | ✅ Installed | `pip install headroom-ai` in `~/.hermes/hermes-agent/venv` (v0.25.0) |
| Plugin silent failure | ✅ Fixed | Now prints `[hermes-compress-shim] ✓ patched agent API hooks` on startup |

### What's running
| Component | Status | Detail |
|-----------|--------|--------|
| Cache proxy | :8787 | ✓ HEALTHY (348 req, 301 frozen) |
| Token proxy | :8788 | ✓ HEALTHY (16 req, 4 frozen) |
| Hermes session | Direct to DeepSeek | Plugin loaded but patch not active this session (needs restart) |

### Verified working
| Test | Result |
|------|--------|
| Structural test (3 sizes) | ✓ 6/6 passed |
| Standalone compression (29 msgs) | ✓ 7% overall (-65% on dedup content) |
| Cold start (Kompress ONNX) | ✓ Loads on first call (~10-15s) |
| Tool output integrity | ✓ All tool outputs preserved |

### Configuration
```
Plugin:        hermes-compress-shim (symlinked from repo)
Config:        protect_recent=1, min_tokens=100, target_ratio=None
headroom-ai:   0.25.0 (installed in hermes-agent venv)
Proxy:         dual (cache :8787 + token :8788)
```

---

## New Session Checklist (UPDATED)

### 1. Verify shim loaded with fix
Start Hermes fresh. Check stderr for:
```
[hermes-compress-shim] ✓ patched agent API hooks
```
If you see `WARNING: no intercept hook found` — agent class changed, investigate.

### 2. Test compression activates
Build ~15+ messages with tool output. The first API call will be slow (10-15s, Kompress ONNX load). Subsequent calls fast (~50-80ms).

Compression activates progressively:
- First 5-8 messages: 0% (ContentRouter protects small payloads)
- 15+ messages with code: 7-15% (code is mostly unique)
- 20+ messages with repeated patterns: 33-37% (dedup + code compression)
- 35+ messages accumulated: 55-67% (skill benchmarks)

### 3. Test headroom_retrieve tool
The `headroom_retrieve` tool is registered. It requires the cache proxy on :8787.
CCR markers only appear in proxy mode — inline mode produces regular messages.

### 4. Known issues unchanged
- v4-pro thinking overhead: needs `max_tokens >= 1200`
- Compression needs accumulated context (12+ messages)
- `protect_recent=0` + `target_ratio <= 0.10` can corrupt code
- Proxy doesn't compress Chat Completions — inline shim is the only path

---

## Test Commands

```bash
# Structural test
.venv/bin/python tests/test_shim_compress.py

# Standalone compression test
.venv/bin/python tests/shim_hermes_compress.py --test

# Dual proxy status
.venv/bin/python scripts/proxy-dual.py --status

# Full benchmark report
HEADROOM_DEEPSEEK_KEY=<key> .venv/bin/python tests/report.py

# Tune configs over frozen cache
.venv/bin/python tests/tune.py

# Verify headroom installed
~/.hermes/hermes-agent/venv/bin/python -c "import headroom; print('OK')"
```

---

## Files modified this session

| File | Change |
|------|--------|
| `plugins/hermes-compress-shim/__init__.py` | Fixed monkey-patch to target real API methods |
| `tests/shim_hermes_compress.py` | Synced with plugin fix |
| `BENCHMARK.md` | Created — live benchmark plan for dedicated session |
| `~/.hermes/hermes-agent/venv` | Installed headroom-ai v0.25.0 |

---

## Next Steps

### Immediate (before restart)
- [x] Monkey-patch targets correct methods
- [x] headroom-ai installed in hermes-agent venv
- [x] BENCHMARK.md ready for dedicated session

### After restart
- [ ] Verify `[hermes-compress-shim] ✓ patched agent API hooks` in logs
- [ ] Run BENCHMARK.md in a dedicated session
- [ ] Verify compression savings on real-world tool output

### Development
- [ ] Replace monkey-patch with proper `pre_api_request` hook
- [ ] Add `pre_llm_call` hook support in Hermes core for message access
- [ ] `hermes-compress install` command to symlink plugin + install deps
- [ ] Report generation with live benchmark results
