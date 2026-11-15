//! The borrowed tool-call metadata struct.

/// Metadata Hermes already sends alongside a tool result (report 05, P2) - the
/// signals the bridge historically dropped on the floor (`Status`,
/// `ErrorType`, `ErrorMessage`, `Args`, `DurationMs`). Threaded through
/// [`crate::Hook::transform_tool_result_with_meta`] into the `ToolEvents`
/// telemetry ring.
///
/// Borrowed, not owned: every field points into the caller's JSON or string
/// slice for the length of one call, so recording a tool event allocates
/// nothing. It is deliberately not `Serialize` - it never crosses the FFI
/// boundary or the proxy wire, and adding a derive would invite exactly that.
#[derive(Debug, Default, Clone, Copy)]
pub struct Struct<'a> {
	pub ArgsJSON:Option<&'a serde_json::Value>,
	pub Status:Option<&'a str>,
	pub ErrorType:Option<&'a str>,
	pub ErrorMessage:Option<&'a str>,
	pub DurationMs:Option<u64>,
}
