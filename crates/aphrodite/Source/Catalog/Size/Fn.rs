//! Byte-count rendering shared by both catalog surfaces.

/// Render a byte count as `B`, `KB` or `MB`. Binary units (1024-based), one
/// decimal for megabytes and none for kilobytes - the same shape the catalog
/// table and the `total_saved_human` field have always reported, so a change
/// here changes what an agent reads in both places at once.
pub(crate) fn Fn(bytes:usize) -> String {
	match () {
		_ if bytes >= 1024 * 1024 => format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0)),
		_ if bytes >= 1024 => format!("{:.0}KB", bytes as f64 / 1024.0),
		_ => format!("{}B", bytes),
	}
}
