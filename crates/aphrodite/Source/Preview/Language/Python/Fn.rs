//! Python evidence.

/// True when the content carries `def` AND one Python-shaped detail.
///
/// One of `import `, `class `, `from ` or `self.` has to appear as well. It is not a parser:
/// a file that merely mentions `def ` in a doc comment can satisfy it, which is why the shape
/// detector runs first and this only refines a verdict.
pub(crate) fn Fn(Content:&str) -> bool {
	Content.contains("def ")
		&& (Content.contains("import ")
			|| Content.contains("class ")
			|| Content.contains("from ")
			|| Content.contains("self."))
}
