//! Decode a tool result into a `PollEvent`, or refuse it.
//!
//! Only a `process` result can be a poll; every other tool passes through, and
//! `None` is how the caller learns to take the normal compression path. The field
//! rules live in their own leaves because each has a different fallback: `Text`
//! reads a JSON string, `Output` falls back to the whole content, and `ExitCode`
//! falls back to a trailing `exit code: N` line.

use serde_json::Value;

use crate::{
	PollWorker::{
		Decode::{ExitCode::Fn::Fn as Exit, Output::Fn::Fn as Out, Text::Fn::Fn as Field},
		Event::Struct::PollEvent,
	},
	State::Identity::Execution::Struct::ExecutionID,
};

/// Decode one tool result. `None` when the tool is not `process`.
///
/// The event borrows the caller's content for its output when the result was never
/// wrapped, so the returned value carries `Content`'s lifetime.
pub(crate) fn Fn<'a>(ToolName:&str, Content:&'a str, ObservedAtUnix:u64) -> Option<PollEvent<'a>> {
	let Content = (ToolName == "process").then_some(Content)?;
	let mut Parsed = serde_json::from_str::<Value>(Content).ok();
	// The envelope's output string is MOVED out of the parsed value here, so the event
	// owns exactly one allocation and the bare-text path owns none.
	let Output = Out(&mut Parsed, Content);

	Some(PollEvent {
		Command:Field(&Parsed, "command"),
		ExecutionID:Field(&Parsed, "session_id").map(ExecutionID::from),
		ExitCode:Exit(&Parsed, Output.as_ref()),
		Output,
		ObservedAtUnix,
	})
}
