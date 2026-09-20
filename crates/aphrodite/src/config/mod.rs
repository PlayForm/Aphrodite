//! CLI configuration for aphrodite.
//!
//! Generic LLM proxy - works with any OpenAI-compatible API.
//! Cache and Token modes with CCR, tool relay, programmatic CCR.

mod cli;
mod env;
mod proxy;

#[cfg(test)]
mod tests;

pub use cli::{Cli, Command, SetupArgs};
pub use env::{env_bool, env_parse_warn};
pub use proxy::{CompressionConfig, Defaults, MultiConfig, PreviewsConfig, PromptsConfig, ProxyConfig, ProxyMode};
