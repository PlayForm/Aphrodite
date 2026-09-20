use std::net::SocketAddr;

use clap::ValueEnum;

use super::{Cli, env_parse_warn};

/// Proxy operation mode.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProxyMode {
	/// Cache mode - in-memory CCR, lightweight compression (>8KB threshold),
	/// preview preserved, no tool injection.
	Cache,
	/// Token mode - SQLite CCR, aggressive compression (>1KB threshold),
	/// tool injection, tool relay.
	Token,
}

/// Multi-proxy configuration loaded from aphrodite.toml.
///
/// `proxies` defaults to an empty list when the `[proxies]` table is
/// absent, so a hook-only configuration (no proxy definitions) parses
/// cleanly instead of failing with `missing field 'proxies'`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MultiConfig {
	pub defaults:Option<Defaults>,
	#[serde(default)]
	pub proxies:Vec<ProxyConfig>,
	pub compression:Option<CompressionConfig>,
	pub previews:Option<PreviewsConfig>,
	pub prompts:Option<PromptsConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Defaults {
	pub api_url:Option<String>,
	pub model:Option<String>,
	pub ccr_ttl_seconds:Option<u64>,
	pub api_key:Option<String>,
}

/// Compression knobs - thresholds, engine, auto-expand, classifier poll.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CompressionConfig {
	pub engine_threshold_pct:Option<u32>,
	pub engine_protect_first:Option<u32>,
	pub engine_protect_last:Option<u32>,
	pub engine_min_msgs:Option<u32>,
	pub tool_threshold_token:Option<u32>,
	pub tool_threshold_cache:Option<u32>,
	pub terminal_threshold:Option<u32>,
	pub inline_threshold:Option<u32>,
	pub auto_expand:Option<bool>,
	pub auto_expand_limit:Option<u32>,
	pub catalog_mode:Option<String>,
	pub classifier_poll:Option<bool>,
	pub code_multiplier:Option<f64>,
}

/// Preview knobs - model-aware templates, code structure maps.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PreviewsConfig {
	pub model_family:Option<String>,
	pub code_structure_map:Option<bool>,
	pub preview_max_chars:Option<u32>,
	pub rust_preview_lines:Option<u32>,
}

/// Prompt knobs - how the system instructs the LLM about CCR.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PromptsConfig {
	pub retrieve_guidance:Option<String>,
	pub ccr_marker_hint:Option<bool>,
	pub catalog_intent_hints:Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxyConfig {
	pub name:Option<String>,
	#[serde(default)]
	pub listen:Option<String>,
	pub mode:Option<String>,
	pub api_key:Option<String>,
	pub api_url:Option<String>,
	pub model:Option<String>,
	pub tool_relay:Option<bool>,
	pub dev:Option<bool>,
	pub ccr_ttl_seconds:Option<u64>,
	pub ccr_db_path:Option<String>,
	pub notify_url:Option<String>,
	pub notify_key:Option<String>,
	pub timeout:Option<u64>,
	pub max_context:Option<usize>,
	pub max_output:Option<usize>,
}

impl MultiConfig {
	/// Load from the given aphrodite.toml path.
	pub fn load(path:&str) -> anyhow::Result<Self> {
		let content = std::fs::read_to_string(path)?;
		Ok(toml::from_str(&content)?)
	}

