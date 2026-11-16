//! Route a hook name to its arm, under the session's bounded lock.

use aphrodite::State::Struct::AphroditeState;

use crate::{
	Hook::Call::{
		Dispatch::{PostLLM, PreLLM, PreTool, Start, Terminal, ToolResult},
		Input::Struct::Struct,
	},
	Session::With::Fn::with_session,
};

/// Dispatch one hook call against its session's state.
///
/// `None` means the session's lock is wedged by an abandoned worker: the caller
/// MUST fail open. An unknown hook name is NOT `None` - it returns error JSON,
/// because that is a caller bug the model should see, not a wedged gateway.
///
/// Every arm is a leaf named `Fn` under its hook's directory, so the table below
/// is the only place a hook name maps to code and each arm's signature is the
/// routing contract: a new hook is a new directory plus one line here.
pub(crate) fn Fn(Name:&str, Input:&Struct) -> Option<serde_json::Value> {
	with_session(&Input.Session, |State:&mut AphroditeState| -> serde_json::Value {
		match Name {
			// Accept both the canonical Hermes name and the legacy alias.
			"on_session_start" | "session_start" => Start::Fn::Fn(State, Input),
			"pre_tool_call" => PreTool::Fn::Fn(State, Input),
			"transform_tool_result" => ToolResult::Fn::Fn(State, Input),
			"transform_terminal_output" => Terminal::Fn::Fn(State, Input),
			"pre_llm_call" => PreLLM::Fn::Fn(State, Input),
			"post_llm_call" => PostLLM::Fn::Fn(State),
			_ => serde_json::json!({ "error": format!("unknown hook: {}", Name) }),
		}
	})
}
