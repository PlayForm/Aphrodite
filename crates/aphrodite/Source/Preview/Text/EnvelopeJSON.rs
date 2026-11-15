//! Hermes wrapper-envelope predicate.
//!
//! The JSON arm has to choose between two honest answers: the key list of the
//! payload, or the payload itself when the payload is wrapped. This module
//! decides which of the two a parsed object is. It is not a JSON parser and it
//! does not validate structure - it only answers "is this object a Hermes
//! envelope?", and getting it wrong makes a wrapped result preview as a list of
//! envelope keys instead of the result.

use serde_json::Value as JSONValue;

/// True when a JSON object is a Hermes wrapper envelope.
///
/// A wrapper envelope carries the real payload under one of a fixed set of
/// field names (`Output`, `Content`, `Matches`, ...), so previewing its key
/// list is intentional and correct: the key list names the payload. This is
/// not a heuristic about JSON shape - a plain object with none of the guard
/// keys is a payload in its own right and must preview as one.
pub(crate) fn Fn(Obj:&serde_json::Map<String, JSONValue>) -> bool {
	const Guard:&[&str] = &[
		"output",
		"exit_code",
		"diff",
		"error",
		"success",
		"total_count",
		"matches",
		"matches_text",
		"content",
		"total_lines",
		"result",
		"message",
		"found",
		"preview",
		"name",
		"description",
	];
	Obj.keys().any(|k| Guard.contains(&k.as_str()))
}
