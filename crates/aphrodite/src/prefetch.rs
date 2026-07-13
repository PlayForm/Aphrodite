//! Prefetch - background file loading into CCR.
//! Port of plugins/aphrodite/_hooks/prefetch.py
//!
//! Agent-agnostic: any agent can preload files into the compression store
//! before they're needed, avoiding round-trips during critical paths.

use std::path::Path;

use crate::state::{AphroditeState, MarkerEntry};

/// Maximum file size for prefetch (10MB).
const MAX_PREFETCH_SIZE:u64 = 10 * 1024 * 1024;

/// Outcome of reading one path, before any state mutation.
pub enum ReadOutcome {
	Missing,
	Error,
	SkippedSize { size:u64 },
	Loaded { content:String, size:u64 },
}

/// Read every path from disk - no state access, so this never needs to hold
/// whatever lock guards the caller's `AphroditeState` (F9: a global
/// process/handle lock held across file I/O serializes every other
/// session's every call behind one slow/cold-mount read).
pub fn read_paths(paths:&[String]) -> Vec<(String, ReadOutcome)> {
	paths
		.iter()
		.map(|path_str| {
			let path = Path::new(path_str);
			if !path.is_file() {
				return (path_str.clone(), ReadOutcome::Missing);
			}
			let size = match std::fs::metadata(path) {
				Ok(m) => m.len(),
				Err(_) => return (path_str.clone(), ReadOutcome::Error),
			};
			if size > MAX_PREFETCH_SIZE {
				return (path_str.clone(), ReadOutcome::SkippedSize { size });
			}
			match std::fs::read_to_string(path) {
				Ok(content) => (path_str.clone(), ReadOutcome::Loaded { content, size }),
				Err(_) => (path_str.clone(), ReadOutcome::Error),
			}
		})
		.collect()
}

/// Classify and store already-read file contents into `state`. Pure state
/// mutation + JSON assembly - no I/O, so this is the only part that needs
/// the lock.
pub fn insert_outcomes(state:&mut AphroditeState, outcomes:Vec<(String, ReadOutcome)>) -> serde_json::Value {
	let total = outcomes.len();
	let mut results = Vec::with_capacity(total);
	let mut loaded = 0u32;
	let mut skipped_size = 0u32;
	let mut missing = 0u32;

	for (path_str, outcome) in outcomes {
		match outcome {
			ReadOutcome::Missing | ReadOutcome::Error => {
				missing += 1;
				results.push(serde_json::json!({"path": path_str, "status": "missing"}));
			},
			ReadOutcome::SkippedSize { size } => {
				skipped_size += 1;
				results.push(serde_json::json!({
					"path": path_str,
					"status": "skipped",
					"reason": format!(
						"exceeds per-file prefetch limit ({} bytes > {} byte max)",
						size, MAX_PREFETCH_SIZE
					),
					"size": size,
				}));
			},
			ReadOutcome::Loaded { content, size } => {
				let ct = headroom_core::transforms::detect(&content);
				let hash = headroom_core::ccr::compute_key(content.as_bytes());
				let type_str = ct.as_str();
				let preview = crate::build_preview(type_str, &content);

				state.inline_store_put(hash.clone(), content);
				state.record_marker(MarkerEntry {
					hash:hash.clone(),
					ccr_type:type_str.to_string(),
					size:size as usize,
					preview:preview.clone(),
					turn:state.turn_counter,
					center:None,
					meta:Some({
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
					// Full 40-char hash - a truncated one is unresolvable via
					// exact-match retrieval (report 05 F3).
					"hash": &hash,
					"type": type_str,
					"size": size,
					"preview": preview,
				}));
			},
		}
	}

	serde_json::json!({
		"total": total,
		"loaded": loaded,
		"skipped_size": skipped_size,
		"missing": missing,
		"results": results,
		// Report 05 F11: prefetched content joins the same byte-budgeted
		// inline store as everything else, and can silently evict older
		// entries (including ones this very batch just loaded, if the batch
		// itself exceeds the budget) - surface the current usage/budget so a
		// caller can tell whether that happened instead of discovering it
		// only when a later retrieve unexpectedly misses.
		"inline_store_bytes": state.inline_store_bytes(),
		"inline_store_byte_budget": state.inline_store_byte_budget(),
	})
}

/// Prefetch a list of file paths into the inline store.
/// Returns JSON with status per file: loaded, skipped (too large), missing.
///
/// Convenience wrapper: does the read and the insert back-to-back, still
/// requiring `state` (and thus whatever lock guards it) for the whole call.
/// Callers that already hold a lock across the whole operation (like this
/// crate's own tests, and `aphrodite-hermes`'s `with_shared`) can keep using
/// this directly. Callers that want to read files *before* taking their
/// lock - the point of this split - call `read_paths` then `insert_outcomes`
/// separately; see the `aphrodite_dispatch` "prefetch" arm in `lib.rs`.
pub fn prefetch_files(state:&mut AphroditeState, paths:&[String]) -> serde_json::Value {
	insert_outcomes(state, read_paths(paths))
}

#[cfg(test)]
mod tests {
	use super::*;

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
		let src = env!("CARGO_MANIFEST_DIR").to_string() + "/src/prefetch.rs";
		let r = prefetch_files(&mut s, &[src.clone()]);
		assert_eq!(r["loaded"], 1, "prefetch failed: {:?}", r);
		assert_eq!(s.recent_markers.len(), 1);
	}

	// ── T11 (F11): prefetch surfaces the inline store's byte budget ──
	#[test]
	fn test_prefetch_response_surfaces_inline_store_budget() {
		let mut s = AphroditeState::default();
		let src = env!("CARGO_MANIFEST_DIR").to_string() + "/src/prefetch.rs";
		let r = prefetch_files(&mut s, &[src]);
		assert_eq!(r["inline_store_byte_budget"], 256 * 1024 * 1024);
		assert!(r["inline_store_bytes"].as_u64().unwrap() > 0);
	}

	#[test]
	fn test_prefetch_skip_size_reason_mentions_byte_limit() {
		// Use a real temp file over MAX_PREFETCH_SIZE rather than mocking
		// metadata, to exercise the actual `read_paths` size check.
		let dir = std::env::temp_dir();
		let path = dir.join(format!("aphrodite_prefetch_oversize_test_{}.txt", std::process::id()));
		std::fs::write(&path, vec![b'x'; (MAX_PREFETCH_SIZE + 1) as usize]).unwrap();

		let mut s = AphroditeState::default();
		let r = prefetch_files(&mut s, &[path.to_string_lossy().to_string()]);
		assert_eq!(r["skipped_size"], 1);
		let reason = r["results"][0]["reason"].as_str().unwrap();
		assert!(
			reason.contains(&MAX_PREFETCH_SIZE.to_string()),
			"reason should surface the byte limit: {reason}"
		);

		let _ = std::fs::remove_file(&path);
	}

	#[test]
	fn test_prefetch_multiple() {
		let mut s = AphroditeState::default();
		let src = env!("CARGO_MANIFEST_DIR").to_string() + "/src/prefetch.rs";
		let r = prefetch_files(&mut s, &[src, "/nonexistent/abc".to_string()]);
		assert_eq!(r["loaded"], 1);
		assert_eq!(r["missing"], 1);
	}
}
