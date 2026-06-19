//! Plugin hooks — 1:1 port of plugins/aphrodite/_hooks/transform.py + terminal.py
//!
//! These are the core compression hooks that any agent calls to compress output.

use crate::marker::ccr_marker;
use crate::state::AphroditeState;
use headroom_core::transforms;

/// Compress tool output. Returns JSON with hash, preview, type, size.
/// Mirrors Python's transform_tool_result logic.
pub fn transform_tool_result(
    state: &mut AphroditeState,
    content: &str,
    tool_name: &str,
) -> serde_json::Value {
    if content.is_empty() {
        return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
    }

    // Skip essential tools — agent needs them uncompressed
    let essential = [
        "skill_view", "skills_list", "skill_manage", "memory",
        "session_search", "read_file", "read_terminal",
    ];
    if essential.contains(&tool_name) {
        return serde_json::json!({"status": "ok", "compressed": false, "reason": "essential_tool"});
    }

    // Classify content using headroom-core detector
    let ct = transforms::detect(content);
    let type_str = ct.as_str();

    // Compute hash
    let hash = headroom_core::ccr::compute_key(content.as_bytes());

    // Store in inline store
    state.inline_store_put(hash.clone(), content.to_string());

    // Build preview
    let preview = crate::build_preview(type_str, content);

    // Build marker
    let marker = ccr_marker(&hash, type_str, content.len(), &preview, None, None, None);

    // Record marker
    state.record_marker(crate::state::MarkerEntry {
        hash: hash.clone(),
        ccr_type: type_str.to_string(),
        size: content.len(),
        preview: preview.clone(),
        turn: state.turn_counter,
        center: None,
        meta: None,
    });

    // Record file references for file tools
    if state.file_tools.contains(&tool_name.to_string()) {
        if let Some(path) = extract_file_path(content) {
            state.record_file(path, tool_name.to_string());
        }
    }

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

/// Compress terminal output. Returns JSON.
/// Mirrors Python's transform_terminal_output logic.
pub fn transform_terminal_output(
    state: &mut AphroditeState,
    content: &str,
) -> serde_json::Value {
    if content.is_empty() {
        return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
    }

    if content.len() < state.terminal_threshold {
        return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
    }

    let ct = transforms::detect(content);
    let type_str = ct.as_str();
    let hash = headroom_core::ccr::compute_key(content.as_bytes());

    state.inline_store_put(hash.clone(), content.to_string());

    let preview = crate::build_preview(type_str, content);
    let marker = ccr_marker(&hash, type_str, content.len(), &preview, None, None, None);

    state.record_marker(crate::state::MarkerEntry {
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

/// Handle session start — reset counters, return version info.
pub fn on_session_start(state: &mut AphroditeState) -> serde_json::Value {
    state.reset_turns();
    state.reset_scanned();

    serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "engine": "aphrodite-ffi",
        "turn": 0,
    })
}

/// Try to extract a file path from tool output content.
fn extract_file_path(_content: &str) -> Option<String> {
    // Simple heuristic: first line often contains the file path
    // Full implementation would parse JSON or tool-specific formats
    None
}
