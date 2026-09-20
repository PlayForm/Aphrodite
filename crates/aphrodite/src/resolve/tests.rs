use super::*;

#[test]
fn test_parse_marker_hash() {
	let hash = parse_marker_hash("<<<CCR:abc123def456|text|100>>>");
	assert_eq!(hash, Some("abc123def456".into()));

	let hash = parse_marker_hash("<<<CCR:abc|code_rust|5000>>>");
	assert_eq!(hash, Some("abc".into()));
}

#[test]
fn test_parse_marker_invalid() {
	assert_eq!(parse_marker_hash("not a marker"), None);
	assert_eq!(parse_marker_hash("<<<CCR:"), None);
}

#[test]
fn test_find_markers_single() {
	let content = "hello <<<CCR:abc|text|100>>> world";
	let markers = find_markers(content);
	assert_eq!(markers.len(), 1);
	assert_eq!(markers[0].0, "<<<CCR:abc|text|100>>>");
	assert_eq!(markers[0].1, "abc");
}

#[test]
fn test_find_markers_multiple() {
	let content = "<<<CCR:a|t|1>>> middle <<<CCR:b|t|2>>>";
	let markers = find_markers(content);
	assert_eq!(markers.len(), 2);
}

#[test]
fn test_find_markers_none() {
	assert!(find_markers("plain text").is_empty());
}

#[test]
fn test_filter_empty_query() {
	assert_eq!(filter_lines("hello\nworld", ""), "hello\nworld");
}

#[test]
fn test_filter_matching() {
	let content = "line one\nerror: broke\nline two\nERROR: fatal\n";
	let result = filter_lines(content, "error");
	assert!(result.contains("error: broke"));
	assert!(result.contains("ERROR: fatal"));
	assert!(!result.contains("line one"));
}

#[test]
fn test_filter_no_match() {
	let content = "hello\nworld\n";
	let result = filter_lines(content, "nonexistent");
	assert!(result.contains("[aphrodite: no lines matched"));
}

#[test]
fn test_resolve_one_inline() {
	let mut s = AphroditeState::default();
	s.inline_store_put("abc123".into(), "hello world".into());
	assert_eq!(resolve_one(&mut s, "abc123"), Some("hello world".into()));
}

#[test]
fn test_resolve_one_missing() {
	let mut s = AphroditeState::default();
	assert_eq!(resolve_one(&mut s, "nonexistent"), None);
}

// ── T8 (F6): the `{hash}#stage2` shadowing lookup was deleted, not
// wired up - a plain resolve of `h` must always return the ORIGINAL
// content, even if some other entry happens to be stored under the
// `h#stage2` naming convention (nothing in this crate writes that key
// today, but this pins that a plain lookup can never be silently
// shadowed by it in the future without an explicit code change here).
#[test]
fn test_resolve_one_never_shadowed_by_stage2_key() {
	let mut s = AphroditeState::default();
	s.inline_store_put("h".into(), "ORIGINAL".into());
	s.inline_store_put("h#stage2".into(), "REDUCED".into());
	assert_eq!(resolve_one(&mut s, "h"), Some("ORIGINAL".to_string()));
}

// ── T15 (F2): regression test for corpus example 14_hash_extraction.py ──
// The original bug: the LLM sometimes echoes the full marker body
// ("hash|type|size") back as the `hash` argument instead of the bare
// hash, and a naive exact-match lookup misses. `resolve_one` (and thus
// every caller: aphrodite_retrieve tool, execute_tool_relay) must
// tolerate the pipe-suffixed form the same way `parse_marker_hash` does
// for markers found in text.
#[test]
fn regression_resolve_one_tolerates_pipe_suffixed_hash() {
	let mut s = AphroditeState::default();
	s.inline_store_put("abc123".into(), "<the real content>".into());

	assert_eq!(resolve_one(&mut s, "abc123"), Some("<the real content>".into()));
	assert_eq!(
		resolve_one(&mut s, "abc123|tool|1024"),
		Some("<the real content>".into()),
		"a pipe-suffixed hash (as an LLM might echo a full marker body) must still resolve"
	);
	assert_eq!(
		resolve_one(&mut s, "  abc123  "),
		Some("<the real content>".into()),
		"whitespace around the hash must be tolerated too"
	);
}

