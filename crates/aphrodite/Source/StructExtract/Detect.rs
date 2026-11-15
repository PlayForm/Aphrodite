//! Source-language detection from content alone.
//!
//! The caller does not always know what a stored blob is, so it passes an empty
//! language and this module decides. It is a keyword heuristic over a sample of
//! non-comment lines - not a tokenizer, not a file-extension lookup, and not an
//! authoritative answer: content that matches no family is reported as unknown, and
//! the preview then carries no structure rather than guessing one.

use crate::StructExtract::Format::Floor::Fn as FloorBoundary;

/// Number of non-comment lines `Fn` scans before giving up.
const AutoDetectScanLines:usize = 60;

/// Is this line a comment/doc-comment line, for the purposes of skipping leading
/// file-header comment blocks in `Fn`?
///
/// Covers the four comment styles the supported languages open files with: `//`
/// (Rust, Go, JS/TS, including `///` and `//!`), `#` (Python, shell, including
/// shebangs), and both the first and continuation lines of a `/* ... */` block. It
/// is a prefix test, not a parser, so a `#` that opens a string literal is skipped
/// as a comment.
fn IsCommentLine(Trimmed:&str) -> bool {
	Trimmed.is_empty()
		|| Trimmed.starts_with("//") // Rust/Go/JS/TS line comments (incl. `///`, `//!`)
		|| Trimmed.starts_with('#') // Python/shell comments (and shebangs)
		|| Trimmed.starts_with('*') // continuation line of a `/* ... */` block
		|| Trimmed.starts_with("/*")
}

/// The detected language tag, or the empty string when nothing matched.
///
/// Returns `"rust"`, `"python"`, `"go"`, `"js"`, `"sh"` or `""`. The sample is the
/// first [`AutoDetectScanLines`] non-comment lines rather than a fixed 500-byte
/// prefix of the raw content: a file that opens with a long `//!` module doc
/// comment - extremely common in this very codebase - pushes every real
/// `fn`/`struct` keyword past a byte window, so a prefix scan saw only comment
/// prose and reported "unknown" for a file that is unambiguously Rust. Skipping
/// comment lines fixes that without a real tokenizer.
///
/// It is not a confidence score: the first family whose keywords appear wins, so a
/// Rust file that merely mentions `function ` in a doc comment it could not skip is
/// reported as JS. `sh` is only claimed on a shebang or an `echo ` in the head, and
/// an empty return means the caller must fall back rather than render structure.
pub(crate) fn Fn(Content:&str) -> String {
	let Sample:String = Content
		.lines()
		.filter(|l| !IsCommentLine(l.trim()))
		.take(AutoDetectScanLines)
		.collect::<Vec<_>>()
		.join("\n");
	let Head = Sample.as_str();
	match () {
		_ if Head.contains("fn ") && Head.contains("->") => "rust".into(),
		_ if Head.contains("def ") && Head.contains(":") => "python".into(),
		_ if Head.contains("func ") && Head.contains("{") => "go".into(),
		_ if Head.contains("function ") || Head.contains("=>") || Head.contains("interface ") => "js".into(),
		_ if Content.trim_start().starts_with("#!/")
			|| (Head.len() > 200 && FloorBoundary(Head, 200).contains("echo ")) =>
		{
			"sh".into()
		},
		_ => String::new(),
	}
}
