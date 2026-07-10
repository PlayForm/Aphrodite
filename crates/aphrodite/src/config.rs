//! CLI configuration for aphrodite.
//!
//! Generic LLM proxy - works with any OpenAI-compatible API.
//! Cache and Token modes with CCR, tool relay, programmatic CCR.

use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, ValueEnum};

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

/// aphrodite subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
	/// Run the proxy server (default).
	Run,
	/// Bootstrap: copy binary, create config, register with hermes, launch proxy.
	Setup {
		/// API key for the upstream LLM provider (uses APHRODITE_API_KEY env).
		#[arg(long, env = "APHRODITE_API_KEY", hide_env_values = true)]
		api_key: Option<String>,

		/// Upstream API base URL.
		#[arg(long, env = "APHRODITE_API_URL", default_value = "https://api.deepseek.com")]
		api_url: String,

		/// Model name to forward.
		#[arg(long, env = "APHRODITE_MODEL", default_value = "deepseek-v4-pro")]
		model: String,

		/// Cache proxy listen port. Override per-instance to run multiple
		/// concurrent Hermes Agents on the same machine.
		#[arg(long, env = "APHRODITE_CACHE_PORT", default_value = "9797")]
		cache_port: u16,

		/// Token proxy listen port. Override per-instance to run multiple
		/// concurrent Hermes Agents on the same machine.
		#[arg(long, env = "APHRODITE_TOKEN_PORT", default_value = "9798")]
		token_port: u16,

		/// Skip launching the proxy after setup.
		#[arg(long)]
		no_launch: bool,

		/// Force re-setup even if already installed.
		#[arg(long)]
		force: bool,
	},
}

/// Arguments for the `setup` subcommand.
#[derive(Debug, Clone)]
pub struct SetupArgs {
	/// API key for the upstream LLM provider (uses APHRODITE_API_KEY env).
	pub api_key: Option<String>,
	/// Upstream API base URL.
	pub api_url: String,
	/// Model name to forward.
	pub model: String,
	/// Cache proxy listen port.
	pub cache_port: u16,
	/// Token proxy listen port.
	pub token_port: u16,
	/// Skip launching the proxy after setup.
	pub no_launch: bool,
	/// Force re-setup even if already installed.
	pub force: bool,
}

impl From<Command> for SetupArgs {
	fn from(cmd: Command) -> Self {
		match cmd {
			Command::Setup { api_key, api_url, model, cache_port, token_port, no_launch, force } => {
				Self { api_key, api_url, model, cache_port, token_port, no_launch, force }
			}
			_ => Self {
				api_key: None,
				api_url: "https://api.deepseek.com".into(),
				model: "deepseek-v4-pro".into(),
				cache_port: 9797,
				token_port: 9798,
				no_launch: false,
				force: false,
			},
		}
	}
}

/// aphrodite - generic LLM proxy with CCR, tool relay, and programmatic CCR.
/// Works with any OpenAI-compatible API (DeepSeek, OpenAI, Anthropic via proxy,
/// etc.)
#[derive(Parser, Debug, Clone)]
#[command(name = "aphrodite", version, about)]
pub struct Cli {
	/// Subcommand: `setup` to bootstrap, or omitted to run the proxy.
	#[command(subcommand)]
	pub command: Option<Command>,
	/// Proxy mode: cache or token
	#[arg(long, default_value = "token", env = "APHRODITE_MODE")]
	pub mode:ProxyMode,

	/// Listen address
	#[arg(long, default_value = "127.0.0.1:9797", env = "APHRODITE_LISTEN")]
	pub listen:SocketAddr,

	/// Upstream API base URL
	#[arg(long, default_value = "https://api.openai.com", env = "APHRODITE_API_URL")]
	pub api_url:String,

	/// Upstream API key
	#[arg(long, env = "APHRODITE_API_KEY", hide_env_values = true)]
	pub api_key:String,

	/// Model name to forward (set via APHRODITE_MODEL env or --model)
	#[arg(long, default_value = "default-model", env = "APHRODITE_MODEL")]
	pub model:String,

	/// Max context tokens
	#[arg(long, default_value = "1000000")]
	pub max_context:usize,

	/// Max output tokens
	#[arg(long, default_value = "384000")]
	pub max_output:usize,

