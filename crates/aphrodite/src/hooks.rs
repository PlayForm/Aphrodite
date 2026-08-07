//! Full hook implementations - expanded from plugins/aphrodite/_hooks/
//!
//! transform_tool_result: content-aware compression with essential tool skip,
//!   file reference tracking, threshold gating, preview generation.
//! transform_terminal_output: terminal-specific compression with exit code
//!   detection, threshold gating, streaming support.

use headroom_core::transforms;

use crate::{
	marker::ccr_marker,
	state::{AphroditeState, MarkerEntry, ToolEvent},
};

/// Metadata Hermes already sends alongside a tool result (report 05, P2) - the
/// signals the bridge historically dropped on the floor (`status`,
/// `error_type`, `error_message`, `args`, `duration_ms`). Threaded through
/// [`transform_tool_result_with_meta`] into the `tool_events` telemetry ring.
#[derive(Debug, Default, Clone, Copy)]
pub struct ToolCallMeta<'a> {
	pub args_json: Option<&'a serde_json::Value>,
	pub status: Option<&'a str>,
	pub error_type: Option<&'a str>,
	pub error_message: Option<&'a str>,
	pub duration_ms: Option<u64>,
}

/// Compute a CCR hash for content using BLAKE3 (40 hex chars).
pub fn compute_hash(content: &str) -> String {
	headroom_core::ccr::compute_key(content.as_bytes())
}

/// Essential tools that must NOT be compressed - agent needs raw output.
const ESSENTIAL_TOOLS: &[&str] = &[
	"skill_view",
	"skills_list",
	"skill_manage",
	"memory",
	"session_search",
	"read_file",
	"read_terminal",
];

/// Transform tool output - full compression pipeline.
pub fn transform_tool_result(state: &mut AphroditeState, content: &str, tool_name: &str) -> serde_json::Value {
	transform_tool_result_inner(state, content, tool_name, None, &ToolCallMeta::default())
}

/// Same as [`transform_tool_result`], but a caller that has already found the
/// "real" payload underneath a wrapper the core classifier can't see through
/// (e.g. an agent-specific JSON envelope) may supply it via `classify` as
/// `(content_to_classify, type)`. Core stays agnostic to what a wrapper looks
/// like - it only ever hashes and stores the ORIGINAL `content`, so
/// `aphrodite_retrieve` always returns exactly what was passed in; `classify`
/// affects only the reported `type` and the generated preview.
pub fn transform_tool_result_classified(
	state: &mut AphroditeState,
	content: &str,
	tool_name: &str,
	classify: Option<(&str, &str)>,
) -> serde_json::Value {
	transform_tool_result_inner(state, content, tool_name, classify, &ToolCallMeta::default())
}

/// Same as [`transform_tool_result_classified`], plus the Hermes-supplied call
/// metadata (report 05, P2/T6) that gets recorded into the `tool_events`
/// telemetry ring. This is the entry point the Hermes bridge routes to so the
/// error/args/duration signals Hermes ships on every call stop being dropped.
pub fn transform_tool_result_with_meta(
	state: &mut AphroditeState,
	content: &str,
	tool_name: &str,
	classify: Option<(&str, &str)>,
	meta: &ToolCallMeta,
) -> serde_json::Value {
	transform_tool_result_inner(state, content, tool_name, classify, meta)
}

/// Record a per-call telemetry event from the supplied metadata (P2/T6).
/// `ok` is fail-open: a missing `status` means success. `wrote_path` is
/// inferred from the args of `write_file`/`patch`-style tools.
fn record_tool_event_from_meta(state: &mut AphroditeState, tool_name: &str, content: &str, meta: &ToolCallMeta) {
	let ok = meta.status.map(|s| s != "error").unwrap_or(true);
	let error_sig = if ok {
		None
	} else {
		Some(crate::flow::error_sig(meta.error_type, meta.error_message))
	};
	let wrote_path = wrote_path_from(tool_name, meta.args_json, content);
	let sig = crate::flow::normalize_args_sig(tool_name, meta.args_json);
	state.record_tool_event(ToolEvent {
		turn: state.turn_counter,
		tool: tool_name.to_string(),
		sig,
		ok,
		error_sig,
		bytes: content.len(),
		wrote_path,
	});
}

