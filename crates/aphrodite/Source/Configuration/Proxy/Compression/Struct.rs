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
	#[serde(rename = "engine_threshold_pct")]
	pub EngineThresholdPct:Option<u32>,
	/// Number of leading messages the engine never compresses.
	#[serde(rename = "engine_protect_first")]
	pub EngineProtectFirst:Option<u32>,
	/// Number of trailing messages the engine never compresses.
	#[serde(rename = "engine_protect_last")]
	pub EngineProtectLast:Option<u32>,
	/// Minimum message count before the engine considers compressing.
	#[serde(rename = "engine_min_msgs")]
	pub EngineMinimumMsgs:Option<u32>,
	/// Tool-result byte threshold in token mode (the aggressive path).
	#[serde(rename = "tool_threshold_token")]
	pub ToolThresholdToken:Option<u32>,
	/// Tool-result byte threshold in cache mode (the lightweight path).
	#[serde(rename = "tool_threshold_cache")]
	pub ToolThresholdCache:Option<u32>,
	/// Terminal-output byte threshold.
	#[serde(rename = "terminal_threshold")]
	pub TerminalThreshold:Option<u32>,
	/// Inline (non-tool) content byte threshold.
	#[serde(rename = "inline_threshold")]
	pub InlineThreshold:Option<u32>,
	/// Resolve stored markers inline instead of leaving them to the model.
	#[serde(rename = "auto_expand")]
	pub AutoExpand:Option<bool>,
	/// Byte ceiling for inline expansion of a single marker.
	#[serde(rename = "auto_expand_limit")]
	pub AutoExpandLimit:Option<u32>,
	/// `[compression] max_request_body_size` (pre-Rust port): proxy
	/// request-body cap in bytes - `Some(n)` rejects over-`n` bodies with
	/// 413 while they're being read; None (absent) = no cap (current
	/// behavior). Mirrors `AphroditeState::max_request_body_size`, which
	/// the Hermes-side config loader resolves from the same env/TOML key.
	#[serde(rename = "max_request_body_size")]
	pub MaximumRequestBodySize:Option<usize>,
	/// Catalog rendering mode for the recall catalog.
	#[serde(rename = "catalog_mode")]
	pub CatalogMode:Option<String>,
	/// Whether the poll worker classifies stored payloads.
	#[serde(rename = "classifier_poll")]
	pub ClassifierPoll:Option<bool>,
	/// Multiplier applied to the threshold for code-shaped content.
	#[serde(rename = "code_multiplier")]
	pub CodeMultiplier:Option<f64>,
	/// `[compression] store_max_entries` - live-row cap for the SQLite
	/// CCR store; LRU rows are evicted past it. Absent or 0 = unlimited.
	/// The idle TTL bounds the store in *time* only, so a burst of
	/// compression inside one TTL window is unbounded without this.
	#[serde(rename = "store_max_entries")]
	pub StoreMaximumEntries:Option<usize>,
	/// `[compression] store_max_bytes` - cap on stored payload bytes
	/// (`SUM(LENGTH(original))`). Absent or 0 = unlimited.
	#[serde(rename = "store_max_bytes")]
	pub StoreMaximumBytes:Option<u64>,
}
