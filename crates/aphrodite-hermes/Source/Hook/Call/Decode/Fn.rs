//! Decode the kwargs JSON a hook was called with.

use anyhow::Context;

use crate::{Hook::Call::Input::Struct::Struct, Session::Registry::Struct::DEFAULT_SESSION};

/// Parse the hook kwargs into the call's [`Struct`].
///
/// The parse error is wrapped with the kwargs' byte length, so the caller's
/// historical `invalid args: {e}` message still names the same fact while the
/// chain adds the operation that failed. Session resolution happens HERE, before
/// any lock: hooks carry `session_id` in their parsed args (`pre_tool_call`,
/// `transform_tool_result`, `pre_llm_call`, `post_llm_call`,
/// `on_session_start`); `transform_terminal_output` does NOT, so it falls back
/// to `Debug::Session::Last::Fn::Fn()` like before. Empty resolves to the default
/// session key.
///
/// It is not a validator: a kwargs object with no `session_id` is a legitimate
/// call, and a caller that must know whether Hermes threaded one reads
/// `Parsed` - which is why the resolved `Session` and the raw `Parsed` are
/// separate fields rather than one being inferred from the other.
pub(crate) fn Fn(Args:&str) -> anyhow::Result<Struct> {
	let Parsed:serde_json::Value =
		serde_json::from_str(Args).with_context(|| format!("parse the hook kwargs ({} bytes)", Args.len()))?;
	let Session = Parsed
		.get("session_id")
		.and_then(|v| v.as_str())
		.filter(|Session| !Session.is_empty())
		.map(str::to_string)
		.unwrap_or_else(|| {
			let Last = crate::Debug::Session::Last::Fn::Fn();

			match Last.is_empty() {
				true => DEFAULT_SESSION.to_string(),
				false => Last,
			}
		});

	Ok(Struct { Parsed, Session, ArgsLength:Args.len() })
}
