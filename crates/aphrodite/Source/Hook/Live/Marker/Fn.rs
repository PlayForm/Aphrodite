//! The instant live-container marker.

/// Render the instant live-container marker: the standard CCR header with
/// the `Live` type, then a single annotation line carrying the progress
/// handle - chunk count, total size, and bytes arrived so far (0 at emit
/// time; the retrieve path advances it). Sanitizes the preview the same
/// way `CCRMarker` does (newlines → spaces).
///
/// The annotation is the point of the whole feature: it tells the model the
/// content exists, how big it is, and that waiting is unnecessary - without
/// it a `Live` marker is indistinguishable from a compression the model
/// should resolve before continuing.
pub(crate) fn Fn(hash:&str, Total:usize, Chunks:usize, Preview:&str) -> String {
	let Safe = Preview.replace(['\n', '\r'], " ").trim().to_string();
	let ChunkNote = match Chunks == 1 {
		true => "1 chunk".to_string(),
		false => format!("{Chunks} chunks"),
	};
	format!(
		"<<<CCR:{}|live|{}>>>\nLive container: {} · {} · {} bytes · 0 bytes arrived. Use aphrodite_retrieve({}) to \
		 fetch when needed. Continue reasoning without waiting.",
		hash, Total, Safe, ChunkNote, Total, hash
	)
}
