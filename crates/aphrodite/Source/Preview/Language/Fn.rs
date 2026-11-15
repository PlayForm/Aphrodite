//! Per-language code refinement.
//!
//! The shape detector can only say "this is code". Which language it is decides
//! which type string the marker carries, and that in turn decides whether the model
//! knows the payload is Rust or Python. The rules are re-expressed line-based from
//! the old proxy classifier so the proxy path and the hook/FFI path classify
//! identically. It is not a language detector in general - it only refines content
//! already classified as code.

/// Refine a `"code"` shape into a language-specific type, or `None`.
///
/// Rust needs a fn/impl/struct/enum signature AND one of `->`, `&` or `use`; Python
/// needs `def ` AND one of `import`, `class`, `from` or `self.`; Go needs
/// `func`/`package` AND `import (`; JS/TS needs `function`/`const`/`=>` AND
/// `import`/`export`. Returns `None` for content of three lines or fewer, because a
/// snippet that short carries no language evidence.
///
/// It is not a parser and it is not a tie-break: the first language whose rule
/// matches wins, and the order of the rules is the order of precedence.
pub(crate) fn Fn(Content:&str) -> Option<&'static str> {
	match () {
		_ if Content.lines().count() <= 3 => None,
		// Rust - require fn keyword PLUS one of arrow, borrow, or use to
		// distinguish from Python/JavaScript that happens to contain "fn ".
		_ if Content.lines().any(|l| {
			let t = l.trim_start();

			t.starts_with("fn ")
				|| t.starts_with("pub fn ")
				|| t.starts_with("async fn ")
				|| t.starts_with("pub async fn ")
				|| t.starts_with("impl ")
				|| t.starts_with("struct ")
				|| t.starts_with("pub struct ")
				|| t.starts_with("enum ")
				|| t.starts_with("pub enum ")
		}) && (Content.contains("-> ") || Content.contains('&') || Content.contains("use ")) =>
		{
			Some("code_rust")
		},
		// Python
		_ if Content.contains("def ")
			&& (Content.contains("import ")
				|| Content.contains("class ")
				|| Content.contains("from ")
				|| Content.contains("self.")) =>
		{
			Some("code_python")
		},
		// Go
		_ if (Content.contains("func ") || Content.contains("package ")) && Content.contains("import (") => {
			Some("code_go")
		},
		// JS/TS
		_ if (Content.contains("function ") || Content.contains("const ") || Content.contains("=> "))
			&& (Content.contains("import ") || Content.contains("export ")) =>
		{
			Some("code_js")
		},
		_ => None,
	}
}
