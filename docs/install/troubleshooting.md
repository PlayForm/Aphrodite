# Install Troubleshooting

## Proxy doesn't auto-launch

When Hermes loads the plugin, it tries to launch the proxy binary
automatically. Here's what actually happens, in order:

| Step | Behavior |
| --- | --- |
| 1. Binary lookup | If no binary exists at the resolved path (`binaries/aphrodite`, `binaries/aphrodite.exe` on Windows, or `APHRODITE_BINARY_PATH` if set), the plugin logs a warning and continues - it does not fail plugin registration. There's nothing Windows-specific here; if launch silently doesn't happen, the binary is almost always missing or misnamed (see [Windows install](windows.md#step-3-get-the-binary-and-dylib) for the common cause: a downloaded release asset that still has its platform-suffixed name instead of the fixed name the loader expects). |
| 2. Launch | If the binary exists, it launches with stderr redirected to `~/.hermes/aphrodite/proxy-stderr.log` - check this file first for anything that goes wrong after launch (bad `aphrodite.toml`, "unable to open database file", port already in use). |
| 3. Health poll | After launching, the plugin polls both proxy ports' `/health` endpoints (default `:9797`/`:9798`, or `APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`) for up to 5 seconds. If either doesn't answer, a warning names the port and points at `proxy-stderr.log`. |

**Checklist, in order:**

| # | Check |
| --- | --- |
| 1 | Does the binary exist at the exact filename the plugin expects for your platform? (`aphrodite.exe` on Windows, plain `aphrodite` elsewhere - no version suffix, no extension mismatch) |
| 2 | Is it executable? (Handled automatically on Unix if not, but can't help if the file itself is wrong, e.g. a 0-byte failed download) |
| 3 | Tail `~/.hermes/aphrodite/proxy-stderr.log` for the actual startup error |
| 4 | Confirm nothing else is already bound to `:9797`/`:9798` |
| 5 | If all of the above look fine, launch the binary yourself in a terminal (see [below](#verify-the-proxy-without-hermes)) so you see errors directly instead of through the log file |

## Verify the proxy without Hermes

You don't need a real Hermes session, a real Hermes install, or a real
upstream API key to confirm the `aphrodite` binary itself works. The API-key
requirement only matters once a request actually needs to reach an upstream
LLM - starting the proxy and hitting `/health` never calls upstream at all,
so a placeholder value is enough:

```bash
# any placeholder string works - it's never sent anywhere for /health
./aphrodite --api-key sk-placeholder --listen 127.0.0.1:9798
```

```bash
curl http://127.0.0.1:9798/health
# {"status":"healthy","ccr":true,"mode":"token","version":"<current version>","fill_pct":...}
```

Invoked this way, with no `aphrodite.toml` in the working directory,
`--api-key` (or `APHRODITE_API_KEY`/`DEEPSEEK_API_KEY`/`HEADROOM_DEEPSEEK_KEY`
in the environment) is required as a flag - see the full
[resolution chain](../config/aphrodite-toml.md#api-key-resolution-chain). This
is a legitimate way to confirm the binary launches and serves
`/health`/`/metrics` correctly, entirely independent of Hermes - it doesn't
need to be a real provider key unless you go on to send a completion request
through the proxy.

Once the plugin is registered with a live Hermes session, two more paths
exist entirely inside the agent, no CLI needed:

| Tool | What it checks |
| --- | --- |
| `aphrodite_stats` | Proxy health + engine status - see [aphrodite_stats](../tool-relay/tools.md#3-aphrodite_stats) |
| `aphrodite_test` | An in-process compress/retrieve smoke test that doesn't depend on the HTTP proxy being reachable at all - see [aphrodite_test](../tool-relay/tools.md#7-aphrodite_test) |

## Two separate config files

Aphrodite touches two config files with no shared keys:

| File | Read by | Example keys |
| --- | --- | --- |
| `aphrodite.toml` | The `aphrodite` proxy binary / dylib | `[[proxies]]`, `[compression]`, `[previews]`, `[prompts]`, `[templates.*]` - full schema in [aphrodite.toml Configuration](../config/aphrodite-toml.md) |
| `config.yaml` | Hermes Agent itself | `providers.*`, `plugins.enabled`, `context.engine`, `model.*`, and hundreds more unrelated to Aphrodite |

Conflating the two is an easy mistake: `cache_port`, `token_port`,
`compression_threshold`, `classifier_poll`, `context_engine`, and `previews`
belong to `aphrodite.toml`, not Hermes's `config.yaml`.

The only Aphrodite-relevant keys that belong in `config.yaml` are
`plugins.enabled: [aphrodite]` (added automatically by
`hermes plugins enable aphrodite`) and, optionally,
`context.engine: aphrodite` / `context.engine_threshold_pct` if you want
Hermes to route its context-engine offloading through Aphrodite. Everything
else that tunes *how* Aphrodite compresses - thresholds, preview style, ports,
prompt wording - belongs in `aphrodite.toml`.

There is no `auto_start` key in either file: the plugin always attempts to
launch the proxy if a binary is present at the resolved path. If you need to
*prevent* it from launching a proxy itself (for example, because you're
launching it manually per
[Verify the proxy without Hermes](#verify-the-proxy-without-hermes)), the only
lever available today is to not place a binary at the expected path, or to
point `APHRODITE_BINARY_PATH` at a nonexistent file.
