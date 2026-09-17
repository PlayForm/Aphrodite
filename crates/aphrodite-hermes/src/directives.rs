//! Materialization of the binary-provided behavioral directives into the
//! runtime home.
//!
//! The shipped directive set is EMBEDDED in the core `aphrodite` crate
//! (`crates/aphrodite/src/builtin_directives/*.md`, compiled in via
//! `include_str!` and exposed through `aphrodite::directives::loaded_builtins()`).
//! This module provisions those builtins into the user-data home
//! (`~/.hermes/aphrodite/directives/` by default) so the plugin directory
//! never has to hold runtime state: the binary is the provider, the runtime
//! home is the store, and the core directive loader reads the same location
//! (its home-namespace candidate, or `$APHRODITE_DIRECTIVES_DIR` when set).
//!
//! Provisioning is idempotent and strictly non-destructive: files are only
//! ever written when missing; a pre-existing file that differs from the
//! embedded bytes is left untouched (user data wins). Files are written as
//! `<name>.md` (the core loader only reads `*.md` files and derives the
//! directive name from the file stem).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

/// Resolve the target directives directory for provisioning.
///
/// Precedence (first non-empty wins):
///   1. `home_param` - explicit caller-supplied home (the FFI argument);
///      directives go to `<home_param>/directives/`.
///   2. `$APHRODITE_DIRECTIVES_DIR` - the directive loader's exact-directory
///      override (core crate candidate 0); materialize straight into it.
///   3. `$APHRODITE_HOME` - home-level override; directives go to
///      `<APHRODITE_HOME>/directives/`.
///   4. `$HOME/.hermes/aphrodite` - the established user-data home.
///   5. `.` - degraded fallback with a warning (never fail).
///
/// Returns `(directives_dir, warnings)`.
pub(crate) fn resolve_directives_dir(home_param: &str) -> (PathBuf, Vec<String>) {
	let mut warnings: Vec<String> = Vec::new();

	if !home_param.is_empty() {
		return (PathBuf::from(home_param).join("directives"), warnings);
	}
	if let Ok(dir) = std::env::var("APHRODITE_DIRECTIVES_DIR") {
		if !dir.trim().is_empty() {
			return (PathBuf::from(dir), warnings);
		}
	}
	if let Ok(home) = std::env::var("APHRODITE_HOME") {
		if !home.trim().is_empty() {
			return (PathBuf::from(home).join("directives"), warnings);
		}
	}
	if let Ok(home) = std::env::var("HOME") {
		if !home.trim().is_empty() {
			return (
				PathBuf::from(home.trim_end_matches('/')).join(".hermes/aphrodite/directives"),
				warnings,
			);
		}
	}
	warnings.push(
		"neither $HOME nor $APHRODITE_HOME nor $APHRODITE_DIRECTIVES_DIR is set; using current directory".into(),
	);
	(PathBuf::from(".").join("directives"), warnings)
}

/// The set to provision: the core crate's embedded builtins.
///
/// `loaded_builtins()` caps content at `MAX_DIRECTIVE_CHARS` (2000); every
/// shipped file is currently well under the cap, so the materialized bytes
/// match the embedded sources verbatim. Sorted by name for deterministic
/// output ordering.
fn loaded_builtin_set() -> Vec<(String, String)> {
	let mut set: Vec<(String, String)> = aphrodite::directives::loaded_builtins()
		.into_iter()
		.map(|(name, d)| (name, d.content))
		.collect();
	set.sort_by(|a, b| a.0.cmp(&b.0));
	set
}

