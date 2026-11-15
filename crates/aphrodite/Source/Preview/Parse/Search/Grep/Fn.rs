//! Recognise one grep/ripgrep hit.

use crate::Preview::{Line::Grep::Hit, Parse::Search::Hit::Struct::Struct as Record};

/// Parse a `path:line:match` line that carries a file-ish path.
///
/// The acceptance rule is the shared grep predicate, which requires the path to contain
/// a `/` or a `.` - that requirement is what separates this producer from the plain one.
/// It is not a match extractor: only the first two fields are kept, so the match text
/// costs nothing.
pub(crate) fn Fn(Line:&str) -> Option<Record> {
	Hit::Fn(Line).then(|| {
		let mut It = Line.splitn(3, ':');

		Record {
			Path:It.next().unwrap_or("").to_string(),
			Line:It.next().unwrap_or("").to_string(),
		}
	})
}
