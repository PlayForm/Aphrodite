//! Match the search envelope without doing its work twice.

use serde_json::{Map, Value};

/// Rebuild a search envelope's rows, typed by the shape builder.
///
/// The cheap guard and the row work are both the resolver's: `total_count` is
/// tested before a single row is rebuilt inside `Search::Payload::Resolve`, so
/// this matcher holds no second copy of the prerequisite. A duplicated guard is
/// a guard that can drift - and the guarded form this replaced evaluated
/// `Search::Fn::Fn` as one half of a tuple scrutinee, so the row-rebuilding work
/// ran even when the `total_count` prerequisite had already failed.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	let (Body, Kind) = crate::Tool::Unwrap::Search::Fn::Fn(Object)?;

	Some((Body, Kind.to_string()))
}
