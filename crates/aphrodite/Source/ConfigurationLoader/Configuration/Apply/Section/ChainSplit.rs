//! `[compression]` chain-split thresholds - the adaptive split floor and
//! ceiling.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Write the chain-split switch and the adaptive threshold bounds.
///
/// The floor is forced to at least 2 (a split into one segment is not a
/// split), and the ceiling is forced to at least the floor - otherwise the
/// adaptation loop would run backwards and the threshold could never rise.
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── Fine-grained chain splitting ──
	// Default OFF for release: the segment markers (`echo __APHRODITE_SEG__`)
	// pollute captured stdout when commands are redirected to files, and
	// every split also adds marker lines to live tool output. Opt in per
	// session with APHRODITE_CHAIN_SPLIT=1 (or TOML chain_split = true).
	State.ChainSplit.Enable = Configuration.Bool("APHRODITE_CHAIN_SPLIT", "compression", "chain_split", false);

	// ── Tier 1 teaching loop: adaptive split threshold ──
	// `ChainSplit::Minimum` is the initial threshold (floor). The
	// threshold adapts within [floor, max] from the retrieval ratio of
	// split segments; see `AdaptChainSplitThreshold`.
	State.ChainSplit.Minimum = Configuration
		.Size(
			"APHRODITE_CHAIN_SPLIT_MIN_SEGMENTS",
			"compression",
			"chain_split_min_segments",
			2,
		)
		.max(2);
	State.ChainSplit.Floor = State.ChainSplit.Minimum;
	State.ChainSplit.Maximum = Configuration
		.Size(
			"APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS",
			"compression",
			"chain_split_max_segments",
			6,
		)
		.max(State.ChainSplit.Minimum);
}
