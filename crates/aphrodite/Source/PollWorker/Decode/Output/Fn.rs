//! The output text of a poll result.
//!
//! An envelope carries it under `output`; a result that was never wrapped is its
//! own output. The whole content is the fallback rather than an empty string,
//! because a text poll's entire body is what the task accumulated before this
//! rewrite and the accumulator's contents are what the model later retrieves.
//!
//! The two sources differ in ownership, and the return type says so. A bare text result
//! borrows the caller's string - that is the whole body, so there is nothing to copy. An
//! envelope's field is MOVED out of the parsed `Value` (`std::mem::take` leaves an empty
//! string behind, and the value is dropped on return), so the decoded bytes are handed on
//! rather than copied a second time.

use std::borrow::Cow;

use serde_json::Value;

/// The output a poll result contributes, taking the envelope's string when there is one.
pub(crate) fn Fn<'a>(Parsed:&mut Option<Value>, Content:&'a str) -> Cow<'a, str> {
	let Envelope = Parsed.as_mut().and_then(|Value| Value.get_mut("output")).and_then(|Out| {
		match Out {
			Value::String(Text) => Some(std::mem::take(Text)),
			_ => None,
		}
	});

	match Envelope {
		Some(Out) => Cow::Owned(Out),
		None => Cow::Borrowed(Content),
	}
}
