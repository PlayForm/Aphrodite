//! One marker found in a document, tied to the document it was found in.

use std::ops::Range;

/// One occurrence of the marker grammar in a document.
///
/// The scanner used to return `(String, String)` - the marker text and the hash, both
/// owned, with nothing saying where in the document they came from. The resolver then
/// substituted them with `String::replace` over the whole document, which re-reads text
/// it had already written: a marker left in place at the depth cap IS raw marker text,
/// so a later replacement of that same marker string substituted inside the text that
/// had just been inserted.
///
/// The span is what makes an occurrence a POSITION in its document rather than a string
/// to search for again, and the borrows are what tie it to that document: an occurrence
/// cannot outlive the content it describes, and cannot be carried into another one.
pub(crate) struct Struct<'a> {
	/// The byte range of the whole marker within its document.
	pub(crate) Span:Range<usize>,
	/// The marker text, exactly as it appears in the document.
	pub(crate) Marker:&'a str,
	/// The hash the marker names: its first `|`-separated field.
	pub(crate) Hash:&'a str,
}
