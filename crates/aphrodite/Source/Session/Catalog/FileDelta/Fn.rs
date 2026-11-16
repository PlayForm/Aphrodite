//! The referenced-file delta line.

use std::collections::VecDeque;

/// The file half of the catalog summary (04-F4): what this turn referenced, or a
/// stable "no change" line.
///
/// Takes the list it reads rather than the session, so this section cannot reach a
/// field it does not report on.
///
/// `RecordFile` pushes new entries to the FRONT of the deque, so the new ones sit
/// at the front; the front is taken and then reversed so names render in insertion
/// order (oldest-of-new first). Reading from the back would select the OLDEST
/// entries and misreport old files as new.
pub(crate) fn Fn(Files:&VecDeque<(String, String)>, New:usize, Current:usize) -> Option<String> {
	let Delta = (Current > 0 && New > 0).then(|| {
		// `take(New)` selects the newest `New` entries and `rev` renders them in
		// insertion order (oldest-of-new first) - one pass, no intermediate buffer
		// to reverse afterwards.
		let Names = Files
			.iter()
			.take(New)
			.map(|(Path, Tool)| format!("{Path} ({Tool})"))
			.rev()
			.collect::<Vec<String>>()
			.join(", ");
		let Plural = match New {
			1 => "",
			_ => "s",
		};

		format!("+{New} new file{Plural}: {Names}. {Current} total.")
	});

	Delta.or_else(|| (Current > 0).then(|| format!("{Current} referenced files (no change).")))
}
