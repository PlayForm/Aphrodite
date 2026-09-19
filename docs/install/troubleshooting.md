# Install Troubleshooting

## Proxy doesn't auto-launch

When Hermes loads the plugin, it tries to launch the proxy binary
automatically. Here's what actually happens, in order:

| Step             | Behavior                                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Binary lookup | The resolved path is `~/.hermes/aphrodite/binaries/aphrodite` (`aphrodite.exe` on Windows), or `APHRODITE_BINARY_PATH` if set. A legacy `binaries/` copy inside the plugin directory still loads, with a warning, but the canonical runtime home wins. If no binary exists anywhere, the plugin logs a warning and continues - it does not fail plugin registration.                                       |
| 2. Auto-download | If the binary or the dylib is missing, the plugin runs `download.sh`/`download.ps1` itself (SHA-256 verified) before launching. A failure here is usually network access to GitHub Releases or a `BINARY_VERSION` mismatch - the plugin logs the script failure and its output tail.                                                                                                                       |
| 3. Launch        | The binary launches with stderr redirected to `~/.hermes/aphrodite/proxy-stderr.log` - check this file first for anything that goes wrong after launch (bad `aphrodite.toml`, "unable to open database file", port already in use).                                                                                                                                                                        |
| 4. Health poll   | Before launching, the plugin probes both proxy ports' `/health` endpoints (default `:9797`/`:9798`, or `APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`); if another process already answers a confirmed Aphrodite health body on both, the launch is skipped. After launching, it polls both endpoints for up to 5 seconds - a warning names any port that doesn't come up and points at `proxy-stderr.log`. |

**Checklist, in order:**

| #   | Check                                                                                                                                                                                                                     |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Does the binary exist at the exact filename the plugin expects for your platform in `~/.hermes/aphrodite/binaries/`? (`aphrodite.exe` on Windows, plain `aphrodite` elsewhere - no version suffix, no extension mismatch) |
| 2   | Is it executable? (Handled automatically on Unix if not, but can't help if the file itself is wrong, e.g. a 0-byte failed download)                                                                                       |
| 3   | Tail `~/.hermes/aphrodite/proxy-stderr.log` for the actual startup error                                                                                                                                                  |
| 4   | Confirm nothing else is already bound to `:9797`/`:9798`                                                                                                                                                                  |
| 5   | If all of the above look fine, launch the binary yourself in a terminal (see [below](#verify-the-proxy-without-hermes)) so you see errors directly instead of through the log file                                        |

To stop the plugin from launching a proxy at all (for example, because you
manage it yourself per [Verify the proxy without Hermes](#verify-the-proxy-without-hermes)),
set `APHRODITE_NO_AUTO_LAUNCH=1` in the environment - there is no
`auto_start` key in either config file.

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
`--api-key` (or `APHRODITE_API_KEY` in the environment) is required as a
flag - the error names it explicitly: "no API key configured - set
APHRODITE_API_KEY env var or api_key in aphrodite.toml". This is a
legitimate way to confirm the binary launches and serves `/health`/`/metrics`
correctly, entirely independent of Hermes - it doesn't need to be a real
provider key unless you go on to send a completion request through the
proxy.

Once the plugin is registered with a live Hermes session, two more paths
exist entirely inside the agent, no CLI needed:

| Tool              | What it checks                                                                                                                                                          |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aphrodite_stats` | Proxy health + engine status - see [aphrodite_stats](../tool-relay/tools.md#3-aphrodite_stats)                                                                          |
| `aphrodite_test`  | An in-process compress/retrieve smoke test that doesn't depend on the HTTP proxy being reachable at all - see [aphrodite_test](../tool-relay/tools.md#8-aphrodite_test) |

## Two separate config files

Aphrodite touches two config files with no shared keys:

| File             | Read by                              | Example keys                                                                                                                                                            |
| ---------------- | ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aphrodite.toml` | The `aphrodite` proxy binary / dylib | `[[proxies]]`, `[compression]`, `[previews]`, `[prompts]`, `[directives]`, `[templates.*]` - full schema in [aphrodite.toml Configuration](../config/aphrodite-toml.md) |
| `config.yaml`    | Hermes Agent itself                  | `providers.*`, `plugins.enabled`, `context.engine`, `model.*`, and hundreds more unrelated to Aphrodite                                                                 |

Conflating the two is an easy mistake: `cache_port`, `token_port`,
`compression_threshold`, `classifier_poll`, `context_engine`, and `previews`
belong to `aphrodite.toml`, not Hermes's `config.yaml`. The proxy resolves
its config from `~/.hermes/aphrodite/aphrodite.toml` (or
`APHRODITE_CONFIG_PATH`) when no `aphrodite.toml` sits in the working
directory.

The only Aphrodite-relevant keys that belong in `config.yaml` are
`plugins.enabled: [aphrodite]` (added automatically by
`hermes plugins enable aphrodite`) and, optionally,
`context.engine: aphrodite` / `context.engine_threshold_pct` if you want
Hermes to route its context-engine offloading through Aphrodite. Everything
else that tunes _how_ Aphrodite compresses - thresholds, preview style,
ports, prompt wording - belongs in `aphrodite.toml`.

There is no `auto_start` key in either file: the plugin always attempts to
launch the proxy if a binary is present at the resolved path. Set
`APHRODITE_NO_AUTO_LAUNCH=1` to prevent it from launching a proxy itself
(for example, because you're launching it manually per
[Verify the proxy without Hermes](#verify-the-proxy-without-hermes)).
