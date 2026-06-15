//! CLI configuration for headroom-proxy.
//!
//! Supports both cache (:9797) and token (:9798) proxy modes,
//! with tool relay and programmatic CCR endpoints.

use clap::{Parser, ValueEnum};
use std::net::SocketAddr;
use std::path::PathBuf;

/// Proxy operation mode.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProxyMode {
    /// Cache mode — standard compression, no tool injection.
    Cache,
    /// Token mode — aggressive compression, CCR, tool injection, tool relay.
    Token,
}

/// Headroom proxy — CCR-enabled, code-aware DeepSeek forwarder.
/// Supports tool relay and programmatic CCR for Hermes integration.
#[derive(Parser, Debug, Clone)]
#[command(name = "headroom-proxy", version, about)]
pub struct Cli {
    /// Proxy mode: cache or token
    #[arg(long, default_value = "token", env = "HEADROOM_PROXY_MODE")]
    pub mode: ProxyMode,

    /// Listen address (default: 127.0.0.1:8788 for token, :9797 for cache)
    #[arg(long, default_value = "127.0.0.1:8788", env = "HEADROOM_PROXY_LISTEN")]
    pub listen: SocketAddr,

    /// DeepSeek API base URL
    #[arg(long, default_value = "https://api.deepseek.com", env = "DEEPSEEK_URL")]
    pub deepseek_url: String,

    /// DeepSeek API key
    #[arg(long, env = "HEADROOM_DEEPSEEK_KEY")]
    pub deepseek_key: String,

    /// Model name to forward
    #[arg(long, default_value = "deepseek-v4-pro", env = "HEADROOM_PROXY_MODEL")]
    pub model: String,

    /// Max context tokens (for token counting)
    #[arg(long, default_value = "1000000")]
    pub max_context: usize,

    /// Max output tokens
    #[arg(long, default_value = "384000")]
    pub max_output: usize,

    /// SQLite database path for CCR storage
    #[arg(long, default_value = ".headroom/proxy-ccr.db", env = "HEADROOM_PROXY_DB")]
    pub ccr_db_path: PathBuf,

    /// CCR TTL in seconds (default: 3600 = 1 hour)
    #[arg(long, default_value = "3600", env = "HEADROOM_PROXY_CCR_TTL")]
    pub ccr_ttl_seconds: u64,

    /// Disable CCR tool injection (token mode only)
    #[arg(long)]
    pub no_ccr_inject_tool: bool,

    /// Disable CCR markers in compressed output
    #[arg(long)]
    pub no_ccr_marker: bool,

    /// Enable tool relay endpoint (POST /tool/relay)
    #[arg(long)]
    pub tool_relay: bool,

    /// Hermes callback URL for CCR notifications
    #[arg(long, env = "HEADROOM_NOTIFY_URL")]
    pub notify_url: Option<String>,

    /// Hermes API key for callback auth
    #[arg(long, env = "HEADROOM_NOTIFY_KEY")]
    pub notify_key: Option<String>,
}
