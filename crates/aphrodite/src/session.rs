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
	state.last_emitted_file_count = 0;
	// Reset the adaptive chain-split threshold to the configured floor (the
	// initial value) so a threshold adapted in a previous session doesn't
	// leak into this one. Uses the configured floor, not a hardcoded number.
	state.chain_split_min_segments = state.chain_split_floor;

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
pub fn archive_turn(state:&mut AphroditeState, hash:&str, summary:&str, size:usize) {
	let turn = state.turn_counter;
	state.conv_index.insert(turn, (hash.to_string(), summary.to_string(), size));
	if state.conv_index.len() > 50 {
		let oldest = state.conv_index.keys().min().copied();
		if let Some(key) = oldest {
			state.conv_index.remove(&key);
		}
	}
}

/// Get the conversation index as a JSON-serializable value, sorted oldest first.
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

/// Per-turn catalog summary - delta-only emission for markers AND files (04-F1/F4).
///
/// Markers: +N new compressions this turn, or "no change" cache-stable line.
/// Files: +N new files this turn, or "no change" cache-stable line.
/// Archived turns: count only (changes slowly, ~40 chars, acceptable).
///
/// Once-per-turn contract: `flow::build_turn_context` is the only production
/// caller and invokes this exactly once per turn. The `last_emitted_*`
/// watermarks are committed only after the summary string is fully
/// assembled, so a failure while building the string can never desync the
/// delta bookkeeping from what was actually emitted; calling this again
/// within the same turn therefore reports "no change" for both deltas.
pub fn catalog_summary(state:&mut AphroditeState) -> String {
	let current_markers = state.recent_markers.len();
	let prev_markers = state.last_emitted_marker_count;

	let mut parts = vec![];

	// ── Delta markers ──
	let new_markers = current_markers.saturating_sub(prev_markers);
	if new_markers > 0 && current_markers > 0 {
		let previews:Vec<String> = state
			.recent_markers
			.iter()
			.rev()
			.take(new_markers)
			.map(|m| m.preview.clone())
			.collect();
		parts.push(format!(
			"[Aphrodite] +{} new compression{} this turn: {}. {} total.",
			new_markers,
			if new_markers == 1 { "" } else { "s" },
			previews.join(", "),
			current_markers,
		));
	} else if current_markers > 0 {
		parts.push(format!("[Aphrodite] {} compressions (no change this turn).", current_markers));
	}

	// ── Archived turns ──
	let conv_count = state.conv_index.len();
	if conv_count > 0 {
		parts.push(format!("{} archived turns available for retrieval.", conv_count));
	}

	// ── Delta files (04-F4) ──
	let current_files = state.referenced_files.len();
	let prev_files = state.last_emitted_file_count;

	if current_files > 0 {
		let new_files = current_files.saturating_sub(prev_files);
		if new_files > 0 {
			// `record_file` pushes new entries to the FRONT of the deque,
			// so the new ones sit at the front; take from the front, then
			// reverse so names render in insertion order (oldest-of-new
			// first). Reading from the back would select the OLDEST entries
			// and misreport old files as new.
			let names:Vec<String> = state
				.referenced_files
				.iter()
				.take(new_files)
				.map(|(p, t)| format!("{} ({})", p, t))
				.collect::<Vec<_>>()
				.into_iter()
				.rev()
				.collect();
			parts.push(format!(
				"+{} new file{}: {}. {} total.",
				new_files,
				if new_files == 1 { "" } else { "s" },
				names.join(", "),
				current_files
			));
		} else {
			parts.push(format!("{} referenced files (no change).", current_files));
		}
	}

	// ── Poll workers ──
	if state.poll_worker_enabled {
		let active:Vec<_> = state
			.bg_tasks
			.iter()
			.filter(|t| t.status == crate::poll_worker::BgStatus::Running)
			.collect();
		if !active.is_empty() {
			parts.push(format!("{} active poll worker(s). Use process(action='poll').", active.len()));
		}
	}

	let summary = parts.join(" ");

	// Commit both watermarks only now, after the string is fully assembled:
	// the two counters advance together, and a mid-build failure can't leave
	// one side of the delta bookkeeping ahead of the other (Fix 4).
	state.last_emitted_marker_count = current_markers;
	state.last_emitted_file_count = current_files;

	summary
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
		assert!(!s.conv_index.contains_key(&1));
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
		assert_eq!(turns[1]["turn"], 2);
	}

	#[test]
	fn test_catalog_summary_delta_on_new_markers() {
		let mut s = AphroditeState::default();
		assert!(catalog_summary(&mut s).is_empty());
		s.record_marker(crate::state::MarkerEntry {
			hash:"abc".into(),
			ccr_type:"text".into(),
			size:100,
			preview:"[text] hello".into(),
			turn:1,
			center:None,
			meta:None,
		});
		assert!(catalog_summary(&mut s).contains("+1 new compression"));
		assert!(catalog_summary(&mut s).contains("no change this turn"));
	}

	#[test]
	fn test_catalog_summary_delta_on_new_files() {
		let mut s = AphroditeState::default();
		// No files yet
		assert!(catalog_summary(&mut s).is_empty());
		// Add files the production way: record_file pushes to the FRONT of
		// the deque, so the delta must read from the front and reverse into
		// insertion order (Fix 16).
		s.record_file("src/a.rs".into(), "read_file".into());
		s.record_file("src/b.rs".into(), "write_file".into());
		let r1 = catalog_summary(&mut s);
		assert!(r1.contains("+2 new files"));
		assert!(r1.contains("src/a.rs (read_file), src/b.rs (write_file)"));
		// Same files, next turn - no change
		let r2 = catalog_summary(&mut s);
		assert!(r2.contains("no change"));
	}

	#[test]
	fn test_session_start_resets_chain_split_threshold() {
		let mut s = AphroditeState::default();
		// Simulate a previous session that adapted the threshold upward
		// (and a non-default configured floor, so the reset is proven to
		// restore the configured value, not a hardcoded constant).
		s.chain_split_floor = 3;
		s.chain_split_min_segments = 9;
		on_session_start(&mut s);
		assert_eq!(s.chain_split_min_segments, s.chain_split_floor);
		assert_eq!(s.chain_split_min_segments, 3);
	}

	#[test]
	fn test_catalog_summary_reset_on_session_start() {
		let mut s = AphroditeState::default();
		s.last_emitted_marker_count = 10;
		s.last_emitted_file_count = 5;
		on_session_start(&mut s);
		assert_eq!(s.last_emitted_marker_count, 0);
		assert_eq!(s.last_emitted_file_count, 0);
	}
}
