//! CLI configuration for aphrodite.
//!
//! Supports cache (:9797) and token (:9798) proxy modes with
//! Chat Completions API forwarding, tool relay, and programmatic CCR.

use clap::{Parser, ValueEnum};
use std::net::SocketAddr;
use std::path::PathBuf;

/// Proxy operation mode.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProxyMode {
    /// Cache mode — in-memory CCR, lightweight compression (>8KB threshold),
    /// preview preserved, no tool injection.
    Cache,
    /// Token mode — SQLite CCR, aggressive compression (>1KB threshold),
    /// tool injection, tool relay.
    Token,
}

/// aphrodite — Chat Completions proxy with CCR, tool relay, and programmatic CCR.
#[derive(Parser, Debug, Clone)]
#[command(name = "aphrodite", version, about)]
pub struct Cli {
    /// Proxy mode: cache or token
    #[arg(long, default_value = "token", env = "APHRODITE_MODE")]
    pub mode: ProxyMode,

    /// Listen address (default: 127.0.0.1:8788 for token, :9797 for cache)
    #[arg(long, default_value = "127.0.0.1:8788", env = "APHRODITE_LISTEN")]
    pub listen: SocketAddr,

    /// DeepSeek API base URL
    #[arg(long, default_value = "https://api.deepseek.com", env = "DEEPSEEK_URL")]
    pub deepseek_url: String,

    /// DeepSeek API key
    #[arg(long, env = "HEADROOM_DEEPSEEK_KEY")]
    pub deepseek_key: String,

    /// Model name to forward
    #[arg(long, default_value = "deepseek-v4-pro", env = "APHRODITE_MODEL")]
    pub model: String,

    /// Max context tokens
    #[arg(long, default_value = "1000000")]
    pub max_context: usize,

    /// Max output tokens
    #[arg(long, default_value = "384000")]
    pub max_output: usize,

    /// SQLite database path for CCR storage
    #[arg(long, default_value = ".headroom/aphrodite-ccr.db", env = "APHRODITE_DB")]
    pub ccr_db_path: PathBuf,

    /// CCR TTL in seconds (default: 3600 = 1 hour)
    #[arg(long, default_value = "3600", env = "APHRODITE_CCR_TTL")]
    pub ccr_ttl_seconds: u64,

    /// Disable CCR tool injection (aphrodite mode only)
    #[arg(long)]
    pub no_ccr_inject_tool: bool,

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

    /// Enable dev mode — verbose request/response logging to stderr + /tmp/aphrodite-dev.log
    #[arg(long)]
    pub dev: bool,
}
