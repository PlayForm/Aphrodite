//! The home-level environment override.

use std::path::PathBuf;

/// `<APHRODITE_HOME>/directives/`, when `$APHRODITE_HOME` is set and not blank.
///
/// A HOME, unlike `Loader`'s exact directory: the `directives/` suffix is part of
/// this candidate's answer, not of the caller's argument.
pub(crate) fn Fn() -> Option<PathBuf> {
	std::env::var("APHRODITE_HOME")
		.ok()
		.filter(|Home| !Home.trim().is_empty())
		.map(|Home| PathBuf::from(Home).join("directives"))
}
