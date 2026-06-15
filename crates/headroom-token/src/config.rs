//! CLI configuration for headroom-token.

use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Headroom token-mode proxy — CCR-enabled, code-aware DeepSeek forwarder.
#[derive(Parser, Debug, Clone)]
#[command(name = "headroom-token", version, about)]
pub struct Cli {
    /// Listen address (default: 127.0.0.1:8788)
    #[arg(long, default_value = "127.0.0.1:8788", env = "HEADROOM_TOKEN_LISTEN")]
    pub listen: SocketAddr,

    /// DeepSeek API base URL
    #[arg(long, default_value = "https://api.deepseek.com", env = "DEEPSEEK_URL")]
    pub deepseek_url: String,

    /// DeepSeek API key
    #[arg(long, env = "HEADROOM_DEEPSEEK_KEY")]
    pub deepseek_key: String,

    /// Model name to forward
    #[arg(long, default_value = "deepseek-v4-pro", env = "HEADROOM_TOKEN_MODEL")]
    pub model: String,

    /// Max context tokens (for token counting)
    #[arg(long, default_value = "1000000")]
    pub max_context: usize,

    /// Max output tokens
    #[arg(long, default_value = "384000")]
    pub max_output: usize,

    /// SQLite database path for CCR storage (default: .headroom/token-ccr.db)
    #[arg(long, default_value = ".headroom/token-ccr.db", env = "HEADROOM_TOKEN_DB")]
    pub ccr_db_path: PathBuf,

    /// CCR TTL in seconds (default: 3600 = 1 hour)
    #[arg(long, default_value = "3600", env = "HEADROOM_TOKEN_CCR_TTL")]
    pub ccr_ttl_seconds: u64,

    /// Disable CCR tool injection
    #[arg(long)]
    pub no_ccr_inject_tool: bool,

    /// Disable CCR markers in compressed output
    #[arg(long)]
    pub no_ccr_marker: bool,
}
