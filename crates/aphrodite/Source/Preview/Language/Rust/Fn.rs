//! Rust evidence.

/// True when the content carries a Rust declaration AND a Rust-shaped detail.
///
/// A declaration keyword alone is not enough - Python and JavaScript both contain `fn` in
/// prose - so one of `->`, `&` or `use ` has to appear as well. It is not a parser: the test
/// is over the whole content, not per line.
pub(crate) fn Fn(Content:&str) -> bool {
	let Declares = Content.lines().any(|Line| {
		let Trimmed = Line.trim_start();

		Trimmed.starts_with("fn ")
			|| Trimmed.starts_with("pub fn ")
			|| Trimmed.starts_with("async fn ")
			|| Trimmed.starts_with("pub async fn ")
			|| Trimmed.starts_with("impl ")
			|| Trimmed.starts_with("struct ")
			|| Trimmed.starts_with("pub struct ")
			|| Trimmed.starts_with("enum ")
			|| Trimmed.starts_with("pub enum ")
	});

	Declares && (Content.contains("-> ") || Content.contains('&') || Content.contains("use "))
}
