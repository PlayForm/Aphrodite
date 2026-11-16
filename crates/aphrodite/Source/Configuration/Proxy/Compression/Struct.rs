//! `[compression]` - thresholds, engine window, auto-expand, retention caps.

/// The `[compression]` table.
///
/// Absent everywhere is the shipped behaviour: each key is read by the proxy
/// (`ResolveThresholds`) and by the Hermes dylib (`Apply::Startup::Fn`), and
/// the two must agree on the key names or a knob silently applies to only one
/// of them. That drift is why the engine's tool threshold was once keyed
/// `tool_threshold` alone - see the F3 note in `ConfigurationLoader`'s engine
/// applier.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// `[compression] engine_threshold_pct`; context-engine window trigger, as a
	/// percentage of the context limit. `None` = the engine's default.
	#[serde(rename = "engine_threshold_pct")]
	pub Trigger:Option<u32>,
	/// `[compression] engine_protect_first`; number of leading messages the
	/// engine never compresses. `None` = the engine's default.
	#[serde(rename = "engine_protect_first")]
	pub First:Option<u32>,
	/// `[compression] engine_protect_last`; number of trailing messages the
	/// engine never compresses. `None` = the engine's default.
	#[serde(rename = "engine_protect_last")]
	pub Last:Option<u32>,
	/// `[compression] engine_min_msgs`; minimum message count before the engine
	/// considers compressing. `None` = the engine's default.
	#[serde(rename = "engine_min_msgs")]
	pub Minimum:Option<u32>,
	/// `[compression] tool_threshold_token`; tool-result byte threshold in token
	/// mode (the aggressive path). `None` = the compiled-in threshold.
	#[serde(rename = "tool_threshold_token")]
	pub Token:Option<u32>,
	/// `[compression] tool_threshold_cache`; tool-result byte threshold in cache
	/// mode (the lightweight path). `None` = the compiled-in threshold.
	#[serde(rename = "tool_threshold_cache")]
	pub Cache:Option<u32>,
	/// `[compression] terminal_threshold`; terminal-output byte threshold.
	/// `None` = the compiled-in threshold.
	#[serde(rename = "terminal_threshold")]
	pub Terminal:Option<u32>,
	/// `[compression] inline_threshold`; inline (non-tool) content byte
	/// threshold. `None` = the compiled-in threshold.
	#[serde(rename = "inline_threshold")]
	pub Inline:Option<u32>,
	/// `[compression] auto_expand`; resolve stored markers inline instead of
	/// leaving them to the model. `None` = off.
	#[serde(rename = "auto_expand")]
	pub Expand:Option<bool>,
	/// `[compression] auto_expand_limit`; byte ceiling for inline expansion of a
	/// single marker. `None` = the compiled-in ceiling.
	#[serde(rename = "auto_expand_limit")]
	pub Limit:Option<u32>,
	/// `[compression] max_request_body_size` (pre-Rust port): proxy
	/// request-body cap in bytes - `Some(n)` rejects over-`n` bodies with
	/// 413 while they're being read; None (absent) = no cap (current
	/// behavior). Mirrors `AphroditeState::max_request_body_size`, which
	/// the Hermes-side config loader resolves from the same env/TOML key.
	#[serde(rename = "max_request_body_size")]
	pub Body:Option<usize>,
	/// `[compression] catalog_mode`; catalog rendering mode for the recall
	/// catalog. `None` = the renderer's default.
	#[serde(rename = "catalog_mode")]
	pub Catalog:Option<String>,
	/// `[compression] classifier_poll`; whether the poll worker classifies
	/// stored payloads. `None` = off.
	#[serde(rename = "classifier_poll")]
	pub Poll:Option<bool>,
	/// `[compression] code_multiplier`; multiplier applied to the threshold for
	/// code-shaped content. `None` = the compiled-in multiplier.
	#[serde(rename = "code_multiplier")]
	pub Multiplier:Option<f64>,
	/// `[compression] store_max_entries` - live-row cap for the SQLite
	/// CCR store; LRU rows are evicted past it. Absent or 0 = unlimited.
	/// The idle TTL bounds the store in *time* only, so a burst of
	/// compression inside one TTL window is unbounded without this.
	#[serde(rename = "store_max_entries")]
	pub Entries:Option<usize>,
	/// `[compression] store_max_bytes` - cap on stored payload bytes
	/// (`SUM(LENGTH(original))`). Absent or 0 = unlimited.
	#[serde(rename = "store_max_bytes")]
	pub Bytes:Option<u64>,
}
