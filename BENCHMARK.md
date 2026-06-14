# HermesCompress — Live Inline Compression Benchmark

**Task**: Run a dedicated Hermes session that validates the inline compression shim
is active and measures real-world token savings.

## Prerequisites

Before starting this session, verify:

```bash
# Plugin file is the fixed version (v0.7.10+)
grep '_interruptible_api_call' ~/.hermes/plugins/hermes-compress-shim/__init__.py | head -3

# hermes_compress importable in Hermes venv
~/.hermes/hermes-agent/venv/bin/python -c "from hermes_compress import Compress; print('OK')"

# Dual proxies running (for headroom_retrieve tool testing)
cd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress
.venv/bin/python scripts/proxy-dual.py --status
```

Expected proxy output: both `[cache] :8787` and `[token] :8788` should show `✓ HEALTHY`.

## Step 1 — Verify Shim Loaded

1. Start Hermes (fresh session with the fixed plugin)
2. Check stderr for: `[hermes-compress-shim] ✓ patched agent API hooks`
3. If you see `WARNING: no intercept hook found` — the agent class changed, investigate

## Step 2 — Verify / Test Compression

Run these tasks in order — each builds context for the next:

### Task A: Prime the compressor (~10 tool outputs)

```bash
# Read several files to accumulate context
# Compression needs ~12+ messages before ContentRouter activates
read_file scripts/proxy-start.py
read_file scripts/proxy-dual.py
read_file tests/test_shim_compress.py
read_file plugins/hermes-compress-shim/__init__.py
read_file README.md  # this is large — good compression target
```

### Task B: Verify compression is working

After Task A, ask the agent to:
```
Run these commands and report the output:
1. cd HermesCompress && .venv/bin/python -c "
from hermes_compress import Compress, CompressOption
opt = CompressOption()
opt.Enabled = True
opt.Mode = 'inline'
opt.ProtectRecent = 1
opt.MinTokensToCompress = 100
c = Compress(model='deepseek-v4-pro', option=opt)

# Build a test conversation from the files we just read
import sys
sys.path.insert(0, '.')
from pathlib import Path
files = {
    'proxy-start.py': Path('scripts/proxy-start.py').read_text(),
    'proxy-dual.py': Path('scripts/proxy-dual.py').read_text(),
}
msgs = [{'role': 'system', 'content': 'Be concise.'}]
for i in range(4):
    msgs.append({'role': 'user', 'content': f'Turn {i+1}'})
    msgs.append({'role': 'assistant', 'content': None, 'tool_calls': [
        {'id': f'c{i}', 'type': 'function', 'function': {'name': 'read_file', 'arguments': f'{{\"path\":\"test{i}.py\"}}'}}
    ]})
    msgs.append({'role': 'tool', 'content': 'REPEATED PADDING ' * 50, 'tool_call_id': f'c{i}'})
    fname = 'proxy-start.py' if i % 2 == 0 else 'proxy-dual.py'
    msgs.append({'role': 'assistant', 'content': f'T{i+1} ok.'})
    msgs.append({'role': 'user', 'content': f'Read {fname}.'})
    msgs.append({'role': 'assistant', 'content': None, 'tool_calls': [
        {'id': f'cc{i}', 'type': 'function', 'function': {'name': 'read_file', 'arguments': f'{{\"path\":\"{fname}\"}}'}}
    ]})
    msgs.append({'role': 'tool', 'content': files[fname], 'tool_call_id': f'cc{i}'})

import json
orig = sum(len(json.dumps(m)) for m in msgs)
result = c.compress(msgs)
comp = sum(len(json.dumps(m)) for m in result.messages)
savings = (1 - comp/orig) * 100 if orig > 0 else 0
print(f'Messages: {len(result.messages)} (was {len(msgs)})')
print(f'Size before: {orig:,} chars')
print(f'Size after:  {comp:,} chars')
print(f'Savings:     {savings:.1f}%')
"

2. Run the structural test: .venv/bin/python tests/test_shim_compress.py
```

### Task C: Multi-turn stress test

Run 5-6 turns that generate large tool output:

```
1. Run: cat tests/report.py  # large file
2. Run: find tests/ -name '*.py' | head -20  # directory listing
3. Run: grep -r 'compress' hermes_compress/ --include='*.py' -c  # search with results
4. Run: cd HermesCompress && .venv/bin/python -c "
import json
# Generate ~20KB of JSON
data = {'items': [{'id': i, 'name': f'item_{i}', 'values': list(range(100))} for i in range(50)]}
print(json.dumps(data, indent=2)[:20000])
"
5. Read README.md again (should hit dedup cache)
```

After each turn, note whether the agent's responses seem correct (tool output not corrupted).

### Task D: Check token usage

Ask the agent:
```
Check the Hermes logs for token counts from this session:
  grep -i 'Request size\|tokens' ~/.hermes/logs/agent.log | tail -10
```

## Step 3 — Verify headroom_retrieve Tool

1. Ask the agent: "What tools are available with 'headroom' in their name?"
2. Test the tool directly by asking the agent to call `headroom_retrieve` with hash `test123`
   - Expected: HTTP 404 response saying "Entry not found" (valid — test hash doesn't exist)

## Step 4 — Report

After completing steps 1-3, have the agent write a summary to:

```
reports/YYYY-MM-DD/live-benchmark.md
```

Include:
- Whether `[hermes-compress-shim] ✓ patched agent API hooks` appeared
- Standalone compression test results (size before/after, % savings)
- Any corrupted tool outputs observed
- Token usage pattern across turns (did tokens grow linearly or sub-linearly?)
- Proxy stats at end of session: `curl http://127.0.0.1:8787/stats`

## Known Issues to Watch

| Symptom | Likely Cause |
|---------|-------------|
| No `patched agent API hooks` message | `register()` not called or import failed |
| `WARNING: no intercept hook found` | Agent class changed, methods renamed |
| First turn slow (15s) | Expected — Kompress ONNX model loading |
| Compression 0% on first 5 turns | Expected — needs 12+ accumulated messages |
| Tool output corrupted | `protect_recent=0` — check COMPRESS_CONFIG |
| `headroom_retrieve` not found | Plugin tools not registered — check plugin.yaml |

---

**Expected outcome**: Compression should show 33-37% savings on accumulated sessions
with `protect_recent=1, target_ratio=None`. The inline pipeline runs 8 phases per
API call (pre-process → optimize → strategies → truncation → dedup → pre-compress
→ headroom → stats).
