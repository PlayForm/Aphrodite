# 07 - Config Resolution Precedence

Two independent resolution stacks share the same **env > TOML > default**
precedence but live in different modules:
- `config.rs` - the proxy's `MultiConfig::resolve` (CLI/`Cli` struct per listener).
- `config_loader.rs` - the FFI/Hermes `Config` loader (`apply_compression` into
  `AphroditeState`).

## Precedence flowchart

```mermaid
flowchart TD
    subgraph proxy["Proxy path - MultiConfig::resolve (config.rs:297)"]
      P0["aphrodite.toml [[proxies]] + [defaults] + [compression]"] --> P1
      P1{"per key"} --> P2["env APHRODITE_* (env_parse_warn / env var)"]
      P2 -->|set| PV["value"]
      P2 -->|unset| P3["proxy TOML field"]
      P3 -->|set| PV
      P3 -->|unset| P4["[defaults] field"]
      P4 -->|set| PV
      P4 -->|unset| P5["hardcoded default"]
      P5 --> PV
    end

    subgraph ffi["FFI/Hermes path - Config (config_loader.rs)"]
      F0["./aphrodite.toml → ~/.hermes/aphrodite/aphrodite.toml"] --> F1
      F1{"get_bool/u64/usize/string"} --> F2["1. runtime override (set_override)"]
      F2 -->|hit| FV["value"]
      F2 -->|miss| F3["2. env var (APHRODITE_*)"]
      F3 -->|hit| FV
      F3 -->|miss| F4["3. toml[section][key]"]
      F4 -->|hit| FV
      F4 -->|miss| F5["4. default arg"]
      F5 --> FV
    end
```

## Live vs inert keys

```mermaid
flowchart LR
    subgraph live["LIVE - actually consumed"]
      L1["tool_threshold_cache/token → cache/token_compress_threshold (atomics)"]
      L2["inline_threshold → inline_ccr_threshold (atomic)"]
      L3["code_multiplier → code_multiplier_x100 (atomic)"]
      L4["defaults.api_url/model, ccr_ttl_seconds, timeout"]
      L5["[flow] budget_chars → flow_budget_chars"]
      L6["[compression] context_engine, tool_threshold_token, terminal_threshold (FFI state)"]
      L7["[directives] active (seeds active_directives)"]
    end
    subgraph hot["HOT-RELOADABLE (main.rs watcher, applies to live atomics)"]
      H1["the 4 threshold atomics only"]
    end
    subgraph inert["INERT / RESERVED (write-only, never read by proxy)"]
      I1["engine_min_msgs, engine_protect_first/last (state.rs 01-F9)"]
      I2["catalog_mode (RESERVED)"]
      I3["previews.* , prompts.* (parsed into MultiConfig, no consumer)"]
      I4["auto_expand / auto_expand_limit / classifier_poll (CompressionConfig fields, unused)"]
      I5["mode/listen: NOT env-overridable in resolve (would break dual-proxy)"]
    end
```

Precedence subtleties:
- The proxy's `resolve` deliberately does **not** give `mode`/`listen` a blanket
  env override - a single process-wide `APHRODITE_MODE`/`APHRODITE_LISTEN` would
  clobber every `[[proxies]]` entry, breaking the cache/token split. Port
  overrides are per-mode (`APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`).
- `apply_compression` maps `tool_threshold` (FFI state) to the
  `tool_threshold_token` TOML key / `APHRODITE_TOOL_THRESHOLD_TOKEN` env var -
  the old `tool_threshold` names shipped in no TOML, so wiring them would have
  silently resolved to default forever (F3).
- `env_bool` unifies truthiness to `1`/`true` (case-insensitive); `env_parse_warn`
  warns loudly on a present-but-malformed numeric override instead of silently
  defaulting.

## Key call sites
- `MultiConfig::resolve` (env>TOML>default chains) - `crates/aphrodite/src/config.rs:297`
- `env_bool` / `env_parse_warn` / `apply_port_override` - `crates/aphrodite/src/config.rs:18,33,406`
- `Config::{load,get_bool,get_u64,get_string}` - `crates/aphrodite/src/config_loader.rs:20,66,80,100`
- `Config::apply_compression` - `crates/aphrodite/src/config_loader.rs:124`
- `resolve_thresholds` (proxy defaults + env) - `crates/aphrodite/src/proxy.rs:121`
- hot-reload watcher (4 atomics) - `crates/aphrodite/src/main.rs:251`
