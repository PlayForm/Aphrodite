//! The conversation index as JSON, oldest turn first.
//!
//! Sorted on read rather than on write: the index is a map, so insertion order
//! says nothing about turn order, and a caller that rendered it unsorted would
//! present the archive out of sequence.

use crate::State::AphroditeState;

/// Get the conversation index as a JSON-serializable value, sorted oldest first.
pub fn GetConvIndex(State:&AphroditeState) -> Vec<serde_json::Value> {
	let mut Turns:Vec<_> = State.ConvIndex.iter().collect();
	Turns.sort_by_key(|(k, _)| *k);
	Turns
		.iter()
		.map(
			|(Turn, (hash, Summary, Size))| serde_json::json!({"turn": Turn, "hash": hash, "summary": Summary, "size": Size}),
		)
		.collect()
}
