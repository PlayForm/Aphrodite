//! One `[[proxies]]` entry.

/// A single proxy definition.
///
/// Every field is optional so a minimal entry (`name` + `Mode`) inherits the
/// rest from `[defaults]`, the environment, or the compiled-in default. The
/// order in which those are consulted is `MultiConfig::resolve`'s, and it is
/// the contract of this table - see that function before reordering anything
/// here.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// Entry name; also selects the port override (`Cache`/`Token`).
	pub name:Option<String>,
	/// Listen address for this proxy, e.g. `127.0.0.1:9797`.
	#[serde(default)]
	#[serde(rename = "listen")]
	pub Listen:Option<String>,
	/// `Cache` or `Token`; an unknown value warns and resolves to token.
	#[serde(rename = "mode")]
	pub Mode:Option<String>,
	/// Upstream API key; falls back to `[defaults] api_key` then the env.
	#[serde(rename = "api_key")]
	pub APIKey:Option<String>,
	/// Upstream API base URL; falls back to `[defaults]` then the env.
	#[serde(rename = "api_url")]
	pub APIURL:Option<String>,
	/// Model name; falls back to `[defaults]` then the env.
	#[serde(rename = "model")]
	pub Model:Option<String>,
	/// Enable the tool-relay endpoint for this proxy.
	#[serde(rename = "tool_relay")]
	pub ToolRelay:Option<bool>,
	/// Verbose request/response logging for this proxy.
	#[serde(rename = "dev")]
	pub Dev:Option<bool>,
	/// CCR TTL in seconds; falls back to `[defaults]` then the env.
	#[serde(rename = "ccr_ttl_seconds")]
	pub CCRTtlSeconds:Option<u64>,
	/// SQLite CCR database path; an empty string counts as unset.
	#[serde(rename = "ccr_db_path")]
	pub CCRDatabasePath:Option<String>,
	/// Hermes callback URL for CCR notifications.
	#[serde(rename = "notify_url")]
	pub NotifyURL:Option<String>,
	/// Hermes API key for callback auth.
	#[serde(rename = "notify_key")]
	pub NotifyKey:Option<String>,
	/// Upstream request timeout in seconds; clamped to 600.
	#[serde(rename = "timeout")]
	pub Timeout:Option<u64>,
	/// Context window size; `MaximumOutput` must stay below it.
	#[serde(rename = "max_context")]
	pub MaximumContext:Option<usize>,
	/// Maximum output tokens; must be less than `MaximumContext`.
	#[serde(rename = "max_output")]
	pub MaximumOutput:Option<usize>,
}
