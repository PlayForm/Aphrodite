//! The shared compression settings every listener resolves its thresholds from.

use aphrodite::Configuration::Proxy::{
	Compression::Struct::Struct as CompressionConfiguration,
	Multi::Struct::Struct as MultiConfiguration,
};

/// The `[compression]` table, if the config file declares one.
///
/// One top-level section shared by every `[[proxies]]` entry (report 07 F2/T15):
/// each listener's `AppState` resolves its own cache-vs-token threshold from the
/// same table via `ResolveThresholds`, so the table is threaded to the listeners
/// rather than folded into them - two listeners with different thresholds still
/// read one document. `None` when running from the CLI, where every listener
/// falls back to the built-in thresholds.
pub fn Compression(MultiConfiguration:Option<&MultiConfiguration>) -> Option<CompressionConfiguration> {
	MultiConfiguration.and_then(|c| c.Compression.clone())
}
