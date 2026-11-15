//! Cut a document down to the requested mode.

use std::num::NonZeroUsize;

use super::super::{Mode, Outcome};

/// Cut `Content` down to `Mode`, or refuse the request.
///
/// `Exact` never touches the document: it answers `Whole`, which the renderer
/// turns back into the original bytes, so a whole-document retrieval is
/// byte-identical to what was stored and hashes back to the marker's own hash.
/// That is also why `Exact` does not collect lines - the round trip is the
/// contract, and the cheapest way to keep it is not to rebuild the string.
///
/// `Window` slices on `\n` and always renders a range, so an
/// offset-with-unlimited-tail request is still annotated. Two pre-existing
/// decisions are preserved verbatim because each has a recorded failure behind
/// it:
///
/// - a zero-line document is a VALID stored entry (`POST /ccr/create` with
///   `content: ""`), so it answers `Whole` instead of failing the offset check -
///   `offset (0) >= total (0)` used to turn a legitimate create("")/retrieve
///   round trip into a 400 (F20);
/// - a window that reaches the end of a document that ended in `\n` keeps that
///   newline, because `str::lines()` discards it and `join` never restores it
///   (02-F4).
///
/// The refusal is the same 400 body as before: the message states the offset and
/// the document's line count, because the caller renders it straight into the
/// response and those are the two numbers a reader needs.
pub fn Fn(Content:&str, Mode:&Mode::Enum::Enum) -> anyhow::Result<Outcome::Enum::Enum> {
	// `match *Mode`: every bound field is `Copy`, so nothing is moved out of the
	// reference and the two-armed decision stays a decomposition of the value.
	let (Offset, Limit) = match *Mode {
		Mode::Enum::Enum::Exact => return Ok(Outcome::Enum::Enum::Whole),
		Mode::Enum::Enum::Window { Offset, Limit } => (Offset, Limit.map(NonZeroUsize::get)),
	};

	let Lines:Vec<&str> = Content.lines().collect();
	let Some(Total) = NonZeroUsize::new(Lines.len()) else {
		return Ok(Outcome::Enum::Enum::Whole);
	};
	let Total = Total.get();

	anyhow::ensure!(Offset < Total, "[offset {} out of range; document has {} lines]", Offset, Total);

	// A `Limit` of `None` is the unlimited tail, so the window ends at the last
	// line. `saturating_add` keeps an offset plus a huge limit from overflowing.
	let End = Limit.map_or(Total, |Limit| Offset.saturating_add(Limit).min(Total));
	let ReachesEnd = End == Total;

	match (Offset, ReachesEnd) {
		(0, true) => Ok(Outcome::Enum::Enum::Whole),
		(Start, ReachesEnd) => {
			let mut Text = Lines[Start..End].join("\n");
			// 02-F4: a partial window that still reaches the document's end
			// keeps the trailing newline the original had.
			let _ = (ReachesEnd && Content.ends_with('\n')).then(|| Text.push('\n'));

			Ok(Outcome::Enum::Enum::Window { Text, Start:Start + 1, End, Total })
		},
	}
}
