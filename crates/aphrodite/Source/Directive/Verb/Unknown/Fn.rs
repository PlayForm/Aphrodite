//! The refusal for a directive name that is not loaded.

/// The error value `swap` and `load` answer for an unknown name.
///
/// One leaf because it is one string: the two verbs refuse for the same reason and a
/// caller must not have to tell two spellings of "unknown directive" apart. `add`
/// deliberately does NOT refuse - see its own leaf.
pub(crate) fn Fn(name:&str) -> serde_json::Value {
	serde_json::json!({"error": format!("unknown directive: {}", name)})
}
