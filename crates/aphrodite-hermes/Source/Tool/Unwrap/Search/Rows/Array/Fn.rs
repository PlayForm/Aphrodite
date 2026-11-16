//! Rows from a `matches` array.

use serde_json::{Map, Value};

/// Rebuild the `matches` array as grep-style `path:line:content` rows.
///
/// An element without a `path`/`file` and a `line` is not a match row and is
/// dropped rather than rendered as an empty line: the preview counts the rows it
/// is given, so an unparsed element must not inflate the count.
///
/// It applies no size cap. The rows feed only the preview and are never stored or
/// hashed, so truncating them would reintroduce the count that lied.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Vec<String> {
	Object
		.get("matches")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|Match| {
			let Path = Match.get("path").or(Match.get("file")).and_then(Value::as_str)?;
			let Line = Match.get("line")?;
			let Content = Match.get("content").and_then(Value::as_str).unwrap_or("");

			Some(format!("{}:{}:{}", Path, Line, Content))
		})
		.collect()
}
