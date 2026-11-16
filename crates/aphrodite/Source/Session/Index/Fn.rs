//! The conversation index as JSON, oldest turn first.
//!
//! Sorted on read rather than on write: the index is a map, so insertion order
//! says nothing about turn order, and a caller that rendered it unsorted would
//! present the archive out of sequence.

use crate::State::Turn::Struct::Struct as Turn;

/// Get the conversation index as a JSON-serializable value, sorted oldest first.
///
/// Takes the turn bookkeeping rather than the session state: this renderer reads
/// one map and must not be able to reach anything else.
pub fn Fn(Turn:&Turn) -> Vec<serde_json::Value> {
	let mut Sorted:Vec<_> = Turn.Index.iter().collect();
	Sorted.sort_by_key(|(k, _)| *k);
	Sorted
		.iter()
		.map(
			|(Number, (hash, Summary, Size))| serde_json::json!({"turn": Number, "hash": hash, "summary": Summary, "size": Size}),
		)
		.collect()
}