/// Provision the embedded builtin directives into `dir`.
///
/// Idempotent and non-destructive: missing files are written, byte-identical
/// files are skipped, and existing DIFFERENT files are never overwritten
/// (skipped with a warning). Always returns `status: "ok"` - every failure
/// degrades to a `warning`, never a panic or an error status.
pub(crate) fn materialize(dir: &Path) -> serde_json::Value {
	let mut written: Vec<String> = Vec::new();
	let mut skipped: Vec<String> = Vec::new();
	let mut warnings: Vec<String> = Vec::new();

	if let Err(e) = fs::create_dir_all(dir) {
		let msg = format!(
			"could not create directives dir {}: {}; nothing materialized",
			dir.display(),
			e
		);
		warnings.push(msg.clone());
		eprintln!("aphrodite-hermes: {msg}");
		return json!({
			"status": "ok",
			"dir": dir.to_string_lossy(),
			"written": written,
			"skipped": skipped,
			"warnings": warnings,
		});
	}

	for (name, content) in loaded_builtin_set() {
		let file_name = format!("{name}.md");
		let dest = dir.join(&file_name);
		match fs::read(&dest) {
			Ok(existing) if existing == content.as_bytes() => skipped.push(file_name),
			Ok(_) => {
				warnings.push(format!(
					"directives/{file_name} exists with different content; leaving user-modified file as-is (not overwritten)"
				));
				skipped.push(file_name);
			}
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
				match fs::write(&dest, content.as_bytes()) {
					Ok(()) => written.push(file_name),
					Err(we) => {
						warnings.push(format!("could not write directives/{file_name}: {we}"));
						eprintln!("aphrodite-hermes: directives/{file_name} write failed: {we}");
					}
				}
			}
			Err(e) => {
				warnings.push(format!("could not read directives/{file_name}: {e}; leaving as-is"));
				skipped.push(file_name);
			}
		}
	}

	written.sort();
	skipped.sort();

	json!({
		"status": "ok",
		"dir": dir.to_string_lossy(),
		"written": written,
		"skipped": skipped,
		"warnings": warnings,
	})
}

/// Resolve the target dir and materialize; used by the FFI export.
///
/// Resolution warnings (e.g. a degraded `$HOME` fallback) are logged and
/// folded into the report's `warnings` array.
pub(crate) fn materialize_into(home_param: &str) -> serde_json::Value {
	let (dir, warnings) = resolve_directives_dir(home_param);
	for w in &warnings {
		eprintln!("aphrodite-hermes: {w}");
	}
	let mut result = materialize(&dir);
	if let Some(arr) = result.get_mut("warnings").and_then(|v| v.as_array_mut()) {
		for w in warnings {
			arr.push(serde_json::Value::String(w));
		}
	}
	result
}

#[cfg(test)]
mod tests {
	use std::ffi::{CStr, CString};

	use super::*;

