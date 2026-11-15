//! Where a dylib was found.

use std::path::PathBuf;

/// Which candidate supplied one dylib.
///
/// The three places setup looks used to be three nested `match` blocks with a
/// mutable `Found` flag between them, so the fallback order was implied by the
/// order the flags were cleared in. As a value the order is the order the
/// candidates are tried in `Locate::Fn`, and the copy step matches where the file
/// came from instead of re-testing flags.
pub(crate) enum Enum {
	/// Next to the setup binary, in `deps/`, or in a system library directory.
	Search(PathBuf),
	/// In the source checkout's `target/release`, for dev builds where the dylib
	/// is not co-located with the setup binary.
	Build(PathBuf),
	/// Nowhere on disk: it has to come from the release.
	Downloaded,
}
