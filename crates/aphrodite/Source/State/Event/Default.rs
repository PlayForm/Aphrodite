//! The event ring's seed: empty.

use std::collections::VecDeque;

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self { ToolEvents:VecDeque::new() }
	}
}
