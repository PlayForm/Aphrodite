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
///
/// The filter is the whole rule: after stripping the `~`, only an empty remainder (the
/// bare form) or one that starts with `/` (the prefix form) is expanded, and a
/// `~user` remainder declines so the input is returned unchanged.
pub(crate) fn Fn(p:PathBuf) -> PathBuf {
	let s = p.to_string_lossy();

	let Expanded = s
		.strip_prefix('~')
		.filter(|Rest| Rest.is_empty() || Rest.starts_with('/'))
		.and_then(|Rest| dirs::home_dir().map(|Home| Home.join(Rest.strip_prefix('/').unwrap_or(Rest))));

	Expanded.unwrap_or(p)
}
