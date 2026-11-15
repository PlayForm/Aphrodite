//! The cumulative counter kinds.

/// Cumulative counter kinds tracked by the debug logger.
///
/// The discriminants are the INDEX into the storage array, so they are
/// load-bearing: renumbering a variant silently moves its tally onto another
/// slot and `MetricsJSON` would report one number under two names. Append new
/// kinds; never reorder, never reuse a retired number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// Markers recorded via `AphroditeState::record_marker` (every path).
	MarkersCreated = 0,
	/// Compress decisions that stored content (tool/terminal/chain-split).
	Compressions = 1,
	/// Auto-expand resolutions (content returned inline instead of a marker).
	AutoExpansions = 2,
	/// Live-container wraps.
	LiveContainers = 3,
	/// Context-engine middle-turn archives.
	EngineArchives = 4,
	/// Top-level retrieval requests (`Resolve::Expand::Fn`).
	Retrieves = 5,
	/// Retrievals that missed (hash not found).
	RetrieveMisses = 6,
	/// Preview/marker template renders (TOML `[templates]` hits).
	TemplateRenders = 7,
	/// Tool results reported with an error status.
	ToolErrors = 8,
	/// Degraded outcomes (retrieval misses, ...) worth counting.
	Warnings = 9,
}
