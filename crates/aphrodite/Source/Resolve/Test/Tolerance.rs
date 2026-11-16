//! The grammar-tolerance policy, as executable cases.
//!
//! `Parse::Tolerance` states four answers to text that is not a usable reference. This file
//! is the same table as tests, one case per row, so a change to the policy has to be made
//! deliberately in both places:
//!
//! - unterminated - stepped over, the scan continues;
//! - hashless - not an occurrence, the scan continues;
//! - reference - emitted and resolved;
//! - unresolved - the original marker text stays in place.

use crate::{
	Resolve::{Expand::Fn as Expand, Parse::Find::Fn as Find},
	State::Struct::AphroditeState,
};

// ── Unterminated: an opening delimiter with no closing one is not the end of the text, and it
// is not a marker either. ──
#[test]
fn TestUnterminatedPrefixIsSteppedOver() {
	assert!(Find("<<<CCR:").is_empty());
	assert!(Find("text <<<CCR:abc|t|1").is_empty());
}

#[test]
fn TestUnterminatedPrefixDoesNotHideAnEarlierReference() {
	// The unterminated prefix comes LAST, so it is stepped over and the complete reference
	// before it is still found.
	let Markers = Find("<<<CCR:ok|t|1>>> and then <<<CCR:unfinished");

	assert_eq!(Markers.len(), 1);
	assert_eq!(Markers[0].Hash, "ok");
}

// ── Hashless: a complete marker that names no hash is not an occurrence. ──
#[test]
fn TestHashlessMarkerIsNotAnOccurrence() {
	assert!(Find("<<<CCR:>>>").is_empty());
	assert!(Find("<<<CCR:|text|1>>>").is_empty());
}

#[test]
fn TestHashlessMarkerDoesNotHideItsNeighbour() {
	let Markers = Find("<<<CCR:>>> <<<CCR:real|t|1>>>");

	assert_eq!(Markers.len(), 1);
	assert_eq!(Markers[0].Hash, "real");
}

// ── Reference: a complete marker naming a hash is emitted, and the walk resolves it. ──
#[test]
fn TestReferenceIsResolved() {
	let mut s = AphroditeState::default();
	s.Store.Put("doc".into(), "before <<<CCR:real|t|1>>> after".into());
	s.Store.Put("real".into(), "RESOLVED".into());

	assert_eq!(Expand(&mut s, "doc"), Some("before RESOLVED after".to_string()));
}

// ── Unresolved: the reference whose hash the store does not hold keeps its original text, so a
// merely evicted or late entry can still heal (F1). ──
#[test]
fn TestUnresolvedReferenceKeepsItsOriginalText() {
	let mut s = AphroditeState::default();
	let Marker = "<<<CCR:missing|text|10>>>";
	s.Store.Put("doc".into(), format!("before {Marker} after"));

	assert_eq!(Expand(&mut s, "doc"), Some(format!("before {Marker} after")));
}
