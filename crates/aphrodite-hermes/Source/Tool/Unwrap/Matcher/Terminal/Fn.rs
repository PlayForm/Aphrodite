//! Extract a terminal envelope's output.

use serde_json::{Map, Value};

use crate::Tool::Unwrap::Matcher::Terminal::Classify;

/// Extract the command output from a terminal envelope.
///
/// The prerequisite is checked FIRST and the payload is transformed LAST: an
/// `output` key with no `exit_code` beside it is a user payload that happens to
/// name one of its fields `output`, so the transform must not run for it.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	let Output = Object
		.get("output")
		.and_then(Value::as_str)
		.filter(|Output| !Output.is_empty())?;

	Object.get("exit_code")?;

	Some((Output.to_string(), Classify::Fn::Fn(Output)))
}
