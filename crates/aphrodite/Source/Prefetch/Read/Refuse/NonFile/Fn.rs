//! Refuse an existing path that is not a file.

use std::path::Path;

/// Refuse a directory, socket, device or FIFO.
///
/// `read_to_string` on a FIFO blocks forever and on a device can return unbounded
/// data, so the refusal has to happen before the read rather than as a read error.
/// `exists()` is checked first so a path that does not exist is left to the
/// caller's "missing" answer instead of being refused as a non-file.
pub(crate) fn Fn(path:&Path) -> Option<String> {
	(path.exists() && !path.is_file()).then(|| format!("refusing non-file path: {path:?}"))
}
