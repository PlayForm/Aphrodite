//! Classify the marker-shaped region that begins at an opening delimiter.

use super::{
	super::{Hash, Prefix, Suffix},
	Enum::Enum,
};

/// Decide what the region starting at `Start` is.
///
/// `Start` must be the byte offset of an opening delimiter, which is what the
/// scanner's own search guarantees: this function never looks for the prefix itself, so
/// there is exactly one search per candidate and the two can never disagree about where
/// a marker begins.
///
/// The suffix search is bounded by the END OF THE DOCUMENT, not by the next prefix, so
/// a complete marker that merely contains the prefix inside its body is still one
/// marker - the grammar has no nesting, and the first closing delimiter ends it.
pub(crate) fn Fn(Content:&str, Start:usize) -> Enum<'_> {
	let Body = Start + Prefix::Const.len();

	// No closing delimiter anywhere after this point: unterminated.
	let Some(Offset) = Content[Body..].find(Suffix::Const) else {
		return Enum::Unterminated;
	};

	let End = Body + Offset + Suffix::Const.len();
	let Marker = &Content[Start..End];

	match Hash::Fn(Marker) {
		Some(Hash) => Enum::Reference { Marker, Hash, End },
		None => Enum::Hashless { End },
	}
}
