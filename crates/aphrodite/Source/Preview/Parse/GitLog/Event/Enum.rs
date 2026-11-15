//! One classified log line.

/// A log line, classified by the only distinction the record machine needs.
///
/// `Commit` opens a record; `Line` is every other line, already trimmed so the state
/// machine never has to trim twice. The variant carries a borrow, not a copy, so
/// classifying a log allocates nothing. It is not a parser of the commit body: the
/// author, date and merge headers stay ordinary lines and the machine decides which of
/// them to skip.
pub(crate) enum Enum<'a> {
	/// A `commit <hash>` header - the boundary between two records.
	Commit { Hash:&'a str },
	/// Any other line, trimmed.
	Line { Trimmed:&'a str },
}
