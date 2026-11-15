//! ISSUE-11 (WS1 + WS2-tools): the JSON-preview collapse family.
//!
//! A JSON tool payload like `{"success":true,"data":{...}}` must never collapse
//! to the literal fragment "ok" (`[text:1L 2B | ok]`); the caller's explicit
//! `type` hint must win over the envelope heuristic; and search results must show
//! the REAL match count instead of a capped or invented one.

use crate::Tool::Dispatch::Fn::Fn as dispatch;

// ── FIXDESIGN 3.1: the issue's exact repro shape, explicit path.
// {"success":true,"data":{...}} with type:"tool_result" must keep the
// hinted type and preview the FULL payload (honest L/B), never
// "[text:1L 2B | ok]". ──
#[test]
fn test_compress_json_payload_with_hint_keeps_type_and_full_preview() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = serde_json::json!({"success": true, "data": {"web": [{"title": "x"}]}}).to_string();
	let compressed = dispatch(
		"aphrodite_compress",
		&serde_json::json!({"content": content, "type": "tool_result"}).to_string(),
	);
	assert_eq!(
		compressed["type"], "tool_result",
		"explicit hint must win over the envelope heuristic"
	);
	let preview = compressed["preview"].as_str().unwrap();
	assert!(
		preview.starts_with("[tool_result:"),
		"preview must reflect the hinted type: {preview}"
	);
	assert!(
		!preview.contains("| ok]"),
		"preview must not collapse to the literal 'ok' fragment: {preview}"
	);
	assert!(
		preview.contains("\"success\""),
		"preview must show the real JSON payload: {preview}"
	);
	assert_eq!(
		compressed["size"],
		content.len(),
		"size must count the full payload, not the fragment"
	);
	// Round-trip stays lossless regardless of preview/type.
	let retrieved = dispatch(
		"aphrodite_retrieve",
		&serde_json::json!({"hash": compressed["hash"]}).to_string(),
	);
	assert_eq!(retrieved["content"], content);
}

// ── FIXDESIGN 3.2 (adapted): no-hint bare {"success":true} must not
// collapse either - the preview must show the payload itself, never a
// 2-byte "ok" fragment. (Note: headroom classifies JSON *objects* as
// text - only arrays get json_array - so we assert preview honesty,
// not a json type, which the classifier cannot produce here.) ──
#[test]
fn test_compress_bare_success_object_preview_is_honest() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = serde_json::json!({"success": true}).to_string();
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let preview = compressed["preview"].as_str().unwrap();
	assert!(
		!preview.contains("| ok]"),
		"bare success object must not collapse to 'ok': {preview}"
	);
	assert!(preview.contains("\"success\""), "preview must show the real payload: {preview}");
	assert_ne!(
		compressed["type"], "search",
		"the unwrap must not hijack a success payload into search"
	);
	assert_eq!(compressed["size"], content.len());
}

// ── WS2-tools: the real Hermes search_files shape ships `matches_text`
// (path-grouped) instead of a `matches` array. The preview must count
// the REAL matches, not collapse to "[search:1L]". ──
#[test]
fn test_compress_search_result_shows_real_match_count() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = serde_json::json!({
		"total_count": 19,
		"matches_format": "path-grouped",
		"matches_text": "src/a.rs\n  10: fn alpha()\nsrc/b.rs\n  3: struct Gamma\n",
	})
	.to_string();
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	assert_eq!(compressed["type"], "search", "search-shaped envelope must classify as search");
	let preview = compressed["preview"].as_str().unwrap();
	assert!(
		preview.starts_with("[search:2 hits in 2 files"),
		"preview must count the real matches: {preview}"
	);
	assert!(
		!preview.contains("[search:1L]"),
		"no [search:1L] collapse for a real search: {preview}"
	);
}

// ── WS2-tools: the take(20) cap is gone - a matches array larger than
// 20 must not preview as "[search:20 hits ...]". ──
#[test]
fn test_compress_search_matches_array_counts_all_matches() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let matches:Vec<serde_json::Value> = (0..25)
		.map(|i| serde_json::json!({"path": format!("f{i}.rs"), "line": i, "content": "x"}))
		.collect();
	let content = serde_json::json!({"total_count": 25, "matches": matches}).to_string();
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let preview = compressed["preview"].as_str().unwrap();
	assert!(
		preview.contains("25 hits"),
		"all real matches must be counted, not capped at 20: {preview}"
	);
}

// ── WS2-tools: a zero-hit / count-only search must surface the REAL
// total ("0 total") instead of an unreadable "[search:1L]". ──
#[test]
fn test_compress_search_zero_or_count_only_surfaces_total() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = serde_json::json!({"total_count": 0, "matches": []}).to_string();
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	let preview = compressed["preview"].as_str().unwrap();
	assert!(
		preview.contains("0 total"),
		"zero-hit search must show the real total: {preview}"
	);
	assert!(
		!preview.contains("[search:"),
		"count-only search must not render [search:1L]: {preview}"
	);
}

// ── WS2-tools: a data object that merely carries a total_count key
// (no matches/matches_text/matches_format/truncated) is NOT a search
// envelope - it must preview as itself, not as a fake search. ──
#[test]
fn test_compress_data_object_with_total_count_key_is_not_search() {
	let _g = crate::Session::Guard::Fn::test_guard();
	let content = serde_json::json!({"total_count": 42, "items": [1, 2, 3]}).to_string();
	let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
	assert_ne!(
		compressed["type"], "search",
		"total_count alone must not hijack the type: {compressed}"
	);
	let preview = compressed["preview"].as_str().unwrap();
	assert!(preview.contains("items"), "preview must show the real payload: {preview}");
}
