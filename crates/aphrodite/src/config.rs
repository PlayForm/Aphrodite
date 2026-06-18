//! CLI configuration for aphrodite.
//!
//! Generic LLM proxy - works with any OpenAI-compatible API.
//! Cache and Token modes with CCR, tool relay, programmatic CCR.

use clap::{Parser, ValueEnum};
use std::net::SocketAddr;
use std::path::PathBuf;

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

/// aphrodite - generic LLM proxy with CCR, tool relay, and programmatic CCR.
/// Works with any OpenAI-compatible API (DeepSeek, OpenAI, Anthropic via proxy, etc.)
#[derive(Parser, Debug, Clone)]
#[command(name = "aphrodite", version, about)]
pub struct Cli {
	/// Proxy mode: cache or token
	#[arg(long, default_value = "token", env = "APHRODITE_MODE")]
	pub mode: ProxyMode,

	/// Listen address
	#[arg(long, default_value = "127.0.0.1:9797", env = "APHRODITE_LISTEN")]
	pub listen: SocketAddr,

	/// Upstream API base URL
	#[arg(long, default_value = "https://api.openai.com", env = "APHRODITE_API_URL")]
	pub api_url: String,

	/// Upstream API key
	#[arg(long, env = "APHRODITE_API_KEY")]
	pub api_key: String,

	/// Model name to forward (set via APHRODITE_MODEL env or --model)
	#[arg(long, default_value = "default-model", env = "APHRODITE_MODEL")]
	pub model: String,

	/// Max context tokens
	#[arg(long, default_value = "1000000")]
	pub max_context: usize,

	/// Max output tokens
	#[arg(long, default_value = "384000")]
	pub max_output: usize,

	/// SQLite database path for CCR storage
	#[arg(long, env = "APHRODITE_DB", default_value = "")]
	pub ccr_db_path: PathBuf,

	/// CCR TTL in seconds (default: 3600 = 1 hour)
	#[arg(long, default_value = "3600", env = "APHRODITE_CCR_TTL")]
	pub ccr_ttl_seconds: u64,

	/// Disable CCR markers in compressed output
	#[arg(long)]
	pub no_ccr_marker: bool,

	/// Enable tool relay endpoint (POST /tool/relay)
	#[arg(long)]
	pub tool_relay: bool,

	/// Hermes callback URL for CCR notifications
	#[arg(long, env = "APHRODITE_NOTIFY_URL")]
	pub notify_url: Option<String>,

	/// Hermes API key for callback auth
	#[arg(long, env = "APHRODITE_NOTIFY_KEY")]
	pub notify_key: Option<String>,

	/// Enable dev mode - verbose request/response logging
	#[arg(long)]
	pub dev: bool,

	/// Use compact log format (no timestamps, no targets)
	#[arg(long, env = "APHRODITE_LOG_COMPACT")]
	pub log_compact: bool,

	/// Upstream request timeout in seconds (default: 300)
	#[arg(long, default_value = "300")]
	pub timeout: u64,
}

/// Multi-proxy configuration loaded from aphrodite.toml.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MultiConfig {
	pub defaults: Option<Defaults>,
	pub proxies: Vec<ProxyConfig>,
	pub compression: Option<CompressionConfig>,
	pub previews: Option<PreviewsConfig>,
	pub prompts: Option<PromptsConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Defaults {
	pub api_url: Option<String>,
	pub model: Option<String>,
	pub ccr_ttl_seconds: Option<u64>,
	pub api_key: Option<String>,
}

/// Compression knobs - thresholds, engine, auto-expand, classifier poll.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CompressionConfig {
	pub engine_threshold_pct: Option<u32>,
	pub engine_protect_first: Option<u32>,
	pub engine_protect_last: Option<u32>,
	pub engine_min_msgs: Option<u32>,
	pub tool_threshold_token: Option<u32>,
	pub tool_threshold_cache: Option<u32>,
	pub terminal_threshold: Option<u32>,
	pub inline_threshold: Option<u32>,
	pub auto_expand: Option<bool>,
	pub auto_expand_limit: Option<u32>,
	pub catalog_mode: Option<String>,
	pub classifier_poll: Option<bool>,
	pub code_multiplier: Option<f64>,
}

/// Preview knobs - model-aware templates, code structure maps.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PreviewsConfig {
	pub model_family: Option<String>,
	pub code_structure_map: Option<bool>,
	pub preview_max_chars: Option<u32>,
	pub rust_preview_lines: Option<u32>,
}

