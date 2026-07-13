//! Full hook implementations - expanded from plugins/aphrodite/_hooks/
//!
//! transform_tool_result: content-aware compression with essential tool skip,
//!   file reference tracking, threshold gating, preview generation.
//! transform_terminal_output: terminal-specific compression with exit code
//!   detection, threshold gating, streaming support.

use headroom_core::transforms;

use crate::{
	marker::ccr_marker,
	state::{AphroditeState, MarkerEntry},
};

/// Compute a CCR hash for content using BLAKE3 (40 hex chars).
pub fn compute_hash(content:&str) -> String { headroom_core::ccr::compute_key(content.as_bytes()) }

/// Essential tools that must NOT be compressed - agent needs raw output.
const ESSENTIAL_TOOLS:&[&str] = &[
	"skill_view",
	"skills_list",
	"skill_manage",
	"memory",
	"session_search",
	"read_file",
	"read_terminal",
];

/// Extract meaningful content from Hermes tool-result JSON wrappers.
/// All Hermes tools return JSON like {"output":"...","exit_code":N} or
/// {"success":true,"diff":"..."}. The classifier sees '{' and returns
/// json_array — hiding the real content behind a useless preview.
/// This extracts the meaningful field(s) and returns (content, type).
pub fn extract_hermes_result(content:&str, classified_type:&str) -> (String, String) {
	if !classified_type.starts_with("json") {
		return (content.to_string(), classified_type.to_string());
	}
	let v:serde_json::Value = match serde_json::from_str(content) {
		Ok(v) => v,
		Err(_) => return (content.to_string(), classified_type.to_string()),
	};
	// Terminal output: {"output":"...","exit_code":N}
	if let Some(output) = v.get("output").and_then(|o| o.as_str()) {
		if output.is_empty() { return (content.to_string(), classified_type.to_string()); }
		let ct = if output.contains("exit code:") || output.contains("Error:") {
			"terminal".to_string()
		} else {
			transforms::detect(output).as_str().to_string()
		};
		return (output.to_string(), ct);
	}
	// Patch/write_file diff: {"success":..., "diff":"..."}
	if let Some(diff) = v.get("diff").and_then(|d| d.as_str()) {
		if !diff.is_empty() {
			let ct = transforms::detect(diff).as_str().to_string();
			return (diff.to_string(), ct);
		}
	}
	// Patch/write_file result message: {"success":true/false, "error":"..."}
	if let Some(msg) = v.get("error").or(v.get("success")).and_then(|m| m.as_str()) {
		if !msg.is_empty() {
			return (format!("result: {}", msg), "text".to_string());
		}
	}
	// Search results: {"total_count":N, "matches":[...]}
	if let Some(count) = v.get("total_count") {
		if let Some(n) = count.as_u64() {
			return (format!("{} results found", n), "search".to_string());
		}
	}
	// File read: {"content":"...","total_lines":N} (read_file is usually
	// essential-skipped, but if it reaches here, extract the content).
	if let Some(text) = v.get("content").and_then(|c| c.as_str()) {
		let ct = transforms::detect(text).as_str().to_string();
		return (text.to_string(), ct);
	}
	(content.to_string(), classified_type.to_string())
}

/// Transform tool output - full compression pipeline.
pub fn transform_tool_result(state:&mut AphroditeState, content:&str, tool_name:&str) -> serde_json::Value {
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

	let ct = transforms::detect(content);
	let type_str = ct.as_str();

	// Hermes wraps ALL tool results in JSON: {"output":"...","exit_code":N},
	// {"success":true,"diff":"..."}, {"total_count":N,"matches":[...]}, etc.
	// The classifier sees '{' and returns json_array — hiding the real
	// content behind a useless "[json:1items 1L]" preview. Extract the
	// meaningful field and re-classify so previews show terminal output,
	// diffs, build errors, search hit counts, etc.
	let (eff_content, eff_type) = extract_hermes_result(content, type_str);
	let hash = headroom_core::ccr::compute_key(eff_content.as_bytes());

	state.inline_store_put(hash.clone(), eff_content.clone());

	let preview = crate::build_preview(&eff_type, &eff_content);
	let marker = ccr_marker(&hash, &eff_type, eff_content.len(), &preview, None, None, None);

	state.record_marker(MarkerEntry {
		hash:hash.clone(),
		ccr_type:eff_type.clone(),
		size:eff_content.len(),
		preview:preview.clone(),
		turn:state.turn_counter,
		center:None,
		meta:None,
	});

	serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": eff_type,
		"size": eff_content.len(),
		"preview": preview,
		"marker": marker,
	})
}

