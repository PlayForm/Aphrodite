//! Decode a tool result into a `PollEvent`, or refuse it.
//!
//! Only a `process` result can be a poll; every other tool passes through, and
//! `None` is how the caller learns to take the normal compression path. The field
//! rules live in their own leaves because each has a different fallback: `Text`
//! reads a JSON string, `Output` falls back to the whole content, and `ExitCode`
//! falls back to a trailing `exit code: N` line.

use serde_json::Value;

use crate::PollWorker::{
	Decode::{ExitCode::Fn::Fn as Exit, Output::Fn::Fn as Out, Text::Fn::Fn as Field},
	Event::Struct::PollEvent,
};

/// Decode one tool result. `None` when the tool is not `process`.
pub(crate) fn Fn(ToolName:&str, Content:&str, ObservedAtUnix:u64) -> Option<PollEvent> {
	let Content = (ToolName == "process").then_some(Content)?;
	let Parsed = serde_json::from_str::<Value>(Content).ok();
	let Output = Out(&Parsed, Content).to_string();

	Some(PollEvent {
		Command:Field(&Parsed, "command"),
		ExecutionID:Field(&Parsed, "session_id"),
		ExitCode:Exit(&Parsed, &Output),
		Output,
		ObservedAtUnix,
	})
}
