//! The inline store: LRU order, entry cap, byte budget, and byte-exact
//! round-tripping of pathological content.
//!
//! The round-trip tests matter because the inline store is the retrieval
//! source of truth: unlike the marker preview, which sanitizes for safe
//! rendering, whatever goes in here must come out byte-for-byte identical.

use crate::State::Struct::AphroditeState;

#[test]
fn Roundtrip() {
	let mut s = AphroditeState::default();
	s.Put("abc".into(), "hello world".into());
	assert_eq!(s.Get("abc"), Some("hello world".into()));
}

#[test]
fn Missing() {
	let mut s = AphroditeState::default();
	assert_eq!(s.Get("nope"), None);
}

#[test]
fn Promotion() {
	let mut s = AphroditeState::default();
	s.Put("a".into(), "first".into());
	s.Put("b".into(), "second".into());
	// Get "a" promotes it to most-recent (front of `InlineOrder`)
	let _ = s.Get("a");
	// "a" should now be at the front of the LRU order (`LruCache::iter`
	// yields most-recent first).
	let Order:Vec<String> = s.InlineOrder.iter().map(|(k, _)| k.clone()).collect();
	assert_eq!(Order.first().map(String::as_str), Some("a"));
}

#[test]
fn Eviction() {
	let mut s = AphroditeState::default();
	// Fill beyond INLINE_MAX (500)
	(0..505).for_each(|i| {
		s.Put(format!("hash{}", i), format!("content{}", i));
	});
	assert!(s.InlineStore.len() <= 500);
	// Oldest should be evicted
	assert_eq!(s.Get("hash0"), None);
	// Newest should remain
	assert_eq!(s.Get("hash504"), Some("content504".into()));
}

// ── T11 (F11): byte-budget eviction ───────────────────────────
#[test]
fn Budget() {
	let mut s = AphroditeState::default();
	s.Set(10 * 1024 * 1024); // 10MB budget
	// 100 x 5MB entries (500MB total) - far beyond both the byte budget
	// and, at this size, would also never be reached by the 500-entry
	// cap, so this specifically exercises the byte accounting rather
	// than the pre-existing entry-count cap.
	let FiveMb = "x".repeat(5 * 1024 * 1024);
	(0..100).for_each(|i| {
		s.Put(format!("hash{i}"), FiveMb.clone());
	});
	assert!(
		s.Bytes() <= 10 * 1024 * 1024,
		"stored bytes ({}) must stay within the 10MB budget",
		s.Bytes()
	);
	// Oldest entries must be the ones evicted.
	assert_eq!(s.Get("hash0"), None);
	// The newest entry must survive.
	assert_eq!(s.Get("hash99"), Some(FiveMb));
}

#[test]
fn Megabytes() {
	let s = AphroditeState::default();
	assert_eq!(s.Budget(), 256 * 1024 * 1024);
}

#[test]
fn Lowering() {
	let mut s = AphroditeState::default();
	s.Put("a".into(), "x".repeat(1000));
	s.Put("b".into(), "x".repeat(1000));
	assert_eq!(s.Bytes(), 2000);
	// Below current usage (2000B) but large enough for the single
	// most-recent entry ("b", 1000B) to survive on its own.
	s.Set(1500);
	assert!(s.Bytes() <= 1500);
	// The most recently inserted entry ("b") must be the one kept.
	assert_eq!(s.Get("b"), Some("x".repeat(1000)));
	assert_eq!(s.Get("a"), None, "oldest entry must have been evicted to fit the new budget");
}

#[test]
fn Defaults() {
	let s = AphroditeState::default();
	assert_eq!(s.TurnCounter, 0);
	assert_eq!(s.ToolThreshold, 512);
	assert_eq!(s.TerminalThreshold, 256);
	assert!(s.ContextEngineEnabled);
}

#[test]
fn Overwrite() {
	let mut s = AphroditeState::default();
	s.Put("hash".into(), "v1".into());
	s.Put("hash".into(), "v2".into());
	assert_eq!(s.InlineStore.len(), 1);
	assert_eq!(s.Get("hash"), Some("v2".into()));
}

// ── 04-T9: pathological-input coverage for the inline store. Unlike
// `CCRMarker`'s preview (which deliberately sanitizes for safe rendering),
// the inline store is the retrieval source-of-truth - it must round-trip
// arbitrary content byte-for-byte, not mutate it. ──

#[test]
fn Nul() {
	let mut s = AphroditeState::default();
	let Content = "before\0after\0\0end";
	s.Put("hash".into(), Content.into());
	assert_eq!(s.Get("hash"), Some(Content.to_string()), "NUL bytes must survive intact");
}

#[test]
fn Multibyte() {
	let mut s = AphroditeState::default();
	// Mix of 1/2/3/4-byte UTF-8 sequences (ASCII, é, 中, emoji) so any
	// byte-oriented mishandling (rather than char-oriented) would show up.
	let Content = "a\u{00e9}\u{4e2d}\u{1f600}b".repeat(20);
	s.Put("hash".into(), Content.clone());
	assert_eq!(s.Get("hash"), Some(Content));
}

#[test]
fn Marker() {
	// Content that happens to already contain marker-shaped text (e.g. a
	// user pasted an example transcript) must round-trip unchanged - the
	// inline store has no reason to reinterpret or mangle it, unlike
	// ccr_marker's preview sanitization.
	let mut s = AphroditeState::default();
	let Content = "before <<<CCR:fake000|text|1>>> after";
	s.Put("hash".into(), Content.into());
	assert_eq!(s.Get("hash"), Some(Content.to_string()));
}