	/// SQLite database path for CCR storage
	#[arg(long, env = "APHRODITE_DB")]
	pub ccr_db_path:Option<PathBuf>,

	/// CCR TTL in seconds (default: 3600 = 1 hour)
	#[arg(long, default_value = "3600", env = "APHRODITE_CCR_TTL")]
	pub ccr_ttl_seconds:u64,

	/// Disable CCR markers in compressed output
	#[arg(long)]
	pub no_ccr_marker:bool,

	/// Enable tool relay endpoint (POST /tool/relay)
	#[arg(long)]
	pub tool_relay:bool,

	/// Hermes callback URL for CCR notifications
	#[arg(long, env = "APHRODITE_NOTIFY_URL")]
	pub notify_url:Option<String>,

	/// Hermes API key for callback auth
	#[arg(long, env = "APHRODITE_NOTIFY_KEY", hide_env_values = true)]
	pub notify_key:Option<String>,

	/// Enable dev mode - verbose request/response logging
	#[arg(long)]
	pub dev:bool,

	/// Use compact log format (no timestamps, no targets)
	#[arg(long, env = "APHRODITE_LOG_COMPACT")]
	pub log_compact:bool,

	/// Upstream request timeout in seconds (default: 300)
	#[arg(long, default_value = "300")]
	pub timeout:u64,
}

/// Multi-proxy configuration loaded from aphrodite.toml.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MultiConfig {
	pub defaults:Option<Defaults>,
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
	/// `APHRODITE_API_KEY` → `DEEPSEEK_API_KEY` → `HEADROOM_DEEPSEEK_KEY`.
	/// Returns an error if no API key is found after all fallbacks.
	pub fn resolve(&self, cfg:&ProxyConfig) -> anyhow::Result<Cli> {
		let d = self.defaults.as_ref();
		let api_key:String = cfg
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
		// Resolve listen: must parse or fail (no silent default when listen is
		// explicitly set).  After parsing, override with env var if set - this
		// lets multiple concurrent Hermes Agent instances each point at their
		// own proxy pair without editing aphrodite.toml.
		let listen:SocketAddr = match cfg.listen.as_deref() {
			Some(s) => s.parse().map_err(|_| anyhow::anyhow!("invalid listen address: {s}"))?,
			None => "127.0.0.1:9797".parse().unwrap(),
		};
		let listen = match (cfg.mode.as_deref(), cfg.name.as_deref()) {
			(_, Some("cache")) | (Some("cache"), _) => {
				Self::apply_port_override(listen, "APHRODITE_CACHE_PORT")
			},
			(_, Some("token")) | (Some("token"), _) => {
				Self::apply_port_override(listen, "APHRODITE_TOKEN_PORT")
			},
			_ => listen,
		};
		// Validate max_output < max_context
		let max_context = cfg.max_context.unwrap_or(1_000_000);
		let max_output = cfg.max_output.unwrap_or(384_000);
		if max_output >= max_context {
			anyhow::bail!("max_output ({max_output}) must be less than max_context ({max_context})");
		}
		Ok(Cli {
			command: None,
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
			api_url:cfg
				.api_url
				.clone()
				.or_else(|| d.and_then(|d| d.api_url.clone()))
				.unwrap_or_else(|| "https://api.openai.com".into()),
			api_key,
			model:cfg
				.model
				.clone()
				.or_else(|| d.and_then(|d| d.model.clone()))
				.unwrap_or_else(|| "default-model".into()),
			max_context,
			max_output,
			// Resolve from toml - proxy.rs handles None default
			ccr_db_path:cfg
				.ccr_db_path
				.clone()
				.filter(|s| !s.is_empty())
				.map(Into::into),
			ccr_ttl_seconds:cfg
				.ccr_ttl_seconds
				.or_else(|| d.and_then(|d| d.ccr_ttl_seconds))
				.unwrap_or(3600),
			no_ccr_marker:false,
			tool_relay:cfg.tool_relay.unwrap_or(false),
			notify_url:cfg.notify_url.clone(),
			notify_key:cfg.notify_key.clone(),
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
			Ok(p) => match p.parse::<u16>() {
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
			},
			Err(_) => listen,
		}
	}
}
