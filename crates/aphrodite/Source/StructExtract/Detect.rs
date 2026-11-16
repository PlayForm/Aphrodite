//! Source-language detection from content alone.
//!
//! The caller does not always know what a stored blob is, so it passes an empty language and this
//! module decides. It is a keyword heuristic over a sample of non-comment lines - not a tokenizer, not
//! a file-extension lookup, and not an authoritative answer: content that matches no family is reported
//! as unknown, and the preview then carries no structure rather than guessing one.

use crate::StructExtract::Format::Floor::Fn as FloorBoundary;

/// Number of non-comment lines this scan reads before giving up: sixty.
const Lines:usize = 60;

/// Bytes of the sample the `sh` rule inspects for an `echo `: two hundred.
const Window:usize = 200;

/// Is this line a comment/doc-comment line, for the purposes of skipping leading file-header comment
/// blocks?
///
/// Covers the four comment styles the supported languages open files with: `//` (Rust, Go, JS/TS,
/// including `///` and `//!`), `#` (Python, shell, including shebangs), and both the first and
/// continuation lines of a `/* ... */` block. It is a prefix test, not a parser, so a `#` that opens a
/// string literal is skipped as a comment.
fn Comment(Trimmed:&str) -> bool {
	Trimmed.is_empty()
		|| Trimmed.starts_with("//")
		|| Trimmed.starts_with('#')
		|| Trimmed.starts_with('*')
		|| Trimmed.starts_with("/*")
}

/// The detected language tag, or the empty string when nothing matched.
///
/// Returns `"rust"`, `"python"`, `"go"`, `"js"`, `"sh"` or `""`. The sample is the first
/// [`Lines`] non-comment lines rather than a fixed byte prefix of the raw content: a file
/// that opens with a long `//!` module doc comment pushes every real `fn`/`struct` keyword past a byte
/// window, so a prefix scan saw only comment prose and reported "unknown" for a file that is
/// unambiguously Rust. Skipping comment lines fixes that without a real tokenizer.
///
/// It is not a confidence score: the first family whose keywords appear wins, so a Rust file that
/// merely mentions `function ` in a doc comment it could not skip is reported as JS. `sh` is only
/// claimed on a shebang or an `echo ` in the head, and an empty return means the caller must fall back
/// rather than render structure.
pub(crate) fn Fn(Content:&str) -> String {
	let Sample:String = Content
		.lines()
		.filter(|Line| !Comment(Line.trim()))
		.take(Lines)
		.collect::<Vec<_>>()
		.join("\n");
	let Head = Sample.as_str();
	let Shell = Content.trim_start().starts_with("#!/")
		|| (Head.len() > Window && FloorBoundary(Head, Window).contains("echo "));

	(Head.contains("fn ") && Head.contains("->"))
		.then_some("rust")
		.or_else(|| (Head.contains("def ") && Head.contains(":")).then_some("python"))
		.or_else(|| (Head.contains("func ") && Head.contains("{")).then_some("go"))
		.or_else(|| (Head.contains("function ") || Head.contains("=>") || Head.contains("interface ")).then_some("js"))
		.or_else(|| Shell.then_some("sh"))
		.map(str::to_string)
		.unwrap_or_default()
}
