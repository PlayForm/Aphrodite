//! Language detection: each family's keyword, the unknown case, and the two content
//! shapes that used to fool the detector - a long leading doc comment, and a
//! multi-byte character straddling the old fixed byte window.

use crate::StructExtract::{Detect::Fn as AutoDetect, Extract::Fn as ExtractCodeStructure};

#[test]
fn TestAutoDetectRust() {
	assert_eq!(AutoDetect("fn main() -> i32 {}\npub struct Foo {}"), "rust");
}

#[test]
fn TestAutoDetectPython() {
	assert_eq!(AutoDetect("def hello():\n    pass\n"), "python");
}

#[test]
fn TestAutoDetectGo() {
	assert_eq!(AutoDetect("func main() {\n}\n"), "go");
}

#[test]
fn TestAutoDetectJS() {
	assert_eq!(AutoDetect("function hello() {\n}\n"), "js");
}

#[test]
fn TestAutoDetectUnknown() {
	assert_eq!(AutoDetect("plain text no code"), "");
}

/// A Rust file that opens with a long `//!` module-doc comment block
/// (extremely common in this codebase) must still auto-detect as Rust -
/// the old byte-prefix-based `AutoDetect` could push every real
/// keyword-bearing line past its fixed 500-byte window.
#[test]
fn TestAutoDetectRustSurvivesLongLeadingDocumentComment() {
	let mut Content = String::new();
	(0..40).for_each(|i| {
		Content.push_str(&format!(
			"//! This is a long module doc comment line number {i} padding it out.\n"
		));
	});
	Content.push_str("fn real_function() -> i32 { 42 }\n");
	assert_eq!(AutoDetect(&Content), "rust");
}

#[test]
fn TestAutoDetectMultibyteNear500ByteBoundary() {
	// 'é' is 2 bytes; place one straddling byte offset 500.
	let mut Content = "x".repeat(499);
	Content.push('é');
	Content.push_str("fn f() -> i32 { 1 }");
	// Must not panic.
	let _ = ExtractCodeStructure(&Content, "");
}