	/// Serializes tests that mutate process-global env vars.
	fn env_guard() -> std::sync::MutexGuard<'static, ()> {
		static G: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
		G.get_or_init(|| std::sync::Mutex::new(()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
	}

	fn temp_home(tag: &str) -> PathBuf {
		let base = std::env::temp_dir().join(format!(
			"aphrodite-directives-{tag}-{}-{}",
			std::process::id(),
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap_or_default()
				.as_nanos(),
		));
		fs::create_dir_all(&base).unwrap();
		base
	}

	/// The expected on-disk files: embedded names with the `.md` extension.
	fn expected_files() -> Vec<(String, String)> {
		loaded_builtin_set()
			.into_iter()
			.map(|(name, content)| (format!("{name}.md"), content))
			.collect()
	}

	#[test]
	fn materialize_writes_all_embedded_files() {
		let home = temp_home("write");
		let dir = home.join("directives");
		let expected = expected_files();

		let v = materialize(&dir);
		assert_eq!(v["status"], "ok");
		assert_eq!(v["written"].as_array().unwrap().len(), expected.len());
		assert_eq!(v["skipped"].as_array().unwrap().len(), 0);
		assert_eq!(v["warnings"].as_array().unwrap().len(), 0);
		for (file_name, content) in &expected {
			let p = dir.join(file_name);
			assert!(p.is_file(), "{file_name} must be materialized");
			assert_eq!(
				fs::read(&p).unwrap(),
				content.as_bytes(),
				"{file_name} must match the embedded bytes"
			);
		}
		fs::remove_dir_all(&home).ok();
	}

	#[test]
	fn materialize_is_idempotent() {
		let home = temp_home("idem");
		let dir = home.join("directives");
		let expected = expected_files();

		let first = materialize(&dir);
		assert_eq!(first["written"].as_array().unwrap().len(), expected.len());

		let second = materialize(&dir);
		assert_eq!(
			second["written"].as_array().unwrap().len(),
			0,
			"re-run must write nothing"
		);
		assert_eq!(second["skipped"].as_array().unwrap().len(), expected.len());
		assert_eq!(second["warnings"].as_array().unwrap().len(), 0);

		for (file_name, content) in &expected {
			assert_eq!(fs::read(dir.join(file_name)).unwrap(), content.as_bytes());
		}
		fs::remove_dir_all(&home).ok();
	}

	#[test]
	fn materialize_never_overwrites_user_modified_file() {
		let home = temp_home("keep");
		let dir = home.join("directives");
		fs::create_dir_all(&dir).unwrap();
		let expected = expected_files();
		let victim = dir.join("focus.md");
		let user_content = b"# focus\nUSER-CUSTOMIZED-DIRECTIVE\n";
		fs::write(&victim, user_content).unwrap();

		let v = materialize(&dir);
		assert_eq!(v["status"], "ok");
		assert!(
			v["skipped"].as_array().unwrap().iter().any(|s| s == "focus.md"),
			"focus.md must be reported as skipped: {v}"
		);
		assert_eq!(v["written"].as_array().unwrap().len(), expected.len() - 1);
		assert!(
			v["warnings"]
				.as_array()
				.unwrap()
				.iter()
				.any(|w| w.as_str().unwrap().contains("user-modified")),
			"a user-modified skip must carry a warning: {v}"
		);
		assert_eq!(
			fs::read(&victim).unwrap(),
			user_content,
			"user-modified file must be left untouched"
		);

		for (file_name, content) in &expected {
			if file_name != "focus.md" {
				assert_eq!(fs::read(dir.join(file_name)).unwrap(), content.as_bytes());
			}
		}
		fs::remove_dir_all(&home).ok();
	}

	#[test]
	fn resolve_directives_dir_precedence() {
		let _g = env_guard();
		std::env::remove_var("APHRODITE_DIRECTIVES_DIR");
		std::env::remove_var("APHRODITE_HOME");
		std::env::remove_var("HOME");

		// No override at all -> degraded fallback with a warning, never fails.
		let (dir, warnings) = resolve_directives_dir("");
		assert!(dir.to_string_lossy().ends_with("directives"));
		assert!(!warnings.is_empty(), "degraded fallback must warn");

		// $APHRODITE_HOME -> <home>/directives.
		std::env::set_var("APHRODITE_HOME", "/tmp/aph-home");
		let (dir, warnings) = resolve_directives_dir("");
		assert_eq!(dir, PathBuf::from("/tmp/aph-home/directives"));
		assert!(warnings.is_empty());

		// $APHRODITE_DIRECTIVES_DIR (the loader's candidate 0) beats the home override.
		std::env::set_var("APHRODITE_DIRECTIVES_DIR", "/tmp/aph-exact");
		let (dir, _) = resolve_directives_dir("");
		assert_eq!(dir, PathBuf::from("/tmp/aph-exact"));

		// An explicit home param beats every env override.
		let (dir, _) = resolve_directives_dir("/tmp/param-home");
		assert_eq!(dir, PathBuf::from("/tmp/param-home/directives"));

		std::env::remove_var("APHRODITE_DIRECTIVES_DIR");
		std::env::remove_var("APHRODITE_HOME");
	}

	// ── FFI round-trip: the exported C ABI entry point works end-to-end. ──
	#[test]
	fn ffi_materialize_directives_roundtrip() {
		let home = temp_home("ffi");
		let home_c = CString::new(home.to_str().unwrap()).unwrap();
		let ptr = crate::aphrodite_hermes_materialize_directives(home_c.as_ptr());
		assert!(!ptr.is_null());
		let json = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		crate::aphrodite_hermes_free_string(ptr);
		let v: serde_json::Value = serde_json::from_str(&json).unwrap();
		assert_eq!(v["status"], "ok");
		assert_eq!(
			v["written"].as_array().unwrap().len(),
			expected_files().len(),
			"all embedded files must be written: {v}"
		);
		assert!(
			home.join("directives").join("focus.md").is_file(),
			"focus.md must be materialized as <home>/directives/focus.md"
		);
		fs::remove_dir_all(&home).ok();
	}
}