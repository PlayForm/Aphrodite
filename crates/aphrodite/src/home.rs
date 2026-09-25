//! Single runtime-home resolution shared by the engine binary (`aphrodite`)
//! and the Hermes bridge dylib (`aphrodite-hermes`).
//!
//! The runtime home is the user-data folder that holds `aphrodite.toml`,
//! `binaries/`, `directives/`, `ccr.db`, and the proxy logs. It must be
//! decided in exactly one place: historically the plugin shim (Python)
//! resolved it from `$HERMES_HOME` while the Rust half resolved it from
//! `$HOME`, so whenever `HERMES_HOME != $HOME/.hermes` (the shipped Docker
//! image, profile gateways) the two halves disagreed and the plugin disabled
//! itself (issue 40).
//!
//! This module resolves the same way the shim does (`$HERMES_HOME` ->
//! platform default, `hermes_constants.get_hermes_home()` precedence:
//! context override -> env var -> platform default), with `$APHRODITE_HOME`
//! staying as the explicit override above it; the shim additionally exports
//! its own decision into `$APHRODITE_HOME` (env setdefault at import), so in
//! a plugin process both halves converge on one directory by construction.

use std::path::PathBuf;

/// Non-empty env var value as a path, if set.
fn env_path(name:&str) -> Option<PathBuf> { std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from) }

/// Expand a leading `~`/`~/` (the Python shim expands env overrides with
/// `Path.expanduser()`; the Rust side must behave identically).
fn expand_tilde(p:PathBuf) -> PathBuf {
	let s = p.to_string_lossy();
	if s == "~" {
		return dirs::home_dir().unwrap_or(p);
	}
	if let Some(rest) = s.strip_prefix("~/")
		&& let Some(home) = dirs::home_dir()
	{
		return home.join(rest);
	}
	p
}

/// Hermes home: `$HERMES_HOME` when set (expanded), else the platform user
/// home + `.hermes`. Mirrors the plugin shim's `_hermes_home()` and
/// `hermes_constants.get_hermes_home()`: context override -> env var ->
/// platform default.
pub fn hermes_home() -> PathBuf { hermes_home_opt().unwrap_or_else(|| PathBuf::from(".")) }

/// `hermes_home()`, but `None` when nothing at all resolves (no env var and
/// no platform user home) so callers that must fail loudly (e.g. `aphrodite
/// setup`) can distinguish that from the degraded `.` fallback.
pub fn hermes_home_opt() -> Option<PathBuf> {
	if let Some(h) = env_path("HERMES_HOME") {
		return Some(expand_tilde(h));
	}
	dirs::home_dir().map(|h| h.join(".hermes"))
}

/// The Hermes plugin loader dir: `<hermes-home>/plugins/aphrodite` - the
/// hooks-only install location Hermes owns (plugin.yaml + `__init__.py`).
pub fn plugin_dir() -> PathBuf { hermes_home().join("plugins").join("aphrodite") }

/// Runtime home (the user-data folder that holds `aphrodite.toml`,
/// `binaries/`, `directives/`, `ccr.db`, logs).
///
/// Precedence (first non-empty wins), identical to the plugin shim's
/// `_runtime_home()`:
///   1. `$APHRODITE_HOME` - explicit home-level override (expanded); never
///      second-guessed. The shim exports its own decision through this var,
///      so in a plugin process this is the normal path.
///   2. `$HERMES_HOME` + `/aphrodite` - the Hermes home, matching the shim
///      (fixes issue 40: a standalone binary run under a non-default Hermes
///      home lands in the same place the shim would pick).
///   3. `$HOME` + `/.hermes/aphrodite` - the legacy user-data home.
///   4. platform user home + `/.hermes/aphrodite`.
///
/// Degrades to `.` (never fails) - the `_opt` variant below distinguishes
/// the degraded case for callers that need it.
pub fn runtime_home() -> PathBuf { runtime_home_opt().unwrap_or_else(|| PathBuf::from(".")) }

/// `runtime_home()`, but `None` when no home is resolvable at all (no
/// `$APHRODITE_HOME`, no `$HERMES_HOME`, no `$HOME`, no platform user home).
pub fn runtime_home_opt() -> Option<PathBuf> {
	if let Some(h) = env_path("APHRODITE_HOME") {
		return Some(expand_tilde(h));
	}
	if let Some(h) = env_path("HERMES_HOME") {
		return Some(expand_tilde(h).join("aphrodite"));
	}
	if let Some(h) = env_path("HOME") {
		return Some(expand_tilde(h).join(".hermes").join("aphrodite"));
	}
	dirs::home_dir().map(|h| h.join(".hermes").join("aphrodite"))
}

