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
	pub Listen:Option<String>,
	/// `Cache` or `Token`; an unknown value warns and resolves to token.
	pub Mode:Option<String>,
	/// Upstream API key; falls back to `[defaults] api_key` then the env.
	pub APIKey:Option<String>,
	/// Upstream API base URL; falls back to `[defaults]` then the env.
	pub APIURL:Option<String>,
	/// Model name; falls back to `[defaults]` then the env.
	pub Model:Option<String>,
	/// Enable the tool-relay endpoint for this proxy.
	pub ToolRelay:Option<bool>,
	/// Verbose request/response logging for this proxy.
	pub Dev:Option<bool>,
	/// CCR TTL in seconds; falls back to `[defaults]` then the env.
	pub CCRTtlSeconds:Option<u64>,
	/// SQLite CCR database path; an empty string counts as unset.
	pub CCRDatabasePath:Option<String>,
	/// Hermes callback URL for CCR notifications.
	pub NotifyURL:Option<String>,
	/// Hermes API key for callback auth.
	pub NotifyKey:Option<String>,
	/// Upstream request timeout in seconds; clamped to 600.
	pub Timeout:Option<u64>,
	/// Context window size; `MaximumOutput` must stay below it.
	pub MaximumContext:Option<usize>,
	/// Maximum output tokens; must be less than `MaximumContext`.
	pub MaximumOutput:Option<usize>,
}
