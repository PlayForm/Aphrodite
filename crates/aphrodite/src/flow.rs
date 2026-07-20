//! Flow-context assembler (report 05, P1) - the single choke point for ALL
//! per-turn injected context.
//!
//! Before this module, three separate call sites composed the model-visible
//! context independently: the Hermes bridge `pre_llm_call` arm, the
//! `context_engine_pre_llm` tool, and core `hooks::pre_llm_call`. The bridge
//! sites never called `build_directive_context`, so the directives system was
//! invisible on the only path real sessions use (bug class 01-F3 / 04-F1).
//!
//! `build_turn_context` assembles every section in a fixed order under one
//! hard byte cap (`state.flow_budget_chars`), dropping low-priority sections
//! from the BOTTOM when over budget. Every injection site now routes through
//! it, so the "bridge reimplements the hook body and forks" failure mode is
//! unrepresentable.

use crate::state::{ActiveDirective, AphroditeState};

/// Assemble the complete per-turn injected context, in fixed order, hard-capped
/// at `state.flow_budget_chars`.
///
/// Section order (top = highest survival priority; drop order is bottom-up:
/// the recall catalog goes first under budget pressure, directives + nudges
/// never drop):
///
/// ```text
/// [directives: focus] ...        (build_directive_context - never dropped)
/// [nudge: ...]                   (ephemeral one-shots, at most 2 - never dropped)
/// [recall] <catalog summary>     (dropped first under budget pressure)
/// ```
///
/// `est_request_bytes` is reserved for the P9 telemetry line (chars/4 estimate)
/// and is accepted now so callers don't have to change signature later; T1
/// itself does not render a telemetry line.
pub fn build_turn_context(state:&mut AphroditeState, est_request_bytes:Option<usize>) -> String {
	let _ = est_request_bytes; // reserved for P9/T27 telemetry line

	let budget = state.flow_budget_chars;

	// ── Always-survive sections (top priority) ──
	let directives = crate::directives::build_directive_context(&state.directives, &state.active_directives);
	let nudges = render_nudges(state);

	// ── Droppable section: recall catalog, delta-only (04-F1) ──
	// If navigation is enabled, use the S2 navigable index instead of prose.
	let recall = if state.navigation_enabled {
		crate::navigate::build_navigable_context(state)
	} else {
		crate::session::catalog_summary(state)
	};

	// ── Poll-worker status (above recall, dropped after recall) ──
	let bg_status = if state.poll_worker_enabled {
		crate::poll_worker::render_bg_task_status(state)
	} else {
		String::new()
	};

	// Assemble top-down, then drop from the bottom until within budget.
	let mut sections:Vec<String> = Vec::new();
	if !directives.is_empty() {
		sections.push(directives.trim_end().to_string());
	}
	if !nudges.is_empty() {
		sections.push(nudges);
	}
	if !bg_status.is_empty() {
		sections.push(bg_status);
	}
	// The recall block — RETRIEVE_HINT removed per 04-F2 (tool schemas
	// in the system prompt already teach retrieval; repeating it every
	// turn was 56 chars of pure noise, ~2.8k chars over 50 turns).
	if !recall.is_empty() {
		sections.push(format!("[recall]\n{}\n", recall.trim_end()));
	}
	let always_survive = directives.is_empty() as usize + nudges_present(state) as usize;
	let always_survive = always_survive.min(sections.len());
	while join_sections(&sections).len() > budget && sections.len() > always_survive {
		sections.pop();
	}

	join_sections(&sections)
}

/// Number of leading "never-drop" sections currently present (directives +
/// nudge block). Used to compute the floor for bottom-up dropping.
fn nudges_present(state:&AphroditeState) -> bool {
	state
		.ephemeral_directives
		.iter()
		.any(|e| e.inline.is_some() && e.expires_after_turn.is_none_or(|exp| exp >= state.turn_counter))
}

fn join_sections(sections:&[String]) -> String { sections.join("\n") }

