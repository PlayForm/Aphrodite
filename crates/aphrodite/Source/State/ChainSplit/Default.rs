//! The chain-split seed: off, floor at 2, ceiling at 6, empty ledger.

use std::collections::{HashMap, VecDeque};

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self {
			Enable:false,
			Minimum:2,
			Floor:2,
			Maximum:6,
			Ledger:VecDeque::new(),
			Segments:HashMap::new(),
			Next:0,
		}
	}
}
