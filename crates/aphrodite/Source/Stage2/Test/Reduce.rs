//! The four reducers, including the size assertion that makes a "reduction"
//! that grows the content a failure rather than a cosmetic surprise.

use crate::Stage2::Reduce::{Build, Code, Diff, JSON};

#[test]
fn TestReduceJSONObject() {
	let JSON = r#"{"name": "Alice", "age": 30, "city": "NYC", "role": "admin"}"#;
	let Result = JSON::Fn::Fn(JSON);
	assert!(Result.is_some());
	let r = Result.unwrap();
	assert!(r.contains("[json:4 keys:"));
	assert!(r.contains("name"));
}

#[test]
fn TestReduceJSONInvalid() {
	assert_eq!(JSON::Fn::Fn("not json at all"), None);
}

#[test]
fn TestReduceBuildErrors() {
	// Use mid-line error/warning (prefix "error:" and "warning:" are intentionally
	// excluded because they're structured prefixes handled by summary extraction)
	let Build = "   Compiling foo v0.1.0\n   process failed with error code 1\n   found a warning in the pipeline\n   \
	             Finished dev [unoptimized] target(s)\n";
	let Result = Build::Fn::Fn(Build);
	assert!(Result.is_some());
	let r = Result.unwrap();
	assert!(r.contains("[build summary]"));
	assert!(r.contains("[1 errors]"));
	assert!(r.contains("[1 warnings]"));
}

#[test]
fn TestReduceBuildEmpty() {
	assert_eq!(Build::Fn::Fn(""), None);
}

#[test]
fn TestReduceDiff() {
	let Diff = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,5 +1,7 @@\n+added line\n-old line\n@@ -10,3 +12,4 \
	            @@\ndiff --git a/Cargo.toml b/Cargo.toml\n@@ -1,1 +1,1 @@\n";
	let Result = Diff::Fn::Fn(Diff);
	assert!(Result.is_some());
	let r = Result.unwrap();
	assert!(r.contains("[diff:2 files]"));
	assert!(r.contains("main.rs"));
	assert!(r.contains("Cargo.toml"));
}

#[test]
fn TestReduceCodeRust() {
	let Code = "pub fn main() {\n    println!(\"hello\");\n}\n\npub struct Foo {\n    x: i32,\n}\n\nfn helper() {}\n";
	let Result = Code::Fn::Fn(Code);
	assert!(Result.is_some());
	let r = Result.unwrap();
	assert!(r.contains("[code structure]"));
	assert!(r.contains("[fn]"));
	assert!(r.contains("[struct]"));
}

#[test]
fn TestIntegrationJSONMinifies() {
	// Heavily whitespace-padded JSON (many keys, deep indentation) so that
	// minification + the structural header is still smaller than the
	// pretty-printed original - a real size assertion, not just "no panic".
	let Content =
		"{\n  \"users\": [\n    {\"id\": 1, \"name\": \"Alice\", \"active\": true, \"role\": \"admin\"},\n    \
		 {\"id\": 2, \"name\": \"Bob\", \"active\": false, \"role\": \"user\"},\n    {\"id\": 3, \"name\": \"Carol\", \
		 \"active\": true, \"role\": \"user\"}\n  ],\n  \"total\": 3,\n  \"page\": 1,\n  \"per_page\": 10\n}"
			.to_string();
	let OriginalLength = Content.len();
	let Reduced = JSON::Fn::Fn(&Content);
	assert!(Reduced.is_some(), "reduce_json returned None");
	let r = Reduced.unwrap();
	assert!(r.starts_with("[json:4 keys:"));
	assert!(r.contains("\"users\""));
	// The whole point of stage2 reduction is to save space - assert it
	// actually does, not merely that it runs without panicking.
	assert!(
		r.len() < OriginalLength,
		"reduced output ({} bytes) should be smaller than the pretty-printed original ({} bytes)",
		r.len(),
		OriginalLength
	);
}
