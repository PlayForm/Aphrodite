//! Route a hook name to its arm, under the session's bounded lock.

use aphrodite::State::AphroditeState;

use crate::{
	Hook::Call::{
		Dispatch::{PostLLM, PreLLM, PreTool, Start, Terminal, ToolResult},
		Input::Struct::HookInput,
	},
	Session::With::Fn::with_session,
};

/// Dispatch one hook call against its session's state.
///
/// `None` means the session's lock is wedged by an abandoned worker: the caller
/// MUST fail open. An unknown hook name is NOT `None` - it returns error JSON,
/// because that is a caller bug the model should see, not a wedged gateway.
pub(crate) fn dispatch(name:&str, input:&HookInput) -> Option<serde_json::Value> {
	with_session(&input.session_id, |state:&mut AphroditeState| -> serde_json::Value {
		match name {
			// Accept both the canonical Hermes name and the legacy alias.
			"on_session_start" | "session_start" => Start::Fn::start(state, input),
			"pre_tool_call" => PreTool::Fn::pre_tool_call(state, input),
			"transform_tool_result" => ToolResult::Fn::transform_tool_result(state, input),
			"transform_terminal_output" => Terminal::Fn::transform_terminal_output(state, input),
			"pre_llm_call" => PreLLM::Fn::pre_llm_call(state, input),
			"post_llm_call" => PostLLM::Fn::post_llm_call(state),
			_ => serde_json::json!({ "error": format!("unknown hook: {}", name) }),
		}
	})
}
