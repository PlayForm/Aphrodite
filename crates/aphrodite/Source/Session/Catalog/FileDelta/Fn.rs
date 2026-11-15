//! The referenced-file delta line.

use crate::State::Struct::AphroditeState;

/// The file half of the catalog summary (04-F4): what this turn referenced, or a
/// stable "no change" line.
///
/// `RecordFile` pushes new entries to the FRONT of the deque, so the new ones sit
/// at the front; the front is taken and then reversed so names render in insertion
/// order (oldest-of-new first). Reading from the back would select the OLDEST
/// entries and misreport old files as new.
pub(crate) fn Fn(State:&AphroditeState, New:usize, Current:usize) -> Option<String> {
	let Delta = (Current > 0 && New > 0).then(|| {
		let Names:Vec<String> = State
			.ReferencedFiles
			.iter()
			.take(New)
			.map(|(Path, Tool)| format!("{Path} ({Tool})"))
			.collect();
		let Names = Names.into_iter().rev().collect::<Vec<String>>().join(", ");
		let Plural = match New {
			1 => "",
			_ => "s",
		};

		format!("+{New} new file{Plural}: {Names}. {Current} total.")
	});

	Delta.or_else(|| (Current > 0).then(|| format!("{Current} referenced files (no change).")))
}
