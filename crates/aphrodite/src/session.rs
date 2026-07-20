//! Session lifecycle - port of plugins/aphrodite/_hooks/session.py
//!
//! Manages: turn counter, conversation index, referenced files, scanned
//! message index, marker-at-end-of-turn archive.

use crate::state::AphroditeState;

/// Handle session start - reset all per-session state.
pub fn on_session_start(state:&mut AphroditeState) -> serde_json::Value {
	state.turn_counter = 0;
	state.scanned_msg_idx = 0;
	state.conv_index.clear();
	state.recent_markers.clear();
	state.referenced_files.clear();
	state.tool_events.clear();
	state.ephemeral_directives.clear();
	state.manual_directive_turn = None;
	state.last_emitted_marker_count = 0;

	serde_json::json!({
		"status": "ok",
		"version": env!("CARGO_PKG_VERSION"),
		"engine": "aphrodite-ffi",
		"turn": 0,
		"inline_entries": state.inline_store.len(),
	})
}

/// Increment turn counter and return new value.
pub fn next_turn(state:&mut AphroditeState) -> usize {
	state.turn_counter += 1;
	state.scanned_msg_idx = 0; // reset scan position for new turn
	state.turn_counter
}

/// Archive a compression at end of turn into the conversation index.
/// Called from `hooks::post_llm_call` (report 06 F11/T13) with the last
/// marker recorded during the turn - previously unwired, so `conv_index`
/// stayed empty and `aphrodite_diff` always returned zero turns.
pub fn archive_turn(state:&mut AphroditeState, hash:&str, summary:&str, size:usize) {
	let turn = state.turn_counter;
	state.conv_index.insert(turn, (hash.to_string(), summary.to_string(), size));
	// Keep last 50 turns
	if state.conv_index.len() > 50 {
		let oldest = state.conv_index.keys().min().copied();
		if let Some(key) = oldest {
			state.conv_index.remove(&key);
		}
	}
}

/// Get the conversation index as a JSON-serializable value, sorted oldest
/// turn first.
pub fn get_conv_index(state:&AphroditeState) -> Vec<serde_json::Value> {
	let mut turns:Vec<_> = state.conv_index.iter().collect();
	turns.sort_by_key(|(k, _)| *k);
	turns
		.iter()
		.map(
			|(turn, (hash, summary, size))| serde_json::json!({"turn": turn, "hash": hash, "summary": summary, "size": size}),
		)
		.collect()
}

/// Per-turn catalog summary — delta-only emission (04-F1 fix).
///
/// Before: re-emitted the same 5 marker previews every turn — prompt-cache
/// poison, ~200-600 chars of pure repetition per turn, 15k chars over 50 turns
/// with zero information gain.
///
/// After: emits a delta line only when new markers landed this turn. When
/// nothing changed, emits a short cache-stable count line. Tracks via
/// `state.last_emitted_marker_count`.
pub fn catalog_summary(state:&mut AphroditeState) -> String {
	let current_count = state.recent_markers.len();
	let prev_count = state.last_emitted_marker_count;

	// Update the tracker for next turn
	state.last_emitted_marker_count = current_count;

	let mut parts = vec![];

	// ── Delta markers: only show what's new this turn ──
	let new_count = current_count.saturating_sub(prev_count);
	if new_count > 0 && current_count > 0 {
		let new_markers:Vec<_> = state.recent_markers.iter().rev().take(new_count).collect();
		let previews:Vec<String> = new_markers.iter().map(|m| m.preview.clone()).collect();
		parts.push(format!(
			"[Aphrodite] +{} new compression{} this turn: {}. {} total.",
			new_count,
			if new_count == 1 { "" } else { "s" },
			previews.join(", "),
			current_count,
		));
	} else if current_count > 0 {
		// No new markers this turn — cache-stable line
		parts.push(format!(
			"[Aphrodite] {} compressions (no change this turn).",
			current_count,
		));
	}

	let conv_count = state.conv_index.len();
	if conv_count > 0 {
		parts.push(format!("{} archived turns available for retrieval.", conv_count));
	}

	let file_count = state.referenced_files.len();
	if file_count > 0 {
		let files:Vec<String> = state
			.referenced_files
			.iter()
			.take(5)
			.map(|(p, t)| format!("{} ({})", p, t))
			.collect();
		parts.push(format!("{} referenced files: {}", file_count, files.join(", ")));
	}

	// Append active poll-worker count if any (only when flag enabled).
	if state.poll_worker_enabled {
		let active_polls:Vec<_> = state
			.bg_tasks
			.iter()
			.filter(|t| t.status == crate::poll_worker::BgStatus::Running)
			.collect();
		if !active_polls.is_empty() {
			let poll_line = format!(
				"{} active poll worker(s). Use process(action='poll').",
				active_polls.len()
			);
			parts.push(poll_line);
		}
	}

	parts.join(" ")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_session_start_resets() {
		let mut s = AphroditeState::default();
		s.turn_counter = 42;
		s.record_marker(crate::state::MarkerEntry {
			hash:"abc".into(),
			ccr_type:"text".into(),
			size:100,
			preview:"[text]".into(),
			turn:1,
			center:None,
			meta:None,
		});

		let r = on_session_start(&mut s);
		assert_eq!(r["status"], "ok");
		assert_eq!(s.turn_counter, 0);
		assert_eq!(s.recent_markers.len(), 0);
	}

	#[test]
	fn test_next_turn() {
		let mut s = AphroditeState::default();
		assert_eq!(next_turn(&mut s), 1);
		assert_eq!(next_turn(&mut s), 2);
		assert_eq!(s.scanned_msg_idx, 0);
	}

	#[test]
	fn test_archive_eviction() {
		let mut s = AphroditeState::default();
		for i in 0..55 {
			archive_turn(&mut s, &format!("hash{}", i), "summary", 100);
			s.turn_counter += 1;
		}
		assert!(s.conv_index.len() <= 50);
		assert!(!s.conv_index.contains_key(&1)); // oldest evicted
	}

	#[test]
	fn test_get_conv_index_sorted_oldest_first() {
		let mut s = AphroditeState::default();
		s.turn_counter = 2;
		archive_turn(&mut s, "hash2", "second", 20);
		s.turn_counter = 1;
		archive_turn(&mut s, "hash1", "first", 10);
		let turns = get_conv_index(&s);
		assert_eq!(turns.len(), 2);
		assert_eq!(turns[0]["turn"], 1);
		assert_eq!(turns[0]["hash"], "hash1");
		assert_eq!(turns[1]["turn"], 2);
		assert_eq!(turns[1]["hash"], "hash2");
	}

	#[test]
	fn test_catalog_summary_delta_on_new_markers() {
		let mut s = AphroditeState::default();
		// No markers yet — should be empty
		let r0 = catalog_summary(&mut s);
		assert!(r0.is_empty());

		// Add one marker — delta should show +1
		s.record_marker(crate::state::MarkerEntry {
			hash:"abc".into(), ccr_type:"text".into(), size:100,
			preview:"[text] hello".into(), turn:1, center:None, meta:None,
		});
		let r1 = catalog_summary(&mut s);
		assert!(r1.contains("+1 new compression"));
		assert!(r1.contains("[text] hello"));

		// Same markers, next turn — should say "no change"
		let r2 = catalog_summary(&mut s);
		assert!(r2.contains("no change this turn"));
	}

	#[test]
	fn test_catalog_summary_reset_on_session_start() {
		let mut s = AphroditeState::default();
		s.last_emitted_marker_count = 10;
		on_session_start(&mut s);
		assert_eq!(s.last_emitted_marker_count, 0);
	}
}
