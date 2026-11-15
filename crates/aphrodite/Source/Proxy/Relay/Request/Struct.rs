//! Inbound body of `POST /tool_relay` - a Hermes-side tool call.
//!
//! The field names are the wire contract a third-party client sends; they are
//! `snake_case` and must not be renamed (contract: wire breaks are not style
//! changes). `CallbackURL` selects the asynchronous shape.

use serde::Deserialize;

/// Inbound request body for `POST /tool_relay`: a Hermes-side tool call to
/// execute against this proxy's CCR state (e.g. `AphroditeRetrieve`).
#[derive(Debug, Deserialize)]
pub struct Struct {
	#[serde(rename = "tool")]
	pub Tool:String,
	#[serde(rename = "params")]
	pub Params:serde_json::Value,
	#[serde(rename = "callback_url")]
	pub CallbackURL:Option<String>,
}
