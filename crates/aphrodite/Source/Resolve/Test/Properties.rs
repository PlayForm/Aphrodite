//! The scanner's property test.
//!
//! One property over arbitrary UTF-8: the scanner never panics, and every occurrence it
//! reports round-trips back to the same hash it was extracted with, at the span it claims.
//! That pair is what makes the scanner safe to run over model output nobody has vetted - and
//! what makes substituting by span equivalent to substituting the marker text.

// ── T8: property tests ───────────────────────────────────────
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::Resolve::Parse::{Find::Fn as FindMarkers, Hash::Fn as ParseMarkerHash};

proptest! {
	/// `FindMarkers` must never panic on arbitrary input, and every occurrence it returns must
	/// round-trip through `ParseMarkerHash` back to the same hash it was extracted with, with a
	/// span that still holds that exact marker text.
	#[test]
	fn PropFindMarkersNeverPanicsAndRoundtrips(s in ".*") {
		let Markers = FindMarkers(&s);

		Markers
			.into_iter()
			.try_for_each(|Occurrence| -> Result<(), proptest::test_runner::TestCaseError> {
				prop_assert_eq!(ParseMarkerHash(Occurrence.Marker), Some(Occurrence.Hash));
				prop_assert_eq!(&s[Occurrence.Span.clone()], Occurrence.Marker);
				prop_assert!(Occurrence.Span.end <= s.len());

				Ok(())
			})?;
	}
}
