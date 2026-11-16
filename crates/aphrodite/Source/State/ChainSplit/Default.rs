//! The chain-split seed: off, floor at 2, ceiling at 6, empty ledger.

use std::collections::{HashMap, VecDeque};

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self {
			ChainSplitEnabled:false,
			ChainSplitMinimumSegments:2,
			ChainSplitFloor:2,
			ChainSplitMaximumSegments:6,
			SplitEvents:VecDeque::new(),
			SplitSegmentMap:HashMap::new(),
			SplitNextEventID:0,
		}
	}
}
