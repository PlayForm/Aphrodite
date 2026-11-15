//! The output text of a poll result.
//!
//! An envelope carries it under `output`; a result that was never wrapped is its
//! own output. The whole content is the fallback rather than an empty string,
//! because a text poll's entire body is what the task accumulated before this
//! rewrite and the accumulator's contents are what the model later retrieves.

use serde_json::Value;

/// The output a poll result contributes.
pub(crate) fn Fn<'a>(Parsed:&'a Option<Value>, Content:&'a str) -> &'a str {
	Parsed
		.as_ref()
		.and_then(|Value| Value.get("output"))
		.and_then(|Out| Out.as_str())
		.unwrap_or(Content)
}
