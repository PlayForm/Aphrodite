//! What one hook call's kwargs carry, parsed once.
//!
//! Hermes passes different field names per hook (`result` for tool output,
//! `output` for terminal output, `content` for direct/test callers), so the
//! accessors encode that contract in one place instead of every arm re-deriving
//! it. `session_id` is resolved during decode, BEFORE any lock is taken - a
//! hook that resolved it inside the lock would hold the session mutex across a
//! `Debug::Session::Last::Fn::Fn()` read.

/// The decoded argument surface of one hook call.
pub(crate) struct Struct {
	/// The raw kwargs JSON Hermes passed, parsed once. Arms read optional
	/// fields (`args`, `status`, `parent_session_id`, `command`, ...) straight
	/// off it.
	pub(crate) Parsed:serde_json::Value,
	/// The session this call belongs to, resolved before any lock was taken.
	pub(crate) Session:String,
	/// Byte length of the raw kwargs JSON - the request-size proxy the turn
	/// telemetry reports (`pre_llm_call`).
	pub(crate) ArgsLength:usize,
}

impl Struct {
	/// Tool name Hermes reports, `"unknown"` when absent.
	pub(crate) fn Tool(&self) -> &str { self.Parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown") }

	/// Tool output: Hermes passes it under `result`; `content` is accepted for
	/// direct/test callers.
	pub(crate) fn ToolContent(&self) -> &str {
		self.Parsed
			.get("result")
			.or_else(|| self.Parsed.get("content"))
			.and_then(|v| v.as_str())
			.unwrap_or("")
	}

	/// Terminal output: Hermes passes it under `output`; `content` is accepted
	/// for direct/test callers.
	pub(crate) fn TermContent(&self) -> &str {
		self.Parsed
			.get("output")
			.or_else(|| self.Parsed.get("content"))
			.and_then(|v| v.as_str())
			.unwrap_or("")
	}
}
