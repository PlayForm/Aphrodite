//! One chain-split event's consequence record.

/// One chain-split event's consequence record (Tier 1 teaching loop).
/// `Produced` = segment markers stored for this chain; `Retrieved` =
/// how many of those distinct hashes the agent later resolved (counted at
/// most once per hash). The adaptation rule reads the retrieval ratio
/// (retrieved/produced) over the recent window to move
/// `ChainSplitMinimumSegments` within its bounds.
#[derive(Debug, Clone)]
pub struct Struct {
	/// Event id (matches `SplitSegmentMap` values).
	pub ID:usize,
	/// Turn on which the chain output was split.
	pub Turn:usize,
	/// Segment markers produced for this chain.
	pub Produced:usize,
	/// Distinct produced hashes resolved since (≤ produced).
	pub Retrieved:usize,
}
