//! The configuration file a watch is aimed at, resolved once.

use std::path::{Path, PathBuf};

/// A configuration file and the directory that holds it.
///
/// The watcher used to decide what it was looking at by asking whether an event's
/// path *contained* `aphrodite.toml`. That is a substring test standing in for an
/// identity: it matches `aphrodite.toml.swp`, `aphrodite.toml.bak` and a temp file
/// named `.aphrodite.toml.tmp`, and it matches nothing at all when the file the
/// process actually loaded is named something else. Resolving the target once, at
/// start, turns the question into "is this that file" - an equality between paths,
/// not a guess about a name.
///
/// `File` is absolute because `notify` reports absolute paths and because a relative
/// path would be re-resolved against whatever the process later does with its cwd.
/// It is canonicalised where that is possible: a symlinked directory is reported by
/// its real path, and comparing the two spellings would silently never match.
#[derive(Clone)]
pub(crate) struct Struct {
	/// The configuration file itself, absolute.
	pub File:PathBuf,
	/// The directory to watch - `notify` watches directories, not files.
	pub Directory:PathBuf,
}

impl Struct {
	/// Resolve `ConfigurationPath` into the file a watch is aimed at.
	///
	/// `canonicalize` answers the real path and fails when the file is gone;
	/// `absolute` then answers the lexical one, which is the honest fallback for a
	/// file that was deleted between the load and the watch. A target that resolves
	/// to neither keeps the operator's own spelling rather than inventing one.
	pub(crate) fn new(ConfigurationPath:&str) -> Self {
		let Raw = PathBuf::from(ConfigurationPath);

		let File = std::fs::canonicalize(&Raw)
			.or_else(|_| std::path::absolute(&Raw))
			.unwrap_or(Raw);

		let Directory = File.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));

		Self { File, Directory }
	}
}
