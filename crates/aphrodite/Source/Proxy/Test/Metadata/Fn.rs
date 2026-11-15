//! The metadata block: language facts, pipe escaping, the 400-char cap, and
//! multibyte safety on the error branch.

use crate::Proxy::Metadata::Fn::Fn as GenerateMetadata;

// ── T3: generate_metadata ───────────────────────────────────
#[test]
fn TestGenerateMetadataRustHasLanguageAndFns() {
	let Src = "fn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
	let meta = GenerateMetadata(Src, "code_rust");
	assert!(meta.contains("lang=rs"));
	assert!(meta.contains("fns=add"));
}

#[test]
fn TestGenerateMetadataEscapesPipes() {
	// The 'On branch' line can legally contain a literal '|' - must be escaped to
	// '/'.
	let Src = "On branch feature|weird\n";
	let meta = GenerateMetadata(Src, "git");
	assert!(!meta.contains('|'), "metadata must not contain a raw pipe: {meta}");
}

#[test]
fn TestGenerateMetadataMaximum400Chars() {
	let Src = (0..100).map(|i| format!("fn f{i}() {{}}")).collect::<Vec<_>>().join("\n");
	let meta = GenerateMetadata(&Src, "code_rust");
	assert!(meta.chars().count() <= 400, "metadata too long: {} chars", meta.chars().count());
}

#[test]
fn TestGenerateMetadataErrorBranchNoPanicOnMultibyteUtf8() {
	// Multi-byte UTF-8 characters sit right around the ".rs:" match position -
	// byte-index slicing here must not panic on a non-char-boundary.
	let Src = "日本語エラー at src/日本.rs:10:5 something\n";
	let meta = GenerateMetadata(Src, "error");
	// No assertion beyond "did not panic" is required, but sanity-check shape.
	assert!(meta.is_empty() || meta.contains("trace=") || meta.contains("msg="));
}

#[test]
fn TestGenerateMetadataTextHasLineCount() {
	let meta = GenerateMetadata("a\nb\nc\n", "text");
	assert_eq!(meta, "ln=3");
}
