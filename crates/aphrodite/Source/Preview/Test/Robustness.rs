//! Pathological-input coverage: UTF-8 boundaries, literal markers, interior NUL.
//!
//! Every arm is a `&str` slicer somewhere, and a slicer that is not char-boundary
//! safe panics on real tool output. These tests do not check what a preview says;
//! they check that it says anything at all instead of aborting the process.

use super::*;

#[test]
fn TestDetectTypeNeverPanicsOnInteriorNul() {
	let _G = CapGuard();
	let Content = "before\0after\0\0end";
	let _ = crate::Preview::Detect::Type::Fn(Content); // must not panic
}

#[test]
fn TestBuildPreviewNeverPanicsOnInteriorNulAcrossTypeBranches() {
	let _G = CapGuard();
	let Content = "line one\0line two\0\0error: boom\nwarning: also this";
	for ty in [
		"build",
		"diff",
		"code_rust",
		"code_sh",
		"code",
		"search",
		"html",
		"json_array",
		"json",
		"json_list",
		"terminal",
		"text",
	] {
		let _ = crate::Preview::Builder::Entry::Fn(ty, Content); // must not panic for any branch
	}
}

#[test]
fn TestBuildPreviewCodeRustTruncatesSignatureOnCharBoundary() {
	let _G = CapGuard();
	// The "code" family truncates the first signature to 48 *chars* via
	// `.chars().take(48)`, not a byte slice - a signature packed with
	// multi-byte UTF-8 must not panic or split a character mid-encoding.
	// This function never panics on reaching an assertion at all (a byte-
	// boundary split would have panicked inside `String` construction
	// before we got here), so the pass condition is simply completing
	// without a panic and producing the expected preview shape.
	let Content = format!("fn \u{4e2d}\u{6587}_{}() {{}}", "x".repeat(60));
	let Out = crate::Preview::Builder::Entry::Fn("code_rust", &Content);
	assert!(Out.starts_with("[code:"));
}

#[test]
fn TestBuildPreviewNeverPanicsOnMultibyteUtf8EveryType() {
	let _G = CapGuard();
	let Content = "a\u{00e9}\u{4e2d}\u{1f600}b".repeat(30);
	for ty in [
		"build",
		"diff",
		"code_rust",
		"code_sh",
		"code",
		"search",
		"html",
		"json_array",
		"json",
		"json_list",
		"terminal",
		"text",
	] {
		let _ = crate::Preview::Builder::Entry::Fn(ty, &Content);
	}
}

#[test]
fn TestBuildPreviewHandlesLiteralMarkerShapedContent() {
	let _G = CapGuard();
	// Content that already contains marker-shaped text (e.g. a pasted
	// example transcript) must not confuse the line/byte counting or
	// panic in any branch - build_preview only ever summarizes, it never
	// re-parses content as a marker.
	let Content = "before <<<CCR:fake000|text|1>>> after\nerror: boom";
	for ty in [
		"build",
		"diff",
		"code_rust",
		"code_sh",
		"code",
		"search",
		"html",
		"json_array",
		"json",
		"json_list",
		"terminal",
		"text",
	] {
		let Out = crate::Preview::Builder::Entry::Fn(ty, Content);
		assert!(!Out.is_empty());
	}
}

#[test]
fn TestDetectTypeNeverPanicsOnMultibyteUtf8() {
	let _G = CapGuard();
	let Content = "\u{1f600}".repeat(500);
	let _ = crate::Preview::Detect::Type::Fn(&Content);
}

#[test]
fn TestNewArmsNeverPanicOnPathologicalInput() {
	let Multibyte = "a\u{00e9}\u{4e2d}\u{1f600}".repeat(20);
	let Inputs = ["", "\0\0\0", Multibyte.as_str()];
	for ty in ["git", "gitlog", "ls", "test", "grep"] {
		for c in Inputs {
			let _ = crate::Preview::Builder::Entry::Fn(ty, c);
			let _ = crate::Preview::Detect::SemanticType::Fn(c);
		}
	}
}
