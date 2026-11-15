//! `~` expansion for env-var overrides.

use std::path::PathBuf;

/// Expand a leading `~`/`~/` (the Python shim expands env overrides with
/// `Path.expanduser()`; the Rust side must behave identically).
///
/// Only the two exact shapes are expanded - a bare `~` and a `~/` prefix. A
/// `~user/...` form is passed through untouched rather than guessed at,
/// because resolving another user's home is not something the shim does
/// either, and a wrong guess would point the runtime home at a directory the
/// process cannot write.
pub(crate) fn Fn(p:PathBuf) -> PathBuf {
	let s = p.to_string_lossy();
	if s == "~" {
		return dirs::home_dir().unwrap_or(p);
	}
	if let Some(Rest) = s.strip_prefix("~/")
		&& let Some(Home) = dirs::home_dir()
	{
		return Home.join(Rest);
	}
	p
}
