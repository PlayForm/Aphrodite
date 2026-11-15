//! `[compression]` - thresholds, engine window, auto-expand, retention caps.

/// The `[compression]` table.
///
/// Absent everywhere is the shipped behaviour: each key is read by the proxy
/// (`ResolveThresholds`) and by the Hermes dylib (`ApplyCompression`), and
/// the two must agree on the key names or a knob silently applies to only one
/// of them. That drift is why `ToolThreshold` was renamed to
/// `ToolThresholdToken` - see the F3 note in `ConfigurationLoader`'s engine
/// applier.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// Context-engine window trigger, as a percentage of the context limit.
	pub EngineThresholdPct:Option<u32>,
	/// Number of leading messages the engine never compresses.
	pub EngineProtectFirst:Option<u32>,
	/// Number of trailing messages the engine never compresses.
	pub EngineProtectLast:Option<u32>,
	/// Minimum message count before the engine considers compressing.
	pub EngineMinimumMsgs:Option<u32>,
	/// Tool-result byte threshold in token mode (the aggressive path).
	pub ToolThresholdToken:Option<u32>,
	/// Tool-result byte threshold in cache mode (the lightweight path).
	pub ToolThresholdCache:Option<u32>,
	/// Terminal-output byte threshold.
	pub TerminalThreshold:Option<u32>,
	/// Inline (non-tool) content byte threshold.
	pub InlineThreshold:Option<u32>,
	/// Resolve stored markers inline instead of leaving them to the model.
	pub AutoExpand:Option<bool>,
	/// Byte ceiling for inline expansion of a single marker.
	pub AutoExpandLimit:Option<u32>,
	/// `[compression] max_request_body_size` (pre-Rust port): proxy
	/// request-body cap in bytes - `Some(n)` rejects over-`n` bodies with
	/// 413 while they're being read; None (absent) = no cap (current
	/// behavior). Mirrors `AphroditeState::max_request_body_size`, which
	/// the Hermes-side config loader resolves from the same env/TOML key.
	pub MaximumRequestBodySize:Option<usize>,
	/// Catalog rendering mode for the recall catalog.
	pub CatalogMode:Option<String>,
	/// Whether the poll worker classifies stored payloads.
	pub ClassifierPoll:Option<bool>,
	/// Multiplier applied to the threshold for code-shaped content.
	pub CodeMultiplier:Option<f64>,
	/// `[compression] store_max_entries` - live-row cap for the SQLite
	/// CCR store; LRU rows are evicted past it. Absent or 0 = unlimited.
	/// The idle TTL bounds the store in *time* only, so a burst of
	/// compression inside one TTL window is unbounded without this.
	pub StoreMaximumEntries:Option<usize>,
	/// `[compression] store_max_bytes` - cap on stored payload bytes
	/// (`SUM(LENGTH(original))`). Absent or 0 = unlimited.
	pub StoreMaximumBytes:Option<u64>,
}
