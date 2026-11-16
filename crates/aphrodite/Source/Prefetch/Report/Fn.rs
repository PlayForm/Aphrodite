//! Render the prefetch response.
//!
//! Takes no session: the shape of the answer is a function of the records and of the
//! two store numbers the caller read, so this leaf has no mutation authority and cannot
//! grow one. The counts are DERIVED from the records rather than accumulated alongside
//! them, which is what makes "total" and the list agree by construction instead of by
//! two updates staying in step.
//!
//! The store's usage and budget are part of the answer (report 05 F11): a prefetched
//! batch can evict older entries, including ones from the same batch, and a caller that
//! cannot see the budget discovers that only when a later retrieve unexpectedly misses.

use crate::Prefetch::Record::Enum::Record;

/// The response document for one prefetch call.
pub(crate) fn Fn(Records:Vec<Record>, Bytes:usize, Budget:usize) -> serde_json::Value {
	let Total = Records.len();
	let mut Loaded = 0u32;
	let mut SkippedSize = 0u32;
	let mut Refused = 0u32;
	let mut Missing = 0u32;

	let Results:Vec<serde_json::Value> = Records
		.into_iter()
		.map(|Record| {
			match Record {
				Record::Loaded { Path, Hash, Type, Size, Preview } => {
					Loaded += 1;

					// Full 40-char hash - a truncated one is unresolvable via exact-match
					// retrieval (report 05 F3).
					serde_json::json!({
						"path": Path,
						"status": "loaded",
						"hash": Hash.as_str(),
						"type": Type,
						"size": Size,
						"preview": Preview,
					})
				},
				Record::Skipped { Path, Size, Maximum } => {
					SkippedSize += 1;

					serde_json::json!({
						"path": Path,
						"status": "skipped",
						"reason": format!("exceeds per-file prefetch limit ({Size} bytes > {Maximum} byte max)"),
						"size": Size,
					})
				},
				Record::Refused { Path, Reason } => {
					Refused += 1;

					serde_json::json!({
						"path": Path,
						"status": "refused",
						"reason": Reason,
					})
				},
				Record::Missing { Path } => {
					Missing += 1;

					serde_json::json!({
						"path": Path,
						"status": "missing",
					})
				},
			}
		})
		.collect();

	serde_json::json!({
		"total": Total,
		"loaded": Loaded,
		"skipped_size": SkippedSize,
		"refused": Refused,
		"missing": Missing,
		"results": Results,
		"inline_store_bytes": Bytes,
		"inline_store_byte_budget": Budget,
	})
}
