//! Dispatch by name.
//!
//! The registry is rebuilt per call, so adding a tool cannot leave a stale table
//! behind. Argument parsing is deliberately the dispatcher's job, not each
//! handler's: a malformed payload must produce the same error shape no matter
//! which tool was named.

use serde_json::Value;

use crate::Tool::Registry;

/// A tool handler: JSON args in, JSON result out.
///
/// Non-capturing closures coerce to this, which is why a family can hand back a
/// `|args| {...}` literal. A handler may not panic - the C ABI wraps every call
/// in `catch_unwind`, so a panic degrades to an error JSON rather than aborting
/// the host process, but it also loses the caller's real result.
pub type ToolHandler = fn(args:&serde_json::Value) -> Value;

/// Dispatch a tool by name. Returns `{"error": "..."}` for unknown tools.
///
/// An unknown name and an unparseable payload are both reported as `error`
/// objects rather than `null`, because a tool result that is not an object reads
/// as success to some callers. It does not validate the args against the schema -
/// each handler does its own required-field checks, which is what keeps the error
/// text specific to the tool.
///
/// One `tool_dispatch` span per call, with the tool name and the session id as
/// fields (§4.5). Tools carry no `session_id` in their args, so the session is
/// the current turn's (`Debug::Session::Last`) - the same resolution
/// `Session::With::Fn::with_shared` uses, hoisted here so a tool call and the
/// state it touches are attributed to one session in the log.
pub fn Fn(name:&str, args_json:&str) -> Value {
	// First real body a tool call runs through: install the subscriber here as
	// well as on the hook path, because a host may call a tool before any hook
	// fires (`Debug::Init` is a no-op on the second call).
	crate::Debug::Init::Fn::Fn();

	let Session = crate::Debug::Session::Last::Fn::Fn();
	let Span = tracing::info_span!("tool_dispatch", tool = name, session = %Session);
	let _Entered = Span.enter();

	let registry = Registry::Fn::Fn();
	match registry.get(name) {
		Some(handler) => {
			let args:Value = match serde_json::from_str(args_json) {
				Ok(v) => v,
				Err(e) => {
					// The message names the failure, never the payload: args can
					// carry a whole file body.
					tracing::warn!(target: crate::Debug::Init::Const::TARGET, tool = name, "unparseable tool args");

					return serde_json::json!({"error": format!("invalid args: {}", e)});
				},
			};
			handler(&args)
		},
		None => {
			tracing::warn!(target: crate::Debug::Init::Const::TARGET, tool = name, "unknown tool");

			serde_json::json!({"error": format!("unknown tool: {}", name)})
		},
	}
}
