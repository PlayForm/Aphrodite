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
/// The parser's owned parts are CONSUMED here, not borrowed: each part's text
/// moves into its segment, so the batch holds the one allocation the parser
/// made. `Prepared::Assemble` then derives the byte totals from those segments,
/// which is what keeps a batch's totals and its contents from disagreeing.
///
/// Nothing here touches `AphroditeState`: detection, hashing, previewing and
/// marker rendering are pure functions of the text (and the process-wide
/// rendering configuration), which is what makes the apply phase the only
/// mutating step.
pub(crate) fn Fn(Content:&str) -> Option<Prepared> {
	let Parts = SplitMarkedOutput(Content);
	let Segments:Vec<_> = Parts.into_iter().map(|Part| PrepareSegment(Part.Index, Part.Text)).collect();

	Prepared::Assemble(Segments)
}
