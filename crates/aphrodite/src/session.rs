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

/// Cap on `conv_index` entries - the eviction bound enforced by
/// [`archive_turn`]. Doubles as the capacity reference for the context
/// engine's fill ratio: the dylib has no token counts and no context
/// length, so the conversation index's own cap is the closest available
/// "full" reference (the index holds at most `CONV_INDEX_CAP` turns).
const CONV_INDEX_CAP:usize = 50;

/// Archive a compression at end of turn into the conversation index.
pub fn archive_turn(state:&mut AphroditeState, hash:&str, summary:&str, size:usize) {
	let turn = state.turn_counter;
	state.conv_index.insert(turn, (hash.to_string(), summary.to_string(), size));
	if state.conv_index.len() > CONV_INDEX_CAP {
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

	// ── Intent hints (prompt knob `[prompts] catalog_intent_hints`, opt-in) ──
	// Pre-Rust port: `intent=read | recent CCRs: <last 3 hashes, 12 chars>`.
	// Default off = current behavior.
	if state.catalog_intent_hints && current_markers > 0 {
		let last3:Vec<_> = state.recent_markers.iter().rev().take(3).collect();
		let hashes:Vec<String> = last3.iter().rev().map(|m| m.hash.chars().take(12).collect()).collect();
		parts.push(format!("intent=read | recent CCRs: {}", hashes.join(" ")));
	}

	let summary = parts.join(" ");

	// Commit both watermarks only now, after the string is fully assembled:
	// the two counters advance together, and a mid-build failure can't leave
	// one side of the delta bookkeeping ahead of the other (Fix 4).
	state.last_emitted_marker_count = current_markers;
	state.last_emitted_file_count = current_files;

	summary
}

/// Context-engine archiving entry (pre-Rust port of `_engine.py`
/// `AphroditeContextEngine`).
///
/// Metric (documented per the pre-Rust-port brief): the dylib never receives
/// the raw conversation message list (Hermes holds it) and has no token
/// counts or context length, so the closest available message-count proxy is
/// the conversation index itself - `conv_index.len()` is the number of
/// archived turns (each turn contributes at least one user + one assistant
/// message in the real conversation). The "message-count fill ratio" is
/// `conv_index.len() * 100 / CONV_INDEX_CAP`: the index's own eviction cap is
/// the only "full" reference the dylib has.
///
/// When the engine is enabled and the session crosses `engine_threshold_pct`
/// (fill ratio at or above the percent; 100+ = engine off, 0 = never, both
/// pre-Rust `should_compress` semantics) with at least `engine_min_msgs`
/// entries, the middle turns of the index - keeping exactly
/// `engine_protect_first` at the start and `engine_protect_last` at the end,
/// with the pre-Rust middle-size floor of ≥ 3 - are packed as one JSON
/// payload, stored in the inline/CCR store under a BLAKE3 key
/// (`headroom_core::ccr::compute_key`), and the middle is REPLACED in
/// `conv_index` by a single marker entry recorded via `archive_turn` so the
/// model sees a compact history (head + marker + tail). The returned string
/// is the CCR marker plus the rendered `engine_offload` prompt (hash + tail)
/// for the caller to inject into the live conversation.
///
/// `None` = nothing archived (engine off / below threshold / below min_msgs /
/// middle too small) with zero mutation - byte-identical behavior when
/// disabled.
pub fn engine_maybe_archive(state:&mut AphroditeState) -> Option<String> {
	// Engine off (config), or threshold 0 (never) / 100+ (documented
	// engine-off escape hatch). All gates run before any mutation.
	if !state.context_engine_enabled || state.engine_threshold_pct == 0 || state.engine_threshold_pct >= 100 {
		return None;
	}
	let total = state.conv_index.len();
	if total < state.engine_min_msgs {
		return None;
	}
	// Fill ratio: archived-turn count against the index's own cap.
	let fill_pct = ((total as u64 * 100) / CONV_INDEX_CAP as u64).min(100);
	if fill_pct < state.engine_threshold_pct {
		return None;
	}
	// Middle slice: exactly protect_first from the start, protect_last from
	// the end; pre-Rust `compress()` refuses a middle shorter than 3.
	let head_n = state.engine_protect_first;
	let tail_n = state.engine_protect_last;
	let middle_n = total.saturating_sub(head_n).saturating_sub(tail_n);
	if middle_n < 3 {
		return None;
	}
	// Sorted turns, oldest first (same ordering as `get_conv_index`).
	let mut turns:Vec<usize> = state.conv_index.keys().copied().collect();
	turns.sort_unstable();
	let end = total.saturating_sub(tail_n);
	let middle_keys:Vec<usize> = turns[head_n..end].to_vec();

	// Pack the REAL middle entries (recorded turn → hash/summary/size) as one
	// compact JSON payload - never fabricated message content.
	let packed_entries:Vec<serde_json::Value> = middle_keys
		.iter()
		.filter_map(|t| {
			state.conv_index.get(t).map(
				|(hash, summary, size)| serde_json::json!({"turn": t, "hash": hash, "summary": summary, "size": size}),
			)
		})
		.collect();
	let packed = match serde_json::to_string(&packed_entries) {
		Ok(p) => p,
		Err(e) => {
			tracing::warn!(error = %e, "engine_maybe_archive: packing middle turns failed; skipping");
			return None;
		},
	};
	let hash = headroom_core::ccr::compute_key(packed.as_bytes());
	state.inline_store_put(hash.clone(), packed.clone());
	let preview = packed.chars().take(120).collect::<String>();
	let marker = crate::marker::ccr_marker(&hash, "context", packed.len(), &preview, None, None, None);
	let rendered =
		crate::flow::render_prompt(state, "engine_offload", &[("hash", &hash), ("tail", &tail_n.to_string())]);
	let prompt = format!("{marker}\n{rendered}");

	// Replace the archived middle with the marker entry: drop the middle
	// turns from the index, then record the new archive at the current turn.
	for t in &middle_keys {
		state.conv_index.remove(t);
	}
	let summary = format!("engine offload: {middle_n} middle turns archived (keep {head_n}/{tail_n})");
	archive_turn(state, &hash, &summary, packed.len());
	state.record_marker(crate::state::MarkerEntry {
		hash:hash.clone(),
		ccr_type:"context".into(),
		size:packed.len(),
		preview,
		turn:state.turn_counter,
		center:None,
		meta:None,
	});
	tracing::info!(
		hash,
		middle = middle_n,
		size = packed.len(),
		"context_engine: archived {} middle turns to CCR:{} ({} bytes)",
		middle_n,
		hash,
		packed.len()
	);
	Some(prompt)
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

	// ── catalog_intent_hints knob (prompt knob, default off) ──
	#[test]
	fn test_catalog_summary_intent_hints_off_by_default() {
		let mut s = AphroditeState::default();
		s.record_marker(crate::state::MarkerEntry {
			hash:"aaaaaaaaaaaa111111111111".into(),
			ccr_type:"text".into(),
			size:100,
			preview:"[text] p".into(),
			turn:1,
			center:None,
			meta:None,
		});
		let r = catalog_summary(&mut s);
		assert!(!r.contains("intent=read"), "intent hints must be off by default: {r}");
	}

	#[test]
	fn test_catalog_summary_intent_hints_on_lists_recent_hashes() {
		let mut s = AphroditeState::default();
		s.catalog_intent_hints = true;
		for h in [
			"aaaaaaaaaaaa111111111111",
			"bbbbbbbbbbbb222222222222",
			"cccccccccccc333333333333",
			"dddddddddddd444444444444",
		] {
			s.record_marker(crate::state::MarkerEntry {
				hash:h.into(),
				ccr_type:"text".into(),
				size:100,
				preview:"[text] p".into(),
				turn:1,
				center:None,
				meta:None,
			});
		}
		let r = catalog_summary(&mut s);
		assert!(
			r.contains("intent=read | recent CCRs: bbbbbbbbbbbb cccccccccccc dddddddddddd"),
			"last 3 hashes in insertion order, 12 chars each: {r}"
		);
	}

	// ── Context-engine archiving (pre-Rust port of _engine.py) ──

	/// Build a state with `entries` archived turns (keys 1..=entries) and
	/// default engine config (enabled, threshold 45, min 8, protect 2/5).
	fn engine_state(entries:usize) -> AphroditeState {
		let mut s = AphroditeState::default();
		for i in 1..=entries {
			s.conv_index.insert(i, (format!("h{i:02}"), format!("summary {i}"), i * 10));
		}
		s
	}

	/// Assert the engine left the state completely untouched: same
	/// conversation index, empty inline store, no recorded markers.
	fn assert_zero_mutation(s:&AphroditeState) {
		assert!(s.inline_store.is_empty(), "no archive may hit the inline store");
		assert!(s.recent_markers.is_empty(), "no marker may be recorded");
	}

	#[test]
	fn test_engine_archive_below_threshold_none_no_mutation() {
		// 10 archived turns = 20% fill; threshold 80 not crossed.
		let mut s = engine_state(10);
		s.engine_threshold_pct = 80;
		let before = s.conv_index.clone();
		assert!(engine_maybe_archive(&mut s).is_none());
		assert_eq!(s.conv_index, before, "below threshold must not touch conv_index");
		assert_zero_mutation(&s);
	}

	#[test]
	fn test_engine_archive_disabled_none_no_mutation() {
		let mut s = engine_state(12);
		s.engine_threshold_pct = 20; // well over threshold if enabled
		s.context_engine_enabled = false;
		let before = s.conv_index.clone();
		assert!(engine_maybe_archive(&mut s).is_none());
		assert_eq!(s.conv_index, before, "disabled engine must be byte-identical no-op");
		assert_zero_mutation(&s);
	}

	#[test]
	fn test_engine_archive_over_threshold_archives_middle_and_returns_prompt() {
		let mut s = engine_state(12);
		s.engine_threshold_pct = 20; // 12 entries = 24% fill
		s.engine_min_msgs = 8;
		s.engine_protect_first = 2;
		s.engine_protect_last = 5;
		let prompt = engine_maybe_archive(&mut s).expect("over threshold must archive");
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
		assert!(prompt.contains(&hash), "prompt must contain the archive hash: {prompt}");
		assert!(prompt.contains("<<<CCR:"), "prompt must carry the CCR marker line");
		assert!(
			prompt.contains("Use aphrodite_retrieve"),
			"prompt must carry retrieval guidance"
		);
		// Middle replaced by the marker entry: keys 3..=7 gone, head (1,2)
		// and tail (8..=12) kept, marker entry recorded at the current turn.
		for t in 3..=7 {
			assert!(!s.conv_index.contains_key(&t), "middle turn {t} must be replaced");
		}
		for t in [1, 2, 8, 9, 10, 11, 12] {
			assert!(s.conv_index.contains_key(&t), "protected turn {t} must survive");
		}
		assert_eq!(s.conv_index.len(), 8, "12 - 5 middle + 1 marker entry");
		let (m_hash, m_summary, m_size) = s.conv_index.get(&0).expect("marker entry recorded");
		assert_eq!(m_hash, &hash);
		assert!(m_summary.starts_with("engine offload: 5 middle turns archived"));
		assert!(*m_size > 0);
		// Archived middle must be retrievable from the inline store.
		assert!(s.inline_store.contains_key(&hash), "archive payload must be stored inline");
	}

	#[test]
	fn test_engine_archive_protect_bounds_respected() {
		// protect_first=2 / protect_last=5 on 12 turns: exactly keys 3..=7
		// (5 entries) are the middle; head/tail survive verbatim.
		let mut s = engine_state(12);
		s.engine_threshold_pct = 20;
		s.engine_min_msgs = 8;
		s.engine_protect_first = 2;
		s.engine_protect_last = 5;
		let _ = engine_maybe_archive(&mut s).expect("must archive");
		assert!(s.conv_index.contains_key(&1) && s.conv_index.contains_key(&2));
		assert!(s.conv_index.contains_key(&8) && s.conv_index.contains_key(&12));
		assert!(!s.conv_index.contains_key(&3) && !s.conv_index.contains_key(&7));
		assert_eq!(s.conv_index.len(), 8);
		// protect_first=0 keeps nothing at the start: middle starts at key 1.
		let mut s0 = engine_state(12);
		s0.engine_threshold_pct = 20;
		s0.engine_min_msgs = 8;
		s0.engine_protect_first = 0;
		s0.engine_protect_last = 5;
		let _ = engine_maybe_archive(&mut s0).expect("must archive");
		assert!(!s0.conv_index.contains_key(&1), "protect_first=0 keeps no head");
		assert!(s0.conv_index.contains_key(&8) && s0.conv_index.contains_key(&12));
	}

	#[test]
	fn test_engine_archive_min_msgs_respected() {
		// 5 entries = 10% fill >= threshold 10, but 5 < min_msgs 8.
		let mut s = engine_state(5);
		s.engine_threshold_pct = 10;
		s.engine_min_msgs = 8;
		let before = s.conv_index.clone();
		assert!(engine_maybe_archive(&mut s).is_none());
		assert_eq!(s.conv_index, before);
		assert_zero_mutation(&s);
	}

	#[test]
	fn test_engine_archive_threshold_100_off() {
		// 100+ = documented engine-off escape hatch; also 0 = never.
		for pct in [0u64, 100, 200] {
			let mut s = engine_state(12);
			s.engine_threshold_pct = pct;
			let before = s.conv_index.clone();
			assert!(
				engine_maybe_archive(&mut s).is_none(),
				"threshold {pct} must leave the engine off"
			);
			assert_eq!(s.conv_index, before);
			assert_zero_mutation(&s);
		}
	}

	#[test]
	fn test_engine_archive_middle_too_small_none() {
		// protect 2/9 on 12 turns leaves a 1-entry middle (< pre-Rust floor 3).
		let mut s = engine_state(12);
		s.engine_threshold_pct = 20;
		s.engine_min_msgs = 8;
		s.engine_protect_first = 2;
		s.engine_protect_last = 9;
		let before = s.conv_index.clone();
		assert!(engine_maybe_archive(&mut s).is_none());
		assert_eq!(s.conv_index, before);
		assert_zero_mutation(&s);
	}

	#[test]
	fn test_engine_archive_payload_round_trips_inline() {
		let mut s = engine_state(12);
		s.engine_threshold_pct = 20;
		s.engine_min_msgs = 8;
		s.engine_protect_first = 2;
		s.engine_protect_last = 5;
		let prompt = engine_maybe_archive(&mut s).expect("must archive");
		// Extract the hash from the marker line and resolve it back.
		let hash = crate::marker::extract_hashes(&prompt)
			.into_iter()
			.next()
			.expect("marker hash must be extractable");
		let resolved = s.inline_store_get(&hash).expect("archive must resolve from inline store");
		let parsed:serde_json::Value = serde_json::from_str(&resolved).expect("packed payload must be JSON");
		assert_eq!(parsed.as_array().map(Vec::len), Some(5), "middle of 5 turns packed");
		assert_eq!(parsed[0]["turn"], 3);
		assert_eq!(parsed[4]["turn"], 7);
	}
}