/// Render at most 2 active inline nudges as `[nudge: <text>]` lines, newest
/// wins (P3/T9). An entry renders while it has not expired
/// (`expires_after_turn >= turn_counter`, or `None` = permanent).
pub fn render_nudges(state:&AphroditeState) -> String {
	let mut lines:Vec<String> = state
		.ephemeral_directives
		.iter()
		.rev() // newest first
		.filter(|e| e.inline.is_some())
		.filter(|e| e.expires_after_turn.is_none_or(|exp| exp >= state.turn_counter))
		.take(2)
		.filter_map(|e| e.inline.as_ref().map(|t| format!("[nudge: {t}]")))
		.collect();
	// Preserve newest-last visual order (we collected newest-first).
	lines.reverse();
	lines.join("\n")
}

/// Push an inline nudge that renders on the next `pre_llm_call` and self-purges
/// after `ttl_turns` (P3/T9). `ttl_turns = 1` is a one-shot: rendered on the
/// next turn, purged at the following `post_llm_call`. At most 4 ephemeral
/// entries are stored; pushing a 5th drops the oldest.
pub fn push_nudge(state:&mut AphroditeState, text:&str, ttl_turns:usize) {
	let expires = Some(state.turn_counter + ttl_turns);
	state.ephemeral_directives.push(ActiveDirective {
		name:String::new(),
		inline:Some(text.to_string()),
		expires_after_turn:expires,
	});
	while state.ephemeral_directives.len() > 4 {
		state.ephemeral_directives.remove(0);
	}
}

/// Purge expired ephemeral directives (P3/T9). Called from `post_llm_call`
/// AFTER `next_turn` advances the counter, so a nudge pushed during turn N
/// (expiring at N + ttl) renders in turn N+1's context exactly once for ttl=1.
pub fn purge_expired_nudges(state:&mut AphroditeState) {
	let counter = state.turn_counter;
	state
		.ephemeral_directives
		.retain(|e| e.expires_after_turn.is_none_or(|exp| exp >= counter));
}

// ── Turn-telemetry helpers (P2) ────────────────────────────

/// FNV-1a offset basis / prime (64-bit).
const FNV_OFFSET:u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME:u64 = 0x0000_0100_0000_01B3;

fn fnv1a(bytes:&[u8]) -> u64 {
	let mut hash = FNV_OFFSET;
	for &b in bytes {
		hash ^= b as u64;
		hash = hash.wrapping_mul(FNV_PRIME);
	}
	hash
}

/// Keys whose values are volatile noise (change run-to-run for the same logical
/// call) - stripped before hashing so the same command twice yields the same
/// signature (P2/T7).
const VOLATILE_KEYS:&[&str] = &["timeout", "timestamp", "session_id", "tool_call_id"];

/// FNV-1a signature of `tool` + normalized args (P2/P8 similarity key).
///
/// Args are normalized by serializing the object with keys sorted and volatile
/// keys stripped, so the same logical call hashes stably; a different file path
/// yields a different signature. Terminal calls (args carrying a `command`
/// string) normalize to the trimmed command string.
pub fn normalize_args_sig(tool:&str, args:Option<&serde_json::Value>) -> u64 {
	let mut buf = String::new();
	buf.push_str(tool);
	buf.push('\u{1}');
	if let Some(v) = args {
		// Terminal-style calls: the command string is the whole identity.
		if let Some(cmd) = v.get("command").and_then(|c| c.as_str()) {
			buf.push_str(cmd.trim_end());
		} else if let Some(obj) = v.as_object() {
			let mut keys:Vec<&String> = obj.keys().filter(|k| !VOLATILE_KEYS.contains(&k.as_str())).collect();
			keys.sort();
			for k in keys {
				buf.push_str(k);
				buf.push('=');
				// Compact, stable string form of the value.
				buf.push_str(&obj[k].to_string());
				buf.push('\u{1f}');
			}
		} else {
			buf.push_str(&v.to_string());
		}
	}
	fnv1a(buf.as_bytes())
}

