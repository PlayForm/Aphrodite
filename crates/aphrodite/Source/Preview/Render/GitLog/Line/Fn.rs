//! Render one log endpoint.

use crate::Preview::Parse::GitLog::Record::Struct::Struct as Record;

/// Characters of a hash a preview shows: seven, the short form git itself prints.
const HashShown:usize = 7;

/// Characters of a subject a preview shows: thirty-two.
const SubjectShown:usize = 32;

/// Render a record as `<short hash> <subject>`.
///
/// Both cuts are display decisions and live here rather than in the parser, so the
/// summary can hold the whole subject and a later renderer can choose a different width
/// without re-parsing the log. It is not a formatter of the message: the subject is
/// printed as it was read, only shortened.
pub(crate) fn Fn(Record:&Record) -> String {
	format!(
		"{} {}",
		Record.Hash.chars().take(HashShown).collect::<String>(),
		Record.Subject.chars().take(SubjectShown).collect::<String>()
	)
}
