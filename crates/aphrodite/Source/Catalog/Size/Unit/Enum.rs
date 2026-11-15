//! Which binary unit a byte count is rendered in.

/// A byte count, classified by magnitude.
///
/// The classification used to be a match with a dummy scrutinee and two range guards,
/// so the value being measured was never bound and the arms were predicates wearing a
/// match's clothes. Each variant now carries the count it renders, so the renderer
/// reads the unit and the number from one place and cannot pair the wrong count with
/// the wrong unit.
pub(crate) enum Enum {
	/// Below one kilobyte.
	Bytes(usize),
	/// At least one kilobyte, below one megabyte.
	Kilobytes(usize),
	/// At least one megabyte.
	Megabytes(usize),
}
