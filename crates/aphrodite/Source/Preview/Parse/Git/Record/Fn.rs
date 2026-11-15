//! Recognise one `git status` line.

use crate::Preview::{Line::GitStatus, Parse::Git::Record::Struct::Struct as Record};

/// Reduce a status line to its code and path, or `None` when it is not one.
///
/// The two columns collapse to the most significant character (the first non-space
/// one, `?` for a line of spaces), and a `R old -> new` rename keeps the new path -
/// which is the side the model needs to open. It is not a diff: the content of a
/// change is never read, so an empty path stays empty rather than being invented.
pub(crate) fn Fn(Line:&str) -> Option<Record> {
	let Code = GitStatus::Fn(Line)?;
	let Character = Code.chars().find(|Character| *Character != ' ').unwrap_or('?');
	let Path = Line.get(3..).unwrap_or("").trim();

	Some(Record { Code:Character, Path:Path.rsplit(" -> ").next().unwrap_or(Path).to_string() })
}
