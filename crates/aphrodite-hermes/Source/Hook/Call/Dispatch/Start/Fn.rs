//! The `on_session_start` / `session_start` arm.

use aphrodite::State::AphroditeState;

use crate::Hook::{Call::Input::Struct::HookInput, Log::Fn::log_session_init};

/// Reset the session's CCR state for a new session.
///
/// The init debug line is emitted ONLY when Hermes actually threads a
/// `session_id` here: the `last_session()` fallback would target a STALE
/// persisted root from another process (e.g. the live parent session) and
/// append the init line to the WRONG debug file. The `pre_llm_call` arm emits
/// the init for the real session.
pub(crate) fn start(state:&mut AphroditeState, input:&HookInput) -> serde_json::Value {
	let r = aphrodite::Session::OnSessionStart(state);
	// DEV-ONLY debug logger (crate::Debug): session-init / config-summary
	// line, emitted once per process.
	if let Some(sid) = input.parsed.get("session_id").and_then(|v| v.as_str()) {
		log_session_init(state, sid);
	}
	r
}