#[test]
fn test_resolve_one_i_prefix() {
	let mut s = AphroditeState::default();
	s.inline_store_put("i:abc123def".into(), "inline content".into());
	assert_eq!(resolve_one(&mut s, "i:abc123def"), Some("inline content".into()));
}

#[test]
fn test_expand_simple() {
	let mut s = AphroditeState::default();
	s.inline_store_put("simple".into(), "just content".into());
	assert_eq!(expand(&mut s, "simple"), Some("just content".into()));
}

#[test]
fn test_expand_with_nesting() {
	let mut s = AphroditeState::default();
	let inner = "<<<CCR:inner123|text|10>>>";
	s.inline_store_put("outer".into(), format!("before {} after", inner));
	s.inline_store_put("inner123".into(), "RESOLVED".into());
	let result = expand(&mut s, "outer");
	assert_eq!(result, Some("before RESOLVED after".into()));
}

#[test]
fn test_expand_unresolved() {
	// F1: an unresolvable nested hash leaves the ORIGINAL marker text
	// untouched rather than substituting `[CCR_UNRESOLVED:...]` - the
	// nested entry may simply be evicted/late and could still heal if
	// re-resolved later; baking in a placeholder (and, pre-fix,
	// persisting it back over the original content) made that
	// impossible.
	let mut s = AphroditeState::default();
	s.inline_store_put("outer".into(), "before <<<CCR:missing|text|10>>> after".into());
	let result = expand(&mut s, "outer");
	assert_eq!(result, Some("before <<<CCR:missing|text|10>>> after".to_string()));
}

#[test]
fn test_expand_missing_hash() {
	let mut s = AphroditeState::default();
	assert_eq!(expand(&mut s, "nope"), None);
}

#[test]
fn test_expand_deep_nesting_respects_limit() {
	// F9: at the depth limit, the leaf hash's RAW content is returned
	// (via `resolve_one`) rather than an unresolved placeholder - the
	// content genuinely exists in the store; the depth limit only stops
	// *further* recursive expansion, it must not misreport presence.
	let mut s = AphroditeState::default();
	s.inline_store_put("h0".into(), "<<<CCR:h1|t|1>>>".into());
	s.inline_store_put("h1".into(), "<<<CCR:h2|t|1>>>".into());
	s.inline_store_put("h2".into(), "<<<CCR:h3|t|1>>>".into());
	s.inline_store_put("h3".into(), "<<<CCR:h4|t|1>>>".into());
	s.inline_store_put("h4".into(), "<<<CCR:h5|t|1>>>".into());
	s.inline_store_put("h5".into(), "DEEP".into());
	let result = expand(&mut s, "h0");
	assert_eq!(result, Some("DEEP".to_string()));
}

#[test]
fn test_expand_cycle_safe() {
	// Self-referential: hA -> hB -> hA. Must terminate (no infinite
	// loop/stack overflow) and must not leak a raw, un-cache-substituted
	// marker into the output - F4's fix means the second visit to hA
	// (while still expanding hB) now correctly substitutes hA's cached
	// pre-expansion content instead of being skipped outright.
	let mut s = AphroditeState::default();
	s.inline_store_put("hA".into(), "<<<CCR:hB|t|1>>>".into());
	s.inline_store_put("hB".into(), "<<<CCR:hA|t|1>>>".into());
	let result = expand(&mut s, "hA").unwrap();
	assert!(
		!result.contains("[CCR_UNRESOLVED"),
		"cycle must not surface as unresolved: {result}"
	);
	assert_eq!(result, "<<<CCR:hB|t|1>>>");
}

