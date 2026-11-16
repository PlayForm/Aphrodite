//! How much of the sample set the smoke test runs.

use crate::Tool::Registry::Test::Const;

/// The breadth of an `aphrodite_test` call.
///
/// Two modes rather than a free-form string because the mode decides a SLICE of
/// one list, and a caller cannot ask for "quick but the third sample".
pub(crate) enum Enum {
	/// The first sample only: the cheapest proof the store round-trips.
	Quick,
	/// Every sample: one per content family the compressor special-cases.
	Full,
}

impl Enum {
	/// The samples this mode runs, in report order.
	pub(crate) fn Samples(&self) -> &'static [(&'static str, &'static str)] {
		match self {
			Enum::Quick => Const::SAMPLES.get(..1).unwrap_or(Const::SAMPLES),
			Enum::Full => Const::SAMPLES,
		}
	}
}
