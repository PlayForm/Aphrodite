//! Catalogue and search visibility: a compression is findable by both.
//!
//! The session state is reset first so the counts are exact - the assertions used
//! to be `>= 1` because other tests left markers behind, which would let a
//! double-recording bug pass unnoticed.

use crate::Tool::Dispatch::Fn::Fn as dispatch;

#[test]
fn test_catalog_and_search_see_compressions() {
	let _g = crate::Session::Guard::Fn::test_guard();
	// Reset shared state so `total` reflects only this test's compression,
	// making the previously-loosened `>= 1` assertions exact.
	crate::Session::With::Fn::with_shared(aphrodite::Session::Start::Fn::Fn);
	dispatch(
		"aphrodite_compress",
		&serde_json::json!({"content": "needle_xyz in a haystack\n".repeat(5), "type": "log"}).to_string(),
	);
	let cat = dispatch("aphrodite_catalog", "{}");
	assert_eq!(cat["total"].as_u64().unwrap(), 1);
	let found = dispatch("aphrodite_search", &serde_json::json!({"query": "log"}).to_string());
	assert_eq!(found["total"].as_u64().unwrap(), 1);
}