	/// Resolve a ProxyConfig with defaults applied.
	/// API key fallback chain: `proxy.api_key` → `defaults.api_key` →
	/// `APHRODITE_API_KEY` (no provider-specific key names are probed).
	/// Returns an error if no API key is found after all fallbacks.
	pub fn resolve(&self, cfg:&ProxyConfig) -> anyhow::Result<Cli> {
		let d = self.defaults.as_ref();
		let api_key:String = cfg
			// API key fallback chain: explicit config → APHRODITE_API_KEY
			// (no provider-specific key names are probed - the upstream is
			// whatever APHRODITE_API_URL points at, keyed by APHRODITE_API_KEY)
			.api_key
			.clone()
			.or_else(|| d.and_then(|d| d.api_key.clone()))
			.or_else(|| std::env::var("APHRODITE_API_KEY").ok())
			.unwrap_or_default();
		if api_key.is_empty() {
			anyhow::bail!("no API key configured - set APHRODITE_API_KEY env var or api_key in aphrodite.toml");
		}
		// Resolve listen: must parse or fail (no silent default when listen is
		// explicitly set).  After parsing, override with env var if set - this
		// lets multiple concurrent Hermes Agent instances each point at their
		// own proxy pair without editing aphrodite.toml.
		let listen:SocketAddr = match cfg.listen.as_deref() {
			Some(s) => s.parse().map_err(|_| anyhow::anyhow!("invalid listen address: {s}"))?,
			None => "127.0.0.1:9797".parse().unwrap(),
		};
		let listen = match (cfg.mode.as_deref(), cfg.name.as_deref()) {
			(_, Some("cache")) | (Some("cache"), _) => Self::apply_port_override(listen, "APHRODITE_CACHE_PORT"),
			(_, Some("token")) | (Some("token"), _) => Self::apply_port_override(listen, "APHRODITE_TOKEN_PORT"),
			_ => listen,
		};
		// Validate max_output < max_context
		let max_context = cfg.max_context.unwrap_or(1_000_000);
		let max_output = cfg.max_output.unwrap_or(384_000);
		if max_output >= max_context {
			anyhow::bail!("max_output ({max_output}) must be less than max_context ({max_context})");
		}
		Ok(Cli {
			command:None,
			mode:match cfg.mode.as_deref() {
				Some("token") => ProxyMode::Token,
				Some("cache") => ProxyMode::Cache,
				None => {
					tracing::info!("no mode specified, defaulting to token");
					ProxyMode::Token
				},
				Some(other) => {
					tracing::warn!("unknown mode {:?}, defaulting to token", other);
					ProxyMode::Token
				},
			},
			listen,
			// env > TOML (proxy > defaults) > hardcoded default (report 07
			// F1/T17) - previously only the API-key chain and the two port
			// vars pierced the TOML in this path, contradicting every shipped
			// TOML's own header comment ("Env vars override TOML values at
			// runtime"). `mode`/`listen` are deliberately NOT given a
			// blanket env override here: a single process-wide
			// `APHRODITE_MODE`/`APHRODITE_LISTEN` would incorrectly apply to
			// every `[[proxies]]` entry at once, breaking the cache/token
			// dual-proxy split that the existing per-mode
			// `APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT` overrides above
			// are already careful to respect.
			api_url:std::env::var("APHRODITE_API_URL")
				.ok()
				.or_else(|| cfg.api_url.clone())
				.or_else(|| d.and_then(|d| d.api_url.clone()))
				.unwrap_or_else(|| "https://api.openai.com".into()),
			api_key,
			model:std::env::var("APHRODITE_MODEL")
				.ok()
				.or_else(|| cfg.model.clone())
				.or_else(|| d.and_then(|d| d.model.clone()))
				.unwrap_or_else(|| "default-model".into()),
			max_context,
			max_output,
			// Resolve from toml - proxy.rs handles None default
			ccr_db_path:std::env::var("APHRODITE_DB")
				.ok()
				.or_else(|| cfg.ccr_db_path.clone())
				.filter(|s| !s.is_empty())
				.map(Into::into),
			ccr_ttl_seconds:env_parse_warn::<u64>("APHRODITE_CCR_TTL")
				.or(cfg.ccr_ttl_seconds)
				.or_else(|| d.and_then(|d| d.ccr_ttl_seconds))
				.unwrap_or(3600),
			no_ccr_marker:false,
			tool_relay:cfg.tool_relay.unwrap_or(false),
			notify_url:std::env::var("APHRODITE_NOTIFY_URL").ok().or_else(|| cfg.notify_url.clone()),
			notify_key:std::env::var("APHRODITE_NOTIFY_KEY").ok().or_else(|| cfg.notify_key.clone()),
			dev:cfg.dev.unwrap_or(false),
			log_compact:false,
			timeout:{
				let t = cfg.timeout.unwrap_or(300);
				if t > 600 {
					tracing::warn!("timeout {}s exceeds maximum 600s, clamping", t);
					600
				} else {
					t
				}
			},
		})
	}

	/// Override `listen`'s port from the named env var, if set.
	///
	/// A missing env var is the common case and silently keeps `listen`
	/// unchanged. A *present but malformed* value (non-numeric, or outside
	/// the u16 port range) also keeps `listen` unchanged, but logs a
	/// warning - silently ignoring a typo'd override left the operator with
	/// no way to tell "my override didn't apply" from "I didn't set an
	/// override", the same silent-failure class as the missing-CCR-
	/// directory bug this override was added alongside.
	fn apply_port_override(listen:SocketAddr, env_var:&str) -> SocketAddr {
		match std::env::var(env_var) {
			Ok(p) => {
				match p.parse::<u16>() {
					Ok(port) => {
						let mut addr = listen;
						addr.set_port(port);
						tracing::info!("{}={} overriding listen to {}", env_var, port, addr);
						addr
					},
					Err(_) => {
						tracing::warn!(
							"{}={:?} is not a valid port (1-65535); ignoring override, using {}",
							env_var,
							p,
							listen,
						);
						listen
					},
				}
			},
			Err(_) => listen,
		}
	}
}
