//! The delta catalog: new markers, new files, and the opt-in intent hints.

use crate::{Session::Catalog::Fn::Fn as Catalog, State::Struct::AphroditeState};

#[test]
fn Markers() {
	let mut s = AphroditeState::default();
	assert!(Catalog(&mut s).is_empty());
	s.RecordMarker(crate::State::Marker::Struct::Struct {
		hash:"abc".into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text] hello".into(),
		Turn:1,
		Center:None,
		meta:None,
	});
	assert!(Catalog(&mut s).contains("+1 new compression"));
	assert!(Catalog(&mut s).contains("no change this turn"));
}

#[test]
fn Files() {
	let mut s = AphroditeState::default();
	// No files yet
	assert!(Catalog(&mut s).is_empty());
	// Add files the production way: record_file pushes to the FRONT of
	// the deque, so the delta must read from the front and reverse into
	// insertion order (Fix 16).
	s.RecordFile("src/a.rs".into(), "read_file".into());
	s.RecordFile("src/b.rs".into(), "write_file".into());
	let R1 = Catalog(&mut s);
	assert!(R1.contains("+2 new files"));
	assert!(R1.contains("src/a.rs (read_file), src/b.rs (write_file)"));
	// Same files, next turn - no change
	let R2 = Catalog(&mut s);
	assert!(R2.contains("no change"));
}

// ── catalog_intent_hints knob (prompt knob, default off) ──
#[test]
fn HintsOff() {
	let mut s = AphroditeState::default();
	s.RecordMarker(crate::State::Marker::Struct::Struct {
		hash:"aaaaaaaaaaaa111111111111".into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text] p".into(),
		Turn:1,
		Center:None,
		meta:None,
	});
	let r = Catalog(&mut s);
	assert!(!r.contains("intent=read"), "intent hints must be off by default: {r}");
}

#[test]
fn HintsOn() {
	let mut s = AphroditeState::default();
	s.CatalogIntentHints = true;
	[
		"aaaaaaaaaaaa111111111111",
		"bbbbbbbbbbbb222222222222",
		"cccccccccccc333333333333",
		"dddddddddddd444444444444",
	]
	.into_iter()
	.for_each(|h| {
		s.RecordMarker(crate::State::Marker::Struct::Struct {
			hash:h.into(),
			CCRType:"text".into(),
			Size:100,
			Preview:"[text] p".into(),
			Turn:1,
			Center:None,
			meta:None,
		});
	});
	let r = Catalog(&mut s);
	assert!(
		r.contains("intent=read | recent CCRs: bbbbbbbbbbbb cccccccccccc dddddddddddd"),
		"last 3 hashes in insertion order, 12 chars each: {r}"
	);
}
