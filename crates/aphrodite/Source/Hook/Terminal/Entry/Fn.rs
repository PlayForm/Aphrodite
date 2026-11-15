//! The terminal-output entry points.
//!
//! Three signatures, one pipeline - the same shape as the tool-result entry
//! points, plus the `command`/`Returncode` pair Hermes reports for a terminal
//! call. `Returncode` is what makes the error-loop breaker work: a failing
//! command is a signal even when the output is too small to compress.

use super::super::Inner::Fn::Fn as TransformTerminalOutputInner;
use crate::State::AphroditeState;

/// Transform terminal output - exit code aware.
pub fn TransformTerminalOutput(State:&mut AphroditeState, Content:&str) -> serde_json::Value {
	TransformTerminalOutputClassified(State, Content, None)
}

/// Same as [`TransformTerminalOutput`], with the same `Classify` contract
/// as [`crate::Hook::transform_tool_result_classified`].
pub fn TransformTerminalOutputClassified(
	State:&mut AphroditeState,
	Content:&str,
	Classify:Option<(&str, &str)>,
) -> serde_json::Value {
	TransformTerminalOutputInner(State, Content, Classify, None, None)
}

/// Same as [`TransformTerminalOutputClassified`], plus the Hermes-supplied
/// `command` and `Returncode` (report 05, P2/T6/T8) recorded into the
/// `ToolEvents` telemetry ring. `returncode == 0` (or absent) is `Okay`.
pub fn TransformTerminalOutputWithMeta(
	State:&mut AphroditeState,
	Content:&str,
	Classify:Option<(&str, &str)>,
	command:Option<&str>,
	Returncode:Option<i64>,
) -> serde_json::Value {
	TransformTerminalOutputInner(State, Content, Classify, command, Returncode)
}
