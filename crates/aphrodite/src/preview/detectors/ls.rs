//! directory-listing shape detector.

use crate::preview::input::Input;
use crate::preview::line::path::is_path_line;

/// `ls -l` mode strings, or a majority of bare path-like tokens (find /
/// plain ls).
pub(crate) fn detect(inp: &Input<'_>) -> bool {
	let ls_long = inp
		.non_empty
		.iter()
		.filter(|l| {
			let b = l.as_bytes();
			b.len() >= 10
				&& matches!(b[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
				&& b[1..10]
					.iter()
					.all(|&c| matches!(c, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'))
		})
		.count();
	if ls_long >= 2 {
		return true;
	}
	let path_lines = inp.non_empty.iter().filter(|l| is_path_line(l)).count();
	path_lines >= 3 && path_lines * 2 >= inp.non_empty.len()
}
