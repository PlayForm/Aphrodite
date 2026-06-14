# HermesCompress — Session Handoff Plan

## Current State (v0.7.10, commit a9850d8)

### What's running
| Component | Status | Detail |
|-----------|--------|--------|
| Cache proxy | :8787 | Running (dual proxy) |
| Token proxy | :8788 | Running (dual proxy) |
| Hermes session | Direct to DeepSeek | No proxy routing |

### Configuration
```
model.base_url:  https://api.deepseek.com/v1        ← direct, no proxy
model.default:   deepseek-v4-pro
max_output:      384,000
plugin:          hermes-compress-shim (auto-patches)
toolset:         headroom
```

### Plugin
```
~/.hermes/plugins/hermes-compress-shim → repo/plugins/hermes-compress-shim
```
Auto-patches `agent/conversation_loop.py` on startup.
Two jobs only: compress api_messages + headroom_retrieve tool.
No measurement. No filtering. No response handling.

### Key files
```
plugins/hermes-compress-shim/__init__.py   ← production plugin (symlinked)
plugins/hermes-compress-shim/plugin.yaml
tests/shim_hermes_compress.py             ← standalone shim + smoke test
tests/test_shim_compress.py               ← structural test (6/6 passed)
tests/test_proxy_compare.py               ← dual proxy comparison
tests/test_token_proxy.py                 ← token proxy integration test
scripts/proxy-dual.py                     ← dual proxy launcher
scripts/proxy-start.py                    ← single proxy starter
```

---

## New Session Checklist

### 1. Verify the shim loaded
Check Hermes logs for `[hermes-compress-shim]` marker on startup.
If missing, verify:
- `plugins.enabled` includes `hermes-compress-shim`
- Symlink resolves: `ls ~/.hermes/plugins/hermes-compress-shim/__init__.py`
- Hermes venv can import: `~/.hermes/hermes-agent/venv/bin/python -c "from hermes_compress import Compress"`

### 2. Test compression activates
Run a session with ~15+ messages (tool outputs, file reads).
Compression needs accumulated context — won't fire on first few turns.
Expected: 33-37% savings once enough tool outputs accumulate.

### 3. Test headroom_retrieve tool
If CCR markers appear in compressed output, use the `headroom_retrieve` tool
to fetch original content. Requires cache proxy running on :8787.

### 4. Known issues to watch
- v4-pro thinking overhead: needs `max_tokens >= 1200` for visible output
- Compression doesn't activate on < 12 messages (ContentRouter protects all)
- `protect_recent=0` + `target_ratio <= 0.10` can corrupt code
- The proxy doesn't compress Chat Completions — inline shim is the only path

### 5. Next development steps
- [ ] Shim into Hermes properly (current monkey-patch is fragile)
- [ ] Add `pre_llm_call` hook support in Hermes core for message access
- [ ] Report generation with shim-mode benchmark results
- [ ] `hermes-compress install` command to symlink plugin automatically

---

## Test Commands

```bash
# Shim standalone smoke test
.venv/bin/python tests/shim_hermes_compress.py --test

# Structural test (3 sizes, integrity checks)
.venv/bin/python tests/test_shim_compress.py

# Dual proxy status
.venv/bin/python scripts/proxy-dual.py --status

# Full benchmark (needs HEADROOM_DEEPSEEK_KEY)
HEADROOM_DEEPSEEK_KEY=<key> .venv/bin/python tests/report.py
```
