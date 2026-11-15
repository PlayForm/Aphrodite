//! Match the search envelope without doing its work twice.

use serde_json::{Map, Value};

/// Rebuild a search envelope's rows, typed by the shape builder.
///
/// The prerequisite comes first and the matcher second, and that order is the
/// whole point: the guarded form evaluated `Search::Fn::Fn` as one half of a
/// tuple scrutinee, so the row-rebuilding work ran even when the `total_count`
/// prerequisite had already failed. The prerequisite is now a `?` and the
/// builder is only reached when it holds.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	Object.get("total_count").and_then(Value::as_u64)?;

	let (Body, Kind) = crate::Tool::Unwrap::Search::Fn::Fn(Object)?;

	Some((Body, Kind.to_string()))
}
