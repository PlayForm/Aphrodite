//! `SanitizeKv`: keep a value on one line.

/// Sanitize a kv value for the single-line structured format.
///
/// Newlines become spaces rather than being stripped: the record format is one
/// line per event, so an embedded newline would split one record into two and
/// the second half would read as a malformed event. Replacing keeps the bytes
/// visible in the record, which is what an investigation needs.
pub(crate) fn Fn(Value:&str) -> String { Value.replace(['\n', '\r'], " ") }
