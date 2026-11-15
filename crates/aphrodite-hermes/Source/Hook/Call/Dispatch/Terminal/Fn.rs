//! The `transform_terminal_output` arm.
//!
//! Hermes does NOT thread `session_id` through this hook
//! (`terminal_tool_result.py:144` passes only
//! command/output/returncode/task_id/env_type), so the session falls back to the
//! current turn's session - terminal output must keep the same session scope as
//! tool results, or a terminal result compresses into a session the model cannot
//! retrieve from.

use aphrodite::State::AphroditeState;

use crate::Hook::{Call::Input::Struct::HookInput, PostTool::Fn::post_tool_log, Replacement::Fn::replacement_from};

/// Compress one terminal result and return the value Hermes substitutes.
pub(crate) fn transform_terminal_output(state:&mut AphroditeState, input:&HookInput) -> serde_json::Value {
	let term_content = input.term_content();
	let parsed = &input.parsed;

	let classify = crate::Tool::Unwrap::Fn(term_content);
	// 05-P2/T8: thread the terminal `command`/`returncode` into
	// the telemetry ring so terminal failures feed error-loop
	// detection.
	let command = parsed.get("command").and_then(|v| v.as_str());
	let returncode = parsed.get("returncode").and_then(|v| v.as_i64());
	let r = aphrodite::Hook::transform_terminal_output_with_meta(
		state,
		term_content,
		classify.as_ref().map(|(c, t)| (c.as_str(), t.as_str())),
		command,
		returncode,
	);
	// Hermes does NOT thread session_id through transform_terminal_output -
	// fall back to the current turn's session so terminal output keeps the
	// same session scope as tool results.
	let sid = parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
	let sid = if sid.is_empty() { crate::Debug::Session::Last::Fn() } else { sid.to_string() };
	// DEV-ONLY debug logger (crate::Debug): post-tool outcome +
	// elapsed microseconds for the terminal path.
	let t0 = crate::Debug::Log::Any::Fn(state, &sid).then(std::time::Instant::now);
	if let Some(t0) = t0 {
		let us = t0.elapsed().as_micros() as u64;
		aphrodite::Debug::Timing::Add::Fn::Fn(aphrodite::Debug::Timing::Enum::Enum::ToolTransform, us);
		post_tool_log(state, &sid, &r, term_content.len(), us);
	}
	replacement_from(&r, &sid)
}