/// Infer the written path for a write-style tool (P2/T6, feeds P11). Reads a
/// `path`/`file` arg for `write_file`/`patch`; falls back to the first-line
/// path heuristic used by `extract_file_path`.
fn wrote_path_from(tool_name: &str, args: Option<&serde_json::Value>, content: &str) -> Option<String> {
	if !matches!(tool_name, "write_file" | "patch") {
		return None;
	}
	if let Some(v) = args {
		for key in ["path", "file", "file_path", "filename"] {
			if let Some(p) = v.get(key).and_then(|x| x.as_str()) {
				if !p.is_empty() {
					return Some(p.to_string());
				}
			}
		}
	}
	extract_file_path(content, tool_name)
}

fn transform_tool_result_inner(
	state: &mut AphroditeState,
	content: &str,
	tool_name: &str,
	classify: Option<(&str, &str)>,
	meta: &ToolCallMeta,
) -> serde_json::Value {
	// Record the telemetry event first (P2/T6): it must happen even when the
	// content is empty or the tool is essential/self/below-threshold - the
	// phase detector and error-loop breaker care about ALL calls, not only the
	// ones that produced a compressible marker. Skip only genuinely empty
	// content (no signal at all).
	if !content.is_empty() {
		record_tool_event_from_meta(state, tool_name, content, meta);
	}
	if content.is_empty() {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
	}

	// Track file references BEFORE the essential-tool early return (F10):
	// `read_file` is both listed in `ESSENTIAL_TOOLS` (never compressed - the
	// agent needs the raw content) and in `state.file_tools` (tracked for
	// `aphrodite_files`/the catalog's `referenced_files` stat). Tracking used
	// to run after the essential-tool check returned early, so the single
	// most common file-reference source - reading a file - was silently
	// never recorded. Skipping compression and recording a reference are
	// independent decisions; do both regardless of which tools land in
	// which list.
	if state.file_tools.contains(&tool_name.to_string()) {
		if let Some(path) = extract_file_path(content, tool_name) {
			state.record_file(path, tool_name.to_string());
		}
	}

	// Skip essential tools
	if ESSENTIAL_TOOLS.contains(&tool_name) {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "essential_tool"});
	}

	// Skip aphrodite's own tools (and headroom helpers): their output is already
	// compact metadata, and compressing aphrodite_retrieve would replace the
	// resolved content with another CCR marker - an infinite preview loop.
	if tool_name.starts_with("aphrodite_") || tool_name.starts_with("headroom") {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "self_tool"});
	}

	// Skip below threshold (0 = always compress)
	if state.tool_threshold > 0 && content.len() < state.tool_threshold {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
	}

	let (type_str, classify_content): (String, &str) = match classify {
		Some((c, t)) => (t.to_string(), c),
		None => {
			// Extend the "terminal" override pattern: let Aphrodite's own
			// semantic detector upgrade a generic classification (git status,
			// ls, test, grep, git log) so the reported `type` AND the preview
			// both carry the high-signal shape. Detection stays in Aphrodite's
			// layer - the vendored classifier is untouched.
			let base = transforms::content_detector::detect_content_type(content)
				.content_type
				.as_str()
				.to_string();
			let t = match base.as_str() {
				"text" | "log" | "plain" | "" => crate::preview::detect_semantic_type(content)
					.map(|s| s.to_string())
					.unwrap_or(base),
				_ => base,
			};
			(t, content)
		},
	};
	let hash = headroom_core::ccr::compute_key(content.as_bytes());

	state.inline_store_put(hash.clone(), content.to_string());

	let preview = crate::build_preview(&type_str, classify_content);
	let marker = ccr_marker(&hash, &type_str, content.len(), &preview, None, None, None);

	state.record_marker(MarkerEntry {
		hash: hash.clone(),
		ccr_type: type_str.clone(),
		size: content.len(),
		preview: preview.clone(),
		turn: state.turn_counter,
		center: None,
		meta: None,
	});

	serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": type_str,
		"size": content.len(),
		"preview": preview,
		"marker": marker,
	})
}

/// Transform terminal output - exit code aware.
pub fn transform_terminal_output(state: &mut AphroditeState, content: &str) -> serde_json::Value {
	transform_terminal_output_classified(state, content, None)
}

/// Same as [`transform_terminal_output`], with the same `classify` contract
/// as [`transform_tool_result_classified`].
pub fn transform_terminal_output_classified(
	state: &mut AphroditeState,
	content: &str,
	classify: Option<(&str, &str)>,
) -> serde_json::Value {
	transform_terminal_output_inner(state, content, classify, None, None)
}

/// Same as [`transform_terminal_output_classified`], plus the Hermes-supplied
/// `command` and `returncode` (report 05, P2/T6/T8) recorded into the
/// `tool_events` telemetry ring. `returncode == 0` (or absent) is `ok`.
pub fn transform_terminal_output_with_meta(
	state: &mut AphroditeState,
	content: &str,
	classify: Option<(&str, &str)>,
	command: Option<&str>,
	returncode: Option<i64>,
) -> serde_json::Value {
	transform_terminal_output_inner(state, content, classify, command, returncode)
}

