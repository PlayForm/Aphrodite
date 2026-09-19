//! grep/ripgrep shape detector.

use crate::preview::input::Input;
use crate::preview::line::grep::is_grep_line;

/// `path:line:match` or `path:match` majority.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let grep_lines = inp.non_empty.iter().filter(|l| is_grep_line(l)).count();
	grep_lines >= 2 && grep_lines * 2 >= inp.non_empty.len()
}