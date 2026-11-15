//! The file-name denylist.

use std::path::Path;

/// Refuse a credential file by its own name, wherever it lives.
///
/// Checked on the RAW path, before any resolution: a relative `.env` in the cwd
/// has to be caught without canonicalising it, and a path that does not exist yet
/// still must not be prefetched the moment it does.
pub(crate) fn Fn(path:&Path) -> Option<String> {
	let Name = path.file_name().and_then(|Name| Name.to_str())?;

	[".env", "auth.json"]
		.contains(&Name)
		.then(|| format!("refusing sensitive file name: {path:?}"))
}
