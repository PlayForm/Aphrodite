//! The chain-split domain's own fields.
//!
//! Split out of `AphroditeState` so the teaching loop takes
//! `&mut ChainSplit::Struct`: recording a split, attributing a resolve and
//! adapting the threshold need the switch, the three segment-count bounds, the
//! consequence ledger and the attribution map. The turn number a split is
//! stamped with is passed in, because the loop has no other business with the
//! session's turn bookkeeping.

use std::collections::{HashMap, VecDeque};

use super::Entry::Struct::Struct as Entry;

/// The chain-split switch, its adaptive segment-count bounds, and the
/// consequence ledger that moves them.
#[derive(Clone)]
pub struct Struct {
	/// Fine-grained chain splitting: rewrite chained shell commands
	/// (`a && b && c`) with segment markers and split the output into
	/// per-segment CCR entries, so the agent sees N compact previews
	/// instead of one giant blob. Struct default false (fix 2), matching
	/// the shipped config default; `Apply::Startup::Fn` resolves the TOML/env
	/// override (opt-in per session via `APHRODITE_CHAIN_SPLIT=1` or TOML
	/// `[compression] chain_split`). Env: `APHRODITE_CHAIN_SPLIT`, TOML:
	/// `[compression] chain_split`.
	pub ChainSplitEnabled:bool,
	/// Current minimum segment count for chain splitting. Only chains with
	/// at least this many segments are rewritten. Starts at the configured
	/// floor (default 2, `SplitChain` already requires ≥2) and adapts
	/// within `[ChainSplitFloor, ChainSplitMaximumSegments]` based on
	/// whether the agent actually retrieves the produced segment markers
	/// (consequence-driven learning). The threshold is machinery - never
	/// observable to the LLM (no directive text, no summary changes).
	pub ChainSplitMinimumSegments:usize,
	/// Lower bound of the adaptive threshold: the configured initial value.
	pub ChainSplitFloor:usize,
	/// Upper bound of the adaptive threshold. Env:
	/// `APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS`, TOML:
	/// `[compression] chain_split_max_segments`. Default 6.
	pub ChainSplitMaximumSegments:usize,
	/// Consequence ledger: one entry per chain-split event recording how
	/// many segment markers were produced and how many distinct ones were
	/// later retrieved. Bounded ring (cap `ChainSplit::Const::Cap`); oldest
	/// evicted. `SplitSegmentMap` maps each produced segment hash to its
	/// event id so a resolve can be attributed to the right event.
	pub SplitEvents:VecDeque<Entry>,
	/// Segment hash → split-event id, for retrieval attribution. A hash
	/// resolves at most once (removed after counting), so re-retrieving the
	/// same marker never double-counts.
	pub SplitSegmentMap:HashMap<String, usize>,
	/// Monotonic event id counter for `SplitEvents`.
	pub SplitNextEventID:usize,
}
