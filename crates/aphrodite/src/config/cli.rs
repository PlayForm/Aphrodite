use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

use super::ProxyMode;

/// aphrodite subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
	/// Run the proxy server (default).
	Run,
	/// Bootstrap: copy binary, create config, register with hermes, launch
	/// proxy.
	Setup {
		/// API key for the upstream LLM provider (uses APHRODITE_API_KEY env).
		#[arg(long, env = "APHRODITE_API_KEY", hide_env_values = true)]
		api_key:Option<String>,

		/// Upstream API base URL.
		#[arg(long, env = "APHRODITE_API_URL", default_value = "")]
		api_url:String,

		/// Model name to forward.
		#[arg(long, env = "APHRODITE_MODEL", default_value = "")]
		model:String,

		/// Cache proxy listen port. Override per-instance to run multiple
		/// concurrent Hermes Agents on the same machine.
		#[arg(long, env = "APHRODITE_CACHE_PORT", default_value = "9797")]
		cache_port:u16,

		/// Token proxy listen port. Override per-instance to run multiple
		/// concurrent Hermes Agents on the same machine.
		#[arg(long, env = "APHRODITE_TOKEN_PORT", default_value = "9798")]
		token_port:u16,

		/// Skip launching the proxy after setup.
		#[arg(long)]
		no_launch:bool,

		/// Force re-setup even if already installed.
		#[arg(long)]
		force:bool,
	},
}

/// Arguments for the `setup` subcommand.
#[derive(Debug, Clone)]
pub struct SetupArgs {
	/// API key for the upstream LLM provider (uses APHRODITE_API_KEY env).
	pub api_key:Option<String>,
	/// Upstream API base URL.
	pub api_url:String,
	/// Model name to forward.
	pub model:String,
	/// Cache proxy listen port.
	pub cache_port:u16,
	/// Token proxy listen port.
	pub token_port:u16,
	/// Skip launching the proxy after setup.
	pub no_launch:bool,
	/// Force re-setup even if already installed.
	pub force:bool,
}

impl From<Command> for SetupArgs {
	fn from(cmd:Command) -> Self {
		match cmd {
			Command::Setup { api_key, api_url, model, cache_port, token_port, no_launch, force } => {
				Self { api_key, api_url, model, cache_port, token_port, no_launch, force }
			},
			_ => {
				Self {
					api_key:None,
					api_url:String::new(),
					model:String::new(),
					cache_port:9797,
					token_port:9798,
					no_launch:false,
					force:false,
				}
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
	pub command:Option<Command>,
	/// Proxy mode: cache or token
	#[arg(long, default_value = "token", env = "APHRODITE_MODE")]
	pub mode:ProxyMode,

	/// Listen address
	#[arg(long, default_value = "127.0.0.1:9797", env = "APHRODITE_LISTEN")]
	pub listen:SocketAddr,

	/// Upstream API base URL
	#[arg(long, default_value = "https://api.openai.com", env = "APHRODITE_API_URL")]
	pub api_url:String,

	/// Upstream API key (optional at parse time - `setup` and keyless
	/// launches don't require it; required only when a proxy must forward
	/// to an upstream). Empty string default so `aphrodite setup` and other
	/// subcommands parse without `--api-key`.
	#[arg(long, env = "APHRODITE_API_KEY", hide_env_values = true, default_value = "")]
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