fn transform_terminal_output_inner(
	state: &mut AphroditeState,
	content: &str,
	classify: Option<(&str, &str)>,
	command: Option<&str>,
	returncode: Option<i64>,
) -> serde_json::Value {
	// Telemetry (P2/T6): record before threshold/empty gating - a failing
	// command with tiny output is exactly what the error-loop breaker needs.
	if !content.is_empty() {
		let ok = returncode.map(|rc| rc == 0).unwrap_or(true);
		let args = command.map(|c| serde_json::json!({"command": c}));
		let sig = crate::flow::normalize_args_sig("terminal", args.as_ref());
		let error_sig = if ok {
			None
		} else {
			// Terminal failures have no error_type; key on the command + first
			// error-looking line of the output so distinct failures differ.
			let first_err = content
				.lines()
				.find(|l| l.contains("error") || l.contains("Error") || l.contains("FAILED") || l.contains("panicked"))
				.or_else(|| content.lines().next());
			Some(crate::flow::error_sig(command, first_err))
		};
		state.record_tool_event(ToolEvent {
			turn: state.turn_counter,
			tool: "terminal".to_string(),
			sig,
			ok,
			error_sig,
			bytes: content.len(),
			wrote_path: None,
		});
	}
	if content.is_empty() {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
	}

	if state.terminal_threshold > 0 && content.len() < state.terminal_threshold {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
	}

	let (type_str, classify_content): (String, &str) = match classify {
		Some((c, t)) => (t.to_string(), c),
		None => {
			let ct = transforms::content_detector::detect_content_type(content).content_type;
			let t = if content.contains("exit code:") || content.contains("Error:") {
				"terminal".to_string()
			} else {
				// Terminal output is very often a git status / ls / test / grep
				// dump; upgrade a generic classification via Aphrodite's detector
				// so the preview is high-signal on the terminal path too.
				let base = ct.as_str().to_string();
				match base.as_str() {
					"text" | "log" | "plain" | "" => crate::preview::detect_semantic_type(content)
						.map(|s| s.to_string())
						.unwrap_or(base),
					_ => base,
				}
			};
			(t, content)
		},
	};

	let hash = headroom_core::ccr::compute_key(content.as_bytes());
	state.inline_store_put(hash.clone(), content.to_string());

	let preview = crate::build_preview(&type_str, classify_content);
	let marker = ccr_marker(&hash, &type_str, content.len(), &preview, None, None, None);

	state.record_marker(MarkerEntry {
		hash: hash.clone(),
		ccr_type: type_str.to_string(),
		size: content.len(),
		preview: preview.clone(),
		turn: state.turn_counter,
		center: None,
		meta: None,
	});

	serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": type_str,
		"size": content.len(),
		"preview": preview,
		"marker": marker,
	})
}

/// Session start hook - full reset.
pub fn on_session_start(state: &mut AphroditeState) -> serde_json::Value {
	crate::session::on_session_start(state)
}

/// Pre-LLM call hook - inject catalog + active directives into context.
///
/// Keeps its historical JSON shape (`catalog`/`directives`/`compressed_count`)
/// for proxy/FFI consumers, but ALSO returns the unified `context` string built
/// by `flow::build_turn_context` (report 05, P1) so this path and the Hermes
/// bridge path can never fork on what the model actually sees. The bridge and
/// `context_engine_pre_llm` return only `{"context": ...}`; core keeps the
/// richer shape for back-compat.
pub fn pre_llm_call(state: &mut AphroditeState) -> serde_json::Value {
	// Poll-worker checkpoint: push nudges for running/completed/failed tasks
	// (only when the master flag is enabled).
	if state.poll_worker_enabled {
		crate::poll_worker::check_bg_tasks(state);
	}
	let directives = crate::directives::build_directive_context(&state.directives, &state.active_directives);
	// catalog_summary is now called inside build_turn_context — don't call
	// it separately (04-F3: double call skipped delta tracking every other turn).
	let context = crate::flow::build_turn_context(state, None);
	serde_json::json!({
		"status": "ok",
		"compressed_count": state.recent_markers.len(),
		"directives": if directives.is_empty() { None } else { Some(directives) },
		"context": if context.is_empty() { None } else { Some(context) },
	})
}

