//! Retrieve tests: hash tolerance, query filtering, and the path guard.
//!
//! The last two are a security boundary, not a convenience: an outside-workspace
//! read must be refused by name, and an inside-workspace read must still work -
//! both directions, because a guard that refuses everything passes the first test
//! on its own.

use Tool::Dispatch::Fn::Fn as dispatch;

// ── T5 (F3): the "aphrodite_retrieve" tool delegates to
// `aphrodite::Resolve::Expand::Fn` -> `resolve_one`, which already strips a
// `|type|size` marker-body suffix and surrounding whitespace - this
// pins that the delegation actually carries the tolerance through.
#[test]
fn test_retrieve_normalizes_pipe_suffixed_and_whitespace_hash() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = "fn answer() -> i32 { 42 }\n".repeat(10);
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let hash = compressed["hash"].as_str().expect("hash present").to_string();

	for hash_arg in [hash.clone(), format!("{hash}|tool|1024"), format!("  {hash}  ")] {
		let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash_arg}).to_string());
		assert_eq!(retrieved["found"], true, "hash arg {hash_arg:?} must resolve: {retrieved:?}");
		assert_eq!(retrieved["content"], content);
	}
}

#[test]
fn test_retrieve_with_query_filters_lines() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = "alpha line\nbeta error here\ngamma line\n";
	let c = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let hash = c["hash"].as_str().unwrap().to_string();
	let r = dispatch(
		"aphrodite_retrieve",
		&serde_json::json!({"hash": hash, "query": "error"}).to_string(),
	);
	let body = r["content"].as_str().unwrap();
	assert!(body.contains("beta error here"));
	assert!(!body.contains("alpha line"));
}

#[test]
fn test_retrieve_path_outside_workspace_is_denied() {
	// /etc/hosts exists and is outside any repo cwd → must be refused.
	let _g = crate::Session::Guard::Fn::test_guard();
	let r = dispatch("aphrodite_retrieve", &serde_json::json!({"path": "/etc/hosts"}).to_string());
	assert_eq!(r["found"], false, "reads outside the workspace must be denied: {r:?}");
	assert!(r["error"].as_str().unwrap().contains("outside the workspace"));
}

#[test]
fn test_retrieve_path_within_workspace_ok() {
	// This source file is inside the crate (workspace) → allowed.
	let _g = crate::Session::Guard::Fn::test_guard();
	let r = dispatch(
		"aphrodite_retrieve",
		&serde_json::json!({"path": concat!(env!("CARGO_MANIFEST_DIR"), "/Source/Tool/Read/Fn.rs")}).to_string(),
	);
	assert_eq!(r["found"], true, "in-workspace path read should succeed: {r:?}");
	assert!(r["content"].as_str().unwrap().contains("MAX_PATH_READ"));
}

#[test]
fn test_retrieve_missing_hash() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let r = dispatch(
		"aphrodite_retrieve",
		&serde_json::json!({"hash": "deadbeefdeadbeefdeadbeef"}).to_string(),
	);
	assert_eq!(r["found"], false);
}
