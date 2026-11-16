//! The `transform_terminal_output` arm.
//!
//! Hermes does NOT thread `session_id` through this hook
//! (`terminal_tool_result.py:144` passes only
//! command/output/returncode/task_id/env_type), so the session falls back to the
//! current turn's session - terminal output must keep the same session scope as
//! tool results, or a terminal result compresses into a session the model cannot
//! retrieve from.

use aphrodite::State::Struct::AphroditeState;

use crate::Hook::{Call::Input::Struct::Struct, PostTool, Replacement};

/// Compress one terminal result and return the value Hermes substitutes.
pub(crate) fn Fn(State:&mut AphroditeState, Input:&Struct) -> serde_json::Value {
	let TermContent = Input.TermContent();
	let Parsed = &Input.Parsed;

	let Classify = crate::Tool::Unwrap::Fn::Fn(TermContent);
	// 05-P2/T8: thread the terminal `command`/`returncode` into
	// the telemetry ring so terminal failures feed error-loop
	// detection.
	let Command = Parsed.get("command").and_then(|v| v.as_str());
	let ReturnCode = Parsed.get("returncode").and_then(|v| v.as_i64());
	let Result = aphrodite::Hook::Terminal::Entry::Fn::TransformTerminalOutputWithMeta(
		State,
		TermContent,
		Classify.as_ref().map(|(Body, Kind)| (Body.as_str(), Kind.as_str())),
		Command,
		ReturnCode,
	);
	// Hermes does NOT thread session_id through transform_terminal_output -
	// fall back to the current turn's session so terminal output keeps the
	// same session scope as tool results.
	let Session = Parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
	let Session = match Session.is_empty() {
		true => crate::Debug::Session::Last::Fn::Fn(),
		false => Session.to_string(),
	};

	// DEV-ONLY debug logger (crate::Debug): post-tool outcome +
	// elapsed microseconds for the terminal path.
	let Started = crate::Debug::Log::Any::Fn::Fn(State, &Session).then(std::time::Instant::now);

	if let Some(Started) = Started {
		let Micros = Started.elapsed().as_micros() as u64;
		aphrodite::Debug::Timing::Add::Fn::Fn(aphrodite::Debug::Timing::Enum::Enum::ToolTransform, Micros);
		PostTool::Fn::Fn(State, &Session, &Result, TermContent.len(), Micros);
	}

	Replacement::Fn::Fn(&Result, &Session)
}
