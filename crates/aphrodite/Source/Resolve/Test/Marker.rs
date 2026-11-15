//! The marker grammar: hash extraction and the scanner.
//!
//! A marker is a wire format, so these pin both directions of the read: the hash
//! comes out of a well-formed marker, a non-marker yields nothing, and the scanner
//! finds every marker in arbitrary text without panicking on the malformed ones.

use crate::Resolve::Parse::{Find::Fn as FindMarkers, Hash::Fn as ParseMarkerHash};

#[test]
fn TestParseMarkerHash() {
	let hash = ParseMarkerHash("<<<CCR:abc123def456|text|100>>>");
	assert_eq!(hash, Some("abc123def456".into()));

	let hash = ParseMarkerHash("<<<CCR:abc|code_rust|5000>>>");
	assert_eq!(hash, Some("abc".into()));
}

#[test]
fn TestParseMarkerInvalid() {
	assert_eq!(ParseMarkerHash("not a marker"), None);
	assert_eq!(ParseMarkerHash("<<<CCR:"), None);
}

#[test]
fn TestFindMarkersSingle() {
	let Content = "hello <<<CCR:abc|text|100>>> world";
	let Markers = FindMarkers(Content);
	assert_eq!(Markers.len(), 1);
	assert_eq!(Markers[0].0, "<<<CCR:abc|text|100>>>");
	assert_eq!(Markers[0].1, "abc");
}

#[test]
fn TestFindMarkersMultiple() {
	let Content = "<<<CCR:a|t|1>>> middle <<<CCR:b|t|2>>>";
	let Markers = FindMarkers(Content);
	assert_eq!(Markers.len(), 2);
}

#[test]
fn TestFindMarkersNone() {
	assert!(FindMarkers("plain text").is_empty());
}

// ── T7: negative-path tests ──────────────────────────────────
#[test]
fn TestFindMarkersNestedUnclosedIsSkipped() {
	// A marker prefix inside another unterminated marker: the parser must
	// not panic and must not treat the inner "<<<CCR:" as a fresh match
	// once the outer one already consumed up through its own ">>>".
	let Content = "<<<CCR:a<<<CCR:b|t|1>>>";
	let Markers = FindMarkers(Content);
	assert_eq!(Markers.len(), 1);
	assert_eq!(Markers[0].1, "a<<<CCR:b");
}

#[test]
fn TestFindMarkersUnterminatedNoPanic() {
	assert!(FindMarkers("<<<CCR:").is_empty());
	assert!(FindMarkers("text <<<CCR:abc|t|1").is_empty());
}

#[test]
fn TestParseMarkerHashEmptyHash() {
	assert_eq!(ParseMarkerHash("<<<CCR:>>>"), Some(String::new()));
}
