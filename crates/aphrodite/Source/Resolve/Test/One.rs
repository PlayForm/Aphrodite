//! One hash, one lookup.
//!
//! Pins what a plain resolve may and may not do: it returns the stored content, it
//! tolerates the marker-body and whitespace-padded forms the model actually sends,
//! it honours the `i:` inline prefix, and it can never be shadowed by a
//! `{hash}#stage2` key.

use crate::{Resolve::One::Fn as ResolveOne, State::Struct::AphroditeState};

#[test]
fn TestResolveOneInline() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("abc123".into(), "hello world".into());
	assert_eq!(ResolveOne(&mut s, "abc123"), Some("hello world".into()));
}

#[test]
fn TestResolveOneMissing() {
	let mut s = AphroditeState::default();
	assert_eq!(ResolveOne(&mut s, "nonexistent"), None);
}

// ── T8 (F6): the `{hash}#stage2` shadowing lookup was deleted, not
// wired up - a plain resolve of `h` must always return the ORIGINAL
// content, even if some other entry happens to be stored under the
// `h#stage2` naming convention (nothing in this crate writes that key
// today, but this pins that a plain lookup can never be silently
// shadowed by it in the future without an explicit code change here).
#[test]
fn TestResolveOneNeverShadowedByStage2Key() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("h".into(), "ORIGINAL".into());
	s.InlineStorePut("h#stage2".into(), "REDUCED".into());
	assert_eq!(ResolveOne(&mut s, "h"), Some("ORIGINAL".to_string()));
}

// ── T15 (F2): regression test for corpus example 14_hash_extraction.py ──
// The original bug: the LLM sometimes echoes the full marker body
// ("hash|type|size") back as the `hash` argument instead of the bare
// hash, and a naive exact-match lookup misses. `ResolveOne` (and thus
// every caller: aphrodite_retrieve tool, execute_tool_relay) must
// tolerate the pipe-suffixed form the same way `ParseMarkerHash` does
// for markers found in text.
#[test]
fn RegressionResolveOneToleratesPipeSuffixedHash() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("abc123".into(), "<the real content>".into());

	assert_eq!(ResolveOne(&mut s, "abc123"), Some("<the real content>".into()));
	assert_eq!(
		ResolveOne(&mut s, "abc123|tool|1024"),
		Some("<the real content>".into()),
		"a pipe-suffixed hash (as an LLM might echo a full marker body) must still resolve"
	);
	assert_eq!(
		ResolveOne(&mut s, "  abc123  "),
		Some("<the real content>".into()),
		"whitespace around the hash must be tolerated too"
	);
}

#[test]
fn TestResolveOneIPrefix() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("i:abc123def".into(), "inline content".into());
	assert_eq!(ResolveOne(&mut s, "i:abc123def"), Some("inline content".into()));
}
