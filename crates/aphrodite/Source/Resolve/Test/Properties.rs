//! The scanner's property test.
//!
//! One property over arbitrary UTF-8: the scanner never panics, and every marker
//! it reports round-trips back to the same hash it was extracted with. That pair
//! is what makes the scanner safe to run over model output nobody has vetted.

// ── T8: property tests ───────────────────────────────────────
use proptest::{prop_assert_eq, proptest};

use crate::Resolve::Parse::{Find::Fn as FindMarkers, Hash::Fn as ParseMarkerHash};

proptest! {
	/// `FindMarkers` must never panic on arbitrary input, and every
	/// marker it returns must round-trip through `ParseMarkerHash`
	/// back to the same hash it was extracted with.
	#[test]
	fn PropFindMarkersNeverPanicsAndRoundtrips(s in ".*") {
		let Markers = FindMarkers(&s);
		Markers
			.into_iter()
			.try_for_each(|(Marker, hash)| -> Result<(), proptest::test_runner::TestCaseError> {
				let Parsed = ParseMarkerHash(&Marker);
				prop_assert_eq!(Parsed, Some(hash));
				Ok(())
			})?;
	}
}
