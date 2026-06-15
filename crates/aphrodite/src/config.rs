//! CLI configuration for aphrodite.
//!
//! Generic LLM proxy — works with any OpenAI-compatible API.
//! Cache and Token modes with CCR, tool relay, programmatic CCR.

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

/// aphrodite — generic LLM proxy with CCR, tool relay, and programmatic CCR.
/// Works with any OpenAI-compatible API (DeepSeek, OpenAI, Anthropic via proxy, etc.)
#[derive(Parser, Debug, Clone)]
#[command(name = "aphrodite", version, about)]
pub struct Cli {
    /// Proxy mode: cache or token
    #[arg(long, default_value = "token", env = "APHRODITE_MODE")]
    pub mode: ProxyMode,

    /// Listen address
    #[arg(long, default_value = "127.0.0.1:8788", env = "APHRODITE_LISTEN")]
    pub listen: SocketAddr,

    /// Upstream API base URL
    #[arg(long, default_value = "https://api.deepseek.com", env = "APHRODITE_API_URL")]
    pub api_url: String,

    /// Upstream API key
    #[arg(long, env = "APHRODITE_API_KEY")]
    pub api_key: String,

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
    #[arg(long, env = "APHRODITE_NOTIFY_URL")]
    pub notify_url: Option<String>,

    /// Hermes API key for callback auth
    #[arg(long, env = "APHRODITE_NOTIFY_KEY")]
    pub notify_key: Option<String>,

    /// Enable dev mode — verbose request/response logging
    #[arg(long)]
    pub dev: bool,
}
