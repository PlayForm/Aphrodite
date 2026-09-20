use super::*;

#[test]
fn test_inline_store_put_get() {
	let mut s = AphroditeState::default();
	s.inline_store_put("abc".into(), "hello world".into());
	assert_eq!(s.inline_store_get("abc"), Some("hello world".into()));
}

#[test]
fn test_inline_store_missing() {
	let mut s = AphroditeState::default();
	assert_eq!(s.inline_store_get("nope"), None);
}

#[test]
fn test_inline_store_lru_promotion() {
	let mut s = AphroditeState::default();
	s.inline_store_put("a".into(), "first".into());
	s.inline_store_put("b".into(), "second".into());
	// Get "a" promotes it to front (most-recent in `inline_order`)
	let _ = s.inline_store_get("a");
	// "a" should now be at the front of the LRU order
	assert_eq!(s.inline_order.front().map(|h| h.as_str()), Some("a"));
}

#[test]
fn test_inline_store_eviction() {
	let mut s = AphroditeState::default();
	// Fill beyond INLINE_MAX (500)
	for i in 0..505 {
		s.inline_store_put(format!("hash{}", i), format!("content{}", i));
	}
	assert!(s.inline_store.len() <= 500);
	// Oldest should be evicted
	assert_eq!(s.inline_store_get("hash0"), None);
	// Newest should remain
	assert_eq!(s.inline_store_get("hash504"), Some("content504".into()));
}

// ── T11 (F11): byte-budget eviction ───────────────────────────
#[test]
fn test_inline_store_byte_budget_evicts_oldest_first() {
	let mut s = AphroditeState::default();
	s.set_inline_store_byte_budget(10 * 1024 * 1024); // 10MB budget
	// 100 x 5MB entries (500MB total) - far beyond both the byte budget
	// and, at this size, would also never be reached by the 500-entry
	// cap, so this specifically exercises the byte accounting rather
	// than the pre-existing entry-count cap.
	let five_mb = "x".repeat(5 * 1024 * 1024);
	for i in 0..100 {
		s.inline_store_put(format!("hash{i}"), five_mb.clone());
	}
	assert!(
		s.inline_store_bytes() <= 10 * 1024 * 1024,
		"stored bytes ({}) must stay within the 10MB budget",
		s.inline_store_bytes()
	);
	// Oldest entries must be the ones evicted.
	assert_eq!(s.inline_store_get("hash0"), None);
	// The newest entry must survive.
	assert_eq!(s.inline_store_get("hash99"), Some(five_mb));
}

#[test]
fn test_inline_store_default_byte_budget_is_256mb() {
	let s = AphroditeState::default();
	assert_eq!(s.inline_store_byte_budget(), 256 * 1024 * 1024);
}

#[test]
fn test_inline_store_lowering_budget_evicts_immediately() {
	let mut s = AphroditeState::default();
	s.inline_store_put("a".into(), "x".repeat(1000));
	s.inline_store_put("b".into(), "x".repeat(1000));
	assert_eq!(s.inline_store_bytes(), 2000);
	// Below current usage (2000B) but large enough for the single
	// most-recent entry ("b", 1000B) to survive on its own.
	s.set_inline_store_byte_budget(1500);
	assert!(s.inline_store_bytes() <= 1500);
	// The most recently inserted entry ("b") must be the one kept.
	assert_eq!(s.inline_store_get("b"), Some("x".repeat(1000)));
	assert_eq!(
		s.inline_store_get("a"),
		None,
		"oldest entry must have been evicted to fit the new budget"
	);
}

#[test]
fn test_record_marker_eviction() {
	let mut s = AphroditeState::default();
	for i in 0..250 {
		s.record_marker(MarkerEntry {
			hash:format!("h{}", i),
			ccr_type:"text".into(),
			size:100,
			preview:"[text]".into(),
			turn:i,
			center:None,
			meta:None,
		});
	}
	assert!(s.recent_markers.len() <= 200);
	assert_eq!(s.recent_markers[0].hash, "h50"); // First 50 evicted
}

#[test]
fn test_record_file_dedup() {
	let mut s = AphroditeState::default();
	s.record_file("/tmp/a".into(), "read".into());
	s.record_file("/tmp/a".into(), "write".into());
	assert_eq!(s.referenced_files.len(), 1);
	assert_eq!(s.referenced_files[0].1, "write"); // Updated tool
}

#[test]
fn test_default_values() {
	let s = AphroditeState::default();
	assert_eq!(s.turn_counter, 0);
	assert_eq!(s.tool_threshold, 512);
	assert_eq!(s.terminal_threshold, 256);
	assert!(s.context_engine_enabled);
}

#[test]
fn test_inline_store_put_overwrite() {
	let mut s = AphroditeState::default();
	s.inline_store_put("hash".into(), "v1".into());
	s.inline_store_put("hash".into(), "v2".into());
	assert_eq!(s.inline_store.len(), 1);
	assert_eq!(s.inline_store_get("hash"), Some("v2".into()));
}

// ── 04-T9: pathological-input coverage for the inline store. Unlike
// `ccr_marker`'s preview (which deliberately sanitizes for safe rendering),
// the inline store is the retrieval source-of-truth - it must round-trip
// arbitrary content byte-for-byte, not mutate it. ──

