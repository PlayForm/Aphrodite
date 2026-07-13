//! Session lifecycle - port of plugins/aphrodite/_hooks/session.py
//!
//! Manages: turn counter, conversation index, git cache, referenced files,
//! scanned message index, marker-at-end-of-turn archive.

use crate::state::AphroditeState;

/// Handle session start - reset all per-session state.
pub fn on_session_start(state:&mut AphroditeState) -> serde_json::Value {
	state.turn_counter = 0;
	state.scanned_msg_idx = 0;
	state.conv_index.clear();
	state.recent_markers.clear();
	state.referenced_files.clear();
	state.git_cache.clear();

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
	// Keep last 50 turns
	if state.conv_index.len() > 50 {
		let oldest = state.conv_index.keys().min().copied();
		if let Some(key) = oldest {
			state.conv_index.remove(&key);
		}
	}
}

/// Record a git operation in the cache.
pub fn record_git(state:&mut AphroditeState, summary:&str) {
	let ts = chrono::Utc::now().format("%H:%M").to_string();
	state.git_cache.insert(ts, summary.to_string());
	// Keep last 20 entries
	if state.git_cache.len() > 20 {
		let oldest = state.git_cache.keys().min().cloned();
		if let Some(key) = oldest {
			state.git_cache.remove(&key);
		}
	}
}

/// Record a referenced file path.
pub fn record_file(state:&mut AphroditeState, path:&str, tool:&str) {
	// Remove existing entry if present (will be re-added at front)
	state.referenced_files.retain(|(p, _)| p != path);
	state.referenced_files.push_front((path.to_string(), tool.to_string()));
	// Keep last 100
	while state.referenced_files.len() > 100 {
		state.referenced_files.pop_back();
	}
}

/// Get the conversation index as a JSON-serializable value.
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

/// Generate a catalog summary for the context engine prompt injection.
pub fn catalog_summary(state:&AphroditeState) -> String {
	if state.recent_markers.is_empty() && state.conv_index.is_empty() {
		return String::new();
	}

	let mut parts = vec![];

	let marker_count = state.recent_markers.len();
	if marker_count > 0 {
		let last_few:Vec<_> = state.recent_markers.iter().rev().take(5).collect();
		let previews:Vec<String> = last_few.iter().map(|m| m.preview.clone()).collect();
		parts.push(format!(
			"[Aphrodite] {} compressions this session. Recent: {}",
			marker_count,
			previews.join(", ")
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
}
