//! What the preview and the type are computed from.

/// The body a preview is built from, and the CCR type it is filed under.
///
/// The two travel together because they are decided together and neither is
/// derivable from the other afterwards: an unwrapped fragment must never be
/// previewed under a caller's `json` hint, and a JSON payload must never be
/// previewed from its fragment. `Body` is NOT the stored content - the caller
/// still hashes and stores the ORIGINAL bytes.
pub(crate) struct Struct {
	/// The text the preview renders: the payload itself, or its unwrapped fragment.
	pub(crate) Body:String,
	/// The CCR type: the caller's hint when it gave one, else the detection.
	pub(crate) Kind:String,
}
