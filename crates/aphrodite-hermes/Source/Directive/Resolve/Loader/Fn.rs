//! The directive loader's exact-directory override.

use std::path::PathBuf;

/// `$APHRODITE_DIRECTIVES_DIR`, materialized into directly.
///
/// This is the core loader's own candidate 0 - an EXACT directory, not a home to
/// append `directives/` to - so a directive written anywhere else is invisible to
/// the loader that reads it.
pub(crate) fn Fn() -> Option<PathBuf> {
	std::env::var("APHRODITE_DIRECTIVES_DIR")
		.ok()
		.filter(|Dir| !Dir.trim().is_empty())
		.map(PathBuf::from)
}
