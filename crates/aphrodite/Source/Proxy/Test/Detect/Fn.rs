//! Content-type classification: one test per family, plus the near-miss cases
//! that used to misclassify (a comment mentioning `error`, an invalid JSON
//! payload, an empty body).

use crate::Proxy::Analysis::Classify::Fn::Fn as Classify;

// ── T3: detect_content_type ─────────────────────────────────
#[test]
fn ToolOutput() {
	let _G = crate::Preview::State::Guard::Fn();
	assert_eq!(Classify(r#"{"exit_code": 0, "output": "ok"}"#), "tool_output");
}

#[test]
fn InvalidJSON() {
	let _G = crate::Preview::State::Guard::Fn();
	// Starts with '{' but isn't valid JSON - must not be misclassified.
	assert_eq!(Classify("{ not json at all"), "text");
}

#[test]
fn JSONArray() {
	let _G = crate::Preview::State::Guard::Fn();
	assert_eq!(Classify(r#"[{"a":1},{"a":2}]"#), "json");
}

#[test]
fn RustCode() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "use std::fmt;\nfn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
	assert_eq!(Classify(Src), "code_rust");
}

#[test]
fn PythonCode() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "import os\nclass Foo:\n    def bar(self):\n        pass\n";
	assert_eq!(Classify(Src), "code_python");
}

#[test]
fn GoCode() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "package main\nimport (\n\t\"fmt\"\n)\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n";
	assert_eq!(Classify(Src), "code_go");
}

#[test]
fn JSCode() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "import { foo } from 'bar';\nexport const add = (a, b) => a + b;\nconst x = 1;\nconst y = 2;\n";
	assert_eq!(Classify(Src), "code_js");
}

#[test]
fn ErrorLine() {
	let _G = crate::Preview::State::Guard::Fn();
	assert_eq!(
		Classify("Traceback (most recent call last):\n  File \"x.py\", line 1\nValueError: bad\n"),
		"error"
	);
}

#[test]
fn ErrorComment() {
	let _G = crate::Preview::State::Guard::Fn();
	// W2-6: the first-line error markers are `starts_with`, not
	// `contains` - a Rust doc comment line mentioning "error recovery"
	// (starting with `///`) must not classify as an error, while a real
	// traceback first line still does.
	assert_ne!(
		Classify("/// Handles error recovery gracefully.\nThe system continues.\n"),
		"error"
	);
	assert_eq!(
		Classify("Traceback (most recent call last):\n  File \"x.py\", line 1\nValueError: bad\n"),
		"error"
	);
}

#[test]
fn Diff() {
	let _G = crate::Preview::State::Guard::Fn();
	let d =
		"diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,4 @@\n+added a line\n";
	assert_eq!(Classify(d), "diff");
}

#[test]
fn LogLines() {
	let _G = crate::Preview::State::Guard::Fn();
	// Must not start with '{'/'[' (that short-circuits to the JSON branch).
	let Log = "starting up\n[INFO] service ready\n[WARN] disk low\n[ERROR] connection lost\n";
	assert_eq!(Classify(Log), "log");
}

#[test]
fn Empty() {
	let _G = crate::Preview::State::Guard::Fn();
	assert_eq!(Classify(""), "text");
}

#[test]
fn PlainText() {
	let _G = crate::Preview::State::Guard::Fn();
	assert_eq!(Classify("just some plain text\nnothing special\n"), "text");
}
