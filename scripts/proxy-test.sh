#!/bin/bash
# Test headroom proxy routing - deepseek-v4-pro (1M ctx, 384K output)
# Usage: bash scripts/proxy-test.sh [port]
#
#   bash scripts/proxy-test.sh 8787   # test cache proxy
#   bash scripts/proxy-test.sh 8788   # test token proxy

PORT="${1:-8787}"
KEY="${HEADROOM_DEEPSEEK_KEY}"

if [ -z "$KEY" ] && [ -f "$(dirname "$0")/../.env" ]; then
    source "$(dirname "$0")/../.env"
    KEY="${HEADROOM_DEEPSEEK_KEY}"
fi

MAX_OUTPUT=384000   # v4-pro max output tokens (thinking + visible)

echo "=== Health Check ==="
curl -s "http://127.0.0.1:$PORT/health" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'Status: {d[\"status\"]}')
print(f'Version: {d[\"version\"]}')
print(f'Uptime: {d[\"uptime_seconds\"]:.0f}s')
print(f'Backend: {d[\"config\"][\"backend\"]}')
print(f'Mode: {d[\"config\"].get(\"savings_profile\", \"N/A\")}')
"

echo ""
echo "=== Tiny Test ==="
curl -s -X POST "http://127.0.0.1:$PORT/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KEY" \
  -d "{\"model\":\"deepseek-v4-pro\",\"messages\":[{\"role\":\"user\",\"content\":\"Say hi\"}],\"max_tokens\":$MAX_OUTPUT}" \
  | python3 -c "
import sys, json
d = json.load(sys.stdin)
if 'error' in d:
    print(f'FAIL: {d[\"error\"][\"message\"][:100]}')
else:
    u = d.get('usage', {})
    print(f'OK: \"{d[\"choices\"][0][\"message\"][\"content\"].strip()}\"')
    print(f'Tokens: {u.get(\"prompt_tokens\",\"?\")} in / {u.get(\"completion_tokens\",\"?\")} out / {u.get(\"total_tokens\",\"?\")} total')
"

echo ""
echo "=== Large Compression Test ==="
curl -s -X POST "http://127.0.0.1:$PORT/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KEY" \
  -d '{
    "model":"deepseek-v4-pro",
    "messages":[
      {"role":"system","content":"You are a code assistant. Return raw data only. No explanations."},
      {"role":"user","content":"Search for all def functions in: hermes_compress/__init__.py has register() hook registration and _pre_llm_call_hook() and _transform_tool_result_hook(). hermes_compress/_compress.py has compress() method and _compress_inline() and _compress_proxy() private methods. hermes_compress/_strategies.py has get_strategy() and compress_thinking_block(). hermes_compress/_dev.py has is_dev() and StatsCollector with record() and summary(). hermes_compress/_install.py has install() and uninstall() and status()."},
      {"role":"assistant","content":"Found: register(), _pre_llm_call_hook(), _transform_tool_result_hook(), compress(), _compress_inline(), _compress_proxy(), get_strategy(), compress_thinking_block(), is_dev(), record(), summary(), install(), uninstall(), status()"},
      {"role":"user","content":"Now read hermes_compress/_strategies.py to confirm the STRATEGIES dict. Also run: find hermes_compress/ -name \"*.py\" | xargs wc -l"}
    ],
    "max_tokens":500
  }' | python3 -c "
import sys, json
d = json.load(sys.stdin)
if 'error' in d:
    print(f'FAIL: {d[\"error\"][\"message\"][:120]}')
else:
    u = d.get('usage', {})
    prompt = u.get('prompt_tokens', 0)
    comp = u.get('completion_tokens', 0)
    total = u.get('total_tokens', 0)
    raw_est = 400
    savings = raw_est - prompt
    print(f'Prompt: {prompt} tokens (est. raw: ~{raw_est}, saved: ~{savings} = {savings/raw_est*100:.0f}%)')
    print(f'Completion: {comp} | Total: {total}')
    content = d['choices'][0]['message']['content'][:150]
    print(f'Response: {content}...')
"

echo ""
echo "=== Proxy Stats ==="
curl -s "http://127.0.0.1:$PORT/stats" | python3 -c "
import sys, json
d = json.load(sys.stdin)
s = d['summary']
c = s['compression']
u = s.get('uncompressed_requests', {})
print(f'Mode: {s[\"mode\"]}')
print(f'Requests: {s[\"api_requests\"]}')
print(f'Compressed: {c[\"requests_compressed\"]} | Avg pct: {c[\"avg_compression_pct\"]}%')
print(f'Tokens removed: {c[\"total_tokens_removed\"]}')
print(f'Prefix frozen: {u.get(\"prefix_frozen\", \"N/A\")}')
print(f'Tip: {s.get(\"tip\", \"N/A\")}')
"
