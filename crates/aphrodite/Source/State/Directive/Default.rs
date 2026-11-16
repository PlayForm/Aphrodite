//! The directive set's seed: nothing loaded, nothing active, no latch.

use std::collections::HashMap;

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self {
			Directives:HashMap::new(),
			ActiveDirectives:Vec::new(),
			EphemeralDirectives:Vec::new(),
			ManualDirectiveTurn:None,
		}
	}
}
