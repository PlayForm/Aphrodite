//! Prepare a whole chain's artifacts with no session access.

use crate::ChainSplit::{
	Marked::Output::Fn::Fn as SplitMarkedOutput,
	Prepared::{Segment::Fn::Fn as PrepareSegment, Struct::Struct as Prepared},
};

/// Build every segment's artifacts, or answer `None` when there is no chain.
///
/// `None` means "not a split": the content carried no segment marker, or the
/// marker split produced fewer than two non-empty parts, or the content was
/// empty. The caller treats it as "fall through to the ordinary gates" rather
/// than as a failure - a chain of one is not a chain.
///
/// Nothing here touches `AphroditeState`: detection, hashing, previewing and
/// marker rendering are pure functions of the text, which is what makes the
/// apply phase the only mutating step.
pub(crate) fn Fn(Content:&str) -> Option<Prepared> {
	let Parts = SplitMarkedOutput(Content);
	let Parts = (Parts.len() >= 2).then_some(Parts)?;

	let Segments:Vec<_> = Parts.iter().map(|(Index, Text)| PrepareSegment(*Index, Text)).collect();
	let OriginalBytes = Segments.iter().map(|Segment| Segment.Text.len()).sum();
	let MarkerBytes = Segments.iter().map(|Segment| Segment.Marker.len()).sum();

	Some(Prepared { Segments, OriginalBytes, MarkerBytes })
}
