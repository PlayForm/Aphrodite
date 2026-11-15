//! The I/O pass: read every requested path and classify the result, with no
//! access to `AphroditeState` at all.
//!
//! That absence is the point (F9). Holding whatever lock guards the state
//! across file I/O serializes every other session behind one slow or cold
//! mount, so reading happens here and only the classify-and-store pass takes
//! the lock. The refusal guard runs first, before `is_file` or `Metadata`, so
//! a sensitive path is never even stat'ed.

use std::path::Path;

use crate::Prefetch::{
	Outcome::Enum::ReadOutcome,
	Read::{Const::MaximumPrefetchSize, Refuse::Fn::RefuseReason},
};

/// Read every path from disk - no state access, so this never needs to hold
/// whatever lock guards the caller's `AphroditeState` (F9: a global
/// process/handle lock held across file I/O serializes every other
/// session's every call behind one slow/cold-mount read).
pub fn ReadPaths(Paths:&[String]) -> Vec<(String, ReadOutcome)> {
	Paths
		.iter()
		.map(|PathStr| {
			let path = Path::new(PathStr);
			if let Some(Reason) = RefuseReason(path) {
				return (PathStr.clone(), ReadOutcome::Refused { Reason });
			}
			if !path.is_file() {
				return (PathStr.clone(), ReadOutcome::Missing);
			}
			let Size = match std::fs::metadata(path) {
				Ok(m) => m.len(),
				Err(_) => return (PathStr.clone(), ReadOutcome::Error),
			};
			if Size > MaximumPrefetchSize {
				return (PathStr.clone(), ReadOutcome::SkippedSize { Size });
			}
			match std::fs::read_to_string(path) {
				Ok(Content) => (PathStr.clone(), ReadOutcome::Loaded { Content, Size }),
				Err(_) => (PathStr.clone(), ReadOutcome::Error),
			}
		})
		.collect()
}
