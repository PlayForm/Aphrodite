//! Full hook implementations — expanded from plugins/aphrodite/_hooks/
//!
//! transform_tool_result: content-aware compression with essential tool skip,
//!   file reference tracking, threshold gating, preview generation.
//! transform_terminal_output: terminal-specific compression with exit code
//!   detection, threshold gating, streaming support.

use crate::marker::ccr_marker;
use crate::state::{AphroditeState, MarkerEntry};
use headroom_core::transforms;

/// Essential tools that must NOT be compressed — agent needs raw output.
const ESSENTIAL_TOOLS: &[&str] = &[
    "skill_view", "skills_list", "skill_manage", "memory",
    "session_search", "read_file", "read_terminal",
];

/// Transform tool output — full compression pipeline.
pub fn transform_tool_result(
    state: &mut AphroditeState,
    content: &str,
    tool_name: &str,
) -> serde_json::Value {
    if content.is_empty() {
        return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
    }

    // Skip essential tools
    if ESSENTIAL_TOOLS.contains(&tool_name) {
        return serde_json::json!({"status": "ok", "compressed": false, "reason": "essential_tool"});
    }

    // Skip below threshold (0 = always compress)
    if state.tool_threshold > 0 && content.len() < state.tool_threshold {
        return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
    }

    let ct = transforms::detect(content);
    let type_str = ct.as_str();
    let hash = headroom_core::ccr::compute_key(content.as_bytes());

    state.inline_store_put(hash.clone(), content.to_string());

    let preview = crate::build_preview(type_str, content);
    let marker = ccr_marker(&hash, type_str, content.len(), &preview, None, None, None);

    state.record_marker(MarkerEntry {
        hash: hash.clone(),
        ccr_type: type_str.to_string(),
        size: content.len(),
        preview: preview.clone(),
        turn: state.turn_counter,
        center: None,
        meta: None,
    });

    // Track file references
    if state.file_tools.contains(&tool_name.to_string()) {
        if let Some(path) = extract_file_path(content, tool_name) {
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

/// Transform terminal output — exit code aware.
pub fn transform_terminal_output(
    state: &mut AphroditeState,
    content: &str,
) -> serde_json::Value {
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

/// Session start hook — full reset.
pub fn on_session_start(state: &mut AphroditeState) -> serde_json::Value {
    crate::session::on_session_start(state)
}

/// Pre-LLM call hook — inject catalog into context.
pub fn pre_llm_call(state: &AphroditeState) -> serde_json::Value {
    let summary = crate::session::catalog_summary(state);
    serde_json::json!({
        "status": "ok",
        "catalog": summary,
        "compressed_count": state.recent_markers.len(),
    })
}

/// Post-LLM call hook — archive turn.
pub fn post_llm_call(state: &mut AphroditeState) -> serde_json::Value {
    crate::session::next_turn(state);
    serde_json::json!({"status": "ok", "turn": state.turn_counter})
}

/// Extract file path from tool output — heuristic.
fn extract_file_path(content: &str, tool: &str) -> Option<String> {
    match tool {
        "read_file" | "write_file" | "patch" => {
            // First line often contains path
            content.lines().next()
                .and_then(|line| {
                    let line = line.trim();
                    if line.starts_with('/') || line.starts_with("./") {
                        Some(line.to_string())
                    } else {
                        None
                    }
                })
        }
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
        let r = transform_tool_result(&mut s, content, "read_file");
        assert_eq!(r["compressed"], true);
        assert!(r["hash"].as_str().unwrap().len() >= 40);
    }

    #[test]
    fn test_terminal_exit_code() {
        let mut s = AphroditeState::default();
        s.terminal_threshold = 0;
        let r = transform_terminal_output(&mut s, "error: broke\nexit code: 1\n");
        assert_eq!(r["type"], "terminal");
    }
}
