//! Head+tail sampling for very long single-line payloads.
//!
//! A tool result can be one 10 KB line (a minified JSON blob, a single-line
//! log). A bare head truncation of such a line hides the end, and the end is
//! often the part that says what happened. This module samples both ends. It
//! is not a line splitter and it never invents an ellipsis position: the
//! budget is fixed, so two callers sampling the same line get the same string.

/// Render a content hint line, sampling head+tail when the line is very long.
///
/// Lines up to 100 characters are shown as-is under a 60-character cap. A
/// longer line keeps its first and last 28 characters joined by `...`, so both
/// ends are visible instead of a bare 60-character head. The result is what
/// the model sees, so a change here changes the contract - the numbers are not
/// tunable cosmetics.
pub(crate) fn Fn(Line:&str) -> String {
	let Chars:Vec<char> = Line.chars().collect();

	match Chars.len() > 100 {
		true => {
			let Head:String = Chars[..28].iter().collect();
			let Tail:String = Chars[Chars.len() - 28..].iter().collect();

			format!("{Head}...{Tail}")
		},
		false => Line.chars().take(60).collect(),
	}
}
