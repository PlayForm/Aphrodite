//! What survived budget enforcement.

use crate::Flow::Section::Struct::Struct as Section;

/// The sections a budget pass kept, and the exact size they produce.
///
/// The sections and their measurement travel together because the caller needs
/// both: the joined string is built from the sections, the overflow report and
/// the debug line need the byte count, and recomputing either from the other
/// would be a second answer to a question already settled.
pub(crate) struct Struct {
	/// The surviving sections, in emission order.
	pub(crate) Sections:Vec<Section>,
	/// Bytes of the string `Join` produces from them.
	pub(crate) Bytes:usize,
	/// How many of them are protected - the floor the pass could not cross.
	pub(crate) Floor:usize,
}