/// Post-LLM call hook - archive turn.
///
/// Archives the last marker recorded this turn into `conv_index` before
/// advancing the turn counter (report 06 F11/T13) - previously `archive_turn`
/// was never called from any hook, so `conv_index` stayed empty forever and
/// `aphrodite_diff` always returned zero turns despite compressions
/// happening every turn.
pub fn post_llm_call(state: &mut AphroditeState) -> serde_json::Value {
	if let Some(last) = state.recent_markers.iter().rev().find(|m| m.turn == state.turn_counter) {
		let (hash, summary, size) = (last.hash.clone(), last.preview.clone(), last.size);
		crate::session::archive_turn(state, &hash, &summary, size);
	}
	crate::session::next_turn(state);
	// P3/T9: purge expired ephemeral nudges AFTER advancing the turn, so a
	// one-shot pushed during turn N renders in turn N+1's context exactly once
	// and is gone by turn N+2.
	crate::flow::purge_expired_nudges(state);
	// Poll-worker: expire stale tasks (not polled in >STALE_TURN_AGE turns)
	// and drop completed/failed tasks older than 16 turns.
	crate::poll_worker::expire_stale_tasks(state);
	serde_json::json!({"status": "ok", "turn": state.turn_counter})
}

/// Extract file path from tool output - heuristic.
fn extract_file_path(content: &str, tool: &str) -> Option<String> {
	match tool {
		"read_file" | "write_file" | "patch" => {
			// First line often contains path
			content.lines().next().and_then(|line| {
				let line = line.trim();
				if line.starts_with('/') || line.starts_with("./") {
					Some(line.to_string())
				} else {
					None
				}
			})
		},
		// `search_files` output is conventionally a `path:line:match` (grep-
		// style) list; the file being referenced is whatever's before the
		// first `:` on the first result line (F10: this tool is listed in
		// `state.file_tools` but was previously never actually matched here,
		// so every search result silently went untracked).
		"search_files" => content.lines().next().and_then(|line| {
			let path = line.split(':').next().unwrap_or("").trim();
			if path.starts_with('/') || path.starts_with("./") {
				Some(path.to_string())
			} else {
				None
			}
		}),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_essential_tool_skip() {
		let mut s = AphroditeState::default();
		let r = transform_tool_result(&mut s, "some content", "skill_view");
		assert_eq!(r["compressed"], false);
		assert_eq!(r["reason"], "essential_tool");
	}

	// ── T9 (F10): `read_file` is both an essential tool (never compressed -
	// the agent needs the raw content) AND a tracked file tool
	// (`state.file_tools`); both must hold at once. Previously the essential-
	// tool early return ran before file-reference tracking, so the single
	// most common file reference (reading a file) was silently never
	// recorded in `referenced_files`.
	#[test]
	fn test_essential_tool_read_file_still_records_file_reference() {
		let mut s = AphroditeState::default();
		let r = transform_tool_result(&mut s, "/tmp/some/file.rs\nfn main() {}\n", "read_file");
		assert_eq!(r["compressed"], false, "read_file must never be compressed");
		assert_eq!(r["reason"], "essential_tool");
		assert_eq!(
			s.referenced_files.len(),
			1,
			"read_file must still be tracked as a file reference"
		);
		assert_eq!(s.referenced_files[0].0, "/tmp/some/file.rs");
	}

	// ── T9 (F10): `search_files` reaches the file-reference tracker (it's
	// listed in `state.file_tools`) but `extract_file_path` had no arm for
	// it, so search results were never recorded despite being tracked tools.
	#[test]
	fn test_search_files_records_file_reference() {
		let mut s = AphroditeState::default();
		s.tool_threshold = 0; // always compress, to also exercise the non-essential path
		let content = "/tmp/some/file.rs:42:    let x = 1;\n/tmp/other/file.rs:7:    let y = 2;\n";
		let _ = transform_tool_result(&mut s, content, "search_files");
		assert_eq!(s.referenced_files.len(), 1);
		assert_eq!(s.referenced_files[0].0, "/tmp/some/file.rs");
	}

	#[test]
	fn test_empty_skip() {
		let mut s = AphroditeState::default();
		let r = transform_tool_result(&mut s, "", "terminal");
		assert_eq!(r["compressed"], false);
	}

	#[test]
	fn test_below_threshold() {
		let mut s = AphroditeState::default();
		s.tool_threshold = 10000;
		let r = transform_tool_result(&mut s, "short", "terminal");
		assert_eq!(r["compressed"], false);
	}

	#[test]
	fn test_transform_success() {
		let mut s = AphroditeState::default();
		s.tool_threshold = 0; // always compress
		let content = "fn main() {\n    println!(\"hello world\");\n}\n";
		let r = transform_tool_result(&mut s, content, "terminal");
		assert_eq!(r["compressed"], true);
		let hash = r["hash"].as_str().unwrap();
		assert!(hash.len() >= 40);
		// The round-trip is the actual promise here: the marker this hook hands
		// back to the LLM must resolve, via aphrodite_retrieve, back to the
		// exact original content - not merely produce a hash-shaped string.
		let resolved = crate::resolve::expand(&mut s, hash);
		assert_eq!(resolved, Some(content.to_string()));
	}

	#[test]
	fn test_terminal_exit_code() {
		let mut s = AphroditeState::default();
		s.terminal_threshold = 0;
		let r = transform_terminal_output(&mut s, "error: broke\nexit code: 1\n");
		assert_eq!(r["type"], "terminal");
	}

	// ── T13 (F11): post_llm_call must archive the turn's last marker into
	// conv_index so aphrodite_diff returns non-zero results.
	#[test]
	fn test_post_llm_call_archives_turn() {
		let mut s = AphroditeState::default();
		s.turn_counter = 2;
		// Record a marker on the current turn.
		s.record_marker(MarkerEntry {
			hash: "hash42".into(),
			ccr_type: "text".into(),
			size: 100,
			preview: "[text] summary".into(),
			turn: 2,
			center: None,
			meta: None,
		});
		let _ = post_llm_call(&mut s);
		assert_eq!(s.turn_counter, 3);
		assert_eq!(s.conv_index.len(), 1, "post_llm_call must archive the turn");
		assert_eq!(s.conv_index[&2].0, "hash42");
	}

	// ── Poll-worker integration tests ──────────────────────────
	// Agent-agnostic tests: check_bg_tasks in pre_llm_call and
	// expire_stale_tasks in post_llm_call.  Agent-specific trigger
	// tests (auto-backgrounding from tool results) live in
	// aphrodite-hermes.

	#[test]
	fn test_pre_llm_call_pushes_poll_nudges() {
		let mut s = AphroditeState::default();
		s.turn_counter = 3;
		crate::poll_worker::insert_bg_task(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);
		let _ = pre_llm_call(&mut s);
		// The nudge should have been pushed into ephemeral_directives.
		assert!(s.ephemeral_directives.len() >= 1, "pre_llm_call should push poll-worker nudge");
		let nudge = s.ephemeral_directives.last().unwrap();
		assert!(
			nudge.inline.as_deref().unwrap().contains("cargo build"),
			"nudge should mention the command"
		);
	}

	#[test]
	fn test_post_llm_call_expires_stale_poll_tasks() {
		let mut s = AphroditeState::default();
		s.turn_counter = 20;
		crate::poll_worker::insert_bg_task(
			&mut s,
			"old-task".into(),
			"terminal".into(),
			"old-build".into(),
			10, // started at turn 10, gap=10 < 16 so retain won't drop it
		);
		s.bg_tasks[0].last_poll_turn = 10; // never polled after start, gap=10 > 8
		let _ = post_llm_call(&mut s);
		// Should be marked stale.
		assert_eq!(
			s.bg_tasks[0].status,
			crate::poll_worker::BgStatus::Stale,
			"unpolled task should be expired as stale after {} turns",
			crate::poll_worker::STALE_TURN_AGE
		);
	}

	#[test]
	fn test_pre_llm_call_skips_nudges_when_poll_worker_disabled() {
		let mut s = AphroditeState::default();
		s.poll_worker_enabled = false;
		s.turn_counter = 3;
		crate::poll_worker::insert_bg_task(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);
		let before = s.ephemeral_directives.len();
		let _ = pre_llm_call(&mut s);
		assert_eq!(
			s.ephemeral_directives.len(),
			before,
			"disabled flag must prevent poll-worker nudges in pre_llm_call"
		);
	}

	#[test]
	fn test_post_llm_call_still_expires_when_poll_worker_disabled() {
		let mut s = AphroditeState::default();
		s.poll_worker_enabled = false;
		s.turn_counter = 20;
		crate::poll_worker::insert_bg_task(&mut s, "old-task".into(), "terminal".into(), "old-build".into(), 10);
		s.bg_tasks[0].last_poll_turn = 10;
		let _ = post_llm_call(&mut s);
		// Expiry is a cleanup concern — should still run even when disabled.
		assert_eq!(
			s.bg_tasks[0].status,
			crate::poll_worker::BgStatus::Stale,
			"expire_stale_tasks must still run when disabled (cleanup)"
		);
	}
}
