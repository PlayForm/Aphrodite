//! The markdown outline.

use crate::Preview::Parse::Markdown::Heading::Struct::Struct as Heading;

/// Deepest heading level a document may use.
const LevelMaximum:usize = 6;

/// A parsed document: a count per heading level, and the first heading.
///
/// The counts are a fixed-size array indexed by level, so a document with ten thousand
/// headings holds six integers rather than ten thousand levels - the previous reader
/// kept one entry per heading and only ever counted them. It is not an outline: the
/// order of headings is not retained, because the preview reports counts.
pub(crate) struct Struct {
	Counts:[usize; LevelMaximum + 1],
	First:Option<String>,
}

impl Struct {
	pub(crate) fn new() -> Self { Self { Counts:[0; LevelMaximum + 1], First:None } }

	/// Account for one heading.
	///
	/// The level is bounded by the predicate that admitted the line, so the index is
	/// always in range. It is not a guard: an out-of-range level is impossible by
	/// construction, and adding a check would be dead code.
	pub(crate) fn Push(&mut self, Heading:Heading) {
		self.Counts[Heading.Level] += 1;

		// The first heading wins; a later one is dropped by the same slot.
		self.First.get_or_insert(Heading.Text);
	}

	/// The levels that occurred, in level order, as `(level, count)` pairs.
	pub(crate) fn Tally(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
		self.Counts
			.iter()
			.enumerate()
			.skip(1)
			.filter_map(|(Level, Count)| (*Count > 0).then_some((Level, *Count)))
	}

	pub(crate) fn First(&self) -> Option<&str> { self.First.as_deref() }
}
