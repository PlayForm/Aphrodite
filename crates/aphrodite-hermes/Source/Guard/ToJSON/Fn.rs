//! Render a JSON payload for the C ABI, or report that it could not be rendered.

use std::ffi::c_char;

use crate::Guard::Fn::{to_c_string, to_json_error};

/// Serialise `Payload` for the caller, or answer with error JSON.
///
/// Every export that returns a JSON document goes through here, so a
/// serialisation failure has one shape instead of one per export. The exports
/// that spelled `serde_json::to_string(...).unwrap_or_default()` were the defect:
/// an empty string is a VALID C string, so the caller received `""` - neither a
/// result nor an error, and indistinguishable from a tool that legitimately
/// returned nothing. A failure has to arrive as `{"error": ...}`, which is the
/// shape the Python shim already branches on.
///
/// It takes a `serde_json::Value` rather than any `Serialize`, so this crate
/// keeps naming only types it already depends on (`serde_json`), and the export
/// that owns the payload is still the one that decides its shape.
pub(crate) fn Fn(Payload:&serde_json::Value) -> *mut c_char {
	match serde_json::to_string(Payload) {
		Ok(JSON) => to_c_string(&JSON),
		Err(Error) => to_json_error(&format!("serialize error: {Error}")),
	}
}
