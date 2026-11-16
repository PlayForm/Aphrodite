//! The current turn's session id, with the persisted-file fallback.
//!
//! Hooks Hermes does NOT thread a session id through (`transform_terminal_output`
//! passes only command/output/returncode/task_id/env_type) still need a session
//! scope, and a fresh process starts with an empty static. The runtime-home file
//! written by `record_session` closes both gaps.

use std::sync::Mutex;

use crate::Debug::{Runtime, Session::State::Current};

/// The most recently seen session id (the current LLM turn's session, set by
/// `pre_llm_call`). Fallback for hooks Hermes does NOT thread session_id
/// through - `transform_terminal_output` passes only command/output/
/// returncode/task_id/env_type (terminal_tool_result.py:144), so the terminal
/// arm falls back to this instead of losing the session scope.
///
/// Persists across process restarts: `Record::Fn::Fn` also writes the id to
/// a tiny file in the runtime home, and a fresh process (or separate shim
/// exec) starts with empty Rust statics and falls back to reading that file.
/// Without this, terminal output with no prior `pre_llm_call` would have no
/// session scope.
///
/// It is not authoritative across processes: the file is the LAST writer's id,
/// so two concurrent Hermes processes share one `session.current` and the
/// fallback can name the other process's session. That is accepted - the
/// fallback only applies before this process has seen a turn.
///
/// An empty string means "nothing recorded and nothing persisted", which the
/// callers read as no session context rather than inventing one.
pub(crate) fn Fn() -> String {
	{
		let Guard = Current
			.get_or_init(|| Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);

		match Guard.as_str() {
			"" => {},
			Session => return Session.to_string(),
		}
	}

	// Reload fallback: read the persisted session id from the runtime home.
	std::fs::read_to_string(Runtime::Fn::Fn().join("session.current"))
		.ok()
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}
