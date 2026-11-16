//! The marker scanner.
//!
//! Resolved content is arbitrary text that may contain zero, one or many markers, in any
//! position, on any line. This module walks that text once and returns each reference with
//! the position it occupies, which is what the recursive resolver substitutes. It is not a
//! parser of the content itself - it knows nothing about what surrounds a marker - and it is
//! not the grammar's judge: what a candidate region IS comes from [`super::Tolerance`], so
//! the tolerance policy has one home and this loop has one job.
//!
//! The scanning rule matters more than it looks. After a reference the cursor moves past its
//! closing delimiter, so a prefix inside an already-consumed marker is never re-matched; an
//! unclosed prefix is stepped over rather than ending the scan. Getting that wrong either
//! panics on a malformed blob or silently drops the markers that follow it - both of which
//! turn a resolve into data loss.

use super::{Occurrence, Prefix, Tolerance};

/// Every reference in `Content`, in document order.
///
/// The occurrences borrow from `Content`, so the caller substitutes by span into the very
/// document it scanned instead of searching the text again for each marker. That is not only
/// faster: a marker left in place at the depth cap IS raw marker text, and a second search
/// would find it inside the output it had just written.
///
/// It is not a de-duplicator - a marker appearing twice is returned twice, at two different
/// spans, and both are substituted.
pub(crate) fn Fn(Content:&str) -> Vec<Occurrence::Struct::Struct<'_>> {
	let mut Occurrences = Vec::new();
	let mut Cursor = 0;

	while let Some(Found) = Content[Cursor..].find(Prefix::Const) {
		let Start = Cursor + Found;

		match Tolerance::Fn::Fn(Content, Start) {
			Tolerance::Enum::Enum::Reference { Marker, Hash, End } => {
				Occurrences.push(Occurrence::Struct::Struct { Span:Start..End, Marker, Hash });

				Cursor = End;
			},
			// Complete but nameless: nothing to look up, so the scan continues after it.
			Tolerance::Enum::Enum::Hashless { End } => Cursor = End,
			// Unclosed: step over the delimiter itself, not the rest of the text.
			Tolerance::Enum::Enum::Unterminated => Cursor = Start + Prefix::Const.len(),
		}
	}

	Occurrences
}
