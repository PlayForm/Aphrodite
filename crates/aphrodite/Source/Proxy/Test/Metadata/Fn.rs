//! The metadata block: language facts, pipe escaping, the 400-char cap, and
//! multibyte safety on the error branch.

use crate::Proxy::{Metadata::Fn::Fn as GenerateMetadata, Test::Support::Analysis::Fn::Fn as NamedAnalysis};

// ── T3: generate_metadata ───────────────────────────────────
#[test]
fn Rust() {
	let Src = "fn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
	let meta = GenerateMetadata(&NamedAnalysis(Src, "code_rust"));

	assert!(meta.contains("lang=rs"));
	assert!(meta.contains("fns=add"));
}

#[test]
fn Pipes() {
	// The 'On branch' line can legally contain a literal '|' - must be escaped
	// to '/'.
	let Src = "On branch feature|weird\n";
	let meta = GenerateMetadata(&NamedAnalysis(Src, "git"));

	assert!(!meta.contains('|'), "metadata must not contain a raw pipe: {meta}");
}

#[test]
fn Limit() {
	let Src = (0..100).map(|i| format!("fn f{i}() {{}}")).collect::<Vec<_>>().join("\n");
	let meta = GenerateMetadata(&NamedAnalysis(&Src, "code_rust"));

	assert!(meta.chars().count() <= 400, "metadata too long: {} chars", meta.chars().count());
}

#[test]
fn Multibyte() {
	// Multi-byte UTF-8 characters sit right around the ".rs:" match position -
	// byte-index slicing here must not panic on a non-char-boundary.
	let Src = "日本語エラー at src/日本.rs:10:5 something\n";
	let meta = GenerateMetadata(&NamedAnalysis(Src, "error"));

	// No assertion beyond "did not panic" is required, but sanity-check shape.
	assert!(meta.is_empty() || meta.contains("trace=") || meta.contains("msg="));
}

#[test]
fn Lines() {
	let meta = GenerateMetadata(&NamedAnalysis("a\nb\nc\n", "text"));

	assert_eq!(meta, "ln=3");
}

// ── Field order is the contract: the model reads the block left to right, so a
// producer's fields must keep the order its own stage comments name. ──
#[test]
fn Order() {
	// A real cargo block: the status leads, the compile count follows, and the
	// first error - the one line the producer can actually find - comes last.
	let meta = GenerateMetadata(&NamedAnalysis(
		"   Compiling aphrodite v1.5.0\nerror[E0308]: mismatched types\n  --> src/x.rs:12:3\n",
		"build_output",
	));

	assert!(meta.starts_with("status=FAIL"), "status leads the build block: {meta}");
	assert!(meta.contains("files=1"), "the compile count follows the status: {meta}");
	assert!(meta.contains("first_err=error[E0308]"), "the first error follows: {meta}");
}
