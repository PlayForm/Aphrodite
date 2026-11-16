//! One directory entry as a directive, if it is one.

use std::fs::DirEntry;

/// Read one directory entry as `(name, body)`, or nothing.
///
/// Only a `.md` file is a directive, and its name is the file stem: the stem is the
/// identifier the model addresses (`focus`) and the extension is the storage detail, so a
/// user's notes file in the same directory is skipped rather than becoming a directive
/// called `notes.txt`.
///
/// Every failure answers `None` - an unreadable file, a name that is not UTF-8, a missing
/// extension. That is the whole point of the `Option`: a directives directory is
/// user-owned, so one unusable file must not fail the load, and the caller's `filter_map`
/// says so without a branch.
pub(crate) fn Fn(Item:DirEntry) -> Option<(String, String)> {
	let path = Item.path();
	let Extension = path.extension().and_then(|Extension| Extension.to_str());

	let Some("md") = Extension else {
		return None;
	};

	let name = path.file_stem().and_then(|name| name.to_str())?;
	let Content = std::fs::read_to_string(&path).ok()?;

	Some((name.to_string(), Content))
}
