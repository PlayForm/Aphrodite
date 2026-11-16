//! One owned part of a marked tool output.

/// One part of a marked output: the chain position it belongs to, and the text
/// it carried, owned.
///
/// A part is what the parser can promise and nothing more: the position in the
/// chain and the exact bytes (already normalized by the parser). It is a value
/// so the preparation stage can CONSUME it - the text moves into the prepared
/// segment instead of being copied out of a borrowed pair, which is why the
/// parser returns owned parts rather than a `Vec<(usize, String)>` that every
/// caller would have to destructure the same way.
pub struct Struct {
	/// Position of the part in the chain, counting from zero.
	pub Index:usize,
	/// The part's text, trimmed and normalized by the parser.
	pub Text:String,
}
