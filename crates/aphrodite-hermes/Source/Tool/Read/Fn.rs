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
pub(crate) fn Fn(path:&str) -> anyhow::Result<String> {
	let root = std::env::current_dir().context("resolve the current workspace directory")?;
	let root = root.canonicalize().unwrap_or(root);
	let canon = std::path::Path::new(path)
		.canonicalize()
		.with_context(|| format!("resolve the path {path}"))?;

	let Some(()) = canon.starts_with(&root).then_some(()) else {
		anyhow::bail!("path is outside the workspace ({}): {path}", root.display());
	};

	let size = std::fs::metadata(&canon).map(|m| m.len()).unwrap_or(0);

	let Some(()) = (size <= MAX_PATH_READ).then_some(()) else {
		anyhow::bail!("file exceeds {MAX_PATH_READ}-byte read cap: {path}");
	};

	std::fs::read_to_string(&canon).with_context(|| format!("read {path}"))
}
