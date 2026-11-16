//! The marker catalog's seed: an empty ring and a zero delta counter.

use std::collections::VecDeque;

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self { RecentMarkers:VecDeque::new(), LastEmittedMarkerCount:0 }
	}
}
