//! `<exe>/../../target/release/<name>` - where a dev build leaves the dylib.

use std::path::{Path, PathBuf};

/// The build-output location of one dylib, relative to the setup binary.
///
/// A dev build leaves `target/release/libaphrodite_hermes.dylib` two directories
/// above `target/release/aphrodite`, and the setup binary is run from there, so
/// this is the second place to look. It is a path computation only: whether
/// anything is there is the caller's question, and a missing directory simply
/// answers no.
pub(crate) fn Fn(ExeDirectory:&Path, Name:&str) -> PathBuf {
	ExeDirectory
		.parent()
		.unwrap_or(Path::new("."))
		.parent()
		.unwrap_or(Path::new("."))
		.join("target")
		.join("release")
		.join(Name)
}
