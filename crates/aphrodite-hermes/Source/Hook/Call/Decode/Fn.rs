//! Decode the kwargs JSON a hook was called with.

use anyhow::Context;

use crate::{Hook::Call::Input::Struct::HookInput, Session::Registry::Struct::DEFAULT_SESSION};

/// Parse the hook kwargs into a [`HookInput`].
///
/// The parse error is wrapped with the kwargs' byte length, so the caller's
/// historical `invalid args: {e}` message still names the same fact while the
/// chain adds the operation that failed. Session resolution happens HERE, before
/// any lock: hooks carry `session_id` in their parsed args (`pre_tool_call`,
/// `transform_tool_result`, `pre_llm_call`, `post_llm_call`,
/// `on_session_start`); `transform_terminal_output` does NOT, so it falls back
/// to `Debug::Session::Last::Fn::Fn()` like before. Empty resolves to the default
/// session key.
pub(crate) fn decode(args:&str) -> anyhow::Result<HookInput> {
	let parsed:serde_json::Value =
		serde_json::from_str(args).with_context(|| format!("parse the hook kwargs ({} bytes)", args.len()))?;
	let session_id = parsed
		.get("session_id")
		.and_then(|v| v.as_str())
		.filter(|s| !s.is_empty())
		.map(str::to_string)
		.unwrap_or_else(|| {
			let last = crate::Debug::Session::Last::Fn::Fn();

			match last.is_empty() {
				true => DEFAULT_SESSION.to_string(),
				false => last,
			}
		});
	Ok(HookInput { parsed, session_id, args_len:args.len() })
}
