//! Tilde expansion and absolutization for the refusal guard's prefix checks.
//!
//! The guard compares a path against `~/.ssh` and the Hermes home, so the
//! path has to be absolute first - a relative `.env` or a `~/...` string
//! would otherwise never match a prefix and slip through. Canonicalization is
//! attempted so `..` and symlinks cannot dodge the check, but it is not
//! required: a refusal like `~/.ssh/id_rsa` must hold before the file exists,
//! so the cwd-joined absolute form is the fallback rather than an error.

use std::path::{Path, PathBuf};

/// Absolute, `~`-expanded form of `path` for the guard's prefix checks:
/// canonicalized when the path resolves (so `..`/symlinks can't dodge the
/// checks), otherwise the cwd-joined absolute path (a refusal like
/// `~/.ssh/id_rsa` must hold even before the file exists). Tilde expansion
/// mirrors `Home`'s `ExpandTilde`.
pub(in crate::Prefetch) fn GuardAbsolute(path:&Path) -> PathBuf {
	let s = path.to_string_lossy();
	let Expanded:PathBuf = if s == "~" {
		dirs::home_dir().unwrap_or_else(|| path.to_path_buf())
	} else if let Some(Rest) = s.strip_prefix("~/")
		&& let Some(Home) = dirs::home_dir()
	{
		Home.join(Rest)
	} else {
		path.to_path_buf()
	};
	let Abs = if Expanded.is_absolute() {
		Expanded
	} else {
		std::env::current_dir().map(|c| c.join(&Expanded)).unwrap_or(Expanded)
	};
	Abs.canonicalize().unwrap_or(Abs)
}