// ── T7: negative-path tests ──────────────────────────────────
#[test]
fn test_find_markers_nested_unclosed_is_skipped() {
	// A marker prefix inside another unterminated marker: the parser must
	// not panic and must not treat the inner "<<<CCR:" as a fresh match
	// once the outer one already consumed up through its own ">>>".
	let content = "<<<CCR:a<<<CCR:b|t|1>>>";
	let markers = find_markers(content);
	assert_eq!(markers.len(), 1);
	assert_eq!(markers[0].1, "a<<<CCR:b");
}

#[test]
fn test_find_markers_unterminated_no_panic() {
	assert!(find_markers("<<<CCR:").is_empty());
	assert!(find_markers("text <<<CCR:abc|t|1").is_empty());
}

#[test]
fn test_parse_marker_hash_empty_hash() {
	assert_eq!(parse_marker_hash("<<<CCR:>>>"), Some(String::new()));
}

// ── T8: property tests ───────────────────────────────────────
use proptest::{prop_assert_eq, proptest};

proptest! {
	/// `find_markers` must never panic on arbitrary input, and every
	/// marker it returns must round-trip through `parse_marker_hash`
	/// back to the same hash it was extracted with.
	#[test]
	fn prop_find_markers_never_panics_and_roundtrips(s in ".*") {
		let markers = find_markers(&s);
		for (marker, hash) in markers {
			let parsed = parse_marker_hash(&marker);
			prop_assert_eq!(parsed, Some(hash));
		}
	}
}

// ── T3 (F1): the stored original must never be overwritten by an
// expanded result, and content that merely LOOKS like a marker must
// survive a round-trip unmangled. ──
#[test]
fn test_expand_never_writes_back_over_the_original_key() {
	let mut s = AphroditeState::default();
	let original = "before <<<CCR:inner|t|1>>> after";
	s.inline_store_put("outer".into(), original.to_string());
	s.inline_store_put("inner".into(), "X".to_string());

	let _ = expand(&mut s, "outer");
	// The raw stored entry under "outer" must be untouched - not
	// silently replaced with the expanded "before X after".
	assert_eq!(resolve_one(&mut s, "outer"), Some(original.to_string()));
}

#[test]
fn test_expand_is_idempotent() {
	let mut s = AphroditeState::default();
	s.inline_store_put("outer".into(), "<<<CCR:inner|t|1>>>".into());
	s.inline_store_put("inner".into(), "X".into());
	let first = expand(&mut s, "outer");
	let second = expand(&mut s, "outer");
	assert_eq!(first, second);
	assert_eq!(first, Some("X".to_string()));
}

#[test]
fn test_expand_preserves_literal_marker_shaped_text_in_content() {
	// Content that isn't itself a real nested marker but happens to
	// contain marker-shaped text (e.g. this crate's own doc/test
	// strings) must never be mistaken for a marker to resolve away -
	// here it simply has no matching stored hash, so F1's fix (keep
	// the original text on failed resolution) is what protects it.
	let mut s = AphroditeState::default();
	let content = "see the format <<<CCR:deadbeef|text|4>>> for reference";
	s.inline_store_put("doc".into(), content.to_string());
	let result = expand(&mut s, "doc");
	assert_eq!(result, Some(content.to_string()));
}

// ── T2 (F4): diamond-shaped nesting must fully expand every reference,
// including one reached a second time through a different path. ──
#[test]
fn test_expand_diamond_nesting_fully_expands() {
	let mut s = AphroditeState::default();
	// outer references both `a` and `b`; `b` also references `a`.
	s.inline_store_put("outer".into(), "<<<CCR:a|t|1>>> and <<<CCR:b|t|1>>>".into());
	s.inline_store_put("a".into(), "A".into());
	s.inline_store_put("b".into(), "wraps <<<CCR:a|t|1>>>".into());
	let result = expand(&mut s, "outer").unwrap();
	assert!(!result.contains("<<<CCR:"), "no raw marker should remain: {result}");
	assert_eq!(result, "A and wraps A");
}
