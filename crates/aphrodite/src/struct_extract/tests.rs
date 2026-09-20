use super::*;

#[test]
fn test_auto_detect_rust() {
	assert_eq!(auto_detect("fn main() -> i32 {}\npub struct Foo {}"), "rust");
}

#[test]
fn test_auto_detect_python() {
	assert_eq!(auto_detect("def hello():\n    pass\n"), "python");
}

#[test]
fn test_auto_detect_go() {
	assert_eq!(auto_detect("func main() {\n}\n"), "go");
}

#[test]
fn test_auto_detect_js() {
	assert_eq!(auto_detect("function hello() {\n}\n"), "js");
}

#[test]
fn test_auto_detect_unknown() {
	assert_eq!(auto_detect("plain text no code"), "");
}

#[test]
fn test_extract_rust_fns() {
	let code = "pub fn main() -> i32 {\n    42\n}\nfn helper(x: i32) -> bool {\n    true\n}\n";
	let r = extract_code_structure(code, "rust");
	assert!(r.contains_key("fns"));
	let fns = &r["fns"];
	assert!(fns.iter().any(|s| s.contains("main")));
	assert!(fns.iter().any(|s| s.contains("helper")));
}

#[test]
fn test_extract_rust_structs() {
	let code = "pub struct Foo {\n    x: i32,\n}\nstruct Bar<T> {}\n";
	let r = extract_code_structure(code, "rust");
	assert!(r.contains_key("structs"));
}

#[test]
fn test_extract_python() {
	let code = "def hello(name: str) -> str:\n    return name\n\nclass MyClass:\n    pass\n";
	let r = extract_code_structure(code, "python");
	assert!(r.contains_key("fns"));
	assert!(r.contains_key("classes"));
}

#[test]
fn test_extract_go() {
	let code = "func main() {\n}\n\nfunc (s *Server) Start(addr string) error {\n}\n";
	let r = extract_code_structure(code, "go");
	assert!(r.contains_key("fns"));
}

// ── T10 (F8): per-language extractor correctness fixes ────────

/// A method's receiver paren `(s *Server)` must not be mistaken for the
/// method's actual parameter list - `params_start` used to find the
/// FIRST `(` on the line (the receiver's), reporting
/// `func Start(s *Server)` instead of `func Start(addr string)`.
#[test]
fn test_extract_go_method_reports_real_params_not_receiver() {
	let code = "func (s *Server) Start(addr string) error {\n}\n";
	let r = extract_code_structure(code, "go");
	let fns = &r["fns"];
	let sig = fns
		.iter()
		.find(|s| s.contains("Start"))
		.expect("Start method should be extracted");
	assert!(sig.contains("addr"), "signature should show the real params: {sig}");
	assert!(
		!sig.contains("*Server"),
		"signature should NOT show the receiver as if it were a param: {sig}"
	);
}

/// `const x = 5; // map => y` must not be recorded as an arrow function -
/// the `=>` is inside a trailing comment, not part of the binding's RHS.
#[test]
fn test_extract_js_arrow_fn_guard_ignores_comment_only_arrow() {
	let code = "const x = 5; // map => y\n";
	let r = extract_code_structure(code, "js");
	if let Some(fns) = r.get("fns") {
		assert!(
			!fns.iter().any(|s| s.contains('x')),
			"trailing-comment `=>` must not be mistaken for an arrow fn: {fns:?}"
		);
	}
}

/// A real arrow function assignment must still be detected.
#[test]
fn test_extract_js_arrow_fn_still_detected() {
	let code = "const add = (a, b) => a + b;\n";
	let r = extract_code_structure(code, "js");
	let fns = &r["fns"];
	assert!(
		fns.iter().any(|s| s.contains("add")),
		"real arrow fn should still be extracted: {fns:?}"
	);
}

/// `pub(crate) async fn` was previously invisible to the Rust extractor's
/// `is_fn` prefix check (only the non-async `pub(crate) fn` was listed).
#[test]
fn test_extract_rust_pub_crate_async_fn() {
	let code = "pub(crate) async fn tick(&self) -> bool {\n    true\n}\n";
	let r = extract_code_structure(code, "rust");
	let fns = r.get("fns").expect("pub(crate) async fn should be extracted");
	assert!(fns.iter().any(|s| s.contains("tick")), "expected tick() in {fns:?}");
}

/// A Rust file that opens with a long `//!` module-doc comment block
/// (extremely common in this codebase) must still auto-detect as Rust -
/// the old byte-prefix-based `auto_detect` could push every real
/// keyword-bearing line past its fixed 500-byte window.
#[test]
fn test_auto_detect_rust_survives_long_leading_doc_comment() {
	let mut content = String::new();
	for i in 0..40 {
		content.push_str(&format!(
			"//! This is a long module doc comment line number {i} padding it out.\n"
		));
	}
	content.push_str("fn real_function() -> i32 { 42 }\n");
	assert_eq!(auto_detect(&content), "rust");
}

#[test]
fn test_budget_respected() {
	// Generate lots of functions to test budget
	let mut code = String::new();
	for i in 0..50 {
		code.push_str(&format!("fn func{}(x: i32, y: i32, z: i32) -> i32 {{ 42 }}\n", i));
	}
	let r = extract_code_structure(&code, "rust");
	// Should have stopped before 50 due to budget
	if let Some(fns) = r.get("fns") {
		assert!(fns.len() < 50, "budget should cap output: got {}", fns.len());
	}
}

// ── T1 (F2): multi-byte UTF-8 must never panic a byte-offset truncation ──
#[test]
fn test_floor_boundary_never_panics_on_multibyte() {
	assert_eq!(floor_boundary("hello", 10), "hello");
	let s = "é".repeat(200); // every char is 2 bytes; any odd offset is mid-char
	let out = floor_boundary(&s, 99);
	assert!(out.len() <= 99);
	assert!(s.is_char_boundary(out.len()));
}

#[test]
fn test_auto_detect_multibyte_near_500_byte_boundary() {
	// 'é' is 2 bytes; place one straddling byte offset 500.
	let mut content = "x".repeat(499);
	content.push('é');
	content.push_str("fn f() -> i32 { 1 }");
	// Must not panic.
	let _ = extract_code_structure(&content, "");
}

#[test]
fn test_sig_multibyte_signature_does_not_panic() {
	let name = "é".repeat(40);
	// Must not panic when truncating a signature full of multi-byte chars.
	let s = sig("fn", &name);
	assert!(s.len() <= MAX_SIG_LEN + 3); // +3 for "..."
}
