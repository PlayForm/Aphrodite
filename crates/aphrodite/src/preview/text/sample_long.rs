//! Head+tail sampling for very long single-line payloads.

/// Render a content hint line: lines up to 100 chars are shown as-is
/// (60-char cap); very long lines (a 10 KB single-line payload) sample
/// head+tail (`head…tail`, 57 chars) so both ends are visible instead of a
/// bare 60-char head (ISSUE-11 residual #4, `long_single_line`/`repeated`).
pub(crate) fn sample_long_line(line:&str) -> String {
	let chars:Vec<char> = line.chars().collect();
	if chars.len() > 100 {
		let head:String = chars[..28].iter().collect();
		let tail:String = chars[chars.len() - 28..].iter().collect();
		format!("{head}…{tail}")
	} else {
		line.chars().take(60).collect()
	}
}