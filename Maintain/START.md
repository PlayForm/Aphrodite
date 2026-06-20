# aphrodite - START

> Generic LLM proxy with CCR + tool relay - any OpenAI-compatible API.

## Port Verification

Before starting, verify ports are free:

```bash
lsof -ti:9797 -ti:9798 -ti:9799 -ti:9800 | xargs kill -9 2> /dev/null
echo "Ports free: $(lsof -i:9797 -i:9798 -i:9799 -i:9800 2> /dev/null | grep LISTEN || echo 'all clear')"
```

## Bootstrap

### Quick Start

```bash
# Multi-proxy from config
aphrodite # reads aphrodite.toml → starts :9797 + :9798

# Cache mode only
aphrodite --mode cache --listen :9797 --api-key $APHRODITE_UPSTREAM_API_KEY

# Token mode with tool relay
aphrodite --mode token --listen :9798 --api-key $APHRODITE_UPSTREAM_API_KEY --tool-relay
```

### Dev rebuild (auto-reload)

```bash
APHRODITE_UPSTREAM_API_KEY=sk-... cargo watch -x 'run -p aphrodite'
```

### Hermes config

```yaml
providers:
    aphrodite-cache:
        api_key_env: APHRODITE_UPSTREAM_API_KEY
        provider: deepseek
        base_url: http://127.0.0.1:9797
    aphrodite-token:
        api_key_env: APHRODITE_UPSTREAM_API_KEY
        provider: deepseek
        base_url: http://127.0.0.1:9798
fallback_providers:
    - deepseek-direct
```

### Profiles

The repo ships ready-to-run Hermes profiles under `profiles/`:

| Profile                       | Ports        | CCR       | Threshold  |
| ----------------------------- | ------------ | --------- | ---------- |
| aphrodite-barebone            | :9797, :9798 | Off       | -          |
| aphrodite-proxy-cache         | :9797        | In-memory | >8KB       |
| aphrodite-proxy-token         | :9798        | SQLite    | >1KB       |
| aphrodite-compress-light      | :9797, :9798 | Both      | Light      |
| aphrodite-compress-medium     | :9797, :9798 | Both      | Medium     |
| aphrodite-compress-aggressive | :9797, :9798 | Both      | Aggressive |
| aphrodite-compress-off        | :9797, :9798 | Off       | -          |

### Health check

```bash
curl -s http://127.0.0.1:9797/health
curl -s http://127.0.0.1:9798/health
```
