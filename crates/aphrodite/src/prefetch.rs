//! Prefetch — background file loading into CCR.
//! Port of plugins/aphrodite/_hooks/prefetch.py
//!
//! Agent-agnostic: any agent can preload files into the compression store
//! before they're needed, avoiding round-trips during critical paths.

use crate::state::{AphroditeState, MarkerEntry};
use std::path::Path;

/// Maximum file size for prefetch (10MB).
const MAX_PREFETCH_SIZE: u64 = 10 * 1024 * 1024;

/// Prefetch a list of file paths into the inline store.
/// Returns JSON with status per file: loaded, skipped (too large), missing.
pub fn prefetch_files(state: &mut AphroditeState, paths: &[String]) -> serde_json::Value {
    let mut results = Vec::new();
    let mut loaded = 0u32;
    let mut skipped_size = 0u32;
    let mut missing = 0u32;

    for path_str in paths {
        let path = Path::new(path_str);

        // Check if file exists
        if !path.is_file() {
            missing += 1;
            results.push(serde_json::json!({
                "path": path_str,
                "status": "missing",
            }));
            continue;
        }

        // Check file size
        let size = match std::fs::metadata(path) {
            Ok(m) => m.len(),
            Err(_) => {
                missing += 1;
                results.push(serde_json::json!({"path": path_str, "status": "error"}));
                continue;
            }
        };

        if size > MAX_PREFETCH_SIZE {
            skipped_size += 1;
            results.push(serde_json::json!({
                "path": path_str,
                "status": "skipped",
                "reason": "exceeds 10MB limit",
                "size": size,
            }));
            continue;
        }

        // Read file content
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                missing += 1;
                results.push(serde_json::json!({"path": path_str, "status": "error"}));
                continue;
            }
        };

        // Classify and store
        let ct = headroom_core::transforms::detect(&content);
        let hash = headroom_core::ccr::compute_key(content.as_bytes());
        let type_str = ct.as_str();
        let preview = crate::build_preview(type_str, &content);

        state.inline_store_put(hash.clone(), content);

        state.record_marker(MarkerEntry {
            hash: hash.clone(),
            ccr_type: type_str.to_string(),
            size: size as usize,
            preview: preview.clone(),
            turn: state.turn_counter,
            center: None,
            meta: Some({
                let mut m = std::collections::HashMap::new();
                m.insert("path".to_string(), path_str.clone());
                m
            }),
        });

        loaded += 1;
        state.record_file(path_str.clone(), "prefetch".to_string());

        results.push(serde_json::json!({
            "path": path_str,
            "status": "loaded",
            "hash": &hash[..12.min(hash.len())],
            "type": type_str,
            "size": size,
            "preview": preview,
        }));
    }

    serde_json::json!({
        "total": paths.len(),
        "loaded": loaded,
        "skipped_size": skipped_size,
        "missing": missing,
        "results": results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_prefetch_missing() {
        let mut s = AphroditeState::default();
        let r = prefetch_files(&mut s, &["/nonexistent/file/xyzzy.txt".to_string()]);
        assert_eq!(r["missing"], 1);
        assert_eq!(r["loaded"], 0);
    }

    #[test]
    fn test_prefetch_real_file() {
        let mut s = AphroditeState::default();
        // Use the test file itself
        let r = prefetch_files(&mut s, &[file!().to_string()]);
        assert_eq!(r["loaded"], 1);
        assert_eq!(s.recent_markers.len(), 1);
    }

    #[test]
    fn test_prefetch_multiple() {
        let mut s = AphroditeState::default();
        let r = prefetch_files(&mut s, &[
            file!().to_string(),
            "/nonexistent/abc".to_string(),
        ]);
        assert_eq!(r["loaded"], 1);
        assert_eq!(r["missing"], 1);
    }
}
