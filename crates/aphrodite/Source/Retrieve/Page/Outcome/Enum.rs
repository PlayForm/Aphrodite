//! What slicing a document produced.

/// The result of slicing a document to a [`super::super::Mode::Enum`].
///
/// The display annotation is NOT here. `[lines a-b/total]` is a rendering of the
/// range, so it lives in [`super::super::Render`]; the exact-content
/// representation stays the content, which is what makes a whole-document
/// retrieval byte-identical to what was stored. `Whole` carries no payload at
/// all: the caller already holds the document, so the honest representation of
/// "nothing was cut" is a variant, not a copy of the input.
pub enum Enum {
	/// Nothing was cut: serve the document itself.
	Whole,
	/// A window of the document, with its 1-based inclusive display range.
	Window { Text:String, Start:usize, End:usize, Total:usize },
}
