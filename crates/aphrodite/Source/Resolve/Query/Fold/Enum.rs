//! How a filter compares a line against a query.

/// Which case fold a query filter applies.
///
/// Two callers narrow content by a query and they have always differed, so the difference
/// is a value rather than two copies of the same loop. `ASCII` is the retrieve endpoint's
/// fold - it lowercases only ASCII, which leaves a multi-byte line's bytes untouched and is
/// what the endpoint's 512-character query cap was written against. `Unicode` is the
/// resolver's fold, which lowercases the whole line.
///
/// Making this explicit is what lets both callers share one line-matching primitive: a
/// helper that silently picked either fold would change one of the two filters' output on
/// non-ASCII content.
pub enum Enum {
	/// Lowercase ASCII letters only.
	ASCII,
	/// Lowercase the whole string.
	Unicode,
}
