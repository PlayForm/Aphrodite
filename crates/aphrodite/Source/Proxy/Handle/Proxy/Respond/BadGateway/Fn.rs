//! The one 502 body the proxy emits.
//!
//! 02-F8: `reqwest::Error`'s `Display` can embed the upstream URL and, in some
//! TLS/DNS failure modes, host/proxy details; a caller should not be able to
//! fingerprint `state.api_url` through this response. The detailed error goes to
//! `/stats`'s `LastErrors` instead.
//!
//! One function rather than the same JSON literal at each failure site, so a
//! client sees the same body whichever upstream-side failure it hit.

use axum::{
	http::StatusCode,
	response::{IntoResponse, Json, Response},
};

/// The generic upstream-failure response.
pub(crate) fn Fn() -> Response {
	(
		StatusCode::BAD_GATEWAY,
		Json(serde_json::json!({"error": "upstream request failed"})),
	)
		.into_response()
}
