//! Render a replacement outcome into the value Hermes reads back.

use serde_json::Value;

use crate::{
	Debug,
	Hook::Replacement::Choice::{Enum::Enum, Resolve},
};

/// Map a hook compression result to the value Hermes uses to replace output.
///
/// Hermes only honors a *string* return from `transform_tool_result` /
/// `transform_terminal_output` (first non-None string wins; non-strings pass
/// through). So this returns the resolved replacement as a string, and `null` for
/// `Passthrough` - leaving the original output untouched.
///
/// It renders and decides nothing: WHICH outcome applies is
/// `Choice::Resolve::Fn::Fn`'s decision, and this match is total over that value,
/// so a new outcome cannot reach Hermes without a rendering arm here.
///
/// Only the single-marker form carries the dev-only debug line, because it is the
/// one form where the model would otherwise see a bare marker with no account of
/// what the engine did. The quiet path (flag off) costs one stat() and returns the
/// marker unchanged.
pub(crate) fn Fn(Result:&Value, Session:&str) -> Value {
	match Resolve::Fn::Fn(Result) {
		Enum::Chain(Joined) => Value::String(Joined),
		Enum::Expanded(ExpandedText) => Value::String(ExpandedText.to_string()),
		Enum::Marker(MarkerText) => {
			Value::String(match Debug::Log::Line::Fn::Fn(Result, Session) {
				Some(Line) => format!("{Line}\n{MarkerText}"),
				None => MarkerText.to_string(),
			})
		},
		Enum::Passthrough => Value::Null,
	}
}
