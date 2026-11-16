//! The recursive walk: depth limit, cycles, shared references, and the size-bounded
//! auto-expand variant.
//!
//! Every test here names a resolution bug that already shipped once - a depth limit that
//! misreported presence, a cycle that leaked a raw marker, a cached reference that was
//! skipped, an auto-expand that inlined over its limit, a capped hash that stayed capped for
//! the rest of the tree, a substitution that re-read its own output - so they pin behaviour,
//! not implementation.

use crate::{
	Resolve::{Expand::Fn as Expand, Recursive::Inline::Fn as ResolveInline},
	State::Struct::AphroditeState,
};

#[test]
fn TestExpandDeepNestingRespectsLimit() {
	// F9: at the depth limit, the leaf hash's RAW content is returned (via the store) rather
	// than an unresolved placeholder - the content genuinely exists; the depth limit only
	// stops *further* recursive expansion, it must not misreport presence.
	let mut s = AphroditeState::default();
	s.InlineStorePut("h0".into(), "<<<CCR:h1|t|1>>>".into());
	s.InlineStorePut("h1".into(), "<<<CCR:h2|t|1>>>".into());
	s.InlineStorePut("h2".into(), "<<<CCR:h3|t|1>>>".into());
	s.InlineStorePut("h3".into(), "<<<CCR:h4|t|1>>>".into());
	s.InlineStorePut("h4".into(), "<<<CCR:h5|t|1>>>".into());
	s.InlineStorePut("h5".into(), "DEEP".into());

	assert_eq!(Expand(&mut s, "h0"), Some("DEEP".to_string()));
}

#[test]
fn TestExpandCycleSafe() {
	// Self-referential: hA -> hB -> hA. Must terminate (no infinite loop/stack overflow) and
	// must not leak a raw, un-cache-substituted marker into the output - F4's fix means the
	// second visit to hA (while still expanding hB) substitutes hA's cached pre-expansion
	// content instead of being skipped outright.
	let mut s = AphroditeState::default();
	s.InlineStorePut("hA".into(), "<<<CCR:hB|t|1>>>".into());
	s.InlineStorePut("hB".into(), "<<<CCR:hA|t|1>>>".into());

	let Result = Expand(&mut s, "hA").unwrap();

	assert!(
		!Result.contains("[CCR_UNRESOLVED"),
		"cycle must not surface as unresolved: {Result}"
	);
	assert_eq!(Result, "<<<CCR:hB|t|1>>>");
}

// ── T2 (F4): diamond-shaped nesting must fully expand every reference, including one reached
// a second time through a different path. ──
#[test]
fn TestExpandDiamondNestingFullyExpands() {
	let mut s = AphroditeState::default();
	// `outer` references both `a` and `b`; `b` also references `a`.
	s.InlineStorePut("outer".into(), "<<<CCR:a|t|1>>> and <<<CCR:b|t|1>>>".into());
	s.InlineStorePut("a".into(), "A".into());
	s.InlineStorePut("b".into(), "wraps <<<CCR:a|t|1>>>".into());

	let Result = Expand(&mut s, "outer").unwrap();

	assert!(!Result.contains("<<<CCR:"), "no raw marker should remain: {Result}");
	assert_eq!(Result, "A and wraps A");
}

// ── The path is a path, not a visited set: a hash first reached AT the cap must still expand
// when a later branch reaches it with depth left. The old never-popped set marked it for the
// whole tree, so the second reference was answered from a cache that had never been written
// and the marker survived into the output. ──
#[test]
fn TestExpandRevisitsACappedHashWithDepthRemaining() {
	let mut s = AphroditeState::default();
	s.RecursiveDepth = 2;
	// `a` reaches `z` at depth 2 (the cap); `top` also reaches `z` at depth 1.
	s.InlineStorePut("top".into(), "<<<CCR:a|t|1>>> and <<<CCR:z|t|1>>>".into());
	s.InlineStorePut("a".into(), "<<<CCR:z|t|1>>>".into());
	s.InlineStorePut("z".into(), "Z".into());

	assert_eq!(Expand(&mut s, "top"), Some("Z and Z".to_string()));
}

// ── The output is built from occurrence spans, not by re-searching it: a marker left in place
// at the depth cap IS marker text, and a later `replace` of that same marker string used to
// substitute inside the text that had just been inserted for it. ──
#[test]
fn TestExpandCappedReferenceKeepsItsMarkerText() {
	let mut s = AphroditeState::default();
	s.RecursiveDepth = 1;
	// At depth 1 both `a` and `b` are at the cap, so `a` is answered as stored - and its
	// stored text is itself the marker for `b`.
	s.InlineStorePut("outer".into(), "<<<CCR:a|t|1>>> <<<CCR:b|t|1>>>".into());
	s.InlineStorePut("a".into(), "<<<CCR:b|t|1>>>".into());
	s.InlineStorePut("b".into(), "B".into());

	assert_eq!(Expand(&mut s, "outer"), Some("<<<CCR:b|t|1>>> B".to_string()));
}

// ── Auto-expand (pre-Rust port): `ResolveInline` returns the fully resolved content only when
// it fits within the limit; over-limit results (and unresolvable hashes) return `None` so the
// caller keeps the marker. ──
#[test]
fn TestResolveInlineReturnsContentWithinLimit() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("outer".into(), "before <<<CCR:inner123|text|10>>> after".into());
	s.InlineStorePut("inner123".into(), "RESOLVED".into());

	let Result = ResolveInline(&mut s, "outer", 1000);

	assert_eq!(Result, Some("before RESOLVED after".into()));
	assert!(
		!Result.unwrap().contains("<<<CCR:"),
		"auto-expand must return the resolved content, not a marker"
	);
}

