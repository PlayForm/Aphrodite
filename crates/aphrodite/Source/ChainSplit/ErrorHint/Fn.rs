//! The per-segment error hint.

use std::borrow::Cow;

use super::{Signals::Const::Vocabulary, Truncate::Fn::Fn as Truncate};

/// Extract a compact error hint from a segment's output, when it carries an
/// error signal (exit code, error:/Error:, FAILED/failed, panic, Traceback,
/// etc.). Returns the first matching line, capped at 60 characters.
///
/// Borrowed-or-owned: most hint lines already fit the cap, so the common case
/// hands back the segment's own slice and allocates nothing. A caller that only
/// prints the hint never takes ownership at all, which is the point - a
/// `String` here would allocate on every failed segment of every chained
/// command.
///
/// Tier 3: per-segment error hints. This is CONTENT-level vocabulary (what the
/// segment itself says), never mechanism vocabulary - the invisibility
/// contract limits hints to the segment's own output, so the LLM learns
/// "that segment failed" from the hint without any chain/split wording.
pub fn Fn(Content:&str) -> Option<Cow<'_, str>> {
	let Candidates:Vec<&str> = Content
		.lines()
		.filter_map(|Line| {
			let Line = Line.trim();
			let Signal =
				Line.contains("exit code:") || Vocabulary.iter().any(|Word| Line.to_lowercase().contains(Word));

			(!Line.is_empty() && Signal).then_some(Line)
		})
		.collect();

	// `error[EXXXX]: msg` is the most informative shape - it wins over the
	// first plain signal.
	let Hint = Candidates
		.iter()
		.find(|Line| Line.contains("error["))
		.copied()
		.or_else(|| Candidates.first().copied())?;

	Some(Truncate(Hint))
}
