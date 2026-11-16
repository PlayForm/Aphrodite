//! One candidate section of the per-turn context.

use super::Protection::Enum::Enum as Protection;

/// One candidate section: the text the model would read, and whether the budget
/// may drop it.
///
/// The two facts belong in one value because the drop rule is a property of the
/// section, not of its position: the budget drops `Optional` sections from the
/// bottom and never crosses a `Required` one, so a section cannot be protected
/// in one place and ordered as droppable in another.
pub(crate) struct Struct {
	/// The exact text this section contributes.
	pub(crate) Text:String,
	/// Whether the budget may drop it.
	pub(crate) Protection:Protection,
}

impl Struct {
	/// A section the budget may never drop.
	pub(crate) fn Required(Text:String) -> Struct { Struct { Text, Protection:Protection::Required } }

	/// A section the budget drops before any required one.
	pub(crate) fn Optional(Text:String) -> Struct { Struct { Text, Protection:Protection::Optional } }
}
