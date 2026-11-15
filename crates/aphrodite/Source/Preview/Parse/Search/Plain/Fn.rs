//! Recognise one `path:line:` search result.

use crate::Preview::{Line::Grep::Search, Parse::Search::Hit::Struct::Struct as Record};

/// Parse a result line without requiring the path to look file-ish.
///
/// The acceptance rule is the search predicate, which additionally rejects a path that
/// starts with a digit and carries no `/` or `.` - a timestamp, not a path. It is not
/// the grep parser: a query result whose path has no extension is a hit here and is not
/// one there, and that is the whole difference between the two arms.
pub(crate) fn Fn(Line:&str) -> Option<Record> {
	(!Line.trim().is_empty() && Search::Fn(Line)).then(|| {
		let (Path, Rest) = Line.split_once(':').unwrap_or((Line, ""));

		Record { Path:Path.to_string(), Line:Rest.split(':').next().unwrap_or("").to_string() }
	})
}
