//! The shared compression settings every listener resolves its thresholds from.

use aphrodite::Configuration::Proxy::Compression::Struct::Struct as CompressionConfiguration;

use super::super::Source;

/// The `[compression]` table, if the configuration has one.
///
/// One top-level section is shared by every `[[proxies]]` entry (report 07
/// F2/T15): each listener's `AppState` resolves its own cache-vs-token threshold
/// from the same table, so the table is threaded to the listeners rather than
/// folded into them - two listeners with different thresholds still read one
/// document.
///
/// The standalone path has no document at all, so it has no table: every
/// listener falls back to the built-in thresholds. That is a property of the
/// configuration source, which is why this takes the source rather than an
/// `Option<&MultiConfiguration>` the caller had to unwrap correctly.
pub(crate) fn Fn(Source:&Source::Enum::Enum) -> Option<CompressionConfiguration> {
	match Source {
		Source::Enum::Enum::Multi(Configuration) => Configuration.Compression.clone(),
		Source::Enum::Enum::Standalone(_) => None,
	}
}