#[test]
fn test_inline_store_roundtrips_interior_nul_bytes() {
	let mut s = AphroditeState::default();
	let content = "before\0after\0\0end";
	s.inline_store_put("hash".into(), content.into());
	assert_eq!(
		s.inline_store_get("hash"),
		Some(content.to_string()),
		"NUL bytes must survive intact"
	);
}

#[test]
fn test_inline_store_roundtrips_multibyte_utf8_at_every_boundary() {
	let mut s = AphroditeState::default();
	// Mix of 1/2/3/4-byte UTF-8 sequences (ASCII, é, 中, emoji) so any
	// byte-oriented mishandling (rather than char-oriented) would show up.
	let content = "a\u{00e9}\u{4e2d}\u{1f600}b".repeat(20);
	s.inline_store_put("hash".into(), content.clone());
	assert_eq!(s.inline_store_get("hash"), Some(content));
}

#[test]
fn test_inline_store_roundtrips_literal_marker_shaped_content() {
	// Content that happens to already contain marker-shaped text (e.g. a
	// user pasted an example transcript) must round-trip unchanged - the
	// inline store has no reason to reinterpret or mangle it, unlike
	// ccr_marker's preview sanitization.
	let mut s = AphroditeState::default();
	let content = "before <<<CCR:fake000|text|1>>> after";
	s.inline_store_put("hash".into(), content.into());
	assert_eq!(s.inline_store_get("hash"), Some(content.to_string()));
}

// ── Tier 1 teaching loop: adaptive split threshold ──

#[test]
fn test_record_chain_split_registers_hashes_and_counts_retrieval_once() {
	let mut s = AphroditeState::default();
	let hashes = vec!["h1".into(), "h2".into(), "h3".into()];
	s.record_chain_split(hashes);
	assert_eq!(s.split_events.len(), 1);
	assert_eq!(s.split_events[0].produced, 3);
	assert_eq!(s.split_events[0].retrieved, 0);

	// Retrieving a segment hash attributes the consequence once.
	assert_eq!(s.note_split_retrieval("h2"), Some(1));
	assert_eq!(s.split_events[0].retrieved, 1);
	// Re-retrieving the same hash must not double-count.
	assert_eq!(s.note_split_retrieval("h2"), None);
	assert_eq!(s.split_events[0].retrieved, 1);
	// Unknown hashes are not attributed.
	assert_eq!(s.note_split_retrieval("nope"), None);
}

#[test]
fn test_split_event_ring_is_bounded() {
	let mut s = AphroditeState::default();
	for i in 0..40 {
		s.record_chain_split(vec![format!("h{i}")]);
	}
	assert!(s.split_events.len() <= SPLIT_EVENT_CAP);
	assert_eq!(s.split_events.len(), SPLIT_EVENT_CAP);
}

#[test]
fn test_adapt_raises_threshold_when_segments_ignored() {
	let mut s = AphroditeState {
		chain_split_min_segments:2,
		chain_split_floor:2,
		chain_split_max_segments:6,
		..Default::default()
	};
	// 4 splits, all segments ignored: ratio 0/8 = 0.0 < 0.25 → raise.
	for i in 0..4 {
		s.record_chain_split(vec![format!("a{i}"), format!("b{i}")]);
	}
	assert_eq!(s.chain_split_min_segments, 3);
}

#[test]
fn test_adapt_keeps_threshold_when_segments_retrieved() {
	let mut s = AphroditeState {
		chain_split_min_segments:2,
		chain_split_floor:2,
		chain_split_max_segments:6,
		..Default::default()
	};
	// 4 splits, all segments retrieved: ratio 8/8 = 1.0 ≥ 0.5 → floor.
	for i in 0..4 {
		let hashes = vec![format!("a{i}"), format!("b{i}")];
		s.record_chain_split(hashes.clone());
		for h in hashes {
			s.note_split_retrieval(&h);
		}
	}
	assert_eq!(s.chain_split_min_segments, 2);
}

#[test]
fn test_adapt_respects_bounds() {
	let mut s = AphroditeState {
		chain_split_min_segments:5,
		chain_split_floor:5,
		chain_split_max_segments:6,
		..Default::default()
	};
	// Ignored segments: threshold rises, but never past max.
	for i in 0..20 {
		s.record_chain_split(vec![format!("a{i}"), format!("b{i}")]);
	}
	assert_eq!(s.chain_split_min_segments, 6);

	// Fully-retrieved segments: falls, but never below floor.
	for i in 20..60 {
		let hashes = vec![format!("a{i}"), format!("b{i}")];
		s.record_chain_split(hashes.clone());
		for h in hashes {
			s.note_split_retrieval(&h);
		}
	}
	assert_eq!(s.chain_split_min_segments, 5);
}

#[test]
fn test_adapt_requires_min_events() {
	let mut s = AphroditeState::default();
	// Fewer than SPLIT_ADAPT_MIN_EVENTS: no adaptation.
	for i in 0..(SPLIT_ADAPT_MIN_EVENTS - 1) {
		s.record_chain_split(vec![format!("a{i}"), format!("b{i}")]);
	}
	assert_eq!(s.chain_split_min_segments, 2);
}