/// `aphrodite.toml` under the runtime home.
pub fn config_path() -> PathBuf { runtime_home().join("aphrodite.toml") }

/// `directives/` under the runtime home (the home-namespace directive store).
pub fn directives_dir() -> PathBuf { runtime_home().join("directives") }

/// `ccr.db` under the runtime home (token-mode SQLite CCR database).
pub fn ccr_db_path() -> PathBuf { runtime_home().join("ccr.db") }

#[cfg(test)]
mod tests {
	use super::*;

	/// Serializes tests that mutate process-global env vars.
	fn env_guard() -> std::sync::MutexGuard<'static, ()> {
		static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
		G.get_or_init(|| std::sync::Mutex::new(()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
	}

	fn clear_all() {
		unsafe {
			std::env::remove_var("APHRODITE_HOME");
			std::env::remove_var("HERMES_HOME");
			std::env::remove_var("HOME");
		}
	}

	#[test]
	fn precedence_aphrodite_home_wins() {
		let _g = env_guard();
		clear_all();
		unsafe { std::env::set_var("APHRODITE_HOME", "/tmp/aph-override") };
		unsafe { std::env::set_var("HERMES_HOME", "/tmp/hermes-home") };
		unsafe { std::env::set_var("HOME", "/tmp/plain-home") };
		assert_eq!(runtime_home(), PathBuf::from("/tmp/aph-override"));
		assert_eq!(runtime_home_opt(), Some(PathBuf::from("/tmp/aph-override")));
	}

	#[test]
	fn precedence_hermes_home_beats_home() {
		// The issue-40 shape: HERMES_HOME set to something that is NOT
		// $HOME/.hermes - the runtime home must follow HERMES_HOME.
		let _g = env_guard();
		clear_all();
		unsafe { std::env::set_var("HERMES_HOME", "/opt/data") };
		unsafe { std::env::set_var("HOME", "/opt/data") };
		assert_eq!(runtime_home(), PathBuf::from("/opt/data/aphrodite"));
		assert_eq!(hermes_home(), PathBuf::from("/opt/data"));
		assert_eq!(plugin_dir(), PathBuf::from("/opt/data/plugins/aphrodite"));
	}

	#[test]
	fn precedence_home_falls_back_to_legacy() {
		let _g = env_guard();
		clear_all();
		unsafe { std::env::set_var("HOME", "/tmp/plain-home") };
		assert_eq!(runtime_home(), PathBuf::from("/tmp/plain-home/.hermes/aphrodite"));
	}

	#[test]
	fn tilde_expansion() {
		let _g = env_guard();
		clear_all();
		unsafe { std::env::set_var("HERMES_HOME", "~/scratch-home") };
		let home = dirs::home_dir().expect("test machine has a home");
		assert_eq!(runtime_home(), home.join("scratch-home").join("aphrodite"));
		unsafe { std::env::set_var("APHRODITE_HOME", "~") };
		assert_eq!(runtime_home(), home);
	}

	#[test]
	fn derived_paths_follow_runtime_home() {
		let _g = env_guard();
		clear_all();
		unsafe { std::env::set_var("HERMES_HOME", "/opt/data") };
		let root = PathBuf::from("/opt/data/aphrodite");
		assert_eq!(config_path(), root.join("aphrodite.toml"));
		assert_eq!(directives_dir(), root.join("directives"));
		assert_eq!(ccr_db_path(), root.join("ccr.db"));
	}

	#[test]
	fn empty_env_values_are_ignored() {
		// Empty-string overrides must fall through to the next level, never
		// produce an empty path (the Python shim strips empty values the
		// same way).
		let _g = env_guard();
		clear_all();
		unsafe { std::env::set_var("APHRODITE_HOME", "") };
		unsafe { std::env::set_var("HERMES_HOME", "/tmp/hermes-home") };
		assert_eq!(runtime_home(), PathBuf::from("/tmp/hermes-home/aphrodite"));
		unsafe { std::env::set_var("HERMES_HOME", "") };
		unsafe { std::env::set_var("HOME", "/tmp/plain-home") };
		assert_eq!(runtime_home(), PathBuf::from("/tmp/plain-home/.hermes/aphrodite"));
	}

	#[test]
	fn degraded_returns_none_and_dot() {
		let _g = env_guard();
		clear_all();
		// HOME/HERMES_HOME/APHRODITE_HOME cleared; dirs::home_dir() still
		// resolves on a real machine, so runtime_home() must keep working.
		assert!(runtime_home().is_absolute() || runtime_home() == PathBuf::from("."));
		assert_eq!(runtime_home_opt().is_some(), dirs::home_dir().is_some());
		assert!(hermes_home().is_absolute() || hermes_home() == PathBuf::from("."));
	}
}
