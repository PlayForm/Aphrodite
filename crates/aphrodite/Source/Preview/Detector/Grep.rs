//! grep/ripgrep shape detector.
//!
//! A grep result is a majority of `path:line:match` lines. This detector votes on
//! that majority. It is not a match reader: the text of a match is never parsed.

use crate::Preview::{Input::Struct::Struct as Input, Line::Grep::Hit};

/// True when the content looks like grep/ripgrep output.
///
/// Two or more hit lines, and hits making up at least half the non-empty lines.
/// It is not the search detector: a hit here needs a file-ish path, so a result
/// whose paths have no extension falls through to the search arm instead.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let GrepLines = Inp.NonEmpty.iter().filter(|l| Hit::Fn(l)).count();
	GrepLines >= 2 && GrepLines * 2 >= Inp.NonEmpty.len()
}