/// Prompt knobs - how the system instructs the LLM about CCR.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PromptsConfig {
	pub retrieve_guidance: Option<String>,
	pub ccr_marker_hint: Option<bool>,
	pub catalog_intent_hints: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxyConfig {
	pub name: Option<String>,
	#[serde(default)]
	pub listen: Option<String>,
	pub mode: Option<String>,
	pub api_key: Option<String>,
	pub api_url: Option<String>,
	pub model: Option<String>,
	pub tool_relay: Option<bool>,
	pub dev: Option<bool>,
	pub ccr_ttl_seconds: Option<u64>,
	pub ccr_db_path: Option<String>,
	pub notify_url: Option<String>,
	pub notify_key: Option<String>,
	pub timeout: Option<u64>,
	pub max_context: Option<usize>,
	pub max_output: Option<usize>,
}

impl MultiConfig {
	/// Load from the given aphrodite.toml path.
	pub fn load(path: &str) -> anyhow::Result<Self> {
		let content = std::fs::read_to_string(path)?;
		Ok(toml::from_str(&content)?)
	}

	/// Resolve a ProxyConfig with defaults applied.
	/// API key fallback chain: `proxy.api_key` → `defaults.api_key` →
	/// `APHRODITE_API_KEY` → `DEEPSEEK_API_KEY` → `HEADROOM_DEEPSEEK_KEY`.
	/// Returns an error if no API key is found after all fallbacks.
	pub fn resolve(&self, cfg: &ProxyConfig) -> anyhow::Result<Cli> {
		let d = self.defaults.as_ref();
		let api_key: String = cfg
			// API key fallback chain: explicit config → APHRODITE_API_KEY → DEEPSEEK_API_KEY → HEADROOM_DEEPSEEK_KEY
		.api_key
			.clone()
			.or_else(|| d.and_then(|d| d.api_key.clone()))
			.or_else(|| std::env::var("APHRODITE_API_KEY").ok())
			.or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
			.or_else(|| std::env::var("HEADROOM_DEEPSEEK_KEY").ok())
			.unwrap_or_default();
		if api_key.is_empty() {
			anyhow::bail!("no API key configured - set APHRODITE_API_KEY env var or api_key in aphrodite.toml");
		}
		// Resolve listen: must parse or fail (no silent default when listen is explicitly set)
		let listen: SocketAddr = match cfg.listen.as_deref() {
			Some(s) => s.parse().map_err(|_| anyhow::anyhow!("invalid listen address: {s}"))?,
			None => "127.0.0.1:9797".parse().unwrap(),
		};
		// Validate max_output < max_context
		let max_context = cfg.max_context.unwrap_or(1_000_000);
		let max_output = cfg.max_output.unwrap_or(384_000);
		if max_output >= max_context {
			anyhow::bail!("max_output ({max_output}) must be less than max_context ({max_context})");
		}
		Ok(Cli {
			mode: match cfg.mode.as_deref() {
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
			api_url: cfg
				.api_url
				.clone()
				.or_else(|| d.and_then(|d| d.api_url.clone()))
				.unwrap_or_else(|| "https://api.openai.com".into()),
			api_key,
			model: cfg
				.model
				.clone()
				.or_else(|| d.and_then(|d| d.model.clone()))
				.unwrap_or_else(|| "default-model".into()),
			max_context,
			max_output,
			// Fall back to ~/.hermes/aphrodite/ if dirs::home_dir() returns None
			ccr_db_path: cfg
				.ccr_db_path
				.clone()
				.filter(|s| !s.is_empty())
				.map(Into::into)
				.unwrap_or_else(|| {
					dirs::home_dir()
						.unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
						.join(".hermes")
						.join("aphrodite")
						.join("ccr.db")
				}),
			ccr_ttl_seconds: cfg
				.ccr_ttl_seconds
				.or_else(|| d.and_then(|d| d.ccr_ttl_seconds))
				.unwrap_or(3600),
			no_ccr_marker: false,
			tool_relay: cfg.tool_relay.unwrap_or(false),
			notify_url: cfg.notify_url.clone(),
			notify_key: cfg.notify_key.clone(),
			dev: cfg.dev.unwrap_or(false),
			log_compact: false,
			timeout: {
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
}
