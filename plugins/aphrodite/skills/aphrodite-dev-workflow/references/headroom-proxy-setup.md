# Headroom Proxy as Hermes Provider

End-to-end setup for using the headroom Python proxy as a caching Hermes provider.

## Architecture

```
Hermes Agent → headroom-cache (:9799) → DeepSeek API
             └─ fallback: deepseek (direct)
```

## Two-Key System

- **Headroom proxy → DeepSeek**: Uses `HEADROOM_DEEPSEEK_KEY` (Key B) for upstream auth via litellm
- **Hermes → proxy**: Sends `HEADROOM_DEEPSEEK_KEY` via `Authorization: Bearer` header
- **Aphrodite/Hermes → DeepSeek**: Uses `APHRODITE_API_KEY` (Key A) for direct API calls
- Both keys in `~/.privateenvsh` (lines 29-33) and `~/.hermes/.env`

## Launch Command

```bash
source ~/.privateenvsh
headroom proxy \
  --port 9799 \
  --host 127.0.0.1 \
  --openai-api-url https://api.deepseek.com/v1 \
  --mode token \
  --workers 1 \
  --no-subscription-tracking \
  --no-optimize \
  --no-ccr-marker \
  --no-telemetry &
```

⚠ Multi-worker CCR fragmentation: `>1` worker means per-process in-memory stores.

## Hermes Provider Config

```yaml
providers:
  headroom-cache:
    provider: deepseek
    api_key_env: HEADROOM_DEEPSEEK_KEY
    base_url: http://127.0.0.1:9799
    max_tokens: 65536
```

## Env Propagation

Proxy must be launched from WezTerm after `source ~/.privateenvsh` — Hermes terminal background has unreliable environment propagation. The proxy needs `HEADROOM_DEEPSEEK_KEY` in its environment at startup.

## Debugging 401/502

1. Verify proxy has key: `ps aux | grep headroom` (no key visible, confirmed by env)
2. Test direct: `curl -H "Authorization: Bearer $HEADROOM_DEEPSEEK_KEY" http://:9799/v1/chat/completions ...`
3. Check Hermes config: `provider: deepseek` + `api_key_env: HEADROOM_DEEPSEEK_KEY`
4. If 502: proxy can't reach upstream — check `HEADROOM_DEEPSEEK_KEY` is valid
5. If 401: client auth mismatch — check `api_key_env` matches available env var
