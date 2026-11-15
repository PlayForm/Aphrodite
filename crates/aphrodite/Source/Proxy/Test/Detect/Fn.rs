//! Content-type classification: one test per family, plus the near-miss cases
//! that used to misclassify (a comment mentioning `error`, an invalid JSON
//! payload, an empty body).

use crate::Proxy::Detect::Fn::Fn as ProxyDetectContentType;

// ── T3: detect_content_type ─────────────────────────────────
#[test]
fn TestDetectContentTypeJSONToolOutput() {
	let _G = crate::Preview::PreviewCapTestGuard();
	assert_eq!(ProxyDetectContentType(r#"{"exit_code": 0, "output": "ok"}"#), "tool_output");
}

#[test]
fn TestDetectContentTypeInvalidJSONIsText() {
	let _G = crate::Preview::PreviewCapTestGuard();
	// Starts with '{' but isn't valid JSON - must not be misclassified.
	assert_eq!(ProxyDetectContentType("{ not json at all"), "text");
}

#[test]
fn TestDetectContentTypeJSONArray() {
	let _G = crate::Preview::PreviewCapTestGuard();
	assert_eq!(ProxyDetectContentType(r#"[{"a":1},{"a":2}]"#), "json");
}

#[test]
fn TestDetectContentTypeRustCode() {
	let _G = crate::Preview::PreviewCapTestGuard();
	let Src = "use std::fmt;\nfn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
	assert_eq!(ProxyDetectContentType(Src), "code_rust");
}

#[test]
fn TestDetectContentTypePythonCode() {
	let _G = crate::Preview::PreviewCapTestGuard();
	let Src = "import os\nclass Foo:\n    def bar(self):\n        pass\n";
	assert_eq!(ProxyDetectContentType(Src), "code_python");
}

#[test]
fn TestDetectContentTypeGoCode() {
	let _G = crate::Preview::PreviewCapTestGuard();
	let Src = "package main\nimport (\n\t\"fmt\"\n)\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n";
	assert_eq!(ProxyDetectContentType(Src), "code_go");
}

#[test]
fn TestDetectContentTypeJSCode() {
	let _G = crate::Preview::PreviewCapTestGuard();
	let Src = "import { foo } from 'bar';\nexport const add = (a, b) => a + b;\nconst x = 1;\nconst y = 2;\n";
	assert_eq!(ProxyDetectContentType(Src), "code_js");
}

#[test]
fn TestDetectContentTypeErrorFirstLine() {
	let _G = crate::Preview::PreviewCapTestGuard();
	assert_eq!(
		ProxyDetectContentType("Traceback (most recent call last):\n  File \"x.py\", line 1\nValueError: bad\n"),
		"error"
	);
}

#[test]
fn TestDetectContentTypeCommentMentioningErrorIsNotError() {
	let _G = crate::Preview::PreviewCapTestGuard();
	// W2-6: the first-line error markers are `starts_with`, not
	// `contains` - a Rust doc comment line mentioning "error recovery"
	// (starting with `///`) must not classify as an error, while a real
	// traceback first line still does.
	assert_ne!(
		ProxyDetectContentType("/// Handles error recovery gracefully.\nThe system continues.\n"),
		"error"
	);
	assert_eq!(
		ProxyDetectContentType("Traceback (most recent call last):\n  File \"x.py\", line 1\nValueError: bad\n"),
		"error"
	);
}

#[test]
fn TestDetectContentTypeDiff() {
	let _G = crate::Preview::PreviewCapTestGuard();
	let d =
		"diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,4 @@\n+added a line\n";
	assert_eq!(ProxyDetectContentType(d), "diff");
}

#[test]
fn TestDetectContentTypeLogLines() {
	let _G = crate::Preview::PreviewCapTestGuard();
	// Must not start with '{'/'[' (that short-circuits to the JSON branch).
	let Log = "starting up\n[INFO] service ready\n[WARN] disk low\n[ERROR] connection lost\n";
	assert_eq!(ProxyDetectContentType(Log), "log");
}

#[test]
fn TestDetectContentTypeEmptyIsText() {
	let _G = crate::Preview::PreviewCapTestGuard();
	assert_eq!(ProxyDetectContentType(""), "text");
}

#[test]
fn TestDetectContentTypePlainText() {
	let _G = crate::Preview::PreviewCapTestGuard();
	assert_eq!(ProxyDetectContentType("just some plain text\nnothing special\n"), "text");
}
