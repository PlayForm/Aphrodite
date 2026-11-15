//! Extract a patch / write_file envelope's diff.

use serde_json::{Map, Value};

/// Extract the diff body, typed by the shared content classifier.
///
/// An empty `diff` is not a payload: `write_file` ships one on a no-op edit, and
/// previewing it as content would hide the `success`/`error` field that actually
/// says what happened.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	let Diff = Object.get("diff").and_then(Value::as_str).filter(|Diff| !Diff.is_empty())?;

	Some((Diff.to_string(), aphrodite::Preview::Detect::Type::Fn(Diff)))
}
