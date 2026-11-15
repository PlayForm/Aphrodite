//! Refuse anything under `~/.ssh`.

use std::path::Path;

/// Refuse a path inside the user's SSH directory.
///
/// Compared against the CANONICAL path so `..` and a symlink cannot dodge the
/// prefix check, and against the home directory resolved the same way the bridge
/// does. `~/.ssh` is the one directory whose contents are keys rather than data.
pub(crate) fn Fn(Canon:&Path, path:&Path) -> Option<String> {
	dirs::home_dir()
		.map(|Home| Home.join(".ssh"))
		.filter(|SSH| Canon.starts_with(SSH))
		.map(|_| format!("refusing path under ~/.ssh: {path:?}"))
}
