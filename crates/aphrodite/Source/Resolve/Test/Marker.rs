//! The marker grammar: hash extraction and the scanner.
//!
//! A marker is a wire format, so these pin both directions of the read: the hash comes out of
//! a well-formed marker, a non-marker yields nothing, and the scanner finds every reference in
//! arbitrary text without panicking on the malformed ones - including the position each
//! reference occupies, which is what the walk substitutes by.

use crate::Resolve::Parse::{Find::Fn as Find, Hash::Fn as Hash};

#[test]
fn TestParseMarkerHash() {
	assert_eq!(Hash("<<<CCR:abc123def456|text|100>>>"), Some("abc123def456"));
	assert_eq!(Hash("<<<CCR:abc|code_rust|5000>>>"), Some("abc"));
}

#[test]
fn TestParseMarkerInvalid() {
	assert_eq!(Hash("not a marker"), None);
	assert_eq!(Hash("<<<CCR:"), None);
}

#[test]
fn TestFindMarkersSingle() {
	let Content = "hello <<<CCR:abc|text|100>>> world";
	let Markers = Find(Content);

	assert_eq!(Markers.len(), 1);
	assert_eq!(Markers[0].Marker, "<<<CCR:abc|text|100>>>");
	assert_eq!(Markers[0].Hash, "abc");
	assert_eq!(Markers[0].Span, 6..28, "the span must be the marker's own byte range");
}

#[test]
fn TestFindMarkersMultiple() {
	let Content = "<<<CCR:a|t|1>>> middle <<<CCR:b|t|2>>>";
	assert_eq!(Find(Content).len(), 2);
}

#[test]
fn TestFindMarkersNone() {
	assert!(Find("plain text").is_empty());
}

// ── T7: negative-path tests ──────────────────────────────────
#[test]
fn TestFindMarkersNestedUnclosedIsSkipped() {
	// A marker prefix inside another marker's body: the grammar has no nesting, so the first
	// closing delimiter ends the marker and the inner prefix is part of the hash field.
	let Content = "<<<CCR:a<<<CCR:b|t|1>>>";
	let Markers = Find(Content);

	assert_eq!(Markers.len(), 1);
	assert_eq!(Markers[0].Hash, "a<<<CCR:b");
}

#[test]
fn TestFindMarkersUnterminatedNoPanic() {
	assert!(Find("<<<CCR:").is_empty());
	assert!(Find("text <<<CCR:abc|t|1").is_empty());
}

// ── The tolerance policy: a complete marker naming no hash is not a reference, so it is not
// an occurrence. It used to be emitted with an empty hash, which then missed the store and
// was reported as an unresolved reference - the wrong reason for the wrong thing. ──
#[test]
fn TestParseMarkerHashEmptyHash() {
	assert_eq!(Hash("<<<CCR:>>>"), None);
	assert_eq!(Hash("<<<CCR:|text|1>>>"), None);
}
