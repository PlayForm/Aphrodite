//! Decode the kwargs JSON a hook was called with.

use crate::{Hook::Call::Input::Struct::HookInput, Session::Registry::Struct::DEFAULT_SESSION};

/// Parse the hook kwargs into a [`HookInput`].
///
/// Returns the serde error unchanged so the caller can render the historical
/// `invalid args: {e}` message. Session resolution happens HERE, before any
/// lock: hooks carry `session_id` in their parsed args (`pre_tool_call`,
/// `transform_tool_result`, `pre_llm_call`, `post_llm_call`,
/// `on_session_start`); `transform_terminal_output` does NOT, so it falls back
/// to `Debug::Session::Last::Fn()` like before. Empty resolves to the default
/// session key.
pub(crate) fn decode(args:&str) -> Result<HookInput, serde_json::Error> {
	let parsed:serde_json::Value = serde_json::from_str(args)?;
	let session_id = parsed
		.get("session_id")
		.and_then(|v| v.as_str())
		.filter(|s| !s.is_empty())
		.map(str::to_string)
		.unwrap_or_else(|| {
			let last = crate::Debug::Session::Last::Fn();
			if last.is_empty() { DEFAULT_SESSION.to_string() } else { last }
		});
	Ok(HookInput { parsed, session_id, args_len:args.len() })
}
