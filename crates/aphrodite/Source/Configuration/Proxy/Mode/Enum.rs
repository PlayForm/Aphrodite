//! The proxy operation mode.

use clap::ValueEnum;

/// Proxy operation mode.
///
/// The mode selects the whole storage and injection strategy, not a tuning
/// knob: cache mode keeps CCR in memory and never injects tools, token mode
/// uses SQLite and injects them. An unknown string in the file is not an
/// error - `MultiConfig::resolve` warns and falls back to `Token`.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Enum {
	/// Cache mode - in-memory CCR, lightweight compression (>8KB threshold),
	/// preview preserved, no tool injection.
	Cache,
	/// Token mode - SQLite CCR, aggressive compression (>1KB threshold),
	/// tool injection, tool relay.
	Token,
}
