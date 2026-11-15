//! The threshold movement, as a value.

/// Which way one adaptation step moves the minimum-segment threshold.
///
/// Three variants, not two booleans and not a signed delta: the dead band
/// between the two ratio thresholds is a real outcome (hold), and it is the
/// reason a single noisy window cannot oscillate the threshold every turn.
/// A `isize` delta would make "hold" and "the bounds clamped me" the same `0`,
/// which is the distinction an investigation needs.
pub(crate) enum Enum {
	/// Segments are used - split more eagerly.
	Decrease,
	/// The ratio sits in the dead band, or a bound blocks the move.
	Hold,
	/// Segments are ignored - only split very long chains.
	Increase,
}
