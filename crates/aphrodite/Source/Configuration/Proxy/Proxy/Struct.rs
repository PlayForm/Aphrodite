//! One `[[proxies]]` entry.

/// A single proxy definition.
///
/// Every field is optional so a minimal entry (`Name` + `Mode`) inherits the
/// rest from `[defaults]`, the environment, or the compiled-in default. The
/// order in which those are consulted is `Multi::Resolve`'s, and it is
/// the contract of this table - see that function before reordering anything
/// here.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// `[[proxies]] name`; also selects the port override (`Cache`/`Token`).
	#[serde(rename = "name")]
	pub Name:Option<String>,
	/// `[[proxies]] listen`; listen address for this proxy, e.g.
	/// `127.0.0.1:9797`. `None` = the mode's default port.
	#[serde(default)]
	#[serde(rename = "listen")]
	pub Listen:Option<String>,
	/// `[[proxies]] mode`; `Cache` or `Token`. `None` = token; an unknown value
	/// warns and resolves to token.
	#[serde(rename = "mode")]
	pub Mode:Option<String>,
	/// `[[proxies]] api_key`; upstream API key, falling back to
	/// `[defaults] api_key` then the environment. `None` = unset.
	#[serde(rename = "api_key")]
	pub Key:Option<String>,
	/// `[[proxies]] api_url`; upstream API base URL, falling back to
	/// `[defaults] api_url` then the environment. `None` = unset.
	#[serde(rename = "api_url")]
	pub URL:Option<String>,
	/// `[[proxies]] model`; model name, falling back to `[defaults] model` then
	/// the environment. `None` = unset.
	#[serde(rename = "model")]
	pub Model:Option<String>,
	/// `[[proxies]] tool_relay`; enable the tool-relay endpoint for this proxy.
	/// `None` = off.
	#[serde(rename = "tool_relay")]
	pub Relay:Option<bool>,
	/// `[[proxies]] dev`; verbose request/response logging for this proxy.
	/// `None` = off.
	#[serde(rename = "dev")]
	pub Dev:Option<bool>,
	/// `[[proxies]] ccr_ttl_seconds`; CCR TTL in seconds, falling back to
	/// `[defaults]` then the environment. `None` = the compiled-in TTL.
	#[serde(rename = "ccr_ttl_seconds")]
	pub TTL:Option<u64>,
	/// `[[proxies]] ccr_db_path`; SQLite CCR database path. `None` or an empty
	/// string = the runtime home's `ccr.db`.
	#[serde(rename = "ccr_db_path")]
	pub Database:Option<String>,
	/// `[[proxies]] notify_url`; Hermes callback URL for CCR notifications.
	/// `None` = no notifications.
	#[serde(rename = "notify_url")]
	pub Callback:Option<String>,
	/// `[[proxies]] notify_key`; Hermes API key for callback auth. `None` = the
	/// callback goes out unauthenticated.
	#[serde(rename = "notify_key")]
	pub Auth:Option<String>,
	/// `[[proxies]] timeout`; upstream request timeout in seconds, clamped to
	/// 600. `None` = the compiled-in timeout.
	#[serde(rename = "timeout")]
	pub Timeout:Option<u64>,
	/// `[[proxies]] max_context`; context window size. `None` = the compiled-in
	/// window.
	#[serde(rename = "max_context")]
	pub Context:Option<usize>,
	/// `[[proxies]] max_output`; maximum output tokens, which must stay less
	/// than `Context`. `None` = the compiled-in cap.
	#[serde(rename = "max_output")]
	pub Output:Option<usize>,
}