/// Transform terminal output - exit code aware.
pub fn transform_terminal_output(state:&mut AphroditeState, content:&str) -> serde_json::Value {
	if content.is_empty() {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
	}

	if state.terminal_threshold > 0 && content.len() < state.terminal_threshold {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
	}

	let ct = transforms::detect(content);
	let type_str = if content.contains("exit code:") || content.contains("Error:") {
		"terminal"
	} else {
		ct.as_str()
	};

	let hash = headroom_core::ccr::compute_key(content.as_bytes());
	state.inline_store_put(hash.clone(), content.to_string());

	let preview = crate::build_preview(type_str, content);
	let marker = ccr_marker(&hash, type_str, content.len(), &preview, None, None, None);

	state.record_marker(MarkerEntry {
		hash:hash.clone(),
		ccr_type:type_str.to_string(),
		size:content.len(),
		preview:preview.clone(),
		turn:state.turn_counter,
		center:None,
		meta:None,
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
pub fn on_session_start(state:&mut AphroditeState) -> serde_json::Value { crate::session::on_session_start(state) }

/// Pre-LLM call hook - inject catalog + active directives into context.
pub fn pre_llm_call(state:&AphroditeState) -> serde_json::Value {
	let summary = crate::session::catalog_summary(state);
	let directives = crate::directives::build_directive_context(
		&state.directives,
		&state.active_directives,
	);
	serde_json::json!({
		"status": "ok",
		"catalog": summary,
		"compressed_count": state.recent_markers.len(),
		"directives": if directives.is_empty() { None } else { Some(directives) },
	})
}

/// Post-LLM call hook - archive turn.
///
/// Archives the last marker recorded this turn into `conv_index` before
/// advancing the turn counter (report 06 F11/T13) - previously `archive_turn`
/// was never called from any hook, so `conv_index` stayed empty forever and
/// `aphrodite_diff` always returned zero turns despite compressions
/// happening every turn.
pub fn post_llm_call(state:&mut AphroditeState) -> serde_json::Value {
	if let Some(last) = state.recent_markers.iter().rev().find(|m| m.turn == state.turn_counter) {
		let (hash, summary, size) = (last.hash.clone(), last.preview.clone(), last.size);
		crate::session::archive_turn(state, &hash, &summary, size);
	}
	crate::session::next_turn(state);
	serde_json::json!({"status": "ok", "turn": state.turn_counter})
}

/// Extract file path from tool output - heuristic.
fn extract_file_path(content:&str, tool:&str) -> Option<String> {
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
		"search_files" => {
			content.lines().next().and_then(|line| {
				let path = line.split(':').next().unwrap_or("").trim();
				if path.starts_with('/') || path.starts_with("./") {
					Some(path.to_string())
				} else {
					None
				}
			})
		},
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
	// conv_index, not just advance the turn counter - otherwise
	// `aphrodite_diff` always returns zero turns despite compressions
	// happening every turn. ──
	#[test]
	fn test_post_llm_call_archives_last_marker_of_turn() {
		let mut s = AphroditeState::default();
		s.tool_threshold = 0; // always compress
		let content = "fn main() {\n    println!(\"hello world\");\n}\n";
		let r = transform_tool_result(&mut s, content, "terminal");
		let hash = r["hash"].as_str().unwrap().to_string();
		assert!(s.conv_index.is_empty(), "not archived until post_llm_call runs");

		let post = post_llm_call(&mut s);
		assert_eq!(post["turn"], 1);
		assert_eq!(s.conv_index.len(), 1, "post_llm_call must archive the turn's marker");
		let turns = crate::session::get_conv_index(&s);
		assert_eq!(turns[0]["hash"], hash);
	}

	#[test]
	fn test_post_llm_call_with_no_markers_this_turn_does_not_archive() {
		let mut s = AphroditeState::default();
		post_llm_call(&mut s);
		assert!(s.conv_index.is_empty(), "nothing to archive when no marker was recorded this turn");
	}
}
