# aphrodite.toml Configuration

Origin: Multi-proxy deployments configure multiple listeners (cache + token, or multiple token instances) via a TOML file. Configuration is resolved at startup: TOML → CLI → env vars → defaults.

Source of truth: `crates/aphrodite/src/config.rs` (lines 92-221), `crates/aphrodite/src/main.rs:run()` (line 45)

## File Location

Default: `aphrodite.toml` in current working directory.

Override: `APHRODITE_CONFIG_PATH` env var (main.rs:45).

If file doesn't exist: falls back to CLI parsing mode (single proxy).

## Full Schema

```toml
[defaults]
# Shared defaults across all [[proxies]]
api_url = "https://api.openai.com"
model = "default-model"
ccr_ttl_seconds = 3600
api_key = "sk-..."

[[proxies]]
name = "cache-proxy"
listen = "127.0.0.1:9797"
mode = "cache"
api_key = "sk-..."           # overrides defaults.api_key
api_url = "https://..."      # overrides defaults.api_url
model = "..."                # overrides defaults.model
tool_relay = false
dev = false
ccr_ttl_seconds = 3600
ccr_db_path = "~/data/ccr.db"
notify_url = "https://..."
notify_key = "bearer-token"
timeout = 300
max_context = 1000000
max_output = 384000

[[proxies]]
name = "token-proxy"
listen = "127.0.0.1:9798"
mode = "token"
tool_relay = true
```

## Struct Definitions

From `config.rs`:

### MultiConfig (line 93)
```rust
pub struct MultiConfig {
    pub defaults: Option<Defaults>,
    pub proxies: Vec<ProxyConfig>,
}
```

### Defaults (line 99)
```rust
pub struct Defaults {
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub ccr_ttl_seconds: Option<u64>,
    pub api_key: Option<String>,
}
```

### ProxyConfig (line 107)
```rust
pub struct ProxyConfig {
    pub name: Option<String>,          // defaults to listen address
    pub listen: Option<String>,        // defaults to 127.0.0.1:9797
    pub mode: Option<String>,          // "cache" or "token" (default: token)
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub tool_relay: Option<bool>,      // default false
    pub dev: Option<bool>,             // default false
    pub ccr_ttl_seconds: Option<u64>,  // default 3600
    pub ccr_db_path: Option<String>,   // default ~/.hermes/aphrodite/ccr.db
    pub notify_url: Option<String>,
    pub notify_key: Option<String>,
    pub timeout: Option<u64>,          // default 300, clamped to 600
    pub max_context: Option<usize>,    // default 1_000_000
    pub max_output: Option<usize>,     // default 384_000
}
```

## API Key Resolution Chain

From `config.rs:resolve()` (line 137):

```
proxy.api_key
  → defaults.api_key
    → APHRODITE_API_KEY env var
      → DEEPSEEK_API_KEY env var
        → HEADROOM_DEEPSEEK_KEY env var
          → Error: "no API key configured"
```

Stops at first non-empty value.

## Default Value Chain

| Field | Resolution |
|-------|-----------|
| listen | proxy.listen → `"127.0.0.1:9797"` |
| mode | proxy.mode → `"token"` (with warn on unknown) |
| api_url | proxy.api_url → defaults.api_url → `"https://api.openai.com"` |
| model | proxy.model → defaults.model → `"default-model"` |
| ccr_ttl_seconds | proxy.ccr_ttl_seconds → defaults.ccr_ttl_seconds → 3600 |
| ccr_db_path | proxy.ccr_db_path (non-empty) → `~/.hermes/aphrodite/ccr.db` (or `/tmp` fallback) |
| tool_relay | proxy.tool_relay → false |
| dev | proxy.dev → false |
| timeout | proxy.timeout → 300 (clamped: max 600) |
| max_context | proxy.max_context → 1_000_000 |
| max_output | proxy.max_output → 384_000 |

## Validation

### Max Output vs Max Context
From `config.rs:resolve()` (line 158):
```rust
if max_output >= max_context {
    anyhow::bail!("max_output ({max_output}) must be less than max_context ({max_context})");
}
```

### Listen Address
Must parse as `SocketAddr` or the proxy fails to start:
```rust
s.parse().map_err(|_| anyhow::anyhow!("invalid listen address: {s}"))?
```

### API Key
Must be non-empty after all fallbacks:
```rust
if api_key.is_empty() {
    anyhow::bail!("no API key configured");
}
```

### Timeout
Clamped to 600s max with warning:
```rust
let t = cfg.timeout.unwrap_or(300);
if t > 600 {
    tracing::warn!("timeout {}s exceeds maximum 600s, clamping", t);
    600
}
```

## Mode

### Valid Values
- `"token"`  -  SQLite CCR, >1KB threshold, tool relay, aggressive compression
- `"cache"`  -  In-memory CCR, >8KB threshold, no tool relay

### Unknown Values
```rust
Some(other) => {
    tracing::warn!("unknown mode {:?}, defaulting to token", other);
    ProxyMode::Token
}
```

### No Mode
```rust
None => {
    tracing::info!("no mode specified, defaulting to token");
    ProxyMode::Token
}
```

## Database Path Resolution

From `main.rs:run_single()` (line 159):
```rust
if !cli.ccr_db_path.as_os_str().is_empty() && !cli.ccr_db_path.is_absolute() {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            cli.ccr_db_path = exe_dir.join(&cli.ccr_db_path);
        }
    }
}
```
Relative paths resolved against binary directory. Parent directories auto-created.

## CLI Equivalents

When running without aphrodite.toml, CLI args mirror these fields:

| TOML Field | CLI Flag | Env Var |
|-----------|----------|---------|
| mode | `--mode` | `APHRODITE_MODE` |
| listen | `--listen` | `APHRODITE_LISTEN` |
| api_url | `--api-url` | `APHRODITE_API_URL` |
| api_key | `--api-key` | `APHRODITE_API_KEY` |
| model | `--model` | `APHRODITE_MODEL` |
| ccr_db_path | `--ccr-db-path` | `APHRODITE_DB` |
| ccr_ttl_seconds | `--ccr-ttl` | `APHRODITE_CCR_TTL` |
| tool_relay | `--tool-relay` |  -  |
| notify_url | `--notify-url` | `APHRODITE_NOTIFY_URL` |
| notify_key | `--notify-key` | `APHRODITE_NOTIFY_KEY` |
| dev | `--dev` |  -  |
| log_compact | `--log-compact` | `APHRODITE_LOG_COMPACT` |
| timeout | `--timeout` |  -  |

## Example: Full Multi-Proxy

```toml
[defaults]
api_url = "https://api.deepseek.com/v1"
model = "deepseek-v4-pro"
ccr_ttl_seconds = 7200

[[proxies]]
name = "cache"
listen = "127.0.0.1:9797"
mode = "cache"
ccr_ttl_seconds = 600

[[proxies]]
name = "token"
listen = "127.0.0.1:9798"
mode = "token"
tool_relay = true
notify_url = "https://hermes.internal/callback"
notify_key = "hermes-api-key-123"
timeout = 120
```
