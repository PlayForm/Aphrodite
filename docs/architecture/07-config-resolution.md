# Config Resolution Precedence

Two independent resolution stacks share the same **env > TOML > default** precedence but live in different modules: the proxy's `MultiConfig::resolve`, which produces one `Cli` per listener, and the Hermes/FFI `Config` loader, whose `apply_compression` writes straight into `AphroditeState`. Both read the same `aphrodite.toml`, and only a small set of keys is actually consumed at runtime - most of the schema is inert or reserved.

## Precedence flowchart

```mermaid
flowchart TD
	subgraph proxy["Proxy path - MultiConfig::resolve"]
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

	subgraph ffi["FFI/Hermes path - Config loader"]
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
		L8["[previews] preview_max_chars → preview cap<br/>(env APHRODITE_PREVIEW_MAX_CHARS > TOML; shipped template default 120, key absent = no cap)"]
	end
	subgraph hot["HOT-RELOADABLE (config-file watcher, applies to live atomics)"]
		H1["the 4 threshold atomics only"]
	end
	subgraph inert["INERT / RESERVED (write-only, never read by the proxy)"]
		I1["engine_min_msgs, engine_protect_first/last (write-only, never read)"]
		I2["catalog_mode (RESERVED)"]
		I3["previews.* remaining inert (model_family / code_structure_map / rust_preview_lines) · prompts.* (never read by the proxy)"]
		I4["auto_expand / auto_expand_limit / classifier_poll (CompressionConfig fields, unused)"]
		I5["mode/listen: not env-overridable in resolve (would break dual-proxy)"]
	end
```

Precedence subtleties:

- The proxy's `resolve` deliberately gives `mode`/`listen` no blanket env override - a process-wide `APHRODITE_MODE`/`APHRODITE_LISTEN` would clobber every `[[proxies]]` entry and break the cache/token split. Port overrides stay per-mode (`APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT`).
- `apply_compression` maps the FFI state's `tool_threshold` to the `tool_threshold_token` TOML key and the `APHRODITE_TOOL_THRESHOLD_TOKEN` env var. The old `tool_threshold` names shipped in no TOML, so wiring them as-is would have silently resolved to the default forever.
- `env_bool` accepts `1`/`true` (case-insensitive) as truthy and everything else as false; `env_parse_warn` warns loudly on a present-but-malformed numeric override instead of silently defaulting.
- `[previews] preview_max_chars` caps rendered previews (env `APHRODITE_PREVIEW_MAX_CHARS` wins over TOML). The shipped config template sets 120; when the key is absent the preview builder applies no cap. It is applied at proxy startup and whenever the FFI config is (re)loaded - a dylib reload re-applies it, but the config-file watcher does not.
- The config-file watcher reacts to `Modify` events on `aphrodite.toml` (500 ms debounce), reloads the file, and stores the four resolved thresholds (`cache`, `token`, `inline`, `code_multiplier`) into every live `AppState`'s atomics. Nothing else is hot-updated.
