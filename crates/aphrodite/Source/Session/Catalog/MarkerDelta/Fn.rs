//! The marker delta line.

use crate::State::Struct::AphroditeState;

/// The marker half of the catalog summary: what this turn added, or a stable
/// "no change" line.
///
/// Two rules in order. A turn that added markers names them (newest first, as
/// they are recorded); a turn that added none reports the running total without
/// re-listing anything, so the line is cache-stable and the model is not shown
/// the same previews twice. Nothing to report at all answers `None`.
///
/// `New` can never exceed `Current` (it is a saturating subtraction), so the
/// first rule's `Current > 0` is a guard against reporting a delta on an empty
/// ring rather than a second condition worth its own branch.
pub(crate) fn Fn(State:&AphroditeState, New:usize, Current:usize) -> Option<String> {
	let Delta = (New > 0 && Current > 0).then(|| {
		let Previews = State
			.RecentMarkers
			.iter()
			.rev()
			.take(New)
			.map(|Marker| Marker.Preview.clone())
			.collect::<Vec<String>>()
			.join(", ");
		let Plural = match New {
			1 => "",
			_ => "s",
		};

		format!("[Aphrodite] +{New} new compression{Plural} this turn: {Previews}. {Current} total.")
	});

	Delta.or_else(|| (Current > 0).then(|| format!("[Aphrodite] {Current} compressions (no change this turn).")))
}