#[test]
fn TestResolveInlineOverLimitReturnsNone() {
	let mut s = AphroditeState::default();
	s.InlineStorePut("outer".into(), "before <<<CCR:inner123|text|10>>> after".into());
	s.InlineStorePut("inner123".into(), "RESOLVED".into());

	// The resolved content is 21 bytes ("before RESOLVED after") - a 10-byte limit must refuse
	// it, leaving the caller's marker in place.
	assert_eq!(ResolveInline(&mut s, "outer", 10), None);
}

#[test]
fn TestResolveInlineMissingHashReturnsNone() {
	assert_eq!(ResolveInline(&mut AphroditeState::default(), "nope", 1000), None);
}

#[test]
fn TestResolveInlineCycleIsSafe() {
	// Cycle-safe: a self-referential pair must terminate and must not surface a raw marker as
	// if it were resolved content.
	let mut s = AphroditeState::default();
	s.InlineStorePut("hA".into(), "<<<CCR:hB|t|1>>>".into());
	s.InlineStorePut("hB".into(), "<<<CCR:hA|t|1>>>".into());

	assert_eq!(ResolveInline(&mut s, "hA", 1000), Some("<<<CCR:hB|t|1>>>".to_string()));
}

// ── The budget governs CONSTRUCTION: a nested expansion that cannot fit is abandoned while it
// is being built, and the abandonment propagates instead of being mistaken for an unresolvable
// reference - inlining a document that still carries a marker is what the cap prevents. ──
#[test]
fn TestResolveInlineAbandonsAnOverBudgetNestedExpansion() {
	let mut s = AphroditeState::default();
	// `outer` itself is 30 bytes, but its only marker expands to a 4 KiB body: no budget of
	// 100 bytes can hold the result, so the answer must be `None` rather than a partial inline.
	s.InlineStorePut("outer".into(), "<<<CCR:big|t|4096>>>".into());
	s.InlineStorePut("big".into(), "x".repeat(4096));

	assert_eq!(ResolveInline(&mut s, "outer", 100), None);
}

// ── Pre-Rust port: `APHRODITE_RECURSIVE_DEPTH` - the depth limit is config-driven via
// `state.RecursiveDepth` (default 5, byte-identical to the pre-port hardcoded constant), not a
// fixed constant. ──
#[test]
fn TestExpandHonorsConfiguredRecursiveDepth() {
	// A chain deeper than the configured depth must stop expanding at the limit and return the
	// readable raw content of the deepest-visited leaf hash (F9) - never absent and never an
	// unresolved placeholder.
	let mut s = AphroditeState::default();
	s.RecursiveDepth = 2;
	s.InlineStorePut("h0".into(), "<<<CCR:h1|t|1>>>".into());
	s.InlineStorePut("h1".into(), "<<<CCR:h2|t|1>>>".into());
	s.InlineStorePut("h2".into(), "<<<CCR:h3|t|1>>>".into());
	s.InlineStorePut("h3".into(), "DEEP".into());

	assert_eq!(Expand(&mut s, "h0"), Some("<<<CCR:h3|t|1>>>".to_string()));
}

#[test]
fn TestExpandRecursiveDepthHigherThanDefaultExpandsFurther() {
	// A 6-deep chain exceeds the old hardcoded 5 but must fully expand when RecursiveDepth is
	// raised to 8 - proves the limit is the configured value, not a min() with the old constant.
	let mut s = AphroditeState::default();
	s.RecursiveDepth = 8;
	// h7 -> DEEP; h6 -> marker(h7); ...; h0 -> marker(h1): an 8-deep chain.
	s.InlineStorePut("h7".into(), "DEEP".into());
	(0..7).rev().for_each(|i| {
		s.InlineStorePut(format!("h{i}"), format!("<<<CCR:h{}|t|1>>>", i + 1));
	});

	assert_eq!(Expand(&mut s, "h0"), Some("DEEP".to_string()));
}

#[test]
fn TestExpandRecursiveDepthFiveIsByteIdenticalToDefault() {
	// The state default is 5 (the pre-port hardcoded constant): a default state and an
	// explicitly-configured 5 must produce identical output on the same chain, and the default
	// must be exactly 5.
	assert_eq!(AphroditeState::default().RecursiveDepth, 5);

	let mut SDefault = AphroditeState::default();
	let mut SFive = AphroditeState::default();
	SFive.RecursiveDepth = 5;

	[
		("h0", "<<<CCR:h1|t|1>>>"),
		("h1", "<<<CCR:h2|t|1>>>"),
		("h2", "<<<CCR:h3|t|1>>>"),
		("h3", "<<<CCR:h4|t|1>>>"),
		("h4", "<<<CCR:h5|t|1>>>"),
		("h5", "DEEP"),
	]
	.into_iter()
	.for_each(|(k, v)| {
		SDefault.InlineStorePut(k.into(), v.into());
		SFive.InlineStorePut(k.into(), v.into());
	});

	assert_eq!(Expand(&mut SDefault, "h0"), Expand(&mut SFive, "h0"));
	assert_eq!(Expand(&mut SDefault, "h0"), Some("DEEP".to_string()));
}
