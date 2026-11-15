//! `aphrodite_debug` - the dev-only per-session debug toggle.
//!
//! Flips the session-scoped flag file in the runtime home. Resolution is
//! root-session based (see `crate::Debug`): the CURRENT session's root id keys the
//! flag, so subagents inherit it and other sessions stay quiet. Pure Rust - the
//! customer-facing Python shim is untouched.
//!
//! Dev-only: gated behind `debug_assertions` so release dylibs register exactly
//! the 13 production tools (catalog validate rule 6 - the manifest declares 13; a
//! release dylib registering `aphrodite_debug` fails `hermes plugins validate` as
//! an undeclared tool). The debug-prefix/session-record machinery in
//! `crate::Debug` remains live in both profiles (it only activates on a flag
//! file).

use crate::Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_debug` handler.
///
/// A missing `on` defaults to `true` - asking for the tool at all is the request
/// to turn debug on, and the schema documents that default.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_debug", |args| {
		let on = args.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
		match crate::Debug::Session::Set::Fn::Fn(on) {
			Ok((session, flag)) => {
				serde_json::json!({
					"status": "ok",
					"debug": on,
					"session": session,
					"flag": flag,
				})
			},
			Err(e) => serde_json::json!({"error": format!("{e:#}")}),
		}
	})
}
