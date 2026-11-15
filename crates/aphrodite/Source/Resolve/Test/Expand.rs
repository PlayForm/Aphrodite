//! The expansion entry point.
//!
//! Pins what `Expand` returns for a plain hash, a nested one, a missing one, and
//! an unresolvable nested marker - and pins the write-back invariant: an expanded
//! result must never land in the store over the original, and expansion must be
//! repeatable with the same answer.

use crate::{
	Resolve::{Expand::Fn as Expand, One::Fn as ResolveOne},
	State::Struct::AphroditeState,
};

#[test]
fn TestExpandSimple() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("simple".into(), "just content".into());
	assert_eq!(Expand(&mut s, "simple"), Some("just content".into()));
}

#[test]
fn TestExpandWithNesting() {
	let mut s = AphroditeState::default();
	let Inner = "<<<CCR:inner123|text|10>>>";
	s.InlineStorePut("outer".into(), format!("before {} after", Inner));
	s.InlineStorePut("inner123".into(), "RESOLVED".into());
	let Result = Expand(&mut s, "outer");
	assert_eq!(Result, Some("before RESOLVED after".into()));
}

#[test]
fn TestExpandUnresolved() {
	// F1: an unresolvable nested hash leaves the ORIGINAL marker text
	// untouched rather than substituting `[CCR_UNRESOLVED:...]` - the
	// nested entry may simply be evicted/late and could still heal if
	// re-resolved later; baking in a placeholder (and, pre-fix,
	// persisting it back over the original content) made that
	// impossible.
	let mut s = AphroditeState::default();
	s.InlineStorePut("outer".into(), "before <<<CCR:missing|text|10>>> after".into());
	let Result = Expand(&mut s, "outer");
	assert_eq!(Result, Some("before <<<CCR:missing|text|10>>> after".to_string()));
}

#[test]
fn TestExpandMissingHash() {
	let mut s = AphroditeState::default();
	assert_eq!(Expand(&mut s, "nope"), None);
}

// ── T3 (F1): the stored original must never be overwritten by an
// expanded result, and content that merely LOOKS like a marker must
// survive a round-trip unmangled. ──
#[test]
fn TestExpandNeverWritesBackOverTheOriginalKey() {
	let mut s = AphroditeState::default();
	let Original = "before <<<CCR:inner|t|1>>> after";
	s.InlineStorePut("outer".into(), Original.to_string());
	s.InlineStorePut("inner".into(), "X".to_string());

	let _ = Expand(&mut s, "outer");
	// The raw stored entry under "outer" must be untouched - not
	// silently replaced with the expanded "before X after".
	assert_eq!(ResolveOne(&mut s, "outer"), Some(Original.to_string()));
}

#[test]
fn TestExpandIsIdempotent() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("outer".into(), "<<<CCR:inner|t|1>>>".into());
	s.InlineStorePut("inner".into(), "X".into());
	let First = Expand(&mut s, "outer");
	let Second = Expand(&mut s, "outer");
	assert_eq!(First, Second);
	assert_eq!(First, Some("X".to_string()));
}

#[test]
fn TestExpandPreservesLiteralMarkerShapedTextInContent() {
	// Content that isn't itself a real nested marker but happens to
	// contain marker-shaped text (e.g. this crate's own doc/test
	// strings) must never be mistaken for a marker to resolve away -
	// here it simply has no matching stored hash, so F1's fix (keep
	// the original text on failed resolution) is what protects it.
	let mut s = AphroditeState::default();
	let Content = "see the format <<<CCR:deadbeef|text|4>>> for reference";
	s.InlineStorePut("doc".into(), Content.to_string());
	let Result = Expand(&mut s, "doc");
	assert_eq!(Result, Some(Content.to_string()));
}
