//! The content-identity claims a prepared batch registers.
//!
//! A batch holds segment OCCURRENCES - `cargo build && cargo build` produces two
//! segments - but the split ledger attributes retrievals by content HASH: one
//! pending key per hash, consumed by the first resolve. Reporting occurrences as
//! "produced" therefore makes the retrieval rate unable to reach its own
//! denominator (a chain with a repeated segment can never be fully retrieved),
//! and the ledger's documented `Retrieved <= Produced` invariant false.
//!
//! So the batch reports what the ledger can actually count: the distinct hashes
//! it stored, in first-occurrence order. The two identities are different
//! questions - "how many segments were shown" and "how many stored outputs were
//! used" - and this is the second one, named here at the producer rather than
//! left implicit in the consumer.

use crate::ChainSplit::Prepared::Struct::Struct as Prepared;

/// The distinct content hashes the batch stored, in first-occurrence order.
///
/// Distinct rather than per-occurrence because the store holds one entry per
/// hash: two segments with identical text overwrite each other, and one resolve
/// serves both markers.
pub(in crate::ChainSplit) fn Fn(Prepared:&Prepared) -> Vec<String> {
	let mut Seen = std::collections::HashSet::new();

	Prepared
		.Segments
		.iter()
		.filter(|Segment| Seen.insert(Segment.Hash.as_str()))
		.map(|Segment| Segment.Hash.clone())
		.collect()
}
