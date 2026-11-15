//! Extract a file-read envelope's content.

use serde_json::{Map, Value};

/// Extract the file body, typed by the shared content classifier.
///
/// `read_file` is normally essential-skipped before it ever reaches the
/// compressor, so this matcher exists for the paths that do not skip it - it is
/// the same envelope shape and there is no reason for those paths to preview a
/// brace.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	Object
		.get("content")
		.and_then(Value::as_str)
		.map(|Text| (Text.to_string(), aphrodite::Preview::Detect::Type::Fn(Text)))
}
