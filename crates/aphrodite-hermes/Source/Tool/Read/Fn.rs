//! The guarded path read itself.
//!
//! Three checks, in order, and all three matter: canonicalise (so `..` and
//! symlinks cannot escape), require the canonical path to start with the
//! canonical workspace root (so `/etc/passwd` and `~/.ssh/id_ed25519` are
//! refused), and cap the size (so a huge file cannot be pulled into context in
//! one call).

use anyhow::Context;

use super::Const::MAX_PATH_READ;

/// Read a file requested via `aphrodite_retrieve(path=...)`, confined to the
/// current workspace and capped at [`MAX_PATH_READ`]. Returns an error if
/// the path escapes the workspace, is too large, or can't be read - so the tool
/// can't be coerced into exfiltrating arbitrary files (e.g. /etc/passwd,
/// ~/.ssh).
///
/// The canonicalisation is what makes the confinement real: a textual prefix
/// check on the un-canonicalised path would accept `<workspace>/../../etc/hosts`
/// and a symlink out of the tree. It reads UTF-8 only - a binary file returns the
/// read error rather than lossy text, since lossy bytes are not the file.
///
/// The workspace refusal names both the resolved root and the requested path:
/// the tool renders the message into its `error` field, and "outside the
/// workspace" without the root leaves the reader unable to tell a wrong path
/// from a wrong working directory.
pub(crate) fn Fn(Path:&str) -> anyhow::Result<String> {
	let Root = std::env::current_dir().context("resolve the current workspace directory")?;
	let Root = Root.canonicalize().unwrap_or(Root);
	let Canon = std::path::Path::new(Path)
		.canonicalize()
		.with_context(|| format!("resolve the path {Path}"))?;

	anyhow::ensure!(
		Canon.starts_with(&Root),
		"path is outside the workspace ({}): {Path}",
		Root.display()
	);

	let Size = std::fs::metadata(&Canon).map(|Metadata| Metadata.len()).unwrap_or(0);

	anyhow::ensure!(Size <= MAX_PATH_READ, "file exceeds {MAX_PATH_READ}-byte read cap: {Path}");

	std::fs::read_to_string(&Canon).with_context(|| format!("read {Path}"))
}
