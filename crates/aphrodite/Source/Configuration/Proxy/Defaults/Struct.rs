//! `[defaults]` - values every `[[proxies]]` entry inherits.

/// The `[defaults]` table.
///
/// These are fallbacks, not overrides: a `[[proxies]]` entry's own value wins
/// over the matching default, and the environment wins over both. Keeping
/// them separate is what lets a two-proxy cache/token setup share one API key
/// and URL while differing on everything else.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// Upstream API base URL inherited by proxies that do not set one.
	#[serde(rename = "api_url")]
	pub APIURL:Option<String>,
	/// Model name inherited by proxies that do not set one.
	#[serde(rename = "model")]
	pub Model:Option<String>,
	/// CCR TTL in seconds inherited by proxies that do not set one.
	#[serde(rename = "ccr_ttl_seconds")]
	pub CCRTtlSeconds:Option<u64>,
	/// Upstream API key inherited by proxies that do not set one.
	#[serde(rename = "api_key")]
	pub APIKey:Option<String>,
}