/// FNV-1a signature of an error: `error_type` + first line of `error_message`
/// (P2/P7). Distinct compiler errors differ on line 1, so this stays
/// discriminating without over-matching.
pub fn error_sig(error_type:Option<&str>, error_message:Option<&str>) -> u64 {
	let mut buf = String::new();
	buf.push_str(error_type.unwrap_or(""));
	buf.push('\u{1}');
	if let Some(msg) = error_message {
		buf.push_str(msg.lines().next().unwrap_or("").trim());
	}
	fnv1a(buf.as_bytes())
}

/// Cheap aggregate over the last `n` turns of `tool_events` (P2/T7). Scans
/// events with `turn > turn_counter - n`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowStats {
	pub reads:usize,
	pub writes:usize,
	pub searches:usize,
	pub errors:usize,
	pub distinct_error_sigs:usize,
	pub new_files:usize,
	pub total_calls:usize,
}

/// Compute `WindowStats` over the last `n` turns (P2/T7).
pub fn turn_window(state:&AphroditeState, n:usize) -> WindowStats {
	let floor = state.turn_counter.saturating_sub(n);
	let mut stats = WindowStats::default();
	let mut error_sigs = std::collections::HashSet::new();
	let mut seen_paths = std::collections::HashSet::new();
	for ev in state.tool_events.iter().filter(|e| e.turn > floor) {
		stats.total_calls += 1;
		if !ev.ok {
			stats.errors += 1;
			if let Some(sig) = ev.error_sig {
				error_sigs.insert(sig);
			}
		}
		if let Some(path) = &ev.wrote_path {
			stats.writes += 1;
			if seen_paths.insert(path.clone()) {
				stats.new_files += 1;
			}
		} else {
			match ev.tool.as_str() {
				"read_file" => stats.reads += 1,
				"search_files" => stats.searches += 1,
				_ => {},
			}
		}
	}
	stats.distinct_error_sigs = error_sigs.len();
	stats
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		directives::Directive,
		state::{MarkerEntry, ToolEvent},
	};

	fn state_with_directive(name:&str, body:&str) -> AphroditeState {
		let mut s = AphroditeState::default();
		s.directives
			.insert(name.into(), Directive { name:name.into(), content:body.into() });
		s.active_directives = vec![name.into()];
		s
	}

	fn add_marker(s:&mut AphroditeState, hash:&str, turn:usize) {
		s.record_marker(MarkerEntry {
			hash:hash.into(),
			ccr_type:"text".into(),
			size:100,
			preview:"[text] some preview content here".into(),
			turn,
			center:None,
			meta:None,
		});
	}

	// ── T1: the assembler drops the recall catalog before directives when the
	// budget is too small - directives are never dropped. ──
	#[test]
	fn test_budget_drops_catalog_before_directives() {
		let mut s = state_with_directive("focus", "stay targeted, minimal tool usage");
		// 50 markers so the catalog_summary is large.
		for i in 0..50 {
			add_marker(&mut s, &format!("hash{i:040}"), i);
		}
		// Budget large enough for the directive block but too small for the
		// recall block to also fit, so the catalog must be dropped first.
		s.flow_budget_chars = 90;
		let ctx = build_turn_context(&mut s, None);
		assert!(ctx.contains("[directives:"), "directives must survive: {ctx}");
		assert!(
			!ctx.contains("[recall]"),
			"catalog must be dropped first under budget pressure: {ctx}"
		);
	}

	// ── T2: the retrieve-hint boilerplate is no longer emitted per turn
	// (04-F2 fix). Tool schemas in the system prompt already teach retrieval. ──
	#[test]
	fn test_retrieve_hint_not_in_per_turn_context() {
		let mut s = AphroditeState::default();
		add_marker(&mut s, &"a".repeat(40), 0);
		s.flow_budget_chars = 4000;
		let ctx = build_turn_context(&mut s, None);
		assert_eq!(
			ctx.matches("retrieve: aphrodite_retrieve").count(),
			0,
			"retrieve hint must NOT appear in per-turn context (04-F2): {ctx}"
		);
	}

	#[test]
	fn test_empty_state_yields_empty_context() {
		let mut s = AphroditeState::default();
		assert_eq!(build_turn_context(&mut s, None), "");
	}

	#[test]
	fn test_normalize_args_sig_stable_across_volatile_keys() {
		let a = serde_json::json!({"path": "src/x.rs", "timeout": 30, "session_id": "abc"});
		let b = serde_json::json!({"path": "src/x.rs", "timeout": 99, "session_id": "zzz"});
		assert_eq!(
			normalize_args_sig("read_file", Some(&a)),
			normalize_args_sig("read_file", Some(&b)),
			"volatile keys must not affect the signature"
		);
		// A different file path yields a different signature.
		let c = serde_json::json!({"path": "src/y.rs"});
		assert_ne!(
			normalize_args_sig("read_file", Some(&a)),
			normalize_args_sig("read_file", Some(&c))
		);
		// Same command twice => same sig (terminal path).
		let cmd1 = serde_json::json!({"command": "cargo test   "});
		let cmd2 = serde_json::json!({"command": "cargo test"});
		assert_eq!(
			normalize_args_sig("terminal", Some(&cmd1)),
			normalize_args_sig("terminal", Some(&cmd2))
		);
	}

	#[test]
	fn test_turn_window_counts_reads_writes_errors() {
		let mut s = AphroditeState::default();
		s.turn_counter = 10;
		s.record_tool_event(ToolEvent {
			turn:10,
			tool:"read_file".into(),
			sig:1,
			ok:true,
			error_sig:None,
			bytes:100,
			wrote_path:None,
		});
		s.record_tool_event(ToolEvent {
			turn:10,
			tool:"write_file".into(),
			sig:2,
			ok:true,
			error_sig:None,
			bytes:50,
			wrote_path:Some("src/a.rs".into()),
		});
		s.record_tool_event(ToolEvent {
			turn:9,
			tool:"terminal".into(),
			sig:3,
			ok:false,
			error_sig:Some(42),
			bytes:20,
			wrote_path:None,
		});
		// old event outside the window
		s.record_tool_event(ToolEvent {
			turn:2,
			tool:"read_file".into(),
			sig:4,
			ok:true,
			error_sig:None,
			bytes:0,
			wrote_path:None,
		});
		let w = turn_window(&s, 5);
		assert_eq!(w.reads, 1);
		assert_eq!(w.writes, 1);
		assert_eq!(w.new_files, 1);
		assert_eq!(w.errors, 1);
		assert_eq!(w.distinct_error_sigs, 1);
		assert_eq!(w.total_calls, 3, "the turn-2 event is outside the 5-turn window");
	}

	#[test]
	fn test_build_turn_context_omits_poll_status_when_disabled() {
		let mut s = AphroditeState::default();
		s.poll_worker_enabled = false;
		s.turn_counter = 5;
		crate::poll_worker::insert_bg_task(
			&mut s,
			"t1".into(),
			"terminal".into(),
			"cargo build".into(),
			1,
		);
		let ctx = build_turn_context(&mut s, None);
		assert!(
			!ctx.contains("[poll workers]"),
			"disabled flag must omit poll worker status: {ctx}"
		);
	}

	#[test]
	fn test_build_turn_context_includes_poll_status_when_enabled() {
		let mut s = AphroditeState::default();
		s.poll_worker_enabled = true;
		s.turn_counter = 5;
		crate::poll_worker::insert_bg_task(
			&mut s,
			"t1".into(),
			"terminal".into(),
			"cargo build".into(),
			1,
		);
		let ctx = build_turn_context(&mut s, None);
		assert!(
			ctx.contains("[poll workers]"),
			"enabled flag must include poll worker status: {ctx}"
		);
	}
}
