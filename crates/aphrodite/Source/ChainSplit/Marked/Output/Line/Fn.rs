//! Classify one line of marked output.

use crate::ChainSplit::{Const::SegMarker, Marked::Output::Line::Enum::Enum};

/// A line is a boundary when its trimmed form is the segment marker, and text
/// otherwise.
///
/// The comparison against the marker is the constant pattern of the match, so the
/// predicate stays in this one place and the split site matches the two variants
/// instead of re-testing the line.
pub(crate) fn Fn(Line:&str) -> Enum<'_> {
	match Line.trim() {
		SegMarker => Enum::Boundary,
		_ => Enum::Text(Line),
	}
}
