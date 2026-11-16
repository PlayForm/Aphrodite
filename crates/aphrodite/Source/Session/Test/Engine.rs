//! The context-engine archiving gates.
//!
//! Every refusal path is asserted to leave the state COMPLETELY untouched -
//! same index, empty inline store, no recorded markers - because the engine
//! runs inside `PreLLMCall`, where a partial mutation would show the model a
//! history that no archive explains.

use crate::{Session::Engine::Fn::Fn as Engine, State::Struct::AphroditeState};

// ── Context-engine archiving (pre-Rust port of _engine.py) ──

/// Build a state with `Entries` archived turns (keys 1..=entries) and
/// default engine config (enabled, threshold 45, min 8, protect 2/5).
fn EngineState(Entries:usize) -> AphroditeState {
	let mut s = AphroditeState::default();
	(1..=Entries).for_each(|i| {
		s.Turn.Index.insert(i, (format!("h{i:02}"), format!("summary {i}"), i * 10));
	});
	s
}

/// Assert the engine left the state completely untouched: same
/// conversation index, empty inline store, no recorded markers.
fn Untouched(s:&AphroditeState) {
	assert!(s.Store.Map.is_empty(), "no archive may hit the inline store");
	assert!(s.Marker.Recent.is_empty(), "no marker may be recorded");
}

#[test]
fn Below() {
	// 10 archived turns = 20% fill; threshold 80 not crossed.
	let mut s = EngineState(10);
	s.Configuration.EngineThresholdPct = 80;
	let Before = s.Turn.Index.clone();
	assert!(Engine(&mut s).is_none());
	assert_eq!(s.Turn.Index, Before, "below threshold must not touch conv_index");
	Untouched(&s);
}

#[test]
fn Disabled() {
	let mut s = EngineState(12);
	s.Configuration.EngineThresholdPct = 20; // well over the fill threshold when enabled
	s.Configuration.ContextEngineEnabled = false;
	let Before = s.Turn.Index.clone();
	assert!(Engine(&mut s).is_none());
	assert_eq!(s.Turn.Index, Before, "disabled engine must be byte-identical no-op");
	Untouched(&s);
}

#[test]
fn Over() {
	let mut s = EngineState(12);
	s.Configuration.EngineThresholdPct = 20; // 12 entries = 24% fill
	s.Configuration.EngineMinimumMsgs = 8;
	s.Configuration.EngineProtectFirst = 2;
	s.Configuration.EngineProtectLast = 5;
	let Prompt = Engine(&mut s).expect("over threshold must archive");
	// Returned prompt carries the marker line AND the rendered guidance
	// (fallback references the hash for aphrodite_retrieve).
	let hash = headroom_core::ccr::compute_key(
		serde_json::json!([
			{"turn": 3, "hash": "h03", "summary": "summary 3", "size": 30},
			{"turn": 4, "hash": "h04", "summary": "summary 4", "size": 40},
			{"turn": 5, "hash": "h05", "summary": "summary 5", "size": 50},
			{"turn": 6, "hash": "h06", "summary": "summary 6", "size": 60},
			{"turn": 7, "hash": "h07", "summary": "summary 7", "size": 70},
		])
		.to_string()
		.as_bytes(),
	);
	assert!(Prompt.contains(&hash), "prompt must contain the archive hash: {Prompt}");
	assert!(Prompt.contains("<<<CCR:"), "prompt must carry the CCR marker line");
	assert!(
		Prompt.contains("Use aphrodite_retrieve"),
		"prompt must carry retrieval guidance"
	);
	// Middle replaced by the marker entry: keys 3..=7 gone, head (1,2)
	// and tail (8..=12) kept, marker entry recorded at the current turn.
	(3..=7).for_each(|t| {
		assert!(!s.Turn.Index.contains_key(&t), "middle turn {t} must be replaced");
	});
	[1, 2, 8, 9, 10, 11, 12].into_iter().for_each(|t| {
		assert!(s.Turn.Index.contains_key(&t), "protected turn {t} must survive");
	});
	assert_eq!(s.Turn.Index.len(), 8, "12 - 5 middle + 1 marker entry");
	let (MHash, MSummary, MSize) = s.Turn.Index.get(&0).expect("marker entry recorded");
	assert_eq!(MHash, &hash);
	assert!(MSummary.starts_with("engine offload: 5 middle turns archived"));
	assert!(*MSize > 0);
	// Archived middle must be retrievable from the inline store.
	assert!(s.Store.Map.contains_key(&hash), "archive payload must be stored inline");
}

