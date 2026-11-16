//! The output under construction: the bytes written and the cursor into the source.

/// The bytes written so far and the position in the source they were read from.
///
/// The two travel together - every step reads from `Cursor` and appends to `Out` -
/// so one value carries both rather than a tuple passed through a fold.
pub struct Struct {
	/// The expanded document, appended to in document order.
	pub Out:String,
	/// How much of the source has been consumed.
	pub Cursor:usize,
}
