# Hermes Profile Testing Matrix

## Creating a multi-profile testbed for aphrodite

### Profile Matrix (7 profiles)

| Profile | Provider | Context Engine | Threshold | Plugin |
|---|---|---|---|---|
| `aphrodite-barebone` | deepseek direct | compressor | off | no |
| `aphrodite-proxy-cache` | aphrodite-cache :9797 | aphrodite | 50% | yes |
| `aphrodite-proxy-token` | aphrodite-token :9798 | aphrodite | 50% | yes |
| `aphrodite-compress-off` | aphrodite-cache :9797 | aphrodite | 0% | yes |
| `aphrodite-compress-light` | aphrodite-cache :9797 | aphrodite | 90% | yes |
| `aphrodite-compress-medium` | aphrodite-cache :9797 | aphrodite | 50% | yes |
| `aphrodite-compress-aggressive` | aphrodite-cache :9797 | aphrodite | 10% | yes |

### Creation

```bash
# Clone from default (copies config, .env, skills)
hermes profile create aphrodite-barebone --clone
hermes profile create aphrodite-proxy-cache --clone
# ... etc.

# Customize each profile's config.yaml:
# - model.provider: aphrodite-cache / aphrodite-token / deepseek
# - context.engine: aphrodite / compressor
# - compression.enabled: true / false
# - compression.threshold: 0.1 / 0.5 / 0.9

# Add per-profile env vars to each .env:
echo "export APHRODITE_ENGINE_THRESHOLD_PCT=50" >> ~/.hermes/profiles/aphrodite-proxy-cache/.env
echo "export APHRODITE_ENGINE_THRESHOLD_PCT=90" >> ~/.hermes/profiles/aphrodite-compress-light/.env
# etc.
```

### Per-profile plugins

Each profile gets its own plugin copy:
```bash
mkdir -p ~/.hermes/profiles/aphrodite-proxy-cache/plugins/aphrodite
cp plugins/aphrodite/*.py plugins/aphrodite/plugin.yaml ~/.hermes/profiles/aphrodite-proxy-cache/plugins/aphrodite/
```

Enable per profile:
```bash
hermes --profile aphrodite-proxy-cache plugins enable aphrodite
```

### Launching all profiles

Use WezTerm pane splits — one profile per pane:
```bash
# Split and launch
wezterm cli split-pane --bottom --percent 50
wezterm cli send-text --pane-id N --no-paste $'hermes --profile aphrodite-barebone\n'

# Verify
mcp_wezterm_get_buffer(pane_id=N, lines=10)
```

### Killing all profiles

```bash
pkill -f "hermes --profile aphrodite"
```

### Pitfalls

- Profiles share the same proxy infrastructure — only ONE proxy instance needed
- Don't set `context.engine: aphrodite` AND `compression.enabled: true` — they conflict
- Per-profile `.env` vars are loaded by Hermes at startup — no need to set inline
- When updating plugin code, sync all 6 profile copies with the latest files
- The `default` profile should NEVER be modified during testing
