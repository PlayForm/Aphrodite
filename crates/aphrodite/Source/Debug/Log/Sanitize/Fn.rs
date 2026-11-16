//! `SanitizeKv`: keep a value on one line.

use std::borrow::Cow;

/// Sanitize a kv value for the single-line structured format.
///
/// Newlines become spaces rather than being stripped: the record format is one
/// line per event, so an embedded newline would split one record into two and
/// the second half would read as a malformed event. Replacing keeps the bytes
/// visible in the record, which is what an investigation needs.
///
/// Borrowed when there is nothing to replace: most kv values are already one line,
/// and a record with six of them should not allocate six strings to say so.
pub(crate) fn Fn(Value:&str) -> Cow<'_, str> {
	match Value.contains(['\n', '\r']) {
		true => Cow::Owned(Value.replace(['\n', '\r'], " ")),
		false => Cow::Borrowed(Value),
	}
}
