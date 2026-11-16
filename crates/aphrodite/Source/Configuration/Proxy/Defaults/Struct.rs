//! `[defaults]` - values every `[[proxies]]` entry inherits.

/// The `[defaults]` table.
///
/// These are fallbacks, not overrides: a `[[proxies]]` entry's own value wins
/// over the matching default, and the environment wins over both. Keeping
/// them separate is what lets a two-proxy cache/token setup share one API key
/// and URL while differing on everything else.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// `[defaults] api_url`; upstream API base URL inherited by proxies that do
	/// not set one. `None` = unset.
	#[serde(rename = "api_url")]
	pub URL:Option<String>,
	/// `[defaults] model`; model name inherited by proxies that do not set one.
	/// `None` = unset.
	#[serde(rename = "model")]
	pub Model:Option<String>,
	/// `[defaults] ccr_ttl_seconds`; CCR TTL in seconds inherited by proxies
	/// that do not set one. `None` = the compiled-in TTL.
	#[serde(rename = "ccr_ttl_seconds")]
	pub TTL:Option<u64>,
	/// `[defaults] api_key`; upstream API key inherited by proxies that do not
	/// set one. `None` = unset.
	#[serde(rename = "api_key")]
	pub Key:Option<String>,
}
