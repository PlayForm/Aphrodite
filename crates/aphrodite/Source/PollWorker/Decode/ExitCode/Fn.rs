//! The exit code a poll result reports, if it reports one.
//!
//! Two sources, in order: the envelope's `exit_code`, then its `returncode`
//! alias, then an `exit code: N` line in the output text (the unwrapped shape).
//! The line search walks from the end because a build's own output may mention
//! the phrase earlier without being the result.

use serde_json::Value;

use crate::PollWorker::Decode::ExitCode::Line::Fn::Fn as TextLine;

/// The exit code, or `None` while the process is still running.
pub(crate) fn Fn(Parsed:&Option<Value>, Output:&str) -> Option<i32> {
	let Envelope = Parsed
		.as_ref()
		.and_then(|Value| Value.get("exit_code").or_else(|| Value.get("returncode")))
		.and_then(|Code| Code.as_i64())
		.map(|Code| Code as i32);

	Envelope.or_else(|| TextLine(Output))
}
