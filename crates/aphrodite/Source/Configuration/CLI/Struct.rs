//! The parsed command line.

use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

use super::Command::Enum::Enum as Command;
use crate::Configuration::Proxy::Mode::Enum::Enum as ProxyMode;

/// aphrodite - generic LLM proxy with CCR, tool relay, and programmatic CCR.
/// Works with any OpenAI-compatible API (DeepSeek, OpenAI, Anthropic via proxy,
/// etc.)
///
/// Every field is the resolved value for one proxy instance: env var first
/// (clap reads them through `env = ...`), then the flag, then the declared
/// default. `MultiConfig::resolve` produces the same struct from
/// `aphrodite.toml`, which is why the two paths must agree on these defaults.
#[derive(Parser, Debug, Clone)]
#[command(name = "aphrodite", version, about)]
pub struct Struct {
	/// Subcommand: `setup` to bootstrap, or omitted to run the proxy.
	#[command(subcommand)]
	pub Command:Option<Command>,
	/// Proxy mode: cache or token
	#[arg(long, default_value = "token", env = "APHRODITE_MODE")]
	pub Mode:ProxyMode,

	/// Listen address
	#[arg(long, default_value = "127.0.0.1:9797", env = "APHRODITE_LISTEN")]
	pub Listen:SocketAddr,

	/// Upstream API base URL
	#[arg(
		long = "apiurl",
		value_name = "APIURL",
		default_value = "https://api.openai.com",
		env = "APHRODITE_API_URL"
	)]
	pub URL:String,

	/// Upstream API key (optional at parse time - `setup` and keyless
	/// launches don't require it; required only when a proxy must forward
	/// to an upstream). Empty string default so `aphrodite setup` and other
	/// subcommands parse without `--api-key`.
	#[arg(
		long = "api-key",
		value_name = "API_KEY",
		env = "APHRODITE_API_KEY",
		hide_env_values = true,
		default_value = ""
	)]
	pub Key:String,

	/// Model name to forward (set via APHRODITE_MODEL env or --model)
	#[arg(long, default_value = "default-model", env = "APHRODITE_MODEL")]
	pub Model:String,

	/// Max context tokens
	#[arg(
		long = "maximum-context",
		value_name = "MAXIMUM_CONTEXT",
		default_value = "1000000"
	)]
	pub Context:usize,

	/// Max output tokens
	#[arg(long = "maximum-output", value_name = "MAXIMUM_OUTPUT", default_value = "384000")]
	pub Output:usize,

	/// SQLite database path for CCR storage
	#[arg(
		long = "ccr-database-path",
		value_name = "CCR_DATABASE_PATH",
		env = "APHRODITE_DB"
	)]
	pub Database:Option<PathBuf>,

	/// CCR TTL in seconds (default: 3600 = 1 hour)
	#[arg(
		long = "ccr-ttl-seconds",
		value_name = "CCR_TTL_SECONDS",
		default_value = "3600",
		env = "APHRODITE_CCR_TTL"
	)]
	pub TTL:u64,

	/// Maximum live rows in the SQLite CCR store before least-recently-
	/// accessed rows are evicted (0 = unlimited). A TTL bounds the store
	/// only in time; this bounds it in space so a burst of compression
	/// inside one TTL window cannot grow the file without limit.
	#[arg(
		long = "store-maximum-entries",
		value_name = "STORE_MAXIMUM_ENTRIES",
		default_value = "100000",
		env = "APHRODITE_STORE_MAX_ENTRIES"
	)]
	pub Entries:usize,

	/// Maximum stored payload bytes (`SUM(LENGTH(original))`) before
	/// least-recently-accessed rows are evicted (0 = unlimited).
	#[arg(
		long = "store-maximum-bytes",
		value_name = "STORE_MAXIMUM_BYTES",
		default_value = "536870912",
		env = "APHRODITE_STORE_MAX_BYTES"
	)]
	pub Bytes:u64,

	/// Disable CCR markers in compressed output
	#[arg(long = "no-ccr-marker")]
	pub NoMarker:bool,

	/// Enable tool relay endpoint (POST /tool/relay)
	#[arg(long = "tool-relay")]
	pub Relay:bool,

	/// Hermes callback URL for CCR notifications
	#[arg(long = "notify-url", value_name = "NOTIFY_URL", env = "APHRODITE_NOTIFY_URL")]
	pub Callback:Option<String>,

	/// Hermes API key for callback auth
	#[arg(
		long = "notify-key",
		value_name = "NOTIFY_KEY",
		env = "APHRODITE_NOTIFY_KEY",
		hide_env_values = true
	)]
	pub Auth:Option<String>,

	/// Enable dev mode - verbose request/response logging
	#[arg(long)]
	pub Dev:bool,

	/// Use compact log format (no timestamps, no targets)
	#[arg(long, env = "APHRODITE_LOG_COMPACT")]
	pub LogCompact:bool,

	/// Upstream request timeout in seconds (default: 300)
	#[arg(long, default_value = "300")]
	pub Timeout:u64,
}
