//! The store-and-retrieve round trip, and what a marker must carry.
//!
//! 01-F2: `unwrap_hermes_result` is a heuristic used to pick a better
//! type/preview - it must never change what a retrieve returns. Caller JSON that
//! merely matches a wrapper shape must round-trip byte-for-byte, and a genuine
//! terminal wrapper must keep its `exit_code`/`error` metadata.

use Tool::Dispatch::Fn::Fn as dispatch;

#[test]
fn test_compress_then_retrieve_roundtrip() {
	// The core promise: a tool can compress content and retrieve it back
	// via a separate dispatch call, because state is shared across calls.
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = "fn answer() -> i32 { 42 }\n".repeat(10);
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let hash = compressed["hash"].as_str().expect("hash present").to_string();
	assert!(!hash.is_empty());

	let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
	assert_eq!(retrieved["found"], true, "retrieve must resolve a just-compressed hash");
	assert_eq!(retrieved["content"], content);
}

#[test]
fn test_compress_preserves_original_when_content_looks_like_a_wrapper() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = serde_json::json!({"content": "hi", "id": 42, "role": "assistant"}).to_string();
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let hash = compressed["hash"].as_str().expect("hash present").to_string();

	let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
	assert_eq!(retrieved["found"], true);
	assert_eq!(
		retrieved["content"], content,
		"retrieve must return the exact original JSON, not just the extracted \"content\" field"
	);
}

#[test]
fn test_compress_preserves_terminal_wrapper_metadata() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = serde_json::json!({"output": "error: broke\nexit code: 1\n", "exit_code": 1}).to_string();
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let hash = compressed["hash"].as_str().expect("hash present").to_string();
	assert_eq!(compressed["type"], "terminal", "type should reflect the unwrapped payload");

	let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
	assert_eq!(retrieved["found"], true);
	assert_eq!(
		retrieved["content"], content,
		"retrieve must return the original wrapper, including exit_code, not just the extracted output"
	);
}

// ── T9 (F11): `_ccr_center` was documented in the schema but silently
// dropped by compress_into; it must now travel with the recorded marker.
#[test]
fn test_compress_wires_ccr_center_through() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = "fn answer() -> i32 { 42 }\n".repeat(10);
	let compressed = dispatch(
		"aphrodite_compress",
		&serde_json::json!({"content": content, "_ccr_center": "my-center"}).to_string(),
	);
	let hash = compressed["hash"].as_str().expect("hash present").to_string();
	let catalog = dispatch("aphrodite_catalog", "{}");
	let entry = catalog["items"]
		.as_array()
		.unwrap()
		.iter()
		.find(|e| e["hash"] == hash)
		.expect("just-compressed entry should be in the catalog");
	assert_eq!(entry["center"], "my-center");
}
