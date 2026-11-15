//! Response body of `POST /tool_relay`.
//!
//! When the request carried a `CallbackURL` the call runs asynchronously and
//! this comes back immediately with `async_call:true` and no `Result`; the
//! real result is POSTed to the callback later. A synchronous caller relies on
//! `Result` being present.

use serde::Serialize;

/// Response for a tool relay call. When `CallbackURL` was set on the
/// request, the call runs asynchronously and this comes back immediately
/// with `async_call:true` and no `Result` - the real result is POSTed to
/// the callback URL later.
#[derive(Debug, Serialize)]
pub struct Struct {
	pub Success:bool,
	pub Result:Option<serde_json::Value>,
	pub error:Option<String>,
	pub AsyncCall:bool,
}

// ── CCR management types ────────────────────────────────────────────
