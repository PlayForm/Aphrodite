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

	match s == "~" {
		true => {
			return dirs::home_dir().unwrap_or(p);
		},
		false => {},
	}

	match s.strip_prefix("~/") {
		Some(Rest) => {
			match dirs::home_dir() {
				Some(Home) => {
					return Home.join(Rest);
				},
				None => {},
			}
		},
		None => {},
	}

	p
}
