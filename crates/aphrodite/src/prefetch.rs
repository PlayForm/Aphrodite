//! Prefetch - background file loading into CCR.
//! Port of plugins/aphrodite/_hooks/prefetch.py
//!
//! Agent-agnostic: any agent can preload files into the compression store
//! before they're needed, avoiding round-trips during critical paths.

use std::path::{Path, PathBuf};

use crate::state::{AphroditeState, MarkerEntry};

/// Maximum file size for prefetch (10MB).
const MAX_PREFETCH_SIZE:u64 = 10 * 1024 * 1024;

/// Outcome of reading one path, before any state mutation.
pub enum ReadOutcome {
	Missing,
	/// Path refused by the prefetch guard (`.env` / `auth.json` / `~/.ssh` /
	/// Hermes home / non-file) - never read, never stored.
	Refused {
		reason:String,
	},
	Error,
	SkippedSize {
		size:u64,
	},
	Loaded {
		content:String,
		size:u64,
	},
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
			if let Some(reason) = refuse_reason(path) {
				return (path_str.clone(), ReadOutcome::Refused { reason });
			}
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

/// Refuse a path that must never be prefetched - the core-crate equivalent
/// of the bridge crate's `read_path_guarded`
/// (crates/aphrodite-hermes/src/tools.rs), same refusal intent, 1:1
/// semantics, implemented here so the core crate needs no dependency on the
/// bridge crate:
///
/// - `.env` / `auth.json` files, wherever they live (even inside the
///   workspace - prefetch must never absorb credentials),
/// - anything under `~/.ssh`,
/// - anything under the Hermes home (`~/.hermes`, or `$HERMES_HOME` when
///   set - resolved the same way the bridge does via `home::hermes_home`),
/// - existing non-file paths (directories etc.).
///
/// Returns the refusal reason, or `None` when the path is allowed through
/// to the size/read checks.
fn refuse_reason(path:&Path) -> Option<String> {
	// File-name denylist first - checked on the raw path so a relative
	// `.env` in the cwd is caught without any resolution.
	if let Some(name) = path.file_name().and_then(|n| n.to_str())
		&& (name == ".env" || name == "auth.json")
	{
		return Some(format!("refusing sensitive file name: {path:?}"));
	}
	let canon = guard_absolute(path);
	if let Some(home) = dirs::home_dir()
		&& canon.starts_with(home.join(".ssh"))
	{
		return Some(format!("refusing path under ~/.ssh: {path:?}"));
	}
	let hh = crate::home::hermes_home();
	let hh = hh.canonicalize().unwrap_or(hh);
	if canon.starts_with(&hh) {
		return Some(format!("refusing path under Hermes home ({}): {path:?}", hh.display()));
	}
	if path.exists() && !path.is_file() {
		return Some(format!("refusing non-file path: {path:?}"));
	}
	None
}

/// Absolute, `~`-expanded form of `path` for the guard's prefix checks:
/// canonicalized when the path resolves (so `..`/symlinks can't dodge the
/// checks), otherwise the cwd-joined absolute path (a refusal like
/// `~/.ssh/id_rsa` must hold even before the file exists). Tilde expansion
/// mirrors `home.rs`'s `expand_tilde`.
fn guard_absolute(path:&Path) -> PathBuf {
	let s = path.to_string_lossy();
	let expanded:PathBuf = if s == "~" {
		dirs::home_dir().unwrap_or_else(|| path.to_path_buf())
	} else if let Some(rest) = s.strip_prefix("~/")
		&& let Some(home) = dirs::home_dir()
	{
		home.join(rest)
	} else {
		path.to_path_buf()
	};
	let abs = if expanded.is_absolute() {
		expanded
	} else {
		std::env::current_dir().map(|c| c.join(&expanded)).unwrap_or(expanded)
	};
	abs.canonicalize().unwrap_or(abs)
}

/// Classify and store already-read file contents into `state`. Pure state
/// mutation + JSON assembly - no I/O, so this is the only part that needs
/// the lock.
pub fn insert_outcomes(state:&mut AphroditeState, outcomes:Vec<(String, ReadOutcome)>) -> serde_json::Value {
	let total = outcomes.len();
	let mut results = Vec::with_capacity(total);
	let mut loaded = 0u32;
	let mut skipped_size = 0u32;
	let mut refused = 0u32;
	let mut missing = 0u32;

	for (path_str, outcome) in outcomes {
		match outcome {
			ReadOutcome::Missing | ReadOutcome::Error => {
				missing += 1;
				results.push(serde_json::json!({"path": path_str, "status": "missing"}));
			},
			ReadOutcome::Refused { reason } => {
				refused += 1;
				results.push(serde_json::json!({
					"path": path_str,
					"status": "refused",
					"reason": reason,
				}));
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
				let ct = headroom_core::transforms::content_detector::detect_content_type(&content).content_type;
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
		"refused": refused,
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
		let r = prefetch_files(&mut s, std::slice::from_ref(&src));
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

	// ── Teknium1 review ask 1: prefetch applies the same guard as
	// `aphrodite_retrieve` (`read_path_guarded` semantics, implemented here
	// in the core crate): refuse `.env` / `auth.json` / `~/.ssh` /
	// Hermes-home / non-file paths with a distinct `refused` outcome. ──

	#[test]
	fn test_prefetch_refuses_dot_env() {
		// Exact file name `.env` is refused even though it exists and is
		// small - prefetch must never absorb credentials.
		let dir = std::env::temp_dir().join(format!("aphrodite_prefetch_dotenv_{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join(".env");
		std::fs::write(&path, "SECRET=value\n").unwrap();

		let mut s = AphroditeState::default();
		let r = prefetch_files(&mut s, &[path.to_string_lossy().to_string()]);
		assert_eq!(r["refused"], 1, "dot-env must be refused: {r:?}");
		assert_eq!(r["loaded"], 0);
		assert_eq!(r["results"][0]["status"], "refused");
		assert!(r["results"][0]["reason"].as_str().unwrap().contains(".env"));

		let _ = std::fs::remove_dir_all(&dir);
	}

	#[test]
	fn test_prefetch_refuses_auth_json() {
		let dir = std::env::temp_dir().join(format!("aphrodite_prefetch_auth_{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("auth.json");
		std::fs::write(&path, "{\"token\":\"x\"}\n").unwrap();

		let mut s = AphroditeState::default();
		let r = prefetch_files(&mut s, &[path.to_string_lossy().to_string()]);
		assert_eq!(r["refused"], 1, "auth.json must be refused: {r:?}");
		assert_eq!(r["loaded"], 0);
		assert_eq!(r["results"][0]["status"], "refused");
		assert!(r["results"][0]["reason"].as_str().unwrap().contains("auth.json"));

		let _ = std::fs::remove_dir_all(&dir);
	}

	#[test]
	fn test_prefetch_refuses_ssh() {
		// Refusal is by `~/.ssh` prefix, so the file need not exist.
		let home = dirs::home_dir().expect("test machine has a home");
		let abs = home.join(".ssh").join("id_rsa");
		let mut s = AphroditeState::default();
		let r = prefetch_files(&mut s, &[abs.to_string_lossy().to_string()]);
		assert_eq!(r["refused"], 1, "~/.ssh path must be refused: {r:?}");
		assert_eq!(r["results"][0]["status"], "refused");

		// The literal tilde form must be refused too.
		let r2 = prefetch_files(&mut s, &["~/.ssh/id_rsa".to_string()]);
		assert_eq!(r2["refused"], 1, "tilde ~/.ssh path must be refused: {r2:?}");
	}

	#[test]
	fn test_prefetch_refuses_hermes_home() {
		// Hold the SHARED env guard (crate::home::env_guard - one mutex for
		// every env-mutating test) for the WHOLE test, and clear HERMES_HOME
		// first: the default-home refusal must never observe a concurrent
		// test's transient override (that would silently turn `refused` into
		// `missing`). Default Hermes home (`~/.hermes`): refusal is by
		// prefix, no file needed on disk.
		let _g = crate::home::env_guard();
		let prior = std::env::var_os("HERMES_HOME");
		unsafe { std::env::remove_var("HERMES_HOME") };
		let home = dirs::home_dir().expect("test machine has a home");
		let p = home.join(".hermes").join("aphrodite").join("tokens.json");
		let mut s = AphroditeState::default();
		let r = prefetch_files(&mut s, &[p.to_string_lossy().to_string()]);
		assert_eq!(r["refused"], 1, "~/.hermes path must be refused: {r:?}");
		assert_eq!(r["results"][0]["status"], "refused");

		// `$HERMES_HOME` override must be honored.
		let dir = std::env::temp_dir().join(format!("aphrodite_prefetch_hh_{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		std::fs::write(dir.join("secrets.toml"), "token = \"x\"\n").unwrap();
		unsafe { std::env::set_var("HERMES_HOME", &dir) };
		let r2 = prefetch_files(&mut s, &[dir.join("secrets.toml").to_string_lossy().to_string()]);
		assert_eq!(r2["refused"], 1, "$HERMES_HOME path must be refused: {r2:?}");
		match prior {
			Some(v) => unsafe { std::env::set_var("HERMES_HOME", v) },
			None => unsafe { std::env::remove_var("HERMES_HOME") },
		}
		let _ = std::fs::remove_dir_all(&dir);
	}

	#[test]
	fn test_prefetch_refuses_non_file_directory() {
		// An existing directory is a non-file path - refused, distinct from
		// a missing file (which stays `missing`).
		let dir = std::env::temp_dir().join(format!("aphrodite_prefetch_dir_{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();

		let mut s = AphroditeState::default();
		let r = prefetch_files(&mut s, &[dir.to_string_lossy().to_string()]);
		assert_eq!(r["refused"], 1, "directory must be refused: {r:?}");

		let _ = std::fs::remove_dir_all(&dir);
	}
}
