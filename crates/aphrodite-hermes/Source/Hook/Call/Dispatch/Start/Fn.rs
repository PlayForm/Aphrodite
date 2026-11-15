//! The `on_session_start` / `session_start` arm.

use aphrodite::State::Struct::AphroditeState;

use crate::Hook::{Call::Input::Struct::HookInput, Log::Fn::log_session_init};

/// Reset the session's CCR state for a new session.
///
/// The init debug line is emitted ONLY when Hermes actually threads a
/// `session_id` here: the `last_session()` fallback would target a STALE
/// persisted root from another process (e.g. the live parent session) and
/// append the init line to the WRONG debug file. The `pre_llm_call` arm emits
/// the init for the real session.
pub(crate) fn start(state:&mut AphroditeState, input:&HookInput) -> serde_json::Value {
	let r = aphrodite::Session::Start::Fn::OnSessionStart(state);
	// DEV-ONLY debug logger (crate::Debug): session-init / config-summary
	// line, emitted once per process.
	match input.parsed.get("session_id").and_then(|v| v.as_str()) {
		Some(sid) => log_session_init(state, sid),
		None => {},
	}

	r
}
