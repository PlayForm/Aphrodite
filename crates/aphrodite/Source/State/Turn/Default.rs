//! The turn bookkeeping's seed: turn zero, nothing scanned, empty index.

use std::collections::HashMap;

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self { TurnCounter:0, ScannedMessageIndex:0, ConvIndex:HashMap::new() }
	}
}
