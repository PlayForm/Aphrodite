//! The I/O pass: read every requested path and classify the result, with no
//! access to `AphroditeState` at all.
//!
//! That absence is the point (F9). Holding whatever lock guards the state across
//! file I/O serializes every other session behind one slow or cold mount, so
//! reading happens in `One/` and only the classify-and-store pass takes the lock.
//! The refusal guard runs first, before `is_file` or `Metadata`, so a sensitive
//! path is never even stat'ed.

use crate::Prefetch::{Outcome::Enum::Enum as Outcome, Read::One::Fn::Fn as ReadOne};

/// Read every path from disk - no state access, so this never needs to hold
/// whatever lock guards the caller's `AphroditeState` (F9: a global
/// process/handle lock held across file I/O serializes every other session's
/// every call behind one slow/cold-mount read).
///
/// The per-path outcome is `Read::One`'s business; this leaf only pairs it with
/// the path it came from, because the caller reports both.
pub fn Fn(Paths:&[String]) -> Vec<(String, Outcome)> {
	Paths.iter().map(|PathStr| (PathStr.clone(), ReadOne(PathStr))).collect()
}
