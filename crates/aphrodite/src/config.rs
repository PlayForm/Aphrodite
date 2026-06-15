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

    /// Upstream request timeout in seconds (default: 300)
    #[arg(long, default_value = "300")]
    pub timeout: u64,
}


/// Multi-proxy configuration loaded from aphrodite.toml.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MultiConfig {
    pub defaults: Option<Defaults>,
    pub proxies: Vec<ProxyConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Defaults {
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub ccr_ttl_seconds: Option<u64>,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxyConfig {
    pub name: Option<String>,
    pub listen: String,
    pub mode: Option<String>,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub tool_relay: Option<bool>,
    pub dev: Option<bool>,
    pub ccr_ttl_seconds: Option<u64>,
    pub ccr_db_path: Option<String>,
}

impl MultiConfig {
    /// Load from aphrodite.toml in the current directory.
    pub fn load() -> anyhow::Result<Self> {
        let content = std::fs::read_to_string("aphrodite.toml")?;
        Ok(toml::from_str(&content)?)
    }

    /// Resolve a ProxyConfig with defaults applied.
    pub fn resolve(&self, cfg: &ProxyConfig) -> Cli {
        let d = self.defaults.as_ref();
        Cli {
            mode: match cfg.mode.as_deref().unwrap_or("cache") {
                "token" => ProxyMode::Token,
                _ => ProxyMode::Cache,
            },
            listen: cfg.listen.parse().unwrap_or_else(|_| "127.0.0.1:8788".parse().unwrap()),
            api_url: cfg.api_url.clone().or_else(|| d.and_then(|d| d.api_url.clone())).unwrap_or_else(|| "https://api.openai.com".into()),
            api_key: cfg.api_key.clone().or_else(|| d.and_then(|d| d.api_key.clone())).or_else(|| std::env::var("APHRODITE_API_KEY").ok()).or_else(|| std::env::var("DEEPSEEK_API_KEY").ok()).unwrap_or_default(),
            model: cfg.model.clone().or_else(|| d.and_then(|d| d.model.clone())).unwrap_or_else(|| "default-model".into()),
            max_context: 1_000_000,
            max_output: 384_000,
            ccr_db_path: cfg.ccr_db_path.clone().map(Into::into).unwrap_or_else(|| ".headroom/aphrodite-ccr.db".into()),
            ccr_ttl_seconds: cfg.ccr_ttl_seconds.or_else(|| d.and_then(|d| d.ccr_ttl_seconds)).unwrap_or(3600),
            no_ccr_inject_tool: false,
            no_ccr_marker: false,
            tool_relay: cfg.tool_relay.unwrap_or(false),
            notify_url: None,
            notify_key: None,
            dev: cfg.dev.unwrap_or(false),
            timeout: 300,
        }
    }
}

