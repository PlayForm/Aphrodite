//! How a counter's number is read and written.

/// How a counter's number is read from the stats document and written out.
///
/// The variant is also the absence policy: `Shown` counters are part of the
/// exposition whether or not the document holds them (a missing one writes
/// `null`, as it always has), while the others contribute no line at all.
#[derive(Clone, Copy)]
pub enum Enum {
	/// Printed as the document holds it, `null` included.
	Shown,
	/// An unsigned integer.
	Count,
	/// A floating-point ratio, two decimals.
	Ratio,
	/// Microseconds rendered as seconds, six decimals.
	Seconds,
}
