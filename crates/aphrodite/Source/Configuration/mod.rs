//! CLI configuration for aphrodite.
//!
//! Generic LLM proxy - works with any OpenAI-compatible API.
//! Cache and Token modes with CCR, tool relay, programmatic CCR.

mod CLI;
mod Environment;
mod Proxy;

#[cfg(test)]
mod Test;

pub use CLI::{Cli, Command, SetupArgs};
pub use Environment::{env_bool, env_parse_warn};
pub use Proxy::{CompressionConfig, Defaults, MultiConfig, PreviewsConfig, PromptsConfig, ProxyConfig, ProxyMode};

pub use crate::State::TemplatesConfig;
