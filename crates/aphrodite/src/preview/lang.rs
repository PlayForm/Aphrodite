//! Per-language code refinement (1.5.0 REFACTOR-PLAN §7, Phase 6).
//!
//! Consulted when the shape detector says `"code"`: refines the generic code
//! arm into `code_rust` / `code_python` / `code_go` / `code_js`. Rules
//! re-expressed line-based from the old proxy classifier
//! (`proxy.rs:1353-1399`) so the proxy path and the hook/FFI path classify
//! identically.

/// Refine a `"code"` shape into a language-specific type, or `None` to keep
/// the generic `"code"` arm. Mirrors the old proxy heuristics exactly:
/// Rust needs a fn/impl/struct/enum signature AND one of `->`/`&`/`use`;
/// Python needs `def ` AND import/class/from/self.; Go needs func/package
/// AND `import (`; JS/TS needs function/const/=> AND import/export.
pub(crate) fn detect_language(content: &str) -> Option<&'static str> {
	if content.lines().count() <= 3 {
		return None;
	}
	// Rust - require fn keyword PLUS one of arrow, borrow, or use to
	// distinguish from Python/JavaScript that happens to contain "fn ".
	if content.lines().any(|l| {
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
	}) && (content.contains("-> ") || content.contains('&') || content.contains("use "))
	{
		return Some("code_rust");
	}
	// Python
	if content.contains("def ")
		&& (content.contains("import ")
			|| content.contains("class ")
			|| content.contains("from ")
			|| content.contains("self."))
	{
		return Some("code_python");
	}
	// Go
	if (content.contains("func ") || content.contains("package ")) && content.contains("import (") {
		return Some("code_go");
	}
	// JS/TS
	if (content.contains("function ") || content.contains("const ") || content.contains("=> "))
		&& (content.contains("import ") || content.contains("export "))
	{
		return Some("code_js");
	}
	None
}
