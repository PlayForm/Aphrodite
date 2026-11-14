//! Language auto-detection for the code structure extractor.

use super::floor_boundary;

/// Number of non-comment lines `auto_detect` scans before giving up.
const AUTO_DETECT_SCAN_LINES:usize = 60;

/// Is this line a comment/doc-comment line, for the purposes of skipping
/// leading file-header comment blocks in `auto_detect`?
fn is_comment_line(trimmed:&str) -> bool {
	trimmed.is_empty()
		|| trimmed.starts_with("//") // Rust/Go/JS/TS line comments (incl. `///`, `//!`)
		|| trimmed.starts_with('#') // Python/shell comments (and shebangs)
		|| trimmed.starts_with('*') // continuation line of a `/* ... */` block
		|| trimmed.starts_with("/*")
}

/// Auto-detect the source language from content, by scanning up to
/// [`AUTO_DETECT_SCAN_LINES`] non-comment lines rather than a fixed 500-BYTE
/// prefix of the raw content (T10/F8): a file that opens with a long `//!`
/// module doc comment (extremely common in this very codebase) could push
/// every real `fn`/`struct`/etc. keyword past that byte window, so the
/// detector saw only comment prose and returned "unknown" for a file that is
/// unambiguously Rust. Skipping comment lines while scanning fixes that
/// without needing a real tokenizer.
pub(crate) fn auto_detect(content:&str) -> String {
	let sample:String = content
		.lines()
		.filter(|l| !is_comment_line(l.trim()))
		.take(AUTO_DETECT_SCAN_LINES)
		.collect::<Vec<_>>()
		.join("\n");
	let head = sample.as_str();
	if head.contains("fn ") && head.contains("->") {
		"rust".into()
	} else if head.contains("def ") && head.contains(":") {
		"python".into()
	} else if head.contains("func ") && head.contains("{") {
		"go".into()
	} else if head.contains("function ") || head.contains("=>") || head.contains("interface ") {
		"js".into()
	} else if content.trim_start().starts_with("#!/")
		|| (head.len() > 200 && floor_boundary(head, 200).contains("echo "))
	{
		"sh".into()
	} else {
		String::new()
	}
}
