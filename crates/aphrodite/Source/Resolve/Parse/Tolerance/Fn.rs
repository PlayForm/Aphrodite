//! Classify the marker-shaped region that begins at an opening delimiter.

use super::{
	super::{Hash, Prefix, Suffix},
	Enum::Enum,
};

/// Decide what the region starting at `AbsStart` is.
///
/// `AbsStart` must be the byte offset of an opening delimiter, which is what the
/// scanner's own search guarantees: this function never looks for the prefix itself, so
/// there is exactly one search per candidate and the two can never disagree about where
/// a marker begins.
///
/// The suffix search is bounded by the END OF THE DOCUMENT, not by the next prefix, so
/// a complete marker that merely contains the prefix inside its body is still one
/// marker - the grammar has no nesting, and the first closing delimiter ends it.
pub(crate) fn Fn(Content:&str, AbsStart:usize) -> Enum<'_> {
	let AfterPrefix = AbsStart + Prefix::Const.len();

	// No closing delimiter anywhere after this point: unterminated.
	let Some(End) = Content[AfterPrefix..].find(Suffix::Const) else {
		return Enum::Unterminated;
	};

	let AbsEnd = AfterPrefix + End + Suffix::Const.len();
	let Marker = &Content[AbsStart..AbsEnd];

	match Hash::Fn(Marker) {
		Some(Hash) => Enum::Reference { Marker, Hash, End:AbsEnd },
		None => Enum::Hashless { End:AbsEnd },
	}
}