#[test]
fn Protect() {
	// protect_first=2 / protect_last=5 on 12 turns: exactly keys 3..=7
	// (5 entries) are the middle; head/tail survive verbatim.
	let mut s = EngineState(12);
	s.Configuration.EngineThresholdPct = 20;
	s.Configuration.EngineMinimumMsgs = 8;
	s.Configuration.EngineProtectFirst = 2;
	s.Configuration.EngineProtectLast = 5;
	let _ = Engine(&mut s).expect("must archive");
	assert!(s.Turn.Index.contains_key(&1) && s.Turn.Index.contains_key(&2));
	assert!(s.Turn.Index.contains_key(&8) && s.Turn.Index.contains_key(&12));
	assert!(!s.Turn.Index.contains_key(&3) && !s.Turn.Index.contains_key(&7));
	assert_eq!(s.Turn.Index.len(), 8);
	// protect_first=0 keeps nothing at the start: middle starts at key 1.
	let mut S0 = EngineState(12);
	S0.Configuration.EngineThresholdPct = 20;
	S0.Configuration.EngineMinimumMsgs = 8;
	S0.Configuration.EngineProtectFirst = 0;
	S0.Configuration.EngineProtectLast = 5;
	let _ = Engine(&mut S0).expect("must archive");
	assert!(!S0.Turn.Index.contains_key(&1), "protect_first=0 keeps no head");
	assert!(S0.Turn.Index.contains_key(&8) && S0.Turn.Index.contains_key(&12));
}

#[test]
fn Minimum() {
	// 5 entries = 10% fill >= threshold 10, but 5 < min_msgs 8.
	let mut s = EngineState(5);
	s.Configuration.EngineThresholdPct = 10;
	s.Configuration.EngineMinimumMsgs = 8;
	let Before = s.Turn.Index.clone();
	assert!(Engine(&mut s).is_none());
	assert_eq!(s.Turn.Index, Before);
	Untouched(&s);
}

#[test]
fn Hundred() {
	// 100+ = documented engine-off escape hatch; also 0 = never.
	[0u64, 100, 200].into_iter().for_each(|Pct| {
		let mut s = EngineState(12);
		s.Configuration.EngineThresholdPct = Pct;
		let Before = s.Turn.Index.clone();
		assert!(Engine(&mut s).is_none(), "threshold {Pct} must leave the engine off");
		assert_eq!(s.Turn.Index, Before);
		Untouched(&s);
	});
}

#[test]
fn Small() {
	// protect 2/9 on 12 turns leaves a 1-entry middle (< pre-Rust floor 3).
	let mut s = EngineState(12);
	s.Configuration.EngineThresholdPct = 20;
	s.Configuration.EngineMinimumMsgs = 8;
	s.Configuration.EngineProtectFirst = 2;
	s.Configuration.EngineProtectLast = 9;
	let Before = s.Turn.Index.clone();
	assert!(Engine(&mut s).is_none());
	assert_eq!(s.Turn.Index, Before);
	Untouched(&s);
}

#[test]
fn Payload() {
	let mut s = EngineState(12);
	s.Configuration.EngineThresholdPct = 20;
	s.Configuration.EngineMinimumMsgs = 8;
	s.Configuration.EngineProtectFirst = 2;
	s.Configuration.EngineProtectLast = 5;
	let Prompt = Engine(&mut s).expect("must archive");
	// Extract the hash from the marker line and resolve it back.
	let hash = crate::Marker::Parse::Extract::Fn(&Prompt)
		.into_iter()
		.next()
		.expect("marker hash must be extractable");
	let Resolved = s.Store.Get(&hash).expect("archive must resolve from inline store");
	let Parsed:serde_json::Value = serde_json::from_str(&Resolved).expect("packed payload must be JSON");
	assert_eq!(Parsed.as_array().map(Vec::len), Some(5), "middle of 5 turns packed");
	assert_eq!(Parsed[0]["turn"], 3);
	assert_eq!(Parsed[4]["turn"], 7);
}
